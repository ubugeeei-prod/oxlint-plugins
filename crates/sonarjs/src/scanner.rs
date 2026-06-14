//! Top-level scanner state, `report*` helpers, and AST traversal for the
//! sonarjs port. Traversal uses the Oxc visitor so every node is reached; each
//! `check_*` rule body lives under [`crate::rules`].

use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, BinaryExpression, BindingIdentifier,
    BlockStatement, CallExpression, CatchClause, Class, ConditionalExpression, DoWhileStatement,
    ExportAllDeclaration, ExportNamedDeclaration, ExpressionStatement, ForInStatement,
    ForOfStatement, ForStatement, Function, FunctionBody, IdentifierReference, IfStatement,
    ImportDeclaration, ImportExpression, JSXAttribute, JSXAttributeValue, JSXElement, JSXFragment,
    LabeledStatement, LogicalExpression, NewExpression, Program, RegExpLiteral, ReturnStatement,
    StaticMemberExpression, StringLiteral, SwitchCase, SwitchStatement, TSIntersectionType,
    TSPropertySignature, TSUnionType, TemplateLiteral, TryStatement, UnaryExpression,
    WhileStatement, YieldExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxlint_plugins_carton::SmallVec;

use crate::{Diagnostic, DiagnosticData, DiagnosticFix, LineIndex, SonarjsOptions};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: &'a SonarjsOptions,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
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
    /// (value, span) of every string literal seen, for `no-duplicate-string`.
    pub(crate) string_literals: SmallVec<[(&'a str, Span); 32]>,
    /// Span-start offsets of string literals in excluded positions (import/export
    /// sources, JSX attribute values) that `no-duplicate-string` must skip.
    pub(crate) excluded_string_starts: SmallVec<[u32; 16]>,
    /// Per-function frame stack for `cyclomatic-complexity`. One frame per open
    /// function/arrow scope: `(function_span, accumulated_complexity)`. Decision
    /// points inside nested functions update only the innermost frame; top-level
    /// decision points find an empty stack and are silently ignored.
    pub(crate) cyclomatic_complexity_stack: SmallVec<[(Span, u32); 8]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn text(&self, span: Span) -> &'a str {
        &self.source_text[span.start as usize..span.end as usize]
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
        self.check_no_same_line_conditional(&it.body);
        walk::walk_program(self, it);
        self.finalize_no_duplicate_string();
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        self.check_no_function_declaration_in_block(it);
        self.check_no_same_line_conditional(&it.body);
        walk::walk_block_statement(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.check_class_name(it);
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
        self.check_max_switch_cases(it);
        self.check_no_case_label_in_switch(it);
        self.check_no_small_switch(it);
        self.check_prefer_default_last(it);
        self.switch_depth += 1;
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_switch_statement(self, it);
        self.leave_nested_control_flow(counted);
        self.switch_depth -= 1;
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        self.check_comma_or_logical_or_case(it);
        self.check_no_same_line_conditional(&it.consequent);
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
        walk::walk_binary_expression(self, it);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.check_no_identical_expressions_logical(it);
        self.add_cyclomatic_complexity();
        walk::walk_logical_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        self.check_no_redundant_boolean_unary(it);
        self.check_no_delete_var(it);
        self.check_no_inverted_boolean_check(it);
        walk::walk_unary_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.check_no_nested_conditional(it);
        self.check_no_redundant_boolean_conditional(it);
        self.add_cyclomatic_complexity();
        self.conditional_depth += 1;
        walk::walk_conditional_expression(self, it);
        self.conditional_depth -= 1;
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.check_no_collapsible_if(it);
        self.check_no_identical_conditions(it);
        self.check_no_all_duplicated_branches_if(it);
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
        self.check_redundant_continue(&it.body);
        self.add_cyclomatic_complexity();
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_for_in_statement(self, it);
        self.leave_nested_control_flow(counted);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.check_prefer_while(it);
        self.check_redundant_continue(&it.body);
        if let Some(test) = &it.test {
            self.check_no_nested_assignment_condition(test);
        }
        self.add_cyclomatic_complexity();
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_for_statement(self, it);
        self.leave_nested_control_flow(counted);
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.check_redundant_continue(&it.body);
        self.check_no_nested_assignment_condition(&it.test);
        self.add_cyclomatic_complexity();
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_while_statement(self, it);
        self.leave_nested_control_flow(counted);
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.check_redundant_continue(&it.body);
        self.check_no_nested_assignment_condition(&it.test);
        self.add_cyclomatic_complexity();
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_do_while_statement(self, it);
        self.leave_nested_control_flow(counted);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.check_redundant_continue(&it.body);
        self.add_cyclomatic_complexity();
        let counted = self.enter_nested_control_flow(it.span);
        walk::walk_for_of_statement(self, it);
        self.leave_nested_control_flow(counted);
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
        walk::walk_assignment_expression(self, it);
    }

    fn visit_ts_union_type(&mut self, it: &TSUnionType<'a>) {
        self.check_no_duplicate_in_composite(&it.types);
        self.check_max_union_size(it);
        walk::walk_ts_union_type(self, it);
    }

    fn visit_ts_intersection_type(&mut self, it: &TSIntersectionType<'a>) {
        self.check_no_duplicate_in_composite(&it.types);
        walk::walk_ts_intersection_type(self, it);
    }

    fn visit_ts_property_signature(&mut self, it: &TSPropertySignature<'a>) {
        self.check_no_redundant_optional(it);
        walk::walk_ts_property_signature(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.check_arguments_usage(it);
        walk::walk_identifier_reference(self, it);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        self.check_no_empty_character_class(it);
        self.check_no_empty_group(it);
        self.check_no_empty_alternatives(it);
        self.check_no_regex_spaces(it);
        self.check_no_control_regex(it);
        self.check_single_char_in_character_classes(it);
        self.check_duplicates_in_character_class(it);
        self.check_anchor_precedence(it);
        walk::walk_reg_exp_literal(self, it);
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.record_string_literal(it);
        walk::walk_string_literal(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.check_no_exclusive_tests(it);
        self.check_no_skipped_tests_member(it);
        walk::walk_static_member_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.check_no_skipped_tests_call(it);
        self.check_array_constructor_call(it);
        self.check_no_nested_incdec_call(it);
        self.record_iife_callee(&it.callee);
        walk::walk_call_expression(self, it);
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.check_no_labels(it);
        walk::walk_labeled_statement(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.check_constructor_for_side_effects(it);
        self.check_no_unthrown_error(it);
        walk::walk_expression_statement(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.check_no_primitive_wrappers(it);
        self.check_array_constructor_new(it);
        self.check_no_nested_incdec_new(it);
        walk::walk_new_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let track = self.enter_generator(it);
        self.enter_return_scope(it.span);
        self.jsx_function_stack.push(false);
        self.enter_cyclomatic_scope(it.span);
        walk::walk_function(self, it, flags);
        self.leave_cyclomatic_scope();
        self.leave_return_scope();
        self.leave_generator(it, track);
        self.check_max_lines_per_function(it.span);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.enter_return_scope(it.span);
        self.jsx_function_stack.push(false);
        self.enter_cyclomatic_scope(it.span);
        walk::walk_arrow_function_expression(self, it);
        self.leave_cyclomatic_scope();
        self.leave_return_scope();
        self.check_max_lines_per_function(it.span);
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.record_return(it.argument.is_some());
        walk::walk_return_statement(self, it);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        self.mark_generator_yield();
        walk::walk_yield_expression(self, it);
    }

    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        self.mark_jsx();
        walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
        self.mark_jsx();
        walk::walk_jsx_fragment(self, it);
    }

    fn visit_jsx_attribute(&mut self, it: &JSXAttribute<'a>) {
        if let Some(JSXAttributeValue::StringLiteral(lit)) = &it.value {
            self.exclude_string(lit);
        }
        walk::walk_jsx_attribute(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.exclude_string(&it.source);
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
        walk::walk_function_body(self, it);
    }
}
