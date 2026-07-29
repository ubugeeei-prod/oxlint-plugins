//! Native implementation of `@stylistic/no-confusing-arrow`.
//!
//! Unlike spacing-only rules, this rule depends on the parsed shape of an
//! arrow body: only a top-level `ConditionalExpression` is ambiguous. Parsing
//! avoids mistaking ternaries nested in calls, arrays, objects, TypeScript
//! wrappers, or JSX expressions for the arrow's direct body.

use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrowFunctionExpression, BindingPattern, Expression, Statement};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::option_object_bool;

const RULE_NAME: &str = "no-confusing-arrow";
const MESSAGE_ID: &str = "confusing";
const MESSAGE: &str = "Arrow function used ambiguously with a conditional expression.";

/// Reports arrow functions whose expression body is a conditional expression.
///
/// The bridge supplies source text rather than the host AST. Try TypeScript
/// with JSX first, then JavaScript's unambiguous and script modes. This keeps
/// TypeScript and JSX node boundaries while still accepting script-only
/// JavaScript grammar. As in ESLint, a parse failure means no rule visitors run.
pub(crate) fn check_no_confusing_arrow(
    source_text: &str,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, source_text, source_type).parse();
        if parsed.errors.is_empty() {
            let mut visitor = NoConfusingArrowVisitor {
                source_text,
                allow_parens: option_object_bool(options, "allowParens", true),
                only_one_simple_param: option_object_bool(options, "onlyOneSimpleParam", false),
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            return;
        }
    }
}

struct NoConfusingArrowVisitor<'source, 'diagnostics> {
    source_text: &'source str,
    allow_parens: bool,
    only_one_simple_param: bool,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for NoConfusingArrowVisitor<'_, '_> {
    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'ast>) {
        self.check_arrow(arrow);
        walk::walk_arrow_function_expression(self, arrow);
    }
}

impl NoConfusingArrowVisitor<'_, '_> {
    fn check_arrow(&mut self, arrow: &ArrowFunctionExpression<'_>) {
        if !arrow.expression {
            return;
        }
        if self.only_one_simple_param && !has_one_simple_parameter(arrow) {
            return;
        }

        let Some(expression) = arrow_body_expression(arrow) else {
            return;
        };
        let (expression, parenthesized) = unwrap_parentheses(expression);
        if !matches!(expression, Expression::ConditionalExpression(_)) {
            return;
        }
        if self.allow_parens && parenthesized {
            return;
        }

        let arrow_span = arrow.span;
        let body_span = expression.span();
        let suggestion = if self.allow_parens {
            self.parenthesize_suggestion(body_span)
        } else {
            None
        };
        let (Ok(start), Ok(end)) = (
            usize::try_from(arrow_span.start),
            usize::try_from(arrow_span.end),
        ) else {
            return;
        };
        if start > end || end > self.source_text.len() {
            return;
        }

        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: MESSAGE.to_owned(),
            data: std::collections::BTreeMap::new(),
            range: TextRange::new(arrow_span.start, arrow_span.end),
            suggestions: suggestion.into_iter().collect(),
        });
    }

    fn parenthesize_suggestion(&self, body_span: Span) -> Option<LintSuggestion> {
        let start = usize::try_from(body_span.start).ok()?;
        let end = usize::try_from(body_span.end).ok()?;
        let body = self.source_text.get(start..end)?;
        let mut replacement = String::with_capacity(body.len().saturating_add(2));
        replacement.push('(');
        replacement.push_str(body);
        replacement.push(')');
        Some(LintSuggestion {
            // Native fixes are transported through the bridge's suggestion
            // channel. Reusing the upstream message ID keeps the public
            // message catalog byte-for-byte compatible.
            message_id: MESSAGE_ID.to_owned(),
            message: MESSAGE.to_owned(),
            fixes: std::iter::once(LintFix::replace_range(
                TextRange::new(body_span.start, body_span.end),
                replacement,
            ))
            .collect(),
        })
    }
}

fn has_one_simple_parameter(arrow: &ArrowFunctionExpression<'_>) -> bool {
    arrow.params.rest.is_none()
        && arrow.params.items.len() == 1
        && arrow.params.items[0].initializer.is_none()
        && matches!(
            arrow.params.items[0].pattern,
            BindingPattern::BindingIdentifier(_)
        )
}

fn arrow_body_expression<'ast>(
    arrow: &'ast ArrowFunctionExpression<'ast>,
) -> Option<&'ast Expression<'ast>> {
    let Statement::ExpressionStatement(statement) = arrow.body.statements.first()? else {
        return None;
    };
    Some(&statement.expression)
}

fn unwrap_parentheses<'ast>(
    mut expression: &'ast Expression<'ast>,
) -> (&'ast Expression<'ast>, bool) {
    let mut parenthesized = false;
    while let Expression::ParenthesizedExpression(parenthesized_expression) = expression {
        parenthesized = true;
        expression = &parenthesized_expression.expression;
    }
    (expression, parenthesized)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the verbatim upstream option matrix readable in tests"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_no_confusing_arrow(source, &options, &mut diagnostics);
        diagnostics
    }

    fn fixed_source(source: &str, diagnostic: &LintDiagnostic) -> Option<String> {
        let fix = diagnostic.suggestions.first()?.fixes.first()?;
        let start = usize::try_from(fix.range.start).ok()?;
        let end = usize::try_from(fix.range.end).ok()?;
        let mut fixed = String::with_capacity(
            source
                .len()
                .saturating_sub(end.saturating_sub(start))
                .saturating_add(fix.replacement_text.len()),
        );
        fixed.push_str(&source[..start]);
        fixed.push_str(&fix.replacement_text);
        fixed.push_str(&source[end..]);
        Some(fixed)
    }

    #[test]
    fn accepts_every_upstream_v5_10_0_valid_case() {
        let cases = [
            ("a => { return 1 ? 2 : 3; }", Value::Null),
            (
                "a => { return 1 ? 2 : 3; }",
                json!([{ "allowParens": false }]),
            ),
            ("var x = a => { return 1 ? 2 : 3; }", Value::Null),
            (
                "var x = a => { return 1 ? 2 : 3; }",
                json!([{ "allowParens": false }]),
            ),
            ("var x = (a) => { return 1 ? 2 : 3; }", Value::Null),
            (
                "var x = (a) => { return 1 ? 2 : 3; }",
                json!([{ "allowParens": false }]),
            ),
            ("var x = a => (1 ? 2 : 3)", Value::Null),
            ("var x = a => (1 ? 2 : 3)", json!([{ "allowParens": true }])),
            ("var x = (a,b) => (1 ? 2 : 3)", Value::Null),
            ("() => 1 ? 2 : 3", json!([{ "onlyOneSimpleParam": true }])),
            (
                "(a, b) => 1 ? 2 : 3",
                json!([{ "onlyOneSimpleParam": true }]),
            ),
            (
                "(a = b) => 1 ? 2 : 3",
                json!([{ "onlyOneSimpleParam": true }]),
            ),
            (
                "({ a }) => 1 ? 2 : 3",
                json!([{ "onlyOneSimpleParam": true }]),
            ),
            (
                "([a]) => 1 ? 2 : 3",
                json!([{ "onlyOneSimpleParam": true }]),
            ),
            (
                "(...a) => 1 ? 2 : 3",
                json!([{ "onlyOneSimpleParam": true }]),
            ),
        ];

        for (source, options) in cases {
            assert!(
                run(source, options).is_empty(),
                "upstream valid case reported: {source}"
            );
        }
    }

    #[test]
    fn reports_and_fixes_every_upstream_v5_10_0_fixable_invalid_case() {
        let cases = [
            ("a => 1 ? 2 : 3", "a => (1 ? 2 : 3)", Value::Null),
            (
                "a => 1 ? 2 : 3",
                "a => (1 ? 2 : 3)",
                json!([{ "allowParens": true }]),
            ),
            (
                "var x = a => 1 ? 2 : 3",
                "var x = a => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = a => 1 ? 2 : 3",
                "var x = a => (1 ? 2 : 3)",
                json!([{ "allowParens": true }]),
            ),
            (
                "var x = (a) => 1 ? 2 : 3",
                "var x = (a) => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = () => 1 ? 2 : 3",
                "var x = () => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = () => 1 ? 2 : 3",
                "var x = () => (1 ? 2 : 3)",
                json!([{}]),
            ),
            (
                "var x = () => 1 ? 2 : 3",
                "var x = () => (1 ? 2 : 3)",
                json!([{ "onlyOneSimpleParam": false }]),
            ),
            (
                "var x = (a, b) => 1 ? 2 : 3",
                "var x = (a, b) => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = (a = b) => 1 ? 2 : 3",
                "var x = (a = b) => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = ({ a }) => 1 ? 2 : 3",
                "var x = ({ a }) => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = ([a]) => 1 ? 2 : 3",
                "var x = ([a]) => (1 ? 2 : 3)",
                Value::Null,
            ),
            (
                "var x = (...a) => 1 ? 2 : 3",
                "var x = (...a) => (1 ? 2 : 3)",
                Value::Null,
            ),
        ];

        for (source, expected, options) in cases {
            let diagnostics = run(source, options);
            assert_eq!(diagnostics.len(), 1, "invalid case not reported: {source}");
            assert_eq!(diagnostics[0].message_id, MESSAGE_ID);
            assert_eq!(
                fixed_source(source, &diagnostics[0]).as_deref(),
                Some(expected),
                "wrong fix for: {source}"
            );
        }
    }

    #[test]
    fn reports_without_a_fix_when_parentheses_are_disallowed() {
        for source in [
            "a => 1 ? 2 : 3",
            "var x = a => 1 ? 2 : 3",
            "a => (1 ? 2 : 3)",
        ] {
            let diagnostics = run(source, json!([{ "allowParens": false }]));
            assert_eq!(diagnostics.len(), 1, "invalid case not reported: {source}");
            assert!(
                diagnostics[0].suggestions.is_empty(),
                "allowParens:false must match upstream output:null: {source}"
            );
        }
    }

    #[test]
    fn honors_only_one_simple_parameter_for_javascript_and_typescript() {
        let options = json!([{ "onlyOneSimpleParam": true }]);
        for source in [
            "a => test ? yes : no",
            "(a) => test ? yes : no",
            "(a: string) => test ? yes : no",
            "async a => test ? yes : no",
            "async (a: string) => test ? yes : no",
        ] {
            assert_eq!(
                run(source, options.clone()).len(),
                1,
                "one simple parameter must report: {source}"
            );
        }

        for source in [
            "() => test ? yes : no",
            "(a, b) => test ? yes : no",
            "(a = fallback) => test ? yes : no",
            "({ a }) => test ? yes : no",
            "([a]) => test ? yes : no",
            "(...a) => test ? yes : no",
        ] {
            assert!(
                run(source, options.clone()).is_empty(),
                "non-simple parameters must be ignored: {source}"
            );
        }
    }

    #[test]
    fn ignores_nested_conditional_expressions_and_non_conditional_arrow_bodies() {
        for source in [
            "a => call(test ? yes : no)",
            "a => [test ? yes : no]",
            "a => ({ value: test ? yes : no })",
            "a => test && (other ? yes : no)",
            "a => test ?? (other ? yes : no)",
            "a => `${test ? yes : no}`",
            "a => function () { return test ? yes : no }",
            "a => class { field = test ? yes : no }",
            "a => <Component value={test ? yes : no} />",
            "a => <>{test ? yes : no}</>",
        ] {
            assert!(
                run(source, Value::Null).is_empty(),
                "nested conditional false positive: {source}"
            );
        }
    }

    #[test]
    fn handles_nested_arrows_jsx_and_typescript_wrappers() {
        let sources = [
            "const render = () => <Button onClick={value => value ? yes : no} />;",
            "const nested = () => (value => value ? yes : no);",
            "const generic = <T,>(value: T) => value ? yes : no;",
            "const typed = (value: string): Result => value ? yes : no;",
        ];
        for source in sources {
            let diagnostics = run(source, Value::Null);
            assert_eq!(
                diagnostics.len(),
                1,
                "expected exactly the ambiguous arrow: {source}"
            );
        }

        for source in [
            "const asserted = (value: string) => (value ? yes : no) as Result;",
            "const nonNull = (value: string) => (value ? yes : no)!;",
            "const satisfied = (value: string) => (value ? yes : no) satisfies Result;",
        ] {
            assert!(
                run(source, Value::Null).is_empty(),
                "TypeScript wrapper changes the direct body node: {source}"
            );
        }
    }

    #[test]
    fn accepts_script_only_javascript_grammar() {
        let source = "with (scope) { const select = value => value ? yes : no; }";
        let diagnostics = run(source, Value::Null);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message_id, MESSAGE_ID);
    }

    #[test]
    fn preserves_nested_ternaries_comments_and_exact_body_ranges() {
        let cases = [
            (
                "value => first ? second ? a : b : c",
                "value => (first ? second ? a : b : c)",
            ),
            (
                "value => first /* test */ ? a : b // tail\n",
                "value => (first /* test */ ? a : b) // tail\n",
            ),
            (
                "value =>\n  first\n    ? a\n    : b",
                "value =>\n  (first\n    ? a\n    : b)",
            ),
        ];
        for (source, expected) in cases {
            let diagnostics = run(source, Value::Null);
            assert_eq!(diagnostics.len(), 1, "expected report: {source}");
            assert_eq!(
                fixed_source(source, &diagnostics[0]).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn reports_all_ambiguous_arrows_in_source_order() {
        let source =
            "const a = x => x ? 1 : 0;\nconst b = y => y ? 'yes' : 'no';\nconst c = z => z;";
        let diagnostics = run(source, Value::Null);
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].range.start < diagnostics[1].range.start);
        assert_eq!(
            fixed_source(source, &diagnostics[0]).as_deref(),
            Some(
                "const a = x => (x ? 1 : 0);\nconst b = y => y ? 'yes' : 'no';\nconst c = z => z;"
            )
        );
    }

    #[test]
    fn emits_utf8_byte_ranges_for_non_ascii_source() {
        let source = "const 日本語 = 値 => 値 ? 'はい' : 'いいえ';";
        let diagnostics = run(source, Value::Null);
        assert_eq!(diagnostics.len(), 1);
        let conditional_start = source.find("値 ?").expect("conditional starts");
        let conditional_end = source.find(';').expect("statement ends");
        let fix = &diagnostics[0].suggestions[0].fixes[0];
        assert_eq!(
            fix.range,
            TextRange::new(conditional_start as u32, conditional_end as u32)
        );
        assert_eq!(fix.replacement_text, "(値 ? 'はい' : 'いいえ')");
    }

    #[test]
    fn parse_failures_do_not_create_heuristic_false_positives() {
        for source in [
            "const broken = value => value ? yes :",
            "const comparison = (value => value) >= other ? yes : no;",
            "const string = 'value => test ? yes : no';",
            "// value => test ? yes : no\nconst value = 1;",
        ] {
            assert!(
                run(source, Value::Null).is_empty(),
                "malformed or non-code arrow must not report: {source}"
            );
        }
    }
}
