//! AST-backed stylistic rules for which a flat token stream cannot faithfully
//! recover the expression tree.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{BinaryExpression, ConditionalExpression, Expression, LogicalExpression};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::precedence::{GetPrecedence, Precedence};
use serde_json::Value;

use crate::{LintDiagnostic, TextRange};

use super::context::{Scan, first_option};

const RULE: &str = "no-mixed-operators";
const MESSAGE_ID: &str = "unexpectedMixedOperator";
const ARITHMETIC_OPERATORS: &[&str] = &["+", "-", "*", "/", "%", "**"];
const BITWISE_OPERATORS: &[&str] = &["&", "|", "^", "~", "<<", ">>", ">>>"];
const COMPARISON_OPERATORS: &[&str] = &["==", "!=", "===", "!==", ">", ">=", "<", "<="];
const LOGICAL_OPERATORS: &[&str] = &["&&", "||"];
const RELATIONAL_OPERATORS: &[&str] = &["in", "instanceof"];
const DEFAULT_GROUPS: &[&[&str]] = &[
    ARITHMETIC_OPERATORS,
    BITWISE_OPERATORS,
    COMPARISON_OPERATORS,
    LOGICAL_OPERATORS,
    RELATIONAL_OPERATORS,
];

#[derive(Clone, Copy)]
struct Operator {
    text: &'static str,
    precedence: Precedence,
    range: TextRange,
}

pub(crate) fn check_no_mixed_operators(
    scan: &Scan<'_>,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let first_diagnostic = diagnostics.len();
    if let Some(source_type) = filename.and_then(|filename| SourceType::from_path(filename).ok()) {
        let _ = parse_and_check(scan, source_type, options, diagnostics);
        diagnostics[first_diagnostic..]
            .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
        return;
    }

    // Direct native API callers may omit a filename. Try the same broad grammar
    // sequence as the other AST-backed rules: TSX first, TypeScript for
    // angle-bracket assertions, then unambiguous and script JavaScript.
    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(scan, source_type, options, diagnostics) {
            diagnostics[first_diagnostic..]
                .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
            return;
        }
    }
    diagnostics[first_diagnostic..]
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
}

fn parse_and_check(
    scan: &Scan<'_>,
    source_type: SourceType,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut checker = NoMixedOperators {
        scan,
        options,
        diagnostics,
    };
    checker.visit_program(&parsed.program);
    true
}

struct NoMixedOperators<'s, 'o, 'd> {
    scan: &'s Scan<'s>,
    options: &'o Value,
    diagnostics: &'d mut Vec<LintDiagnostic>,
}

impl NoMixedOperators<'_, '_, '_> {
    fn binary_operator(&self, expression: &BinaryExpression<'_>) -> Option<Operator> {
        let text = expression.operator.as_str();
        self.operator_between(
            expression.left.span().end,
            expression.right.span().start,
            text,
            expression.operator.precedence(),
        )
    }

    fn logical_operator(&self, expression: &LogicalExpression<'_>) -> Option<Operator> {
        let text = expression.operator.as_str();
        self.operator_between(
            expression.left.span().end,
            expression.right.span().start,
            text,
            expression.operator.precedence(),
        )
    }

    fn conditional_operator(&self, expression: &ConditionalExpression<'_>) -> Option<Operator> {
        self.operator_between(
            expression.test.span().end,
            expression.consequent.span().start,
            "?",
            Precedence::Conditional,
        )
        .map(|operator| Operator {
            text: "?:",
            ..operator
        })
    }

    fn expression_operator(&self, expression: &Expression<'_>) -> Option<Operator> {
        match expression {
            Expression::BinaryExpression(binary) => self.binary_operator(binary),
            Expression::LogicalExpression(logical) => self.logical_operator(logical),
            _ => None,
        }
    }

    fn operator_between(
        &self,
        start: u32,
        end: u32,
        text: &'static str,
        precedence: Precedence,
    ) -> Option<Operator> {
        let start = usize::try_from(start).ok()?;
        let end = usize::try_from(end).ok()?;
        self.scan
            .tokens()
            .iter()
            .find(|token| {
                token.start >= start
                    && token.end <= end
                    && self.scan.slice(token.start, token.end) == text
            })
            .and_then(|token| {
                Some(Operator {
                    text,
                    precedence,
                    range: TextRange::new(
                        u32::try_from(token.start).ok()?,
                        u32::try_from(token.end).ok()?,
                    ),
                })
            })
    }

    fn check_child(&mut self, parent: Operator, child: &Expression<'_>, is_left: bool) {
        let Some(child) = self.expression_operator(child) else {
            return;
        };
        if child.text == parent.text
            || !operators_share_group(self.options, child.text, parent.text)
            || (allow_same_precedence(self.options) && child.precedence == parent.precedence)
        {
            return;
        }

        let (left, right) = if is_left {
            (child, parent)
        } else {
            (parent, child)
        };
        let mut message =
            String::with_capacity(91usize.saturating_add(left.text.len() + right.text.len()));
        message.push_str("Unexpected mix of '");
        message.push_str(left.text);
        message.push_str("' and '");
        message.push_str(right.text);
        message.push_str("'. Use parentheses to clarify the intended order of operations.");
        let data = BTreeMap::from([
            ("leftOperator".to_owned(), left.text.to_owned()),
            ("rightOperator".to_owned(), right.text.to_owned()),
        ]);

        for operator in [left, right] {
            self.diagnostics.push(LintDiagnostic {
                rule_name: RULE.to_owned(),
                message_id: MESSAGE_ID.to_owned(),
                message: message.clone(),
                data: data.clone(),
                range: operator.range,
                suggestions: Vec::new(),
            });
        }
    }
}

impl<'a> Visit<'a> for NoMixedOperators<'_, '_, '_> {
    fn visit_binary_expression(&mut self, expression: &BinaryExpression<'a>) {
        if let Some(parent) = self.binary_operator(expression) {
            self.check_child(parent, &expression.left, true);
            self.check_child(parent, &expression.right, false);
        }
        walk::walk_binary_expression(self, expression);
    }

    fn visit_logical_expression(&mut self, expression: &LogicalExpression<'a>) {
        if let Some(parent) = self.logical_operator(expression) {
            self.check_child(parent, &expression.left, true);
            self.check_child(parent, &expression.right, false);
        }
        walk::walk_logical_expression(self, expression);
    }

    fn visit_conditional_expression(&mut self, expression: &ConditionalExpression<'a>) {
        if let Some(parent) = self.conditional_operator(expression) {
            self.check_child(parent, &expression.test, true);
            self.check_child(parent, &expression.consequent, false);
            self.check_child(parent, &expression.alternate, false);
        }
        walk::walk_conditional_expression(self, expression);
    }
}

fn allow_same_precedence(options: &Value) -> bool {
    first_option(options)
        .and_then(|option| option.get("allowSamePrecedence"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn operators_share_group(options: &Value, left: &str, right: &str) -> bool {
    let configured_groups = first_option(options)
        .and_then(|option| option.get("groups"))
        .and_then(Value::as_array);
    if let Some(groups) = configured_groups {
        return groups.iter().any(|group| {
            group.as_array().is_some_and(|operators| {
                contains_json_operator(operators, left) && contains_json_operator(operators, right)
            })
        });
    }

    DEFAULT_GROUPS
        .iter()
        .any(|group| group.contains(&left) && group.contains(&right))
}

fn contains_json_operator(operators: &[Value], expected: &str) -> bool {
    operators
        .iter()
        .any(|operator| operator.as_str() == Some(expected))
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "dense compatibility fixtures use serde_json::json and Vec only in tests"
)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::native_stylistic::{StylisticRuleConfig, StylisticRunConfig, run_stylistic_lint};

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        run_stylistic_lint(
            source,
            &StylisticRunConfig {
                filename: None,
                rules: vec![StylisticRuleConfig {
                    name: RULE.to_owned(),
                    options,
                }],
            },
        )
        .expect("rule should run")
    }

    fn ranges(source: &str, options: Value) -> Vec<(usize, usize)> {
        run(source, options)
            .into_iter()
            .map(|diagnostic| {
                (
                    diagnostic.range.start as usize,
                    diagnostic.range.end as usize,
                )
            })
            .collect()
    }

    #[test]
    fn accepts_all_upstream_valid_fixtures() {
        let fixtures = [
            ("a && b && c && d", json!([])),
            ("a || b || c || d", json!([])),
            ("(a || b) && c && d", json!([])),
            ("a || (b && c && d)", json!([])),
            ("(a || b || c) && d", json!([])),
            ("a || b || (c && d)", json!([])),
            ("a + b + c + d", json!([])),
            ("a * b * c * d", json!([])),
            ("a == 0 && b == 1", json!([])),
            ("a == 0 || b == 1", json!([])),
            (
                "(a == 0) && (b == 1)",
                json!([{ "groups": [["&&", "=="]] }]),
            ),
            ("a + b - c * d / e", json!([{ "groups": [["&&", "||"]] }])),
            ("a + b - c", json!([])),
            ("a * b / c", json!([])),
            ("a + b - c", json!([{ "allowSamePrecedence": true }])),
            ("a * b / c", json!([{ "allowSamePrecedence": true }])),
            (
                "(a || b) ? c : d",
                json!([{ "groups": [["&&", "||", "?:"]] }]),
            ),
            (
                "a ? (b || c) : d",
                json!([{ "groups": [["&&", "||", "?:"]] }]),
            ),
            (
                "a ? b : (c || d)",
                json!([{ "groups": [["&&", "||", "?:"]] }]),
            ),
            (
                "a || (b ? c : d)",
                json!([{ "groups": [["&&", "||", "?:"]] }]),
            ),
            (
                "(a ? b : c) || d",
                json!([{ "groups": [["&&", "||", "?:"]] }]),
            ),
            ("a || (b ? c : d)", json!([])),
            ("(a || b) ? c : d", json!([])),
            ("a || b ? c : d", json!([])),
            ("a ? (b || c) : d", json!([])),
            ("a ? b || c : d", json!([])),
            ("a ? b : (c || d)", json!([])),
            ("a ? b : c || d", json!([])),
        ];

        for (source, options) in fixtures {
            assert_eq!(run(source, options), [], "{source}");
        }
    }

    #[test]
    fn matches_upstream_default_logical_fixture() {
        let source = "a && b || c";
        let diagnostics = run(source, json!([]));
        assert_eq!(ranges(source, json!([])), [(2, 4), (7, 9)]);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [
                "Unexpected mix of '&&' and '||'. Use parentheses to clarify the intended order of operations.",
                "Unexpected mix of '&&' and '||'. Use parentheses to clarify the intended order of operations.",
            ]
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.suggestions.is_empty())
        );
    }

    #[test]
    fn matches_upstream_nested_custom_group_fixture() {
        let source = "a && b > 0 || c";
        assert_eq!(
            ranges(source, json!([{ "groups": [["&&", "||", ">"]] }])),
            [(2, 4), (2, 4), (7, 8), (11, 13)]
        );
        assert_eq!(
            ranges(source, json!([{ "groups": [["&&", "||"]] }])),
            [(2, 4), (11, 13)]
        );
    }

    #[test]
    fn matches_upstream_multiple_group_fixture() {
        let source = "a && b + c - d / e || f";
        let options = json!([{ "groups": [["&&", "||"], ["+", "-", "*", "/"]] }]);
        assert_eq!(
            ranges(source, options.clone()),
            [(2, 4), (11, 12), (15, 16), (19, 21)]
        );
        assert_eq!(
            ranges(
                source,
                json!([{
                    "groups": [["&&", "||"], ["+", "-", "*", "/"]],
                    "allowSamePrecedence": true
                }])
            ),
            [(2, 4), (11, 12), (15, 16), (19, 21)]
        );
    }

    #[test]
    fn matches_upstream_same_precedence_false_fixtures() {
        assert_eq!(
            ranges("a + b - c", json!([{ "allowSamePrecedence": false }])),
            [(2, 3), (6, 7)]
        );
        assert_eq!(
            ranges("a * b / c", json!([{ "allowSamePrecedence": false }])),
            [(2, 3), (6, 7)]
        );
    }

    #[test]
    fn matches_upstream_conditional_fixtures() {
        let options = json!([{ "groups": [["&&", "||", "?:"]] }]);
        assert_eq!(ranges("a || b ? c : d", options.clone()), [(2, 4), (7, 8)]);
        assert_eq!(ranges("a && b ? 1 : 2", options.clone()), [(2, 4), (7, 8)]);
        assert_eq!(ranges("x ? a && b : 0", options.clone()), [(2, 3), (6, 8)]);
        assert_eq!(ranges("x ? 0 : a && b", options), [(2, 3), (10, 12)]);
    }

    #[test]
    fn matches_upstream_nullish_fixture() {
        assert_eq!(
            ranges("a + b ?? c", json!([{ "groups": [["+", "??"]] }])),
            [(2, 3), (6, 8)]
        );
    }

    #[test]
    fn covers_arithmetic_bitwise_comparison_and_relational_defaults() {
        for (source, expected) in [
            ("a + b * c", vec![(2, 3), (6, 7)]),
            ("a ** b % c", vec![(2, 4), (7, 8)]),
            ("a | b & c", vec![(2, 3), (6, 7)]),
            ("a << b ^ c", vec![(2, 4), (7, 8)]),
            ("a == b < c", vec![(2, 4), (7, 8)]),
        ] {
            assert_eq!(ranges(source, json!([])), expected, "{source}");
        }
        assert_eq!(
            ranges(
                "a in b instanceof c",
                json!([{ "allowSamePrecedence": false }])
            ),
            [(2, 4), (7, 17)]
        );
    }

    #[test]
    fn honors_parentheses_at_every_expression_side() {
        for source in [
            "(a + b) * c",
            "a + (b * c)",
            "((a && b)) || c",
            "a && ((b || c))",
            "(a in b) instanceof c",
            "a in (b instanceof c)",
            "(a ?? b) + c",
            "a + (b ?? c)",
        ] {
            assert_eq!(
                run(
                    source,
                    json!([{ "groups": [["+", "*", "??"], ["&&", "||"], ["in", "instanceof"]] }])
                ),
                [],
                "{source}"
            );
        }
    }

    #[test]
    fn handles_typescript_jsx_optional_chaining_and_nested_expressions() {
        let source = r#"
type T = A | B & C;
const generic = foo<A | B, C & D>();
const asserted = (a + b * c) as number;
const optional = obj?.value + fallback * scale;
const view = <Panel value={a + b * c} label="x || y" />;
const nested = fn(() => a + b * c, { value: d && e || f });
"#;
        let diagnostics = run(source, json!([]));
        let operator_texts = diagnostics
            .iter()
            .map(|diagnostic| {
                &source[diagnostic.range.start as usize..diagnostic.range.end as usize]
            })
            .collect::<Vec<_>>();
        assert_eq!(
            operator_texts,
            ["+", "*", "+", "*", "+", "*", "+", "*", "&&", "||"]
        );
    }

    #[test]
    fn ignores_operators_in_comments_literals_regex_templates_and_unicode_prefixes() {
        let source = r#"
// a + b * c
const 文 = "a + b * c";
const regex = /a+b*c/;
const raw = `a + b * c`;
const template = `value ${a + b * c}`;
const actual = 文 + left * right;
"#;
        let diagnostics = run(source, json!([]));
        assert_eq!(diagnostics.len(), 4);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [
                "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
                "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
                "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
                "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
            ]
        );
    }

    #[test]
    fn supports_angle_bracket_type_assertion_fallback_without_type_false_positives() {
        let source = "const value = <number>(a + b * c);\ntype T = A | B & C;\n";
        assert_eq!(run(source, json!([])).len(), 2);
    }

    #[test]
    fn supports_script_only_javascript_fallback() {
        let source = "with (scope) { result = a + b * c; }";
        assert_eq!(ranges(source, json!([])), [(26, 27), (30, 31)]);
    }

    #[test]
    fn preserves_multiline_and_comment_separated_operator_ranges() {
        let source = "a\n  + /* left */ b\n  * // right\n  c";
        assert_eq!(ranges(source, json!([])), [(4, 5), (21, 22)]);
    }

    #[test]
    fn empty_custom_groups_disable_the_rule() {
        assert_eq!(
            run(
                "a + b * c && d || e",
                json!([{ "groups": [], "allowSamePrecedence": false }])
            ),
            []
        );
    }
}
