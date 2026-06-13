//! Top-level scanner state, `report*` helpers, and AST traversal for the
//! sonarjs port. Traversal uses the Oxc visitor so every node is reached; each
//! `check_*` rule body lives under [`crate::rules`].

use oxc_ast::ast::{
    AssignmentExpression, BinaryExpression, BindingIdentifier, ConditionalExpression,
    ExpressionStatement, Function, IdentifierReference, IfStatement, LabeledStatement,
    LogicalExpression, RegExpLiteral, StaticMemberExpression, SwitchCase, SwitchStatement,
    TSIntersectionType, TSUnionType, TemplateLiteral, UnaryExpression, YieldExpression,
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
        self.switch_depth += 1;
        walk::walk_switch_statement(self, it);
        self.switch_depth -= 1;
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        self.check_comma_or_logical_or_case(it);
        walk::walk_switch_case(self, it);
    }

    fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
        self.check_no_redundant_boolean_binary(it);
        self.check_no_identical_expressions_binary(it);
        walk::walk_binary_expression(self, it);
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        self.check_no_identical_expressions_logical(it);
        walk::walk_logical_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        self.check_no_redundant_boolean_unary(it);
        self.check_no_delete_var(it);
        walk::walk_unary_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.check_no_nested_conditional(it);
        self.check_no_redundant_boolean_conditional(it);
        self.conditional_depth += 1;
        walk::walk_conditional_expression(self, it);
        self.conditional_depth -= 1;
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.check_no_collapsible_if(it);
        self.check_no_identical_conditions(it);
        self.check_no_all_duplicated_branches_if(it);
        walk::walk_if_statement(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.check_no_built_in_override_binding(it);
        walk::walk_binding_identifier(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        self.check_non_existent_operator(it);
        self.check_no_built_in_override_assignment(it);
        self.check_class_prototype(it);
        walk::walk_assignment_expression(self, it);
    }

    fn visit_ts_union_type(&mut self, it: &TSUnionType<'a>) {
        self.check_no_duplicate_in_composite(&it.types);
        walk::walk_ts_union_type(self, it);
    }

    fn visit_ts_intersection_type(&mut self, it: &TSIntersectionType<'a>) {
        self.check_no_duplicate_in_composite(&it.types);
        walk::walk_ts_intersection_type(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.check_arguments_usage(it);
        walk::walk_identifier_reference(self, it);
    }

    fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
        self.check_no_empty_character_class(it);
        walk::walk_reg_exp_literal(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.check_no_exclusive_tests(it);
        walk::walk_static_member_expression(self, it);
    }

    fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
        self.check_no_labels(it);
        walk::walk_labeled_statement(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.check_constructor_for_side_effects(it);
        walk::walk_expression_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let track = self.enter_generator(it);
        walk::walk_function(self, it, flags);
        self.leave_generator(it, track);
    }

    fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
        self.mark_generator_yield();
        walk::walk_yield_expression(self, it);
    }
}
