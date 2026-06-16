//! Top-level scanner state, `report*` helpers, and AST traversal for the
//! sonarjs port. Traversal uses the Oxc visitor so every node is reached; each
//! `check_*` rule body lives under [`crate::rules`].

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AccessorProperty, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget,
    BinaryExpression, BindingIdentifier, BindingPattern, BlockStatement, BreakStatement,
    CallExpression, CatchClause, Class, ConditionalExpression, ContinueStatement,
    DebuggerStatement, DoWhileStatement, ExportAllDeclaration, ExportNamedDeclaration, Expression,
    ExpressionStatement, ForInStatement, ForOfStatement, ForStatement, Function, FunctionBody,
    IdentifierReference, IfStatement, ImportDeclaration, ImportExpression, JSXAttribute,
    JSXAttributeValue, JSXElement, JSXFragment, JSXOpeningElement, LabeledStatement,
    LogicalExpression, NewExpression, ObjectExpression, ObjectProperty, ParenthesizedExpression,
    Program, PropertyDefinition, RegExpLiteral, ReturnStatement, SimpleAssignmentTarget, Statement,
    StaticBlock, StaticMemberExpression, StringLiteral, SwitchCase, SwitchStatement,
    TSIntersectionType, TSPropertySignature, TSTypeAliasDeclaration, TSUnionType,
    TaggedTemplateExpression, TemplateLiteral, ThisExpression, TryStatement, UnaryExpression,
    UpdateExpression, VariableDeclaration, VariableDeclarator, WhileStatement, YieldExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, Scoping, SymbolId};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::scope::ScopeFlags;
use oxlint_plugins_carton::SmallVec;

use crate::{Diagnostic, DiagnosticData, DiagnosticFix, LineIndex, SonarjsOptions};

/// Distinguishes loop frames from switch frames on the breakable stack, so
/// that the `too-many-break-or-continue-in-loop` rule can correctly decide
/// which jumps actually target a given loop.
pub(crate) enum BreakableKind {
    Loop,
    Switch,
}

/// One entry on the invariant-return stack, one per open function or arrow
/// scope. The `no-invariant-returns` rule flags scopes where every explicit
/// value return yields the same source text.
pub(crate) struct InvariantReturnFrame<'a> {
    /// Span of the enclosing function or arrow, used as the report location.
    pub(crate) span: Span,
    /// Source text of each `return <expr>;` argument seen so far.
    pub(crate) return_values: SmallVec<[&'a str; 4]>,
    /// Set to `true` when a bare `return;` is seen; a bare return exempts the
    /// scope from the invariant-return check.
    pub(crate) has_bare_return: bool,
}

/// One entry on the loop-counter stack, one per enclosing classic `for` loop
/// whose update clause names at least one resolvable counter symbol. The
/// `updated-loop-counter` rule flags writes to these symbols inside the loop
/// body.
pub(crate) struct LoopCounterFrame {
    /// Symbols modified by the loop's update clause (the counters).
    pub(crate) counters: SmallVec<[SymbolId; 2]>,
    /// Span of the update-clause expression, so that the counter's own update
    /// (`i++` in `for (…; i++)`) is excluded from the body-write check.
    pub(crate) update_span: Span,
}

/// One entry on the breakable stack, representing an open loop or switch.
pub(crate) struct BreakableFrame<'a> {
    pub(crate) kind: BreakableKind,
    /// Label attached to this loop or switch, if the statement was directly
    /// preceded by a labeled-statement wrapper.
    pub(crate) label: Option<&'a str>,
    /// Number of `break`/`continue` statements that target this frame.
    pub(crate) jump_count: u32,
    pub(crate) span: Span,
}

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    /// Path or name of the file being scanned. Used by rules that compare the
    /// filename to identifiers in the source, such as
    /// `file-name-differ-from-class`.
    pub(crate) filename: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: &'a SonarjsOptions,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
    /// Scoping information from semantic analysis, present only when a rule that
    /// needs reference resolution (`no-misleading-array-reverse`,
    /// `no-alphabetical-sort`) is enabled. Used to resolve an identifier use to
    /// its declaration symbol.
    pub(crate) scoping: Option<&'a Scoping>,
    /// AST nodes from semantic analysis, paired with `scoping`. Used to look up
    /// a symbol's declaration site (e.g. the `VariableDeclarator` initialiser).
    pub(crate) nodes: Option<&'a AstNodes<'a>>,
    /// Number of template literals currently open on the traversal stack.
    pub(crate) template_literal_depth: u32,
    /// Number of switch statements currently open on the traversal stack.
    pub(crate) switch_depth: u32,
    /// Number of conditional (ternary) expressions currently open on the traversal stack.
    pub(crate) conditional_depth: u32,
    /// Span start offsets of IfStatement nodes that are `else if` members of a
    /// chain already processed by their head; used by `no-identical-conditions`
    /// to avoid double-processing a chain.
    pub(crate) if_chain_seen: SmallVec<[u32; 16]>,
    /// Stack of boolean frames tracking whether each currently-open generator
    /// function has seen at least one `yield` expression. A frame is pushed on
    /// entry to a generator with a body and popped on exit; `generator-without-yield`
    /// reports generators whose frame is still `false` when popped.
    pub(crate) generator_yield_stack: SmallVec<[bool; 8]>,
    /// Stack of frames, one per currently-open function or arrow scope, tracking
    /// whether that scope has seen an explicit value `return x;` and an explicit
    /// bare `return;`. Each tuple is `(span, has_value_return, has_bare_return)`.
    /// Pushed on entry to a function/arrow and popped on exit;
    /// `no-inconsistent-returns` reports a scope whose frame has both kinds set.
    pub(crate) return_kind_stack: SmallVec<[(Span, bool, bool); 8]>,
    /// Stack of frames, one per currently-open function or arrow scope, collecting
    /// the source-text of every explicit value `return <expr>;` and recording
    /// whether a bare `return;` was seen. Pushed on entry and popped on exit;
    /// `no-invariant-returns` reports a scope where all collected return
    /// expressions are byte-identical (at least two, no bare return).
    pub(crate) invariant_return_stack: SmallVec<[InvariantReturnFrame<'a>; 8]>,
    /// Current nesting depth of control-flow statements (if/for/while/switch/try),
    /// used by `nested-control-flow`. `else if` branches do not add depth.
    pub(crate) control_flow_depth: u32,
    /// Span-start offsets of `if` statements that are the `else`-branch of a parent
    /// `if` (an `else if`); these do not increment the nesting depth.
    pub(crate) else_if_starts: SmallVec<[u32; 8]>,
    /// Comment spans for the current file, collected in `visit_program`.
    pub(crate) comment_spans: SmallVec<[Span; 16]>,
    /// JSX-tracking frames, one per open function/arrow; `max-lines-per-function`
    /// skips any function whose frame is `true`.
    pub(crate) jsx_function_stack: SmallVec<[bool; 8]>,
    /// Span-start offsets of functions/arrows that are IIFEs (callee of a call);
    /// `max-lines-per-function` never reports these.
    pub(crate) iife_function_starts: SmallVec<[u32; 8]>,
    /// (impl_text, start_line) of every qualifying function already visited, used
    /// by `no-identical-functions` to detect duplicate implementations. A function
    /// qualifies when its span spans at least 3 lines.
    pub(crate) seen_function_impls: SmallVec<[(&'a str, u32); 16]>,
    /// (value, span) of every string literal seen, for `no-duplicate-string`.
    pub(crate) string_literals: SmallVec<[(&'a str, Span); 32]>,
    /// Span-start offsets of string literals in excluded positions (import/export
    /// sources, JSX attribute values) that `no-duplicate-string` must skip.
    pub(crate) excluded_string_starts: SmallVec<[u32; 16]>,
    /// (source-text, span) of every union/intersection type seen, for
    /// `use-type-alias`, which flags composite types repeated identically.
    pub(crate) composite_types: SmallVec<[(&'a str, Span); 16]>,
    /// Per-function frame stack for `cyclomatic-complexity`. One frame per open
    /// function/arrow scope: `(function_span, accumulated_complexity)`. Decision
    /// points inside nested functions update only the innermost frame; top-level
    /// decision points find an empty stack and are silently ignored.
    pub(crate) cyclomatic_complexity_stack: SmallVec<[(Span, u32); 8]>,
    /// Per-scope stack for `expression-complexity` (S1067). One frame is pushed
    /// at program entry (for module-level expressions) and an additional frame on
    /// entry to every function or arrow-function scope so that nested function
    /// bodies are scored independently. Each frame stores
    /// `(nesting_level, operator_count, top_span)`:
    /// - `nesting_level`: how deep inside the current logical/conditional chain
    ///   we currently are; 0 means we are between top-level expressions.
    /// - `operator_count`: total `&&`/`||`/`??`/`?:` operators accumulated since
    ///   the current top-level expression began.
    /// - `top_span`: span of the outermost operator in the current chain, used
    ///   as the report location when the count exceeds the threshold.
    pub(crate) expression_complexity_stack: SmallVec<[(u32, u32, Span); 8]>,
    /// Current nesting depth of function/arrow definitions, used by
    /// `no-nested-functions`. Incremented on entry to any function-like node and
    /// decremented on exit. Depth 1 = outermost function in the file.
    pub(crate) function_nesting_depth: u32,
    /// Number of `this`-rebinding scopes currently open on the traversal stack,
    /// used by `no-global-this`. Incremented on entry to a regular (non-arrow)
    /// function, class field/property initializer, or class static block.
    /// A `ThisExpression` encountered when this depth is zero refers to the
    /// global object and is flagged.
    pub(crate) this_binding_depth: u32,
    /// Stack of open breakable contexts (loops and switch statements), used by
    /// `too-many-break-or-continue-in-loop` to count jumps that target each
    /// loop. One frame is pushed on entry to each loop or switch and popped on
    /// exit; break/continue handlers update the innermost matching frame.
    pub(crate) breakable_stack: SmallVec<[BreakableFrame<'a>; 8]>,
    /// Holds the label name while we are inside a `LabeledStatement` whose
    /// body is directly a loop or switch statement, so that the loop/switch
    /// visitor can attach the label to the newly-pushed frame. Consumed (via
    /// `take`) by the loop/switch visitor and reset to `None` defensively
    /// after each `walk_labeled_statement` completes.
    pub(crate) pending_loop_label: Option<&'a str>,
    /// Stack of loop-counter frames, one per currently-open classic `for` loop
    /// whose update clause names a resolvable counter. Pushed on entry and
    /// popped on exit; `updated-loop-counter` checks every assignment/update
    /// target against the counters of all active frames.
    pub(crate) loop_counter_symbols: SmallVec<[LoopCounterFrame; 4]>,
    /// Per-function-scope stack of loop-nesting depths, used by
    /// `function-inside-loop`. One frame per open function/arrow scope plus a
    /// base frame for module/top-level scope. Entering a loop increments the top
    /// frame; entering a function pushes a fresh `0` frame, so loop context is
    /// reset at every function boundary. A function entered while the top frame
    /// is `> 0` is defined inside a loop.
    pub(crate) loop_depth_in_function: SmallVec<[u32; 8]>,
    /// Stack of span values for every currently-open function or arrow scope,
    /// pushed on entry and popped on exit. Used by
    /// `no-variable-usage-before-declaration` to determine whether a reference
    /// and its variable declarator are in the same enclosing function (same
    /// depth in the stack), or whether the reference is inside a nested
    /// function/arrow (deeper depth) and therefore a safe closure.
    pub(crate) fn_span_stack: SmallVec<[Span; 8]>,
    /// Records of `(symbol_id, was_called, was_newed, first_site_span)` for
    /// every identifier callee resolved to a locally-declared function that has
    /// been used in a call expression or a `new` expression. Populated during
    /// `visit_call_expression` / `visit_new_expression`; consumed in
    /// `finalize_inconsistent_function_call` after the walk completes.
    pub(crate) fn_call_new_records: SmallVec<[(SymbolId, bool, bool, Span); 16]>,
    /// Set to `true` as soon as a call expression whose callee is `it` or
    /// `test` (or a `.only`/`.skip`/`.each` variant) is encountered during
    /// the AST walk. Consumed by `finalize_no_empty_test_file` to decide
    /// whether a test file contains any test cases.
    pub(crate) saw_test_call: bool,
    /// Symbol IDs of function/class declarations that are immediately preceded
    /// by a block comment containing `@deprecated`. Populated during the walk
    /// (in `check_deprecation_function` / `check_deprecation_class`) and
    /// consulted by `check_deprecation_reference` to flag calls to those
    /// symbols.
    pub(crate) deprecated_symbols: SmallVec<[SymbolId; 8]>,
    /// Depth of currently-open function scopes during the OXC visitor walk,
    /// used by `cognitive-complexity`. When depth > 0 the visitor is inside a
    /// function whose body was already scored recursively by the outer
    /// function's standalone `score_function_body` call, so the inner function
    /// must not be scored again.
    pub(crate) cognitive_complexity_fn_depth: u32,
}

impl<'a> Scanner<'a> {
    pub(crate) fn text(&self, span: Span) -> &'a str {
        &self.source_text[span.start as usize..span.end as usize]
    }

    /// Conservatively resolves an identifier use to its declaration's
    /// initializer expression, or `None` when ambiguous/unsupported.
    ///
    /// Returns `None` unless semantic analysis ran, the reference resolves to a
    /// single never-reassigned symbol, and that symbol is declared by a simple
    /// `let`/`const`/`var` binding identifier with an initializer. The mutation
    /// guard ensures the initializer still reflects the symbol's value at the
    /// use site, so callers can treat the returned expression as authoritative.
    pub(crate) fn resolve_identifier_initializer(
        &self,
        ident: &IdentifierReference<'a>,
    ) -> Option<&'a Expression<'a>> {
        let scoping = self.scoping?;
        let nodes = self.nodes?;
        let reference_id = ident.reference_id.get()?;
        let symbol_id = scoping.get_reference(reference_id).symbol_id()?;
        if scoping.symbol_is_mutated(symbol_id) {
            return None;
        }
        let decl = scoping.symbol_declaration(symbol_id);
        let AstKind::VariableDeclarator(declarator) = nodes.get_node(decl).kind() else {
            return None;
        };
        if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
            return None;
        }
        declarator.init.as_ref()
    }
}

impl Scanner<'_> {
    pub(crate) fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span, None);
    }

    pub(crate) fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
        fix: Option<DiagnosticFix>,
    ) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix,
        });
    }
}

impl<'a> Visit<'a> for Scanner<'a> {
    fn visit_program(&mut self, it: &Program<'a>) {
        for comment in &it.comments {
            self.comment_spans.push(comment.span);
        }
        self.check_no_tab();
        self.check_max_lines();
        self.check_fixme_tag(&it.comments);
        self.check_todo_tag(&it.comments);
        self.check_no_sonar_comments(&it.comments);
        self.check_no_commented_code(&it.comments);
        self.check_declarations_in_global_scope_program(it);
        self.check_no_same_line_conditional(&it.body);
        self.check_no_unenclosed_multiline_block(&it.body);
        self.check_prefer_object_literal(&it.body);
        self.check_destructuring_assignment_syntax(&it.body);
        self.check_no_element_overwrite(&it.body);
        self.check_no_redundant_assignments(&it.body);
        self.enter_expression_complexity_scope();
        walk::walk_program(self, it);
        self.leave_expression_complexity_scope();
        self.finalize_no_duplicate_string();
        self.finalize_use_type_alias();
        self.finalize_inconsistent_function_call();
        self.check_file_name_differ_from_class(it);
        self.finalize_no_empty_test_file(it);
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.check_no_function_declaration_in_block(it);
        self.check_no_same_line_conditional(&it.body);
        self.check_no_unenclosed_multiline_block(&it.body);
        self.check_prefer_object_literal(&it.body);
        self.check_destructuring_assignment_syntax(&it.body);
        self.check_no_element_overwrite(&it.body);
        self.check_no_redundant_assignments(&it.body);
        walk::walk_block_statement(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.check_class_name(it);
        self.check_function_name_class(it);
        self.check_deprecation_class(it);
        self.check_no_async_constructor(it);
        walk::walk_class(self, it);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.check_no_nested_template_literals(it);
        self.template_literal_depth += 1;
        walk::walk_template_literal(self, it);
        self.template_literal_depth -= 1;
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.check_no_nested_switch(it);
        self.check_no_all_duplicated_branches_switch(it);
        self.check_no_duplicated_branches_switch(it);
        self.check_max_switch_cases(it);
        self.check_no_case_label_in_switch(it);
        self.check_no_fallthrough(it);
        self.check_no_small_switch(it);
        self.check_prefer_default_last(it);
        self.switch_depth += 1;
        let counted = self.enter_nested_control_flow(it.span);
        let sw_label = self.pending_loop_label.take();
        self.enter_breakable_switch(it.span, sw_label);
        walk::walk_switch_statement(self, it);
        self.leave_breakable_switch();
        self.leave_nested_control_flow(counted);
        self.switch_depth -= 1;
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        self.check_comma_or_logical_or_case(it);
        self.check_no_same_line_conditional(&it.consequent);
        self.check_no_unenclosed_multiline_block(&it.consequent);
        self.check_prefer_object_literal(&it.consequent);
        self.check_destructuring_assignment_syntax(&it.consequent);
        self.check_no_element_overwrite(&it.consequent);
        self.check_no_redundant_assignments(&it.consequent);
        if it.test.is_some() {
            self.add_cyclomatic_complexity();
        }
        walk::walk_switch_case(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.check_no_redundant_boolean_binary(it);
        self.check_no_identical_expressions_binary(it);
        self.check_no_collection_size_mischeck(it);
        self.check_index_of_compare_to_positive_number(it);
        self.check_bitwise_operators(it);
        self.check_no_in_misuse(it);
        self.check_in_operator_type_error(it);
        self.check_different_types_comparison(it);
        self.check_operation_returning_nan(it);
        walk::walk_binary_expression(self, it);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.check_no_identical_expressions_logical(it);
        self.check_jsx_no_leaked_render(it);
        self.add_cyclomatic_complexity();
        self.enter_expression_complexity_op(it.span);
        walk::walk_logical_expression(self, it);
        self.leave_expression_complexity_op();
    }

    fn visit_parenthesized_expression(&mut self, it: &ParenthesizedExpression<'a>) {
        self.check_redundant_parentheses(it);
        walk::walk_parenthesized_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        self.check_no_redundant_boolean_unary(it);
        self.check_no_delete_var(it);
        self.check_no_inverted_boolean_check(it);
        self.check_void_use(it);
        self.check_no_array_delete(it);
        walk::walk_unary_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.check_no_nested_conditional(it);
        self.check_no_redundant_boolean_conditional(it);
        self.add_cyclomatic_complexity();
        self.enter_expression_complexity_op(it.span);
        self.conditional_depth += 1;
        walk::walk_conditional_expression(self, it);
        self.conditional_depth -= 1;
        self.leave_expression_complexity_op();
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.check_no_collapsible_if(it);
        self.check_no_identical_conditions(it);
        self.check_no_all_duplicated_branches_if(it);
        self.check_no_duplicated_branches_if(it);
        self.check_elseif_without_else(it);
        self.check_prefer_single_boolean_return(it);
        self.check_no_nested_assignment_condition(&it.test);
        self.add_cyclomatic_complexity();
        let counted = self.enter_nested_control_flow_if(it);
        walk::walk_if_statement(self, it);
        self.leave_nested_control_flow(counted);
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.check_for_in(it);
        self.check_no_for_in_iterable(it);
        self.check_updated_const_var_for_left(&it.left);
        self.check_redundant_continue(&it.body);
        self.add_cyclomatic_complexity();
        let label = self.pending_loop_label.take();
        self.enter_breakable_loop(it.span, label);
        let counted = self.enter_nested_control_flow(it.span);
        self.enter_loop_depth();
        walk::walk_for_in_statement(self, it);
        self.leave_loop_depth();
        self.leave_nested_control_flow(counted);
        self.leave_breakable_loop();
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.check_prefer_while(it);
        self.check_for_loop_increment_sign(it);
        self.check_misplaced_loop_counter(it);
        self.check_no_equals_in_for_termination(it);
        self.check_redundant_continue(&it.body);
        if let Some(test) = &it.test {
            self.check_no_nested_assignment_condition(test);
        }
        self.add_cyclomatic_complexity();
        let label = self.pending_loop_label.take();
        self.enter_breakable_loop(it.span, label);
        let counted = self.enter_nested_control_flow(it.span);
        let counter_frame = self.enter_updated_loop_counter(it);
        self.enter_loop_depth();
        walk::walk_for_statement(self, it);
        self.leave_loop_depth();
        self.leave_updated_loop_counter(counter_frame);
        self.leave_nested_control_flow(counted);
        self.leave_breakable_loop();
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.check_redundant_continue(&it.body);
        self.check_no_nested_assignment_condition(&it.test);
        self.add_cyclomatic_complexity();
        let label = self.pending_loop_label.take();
        self.enter_breakable_loop(it.span, label);
        let counted = self.enter_nested_control_flow(it.span);
        self.enter_loop_depth();
        walk::walk_while_statement(self, it);
        self.leave_loop_depth();
        self.leave_nested_control_flow(counted);
        self.leave_breakable_loop();
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.check_redundant_continue(&it.body);
        self.check_no_nested_assignment_condition(&it.test);
        self.add_cyclomatic_complexity();
        let label = self.pending_loop_label.take();
        self.enter_breakable_loop(it.span, label);
        let counted = self.enter_nested_control_flow(it.span);
        self.enter_loop_depth();
        walk::walk_do_while_statement(self, it);
        self.leave_loop_depth();
        self.leave_nested_control_flow(counted);
        self.leave_breakable_loop();
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.check_updated_const_var_for_left(&it.left);
        self.check_redundant_continue(&it.body);
        self.add_cyclomatic_complexity();
        let label = self.pending_loop_label.take();
        self.enter_breakable_loop(it.span, label);
        let counted = self.enter_nested_control_flow(it.span);
        self.enter_loop_depth();
        walk::walk_for_of_statement(self, it);
        self.leave_loop_depth();
        self.leave_nested_control_flow(counted);
        self.leave_breakable_loop();
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.check_no_built_in_override_binding(it);
        walk::walk_binding_identifier(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.check_non_existent_operator(it);
        self.check_no_built_in_override_assignment(it);
        self.check_class_prototype(it);
        self.check_no_nested_assignment_chain(it);
        self.check_no_useless_increment(it);
        self.check_no_associative_arrays(it);
        self.check_no_undefined_assignment(it);
        self.check_no_hardcoded_passwords_assignment(it);
        self.check_no_hardcoded_secrets_assignment(it);
        self.check_updated_const_var_assignment(&it.left);
        if matches!(it.operator, AssignmentOperator::Assign) {
            self.check_no_misleading_array_reverse(&it.right);
        }
        if let AssignmentTarget::AssignmentTargetIdentifier(ident) = &it.left {
            self.check_no_parameter_reassignment_assignment(ident, it.span);
            self.check_updated_loop_counter(ident, it.span);
        }
        self.check_no_use_of_empty_return_value_assign(it);
        self.check_disabled_auto_escaping_assignment(it);
        self.check_content_length_assignment(it);
        self.check_cookies_assignment(it);
        walk::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) = &it.argument {
            self.check_no_parameter_reassignment_update(ident, it.span);
            self.check_updated_loop_counter(ident, it.span);
        }
        self.check_updated_const_var_update(&it.argument);
        walk::walk_update_expression(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if self.function_nesting_depth == 0 && self.this_binding_depth == 0 {
            self.check_declarations_in_global_scope_var(it);
        }
        walk::walk_variable_declaration(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            self.check_no_misleading_array_reverse(init);
        }
        self.check_no_hardcoded_passwords_declarator(it);
        self.check_no_hardcoded_secrets_declarator(it);
        self.check_no_use_of_empty_return_value_var(it);
        self.check_block_scoped_var(it);
        self.check_function_name_variable(it);
        self.check_unused_collection(it);
        self.check_empty_collection(it);
        walk::walk_variable_declarator(self, it);
    }

    fn visit_ts_union_type(&mut self, it: &TSUnionType<'a>) {
        self.check_no_duplicate_in_composite(&it.types);
        self.check_max_union_size(it);
        self.record_composite_type(it.span);
        walk::walk_ts_union_type(self, it);
    }

    fn visit_ts_intersection_type(&mut self, it: &TSIntersectionType<'a>) {
        self.check_no_duplicate_in_composite(&it.types);
        self.check_no_useless_intersection(it);
        self.record_composite_type(it.span);
        walk::walk_ts_intersection_type(self, it);
    }

    fn visit_ts_property_signature(&mut self, it: &TSPropertySignature<'a>) {
        self.check_no_redundant_optional(it);
        walk::walk_ts_property_signature(self, it);
    }

    fn visit_ts_type_alias_declaration(&mut self, it: &TSTypeAliasDeclaration<'a>) {
        self.check_redundant_type_aliases(it);
        walk::walk_ts_type_alias_declaration(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.check_arguments_usage(it);
        self.check_no_variable_usage_before_declaration(it);
        self.check_deprecation_reference(it);
        walk::walk_identifier_reference(self, it);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        // Non-parsing checks: these read `lit.regex.pattern.text`/flags directly
        // and never parse the regex AST, so they are called as before.
        self.check_no_empty_character_class(it);
        self.check_unicode_aware_regex(it);
        self.check_regular_expr_literal(it);

        // Parsing checks: parse the literal exactly ONCE and share the resulting
        // `&Pattern` with every check that needs the regex AST. On a parse error
        // `with_parsed_regex_literal` runs nothing (closure returns `()`),
        // preserving the previous "report nothing on parse error" behavior for
        // each of these checks.
        let source_text = self.source_text; // &'a str is Copy; avoids a borrow conflict.
        crate::regex_ast::with_parsed_regex_literal(it, source_text, |pattern| {
            self.check_no_empty_group_with_pattern(it, pattern);
            self.check_no_empty_alternatives_with_pattern(it, pattern);
            self.check_no_empty_after_reluctant_with_pattern(it, pattern);
            self.check_no_regex_spaces_with_pattern(it, pattern);
            self.check_no_control_regex_with_pattern(it, pattern);
            self.check_single_char_in_character_classes_with_pattern(it, pattern);
            self.check_duplicates_in_character_class_with_pattern(it, pattern);
            self.check_concise_regex_with_pattern(it, pattern);
            self.check_anchor_precedence_with_pattern(it, pattern);
            self.check_single_character_alternation_with_pattern(it, pattern);
            self.check_empty_string_repetition_with_pattern(it, pattern);
            self.check_no_misleading_character_class_with_pattern(it, pattern);
            self.check_slow_regex_with_pattern(it, pattern);
        });

        walk::walk_reg_exp_literal(self, it);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.record_string_literal(it);
        self.check_no_clear_text_protocols(it);
        self.check_no_hardcoded_ip(it);
        self.check_publicly_writable_directories_string(it);
        walk::walk_string_literal(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.check_no_exclusive_tests(it);
        self.check_no_skipped_tests_member(it);
        self.check_process_argv(it);
        self.check_standard_input(it);
        self.check_publicly_writable_directories_member(it);
        self.check_chai_determinate_assertion_member(it);
        walk::walk_static_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.check_no_skipped_tests_call(it);
        self.check_no_code_after_done(it);
        self.check_no_empty_test_file_call(it);
        self.check_array_callback_without_return(it);
        self.check_array_constructor_call(it);
        self.check_no_nested_incdec_call(it);
        self.check_no_invalid_regexp_call(it);
        self.check_no_undefined_argument_call(it);
        self.check_code_eval_call(it);
        self.check_pseudo_random(it);
        self.check_post_message(it);
        self.check_no_angular_bypass_sanitization(it);
        self.check_aws_restricted_ip_admin_access(it);
        self.check_csrf(it);
        self.check_cors(it);
        self.check_file_permissions(it);
        self.check_file_uploads(it);
        self.check_dns_prefetching(it);
        self.check_web_sql_database(it);
        self.check_no_intrusive_permissions(it);
        self.check_no_unsafe_unzip(it);
        self.check_hashing(it);
        self.check_no_weak_cipher(it);
        self.check_no_weak_keys(it);
        self.check_strict_transport_security(it);
        self.check_os_command(it);
        self.check_no_os_command_from_path(it);
        self.check_encryption_secure_mode(it);
        self.check_no_same_argument_assert(it);
        self.check_inverted_assertion_arguments(it);
        self.check_chai_determinate_assertion_call(it);
        self.check_no_alphabetical_sort(it);
        self.check_reduce_initial_value(it);
        self.check_no_literal_call(it);
        self.check_call_argument_line(it);
        self.check_no_require_or_define(it);
        self.check_no_extra_arguments(it);
        self.check_arguments_order(it);
        self.check_prefer_regexp_exec(it);
        self.check_disabled_timeout(it);
        self.check_argument_type(it);
        self.record_iife_callee(&it.callee);
        self.record_call_inconsistent_function_call(it);
        self.check_cookies_call(it);
        self.check_xpath(it);
        self.check_sockets_call(it);
        self.check_existing_groups(it);
        self.check_encryption(it);
        self.check_regular_expr_call(it);
        walk::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.check_no_literal_tagged_template(it);
        walk::walk_tagged_template_expression(self, it);
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.check_no_labels(it);
        self.check_label_position(it);
        // If the body is directly a loop or switch, hand the label off so the
        // loop/switch visitor can attach it to the breakable-stack frame.
        match &it.body {
            Statement::ForStatement(_)
            | Statement::ForInStatement(_)
            | Statement::ForOfStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::SwitchStatement(_) => {
                self.pending_loop_label = Some(it.label.name.as_str());
            }
            _ => {}
        }
        walk::walk_labeled_statement(self, it);
        // Defensive clear in case the loop/switch visitor was not reached.
        self.pending_loop_label = None;
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.check_constructor_for_side_effects(it);
        self.check_no_unthrown_error(it);
        self.check_no_ignored_return(it);
        self.check_no_incomplete_assertions(it);
        walk::walk_expression_statement(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.check_new_operator_misuse(it);
        self.check_aws_iam_public_access(it);
        self.check_aws_ec2_unencrypted_ebs_volume(it);
        self.check_aws_efs_unencrypted(it);
        self.check_aws_sagemaker_unencrypted_notebook(it);
        self.check_aws_sns_unencrypted_topics(it);
        self.check_confidential_information_logging(it);
        self.check_no_invalid_regexp_new(it);
        self.check_no_primitive_wrappers(it);
        self.check_array_constructor_new(it);
        self.check_no_nested_incdec_new(it);
        self.check_no_undefined_argument_new(it);
        self.check_code_eval_new(it);
        self.check_prefer_promise_shorthand(it);
        self.record_new_inconsistent_function_call(it);
        self.check_sockets_new(it);
        self.check_regular_expr_new(it);
        walk::walk_new_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.check_function_name_declaration(it);
        self.check_deprecation_function(it);
        if let Some(body) = &it.body {
            self.check_no_identical_functions(it.params.span.start, body.span.end, it.span);
            // Only concrete functions (those with a body) can carry a default
            // value; overload/abstract/ambient signatures (`body == None`) and
            // type-only signature nodes must not be flagged.
            self.check_bool_param_default(&it.params);
            self.check_no_hook_setter_in_body(&body.statements);
            self.check_no_useless_react_setstate(&body.statements);
        }
        self.check_cognitive_complexity_fn(it);
        let track = self.enter_generator(it);
        self.enter_return_scope(it.span);
        self.enter_invariant_return_scope(it.span);
        self.jsx_function_stack.push(false);
        self.enter_cyclomatic_scope(it.span);
        self.enter_expression_complexity_scope();
        self.enter_nested_function(it.span);
        self.enter_function_inside_loop(it.span);
        self.enter_this_binding_scope();
        self.check_no_unused_function_argument_fn(it);
        self.fn_span_stack.push(it.span);
        self.cognitive_complexity_fn_depth += 1;
        walk::walk_function(self, it, flags);
        self.cognitive_complexity_fn_depth -= 1;
        self.fn_span_stack.pop();
        self.leave_this_binding_scope();
        self.leave_function_inside_loop();
        self.leave_nested_function();
        self.leave_expression_complexity_scope();
        self.leave_cyclomatic_scope();
        self.leave_return_scope();
        self.leave_invariant_return_scope();
        self.leave_generator(it, track);
        self.check_max_lines_per_function(it.span);
    }

    fn visit_this_expression(&mut self, it: &ThisExpression) {
        self.check_global_this(it.span);
        walk::walk_this_expression(self, it);
    }

    fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
        self.check_shorthand_property_grouping(it);
        self.check_insecure_cookie(it);
        walk::walk_object_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        self.check_no_hardcoded_passwords_object_property(it);
        self.check_no_hardcoded_secrets_object_property(it);
        self.check_function_name_object_property(it);
        self.check_cookie_no_httponly_object_property(it);
        self.check_xml_parser_xxe_object_property(it);
        self.check_unverified_certificate_object_property(it);
        self.check_content_length_object_property(it);
        self.check_content_security_policy_object_property(it);
        self.check_no_mime_sniff_object_property(it);
        self.check_certificate_transparency_object_property(it);
        self.check_disabled_auto_escaping_object_property(it);
        self.check_aws_s3_bucket_granted_access_object_property(it);
        self.check_aws_rds_unencrypted_databases_object_property(it);
        self.check_hidden_files_object_property(it);
        self.check_aws_sqs_unencrypted_queue_object_property(it);
        self.check_aws_apigateway_public_api_object_property(it);
        self.check_aws_iam_all_privileges_object_property(it);
        self.check_aws_s3_bucket_versioning_object_property(it);
        self.check_aws_ec2_rds_dms_public_object_property(it);
        self.check_aws_s3_bucket_public_access_object_property(it);
        self.check_aws_iam_all_resources_accessible_object_property(it);
        self.check_no_ip_forward_object_property(it);
        self.check_insecure_jwt_token_object_property(it);
        self.check_no_referrer_policy_object_property(it);
        self.check_weak_ssl_object_property(it);
        self.check_unverified_hostname_object_property(it);
        self.check_frame_ancestors_object_property(it);
        self.check_no_vue_bypass_sanitization_object_property(it);
        self.check_aws_s3_bucket_insecure_http_object_property(it);
        self.check_aws_s3_bucket_server_encryption_object_property(it);
        self.check_aws_opensearchservice_domain_object_property(it);
        walk::walk_object_property(self, it);
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.check_public_static_readonly(it);
        self.enter_this_binding_scope();
        walk::walk_property_definition(self, it);
        self.leave_this_binding_scope();
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.enter_this_binding_scope();
        walk::walk_static_block(self, it);
        self.leave_this_binding_scope();
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        self.enter_this_binding_scope();
        walk::walk_accessor_property(self, it);
        self.leave_this_binding_scope();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if !it.expression {
            self.check_no_identical_functions(it.params.span.start, it.body.span.end, it.span);
            self.check_no_hook_setter_in_body(&it.body.statements);
            self.check_no_useless_react_setstate(&it.body.statements);
        }
        // Arrow functions always have a body (block or expression), so their
        // parameters can always carry a default value.
        self.check_bool_param_default(&it.params);
        self.check_cognitive_complexity_arrow(it);
        self.enter_return_scope(it.span);
        if !it.expression {
            self.enter_invariant_return_scope(it.span);
        }
        self.jsx_function_stack.push(false);
        self.enter_cyclomatic_scope(it.span);
        self.enter_expression_complexity_scope();
        self.enter_nested_function(it.span);
        self.enter_function_inside_loop(it.span);
        self.check_no_unused_function_argument_arrow(it);
        self.fn_span_stack.push(it.span);
        self.cognitive_complexity_fn_depth += 1;
        walk::walk_arrow_function_expression(self, it);
        self.cognitive_complexity_fn_depth -= 1;
        self.fn_span_stack.pop();
        self.leave_function_inside_loop();
        self.leave_nested_function();
        self.leave_expression_complexity_scope();
        self.leave_cyclomatic_scope();
        self.leave_return_scope();
        if !it.expression {
            self.leave_invariant_return_scope();
        }
        self.check_max_lines_per_function(it.span);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.record_return(it.argument.is_some());
        let value_text = it.argument.as_ref().map(|arg| self.text(arg.span()));
        self.record_invariant_return(value_text);
        self.check_no_use_of_empty_return_value_return(it);
        walk::walk_return_statement(self, it);
    }

    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        let label = it.label.as_ref().map(|l| l.name.as_str());
        self.handle_break_jump(label);
        walk::walk_break_statement(self, it);
    }

    fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
        let label = it.label.as_ref().map(|l| l.name.as_str());
        self.handle_continue_jump(label);
        walk::walk_continue_statement(self, it);
    }

    fn visit_debugger_statement(&mut self, it: &DebuggerStatement) {
        self.check_production_debug(it);
        walk::walk_debugger_statement(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        self.mark_generator_yield();
        walk::walk_yield_expression(self, it);
    }

    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        self.mark_jsx();
        self.check_object_alt_content(it);
        walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_opening_element(&mut self, it: &JSXOpeningElement<'a>) {
        self.check_link_with_target_blank(it);
        self.check_no_table_as_layout(it);
        walk::walk_jsx_opening_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
        self.mark_jsx();
        walk::walk_jsx_fragment(self, it);
    }

    fn visit_jsx_attribute(&mut self, it: &JSXAttribute<'a>) {
        if let Some(JSXAttributeValue::StringLiteral(lit)) = &it.value {
            self.exclude_string(lit);
        }
        self.check_no_uniq_key_jsx_attribute(it);
        self.check_no_vue_bypass_sanitization_jsx_attribute(it);
        walk::walk_jsx_attribute(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.exclude_string(&it.source);
        self.check_no_wildcard_import(it);
        self.check_unused_import(it);
        walk::walk_import_declaration(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &it.source {
            self.exclude_string(source);
        }
        walk::walk_export_named_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.exclude_string(&it.source);
        walk::walk_export_all_declaration(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        self.exclude_string_expression(&it.source);
        walk::walk_import_expression(self, it);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        self.check_no_useless_catch(it);
        self.check_no_ignored_exceptions(it);
        self.add_cyclomatic_complexity();
        walk::walk_catch_clause(self, it);
    }

    fn visit_try_statement(&mut self, it: &TryStatement<'a>) {
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_try_statement(self, it);
        self.leave_nested_control_flow(counted);
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.check_prefer_immediate_return(it);
        self.check_redundant_return(it);
        self.check_no_same_line_conditional(&it.statements);
        self.check_no_unenclosed_multiline_block(&it.statements);
        self.check_prefer_object_literal(&it.statements);
        self.check_destructuring_assignment_syntax(&it.statements);
        self.check_no_element_overwrite(&it.statements);
        self.check_no_redundant_assignments(&it.statements);
        walk::walk_function_body(self, it);
    }
}
