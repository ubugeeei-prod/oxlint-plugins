//! Native implementation of `@stylistic/multiline-ternary`.
//!
//! Conditional-expression nodes provide the only reliable way to distinguish
//! ternary punctuation from optional chaining, TypeScript syntax, JSX text,
//! strings, comments, and nested expressions. The shared lexer supplies the
//! exact significant-token boundaries used by ESLint's `SourceCode` helpers.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ConditionalExpression, Expression, JSXExpression, JSXExpressionContainer,
    ParenthesizedExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::Scan;

const RULE: &str = "multiline-ternary";
const EXPECTED_TEST_CONS: &str =
    "Expected newline between test and consequent of ternary expression.";
const EXPECTED_CONS_ALT: &str =
    "Expected newline between consequent and alternate of ternary expression.";
const UNEXPECTED_TEST_CONS: &str =
    "Unexpected newline between test and consequent of ternary expression.";
const UNEXPECTED_CONS_ALT: &str =
    "Unexpected newline between consequent and alternate of ternary expression.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Style {
    Always,
    AlwaysMultiline,
    Never,
}

#[derive(Clone, Copy, Debug)]
struct Options {
    style: Style,
    ignore_jsx: bool,
}

impl Options {
    fn from_value(value: &Value) -> Self {
        let values = value.as_array();
        let style = match values
            .and_then(|options| options.first())
            .and_then(Value::as_str)
        {
            Some("always-multiline") => Style::AlwaysMultiline,
            Some("never") => Style::Never,
            _ => Style::Always,
        };
        let ignore_jsx = values
            .and_then(|options| options.get(1))
            .and_then(|options| options.get("ignoreJSX"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Self { style, ignore_jsx }
    }
}

pub(crate) fn check_multiline_ternary(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let scan = Scan::new(source);
    let normalized = Options::from_value(options);

    if let Some(source_type) = filename.and_then(|value| SourceType::from_path(value).ok()) {
        let _ = parse_and_check(&scan, source_type, normalized, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(&scan, source_type, normalized, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    scan: &Scan<'_>,
    source_type: SourceType,
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = MultilineTernary {
        scan,
        options,
        ignored_jsx_conditional: None,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct MultilineTernary<'scan, 'diagnostics> {
    scan: &'scan Scan<'scan>,
    options: Options,
    ignored_jsx_conditional: Option<Span>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for MultilineTernary<'_, '_> {
    fn visit_conditional_expression(&mut self, expression: &ConditionalExpression<'ast>) {
        if self.ignored_jsx_conditional != Some(expression.span) {
            self.check(expression);
        }
        walk::walk_conditional_expression(self, expression);
    }

    fn visit_jsx_expression_container(&mut self, container: &JSXExpressionContainer<'ast>) {
        let previous = self.ignored_jsx_conditional;
        self.ignored_jsx_conditional = self
            .options
            .ignore_jsx
            .then(|| direct_jsx_conditional(&container.expression))
            .flatten();
        walk::walk_jsx_expression_container(self, container);
        self.ignored_jsx_conditional = previous;
    }
}

impl MultilineTernary<'_, '_> {
    fn check(&mut self, expression: &ConditionalExpression<'_>) {
        let Some(tokens) = ConditionalTokens::find(self.scan, expression) else {
            return;
        };

        let test_and_consequent_same_line =
            self.same_line(tokens.last_test, tokens.first_consequent);
        let consequent_and_alternate_same_line =
            self.same_line(tokens.last_consequent, tokens.first_alternate);
        let has_comments = self.scan.tokens().iter().any(|token| {
            token.kind.is_comment()
                && token.start >= expression.span.start as usize
                && token.end <= expression.span.end as usize
        });

        if self.options.style == Style::Never {
            if !test_and_consequent_same_line {
                let fixes = if has_comments {
                    Vec::new()
                } else {
                    self.removal_fixes(tokens.last_test, tokens.question, tokens.first_consequent)
                };
                self.report(
                    "unexpectedTestCons",
                    UNEXPECTED_TEST_CONS,
                    tokens.first_test,
                    tokens.last_test,
                    fixes,
                );
            }

            if !consequent_and_alternate_same_line {
                let fixes = if has_comments {
                    Vec::new()
                } else {
                    self.removal_fixes(tokens.last_consequent, tokens.colon, tokens.first_alternate)
                };
                self.report(
                    "unexpectedConsAlt",
                    UNEXPECTED_CONS_ALT,
                    tokens.first_consequent,
                    tokens.last_consequent,
                    fixes,
                );
            }
            return;
        }

        if self.options.style == Style::AlwaysMultiline
            && !has_line_terminator(self.span_text(expression.span))
        {
            return;
        }

        if test_and_consequent_same_line {
            let fixes = if has_comments {
                Vec::new()
            } else {
                std::iter::once(LintFix::replace_range(
                    self.gap_range(tokens.last_test, tokens.question),
                    "\n",
                ))
                .collect()
            };
            self.report(
                "expectedTestCons",
                EXPECTED_TEST_CONS,
                tokens.first_test,
                tokens.last_test,
                fixes,
            );
        }

        if consequent_and_alternate_same_line {
            let fixes = if has_comments {
                Vec::new()
            } else {
                std::iter::once(LintFix::replace_range(
                    self.gap_range(tokens.last_consequent, tokens.colon),
                    "\n",
                ))
                .collect()
            };
            self.report(
                "expectedConsAlt",
                EXPECTED_CONS_ALT,
                tokens.first_consequent,
                tokens.last_consequent,
                fixes,
            );
        }
    }

    fn removal_fixes(&self, left: usize, operator: usize, right: usize) -> Vec<LintFix> {
        let mut fixes = Vec::with_capacity(2);
        if !self.same_line(left, operator) {
            fixes.push(LintFix::remove_range(self.gap_range(left, operator)));
        }
        if !self.same_line(operator, right) {
            fixes.push(LintFix::remove_range(self.gap_range(operator, right)));
        }
        fixes
    }

    fn same_line(&self, left: usize, right: usize) -> bool {
        let tokens = self.scan.tokens();
        !has_line_terminator(self.scan.slice(tokens[left].end, tokens[right].start))
    }

    fn gap_range(&self, left: usize, right: usize) -> TextRange {
        TextRange::new(
            self.scan.tokens()[left].end as u32,
            self.scan.tokens()[right].start as u32,
        )
    }

    fn span_text(&self, span: Span) -> &str {
        self.scan.slice(span.start as usize, span.end as usize)
    }

    fn report(
        &mut self,
        message_id: &'static str,
        message: &'static str,
        first: usize,
        last: usize,
        fixes: Vec<LintFix>,
    ) {
        let tokens = self.scan.tokens();
        let suggestions = (!fixes.is_empty())
            .then(|| LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes,
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: Default::default(),
            range: TextRange::new(tokens[first].start as u32, tokens[last].end as u32),
            suggestions,
        });
    }
}

#[derive(Clone, Copy, Debug)]
struct ConditionalTokens {
    first_test: usize,
    last_test: usize,
    question: usize,
    first_consequent: usize,
    last_consequent: usize,
    colon: usize,
    first_alternate: usize,
}

impl ConditionalTokens {
    fn find(scan: &Scan<'_>, expression: &ConditionalExpression<'_>) -> Option<Self> {
        let question = token_between(
            scan,
            expression.test.span().end,
            expression.consequent.span().start,
            "?",
        )?;
        let colon = token_between(
            scan,
            expression.consequent.span().end,
            expression.alternate.span().start,
            ":",
        )?;
        let first_test = scan.tokens().iter().position(|token| {
            !token.kind.is_comment() && token.start >= expression.span.start as usize
        })?;
        let last_test = scan.prev_significant(question)?;
        let first_consequent = scan.next_significant(question)?;
        let last_consequent = scan.prev_significant(colon)?;
        let first_alternate = scan.next_significant(colon)?;
        Some(Self {
            first_test,
            last_test,
            question,
            first_consequent,
            last_consequent,
            colon,
            first_alternate,
        })
    }
}

fn token_between(scan: &Scan<'_>, start: u32, end: u32, expected: &str) -> Option<usize> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    scan.tokens().iter().position(|token| {
        !token.kind.is_comment()
            && token.start >= start
            && token.end <= end
            && scan.slice(token.start, token.end) == expected
    })
}

fn direct_jsx_conditional(expression: &JSXExpression<'_>) -> Option<Span> {
    match expression {
        JSXExpression::ConditionalExpression(expression) => Some(expression.span),
        JSXExpression::ParenthesizedExpression(expression) => {
            direct_parenthesized_conditional(expression)
        }
        _ => None,
    }
}

fn direct_parenthesized_conditional(expression: &ParenthesizedExpression<'_>) -> Option<Span> {
    match &expression.expression {
        Expression::ConditionalExpression(expression) => Some(expression.span),
        Expression::ParenthesizedExpression(expression) => {
            direct_parenthesized_conditional(expression)
        }
        _ => None,
    }
}

fn has_line_terminator(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the option regression matrix readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    struct Generated {
        version: String,
        commit: String,
    }

    #[derive(Deserialize)]
    struct FixtureCase {
        code: String,
        options: Value,
        #[serde(default)]
        errors: Vec<FixtureError>,
        #[serde(default)]
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureError {
        message_id: String,
        message: String,
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
        fix: Option<FixtureFix>,
    }

    #[derive(Deserialize)]
    struct FixtureFix {
        range: [u32; 2],
        text: String,
    }

    fn upstream_fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/multiline-ternary-v5.10.0.json"
        ))
        .expect("generated multiline-ternary fixture is valid JSON")
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_multiline_ternary(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn location(source: &str, offset: u32) -> (usize, usize) {
        let offset = usize::try_from(offset).expect("fixture offset fits usize");
        let prefix = source
            .get(..offset)
            .expect("diagnostic offset is a UTF-8 boundary");
        let mut line = 1;
        let mut line_start = 0;
        for (index, character) in prefix.char_indices() {
            if matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                line += 1;
                line_start = index + character.len_utf8();
            }
        }
        (line, prefix[line_start..].chars().count() + 1)
    }

    fn merged_fix(source: &str, diagnostic: &LintDiagnostic) -> Option<LintFix> {
        let fixes = &diagnostic.suggestions.first()?.fixes;
        let first = fixes.first()?;
        let last = fixes.last()?;
        let mut replacement = String::new();
        let mut cursor = first.range.start as usize;
        for fix in fixes {
            let start = fix.range.start as usize;
            let end = fix.range.end as usize;
            replacement.push_str(source.get(cursor..start)?);
            replacement.push_str(&fix.replacement_text);
            cursor = end;
        }
        Some(LintFix::replace_range(
            TextRange::new(first.range.start, last.range.end),
            replacement,
        ))
    }

    fn iterative_fixed_output(
        source: &str,
        filename: Option<&str>,
        options: &Value,
    ) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;

        for _ in 0..10 {
            let diagnostics = run(&output, filename, options);
            let mut fixes = diagnostics
                .iter()
                .enumerate()
                .filter_map(|(index, diagnostic)| {
                    merged_fix(&output, diagnostic).map(|fix| (index, fix))
                })
                .collect::<Vec<_>>();
            fixes.sort_by_key(|(index, fix)| (fix.range.start, fix.range.end, *index));
            if fixes.is_empty() {
                break;
            }

            let mut next = String::with_capacity(output.len());
            let mut cursor = 0usize;
            let mut last_end = None;
            let mut applied = false;
            for (_, fix) in fixes {
                if last_end.is_some_and(|end| end >= fix.range.start) {
                    continue;
                }
                let start = fix.range.start as usize;
                let end = fix.range.end as usize;
                let (Some(prefix), Some(_)) = (output.get(cursor..start), output.get(start..end))
                else {
                    continue;
                };
                next.push_str(prefix);
                next.push_str(&fix.replacement_text);
                cursor = end;
                last_end = Some(fix.range.end);
                applied = true;
            }
            if !applied {
                break;
            }
            next.push_str(&output[cursor..]);
            output = next;
            changed = true;
        }

        changed.then_some(output)
    }

    #[test]
    fn accepts_every_stable_v5_10_0_valid_fixture() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.generated.version, "v5.10.0");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.valid.len(), 84);

        for (index, test) in fixture.valid.iter().enumerate() {
            let diagnostics = run(&test.code, None, &test.options);
            assert!(
                diagnostics.is_empty(),
                "upstream valid fixture {index} reported {diagnostics:#?}\n{}",
                test.code
            );
        }
    }

    #[test]
    fn replays_every_stable_v5_10_0_invalid_fixture_exactly() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.invalid.len(), 63);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .map(|test| test.errors.len())
                .sum::<usize>(),
            104
        );

        for (index, test) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test.code, None, &test.options);
            assert_eq!(
                diagnostics.len(),
                test.errors.len(),
                "diagnostic count differs for upstream invalid fixture {index}\n{}",
                test.code
            );
            for (diagnostic, expected) in diagnostics.iter().zip(&test.errors) {
                assert_eq!(
                    diagnostic.message_id, expected.message_id,
                    "message ID differs for fixture {index}"
                );
                assert_eq!(
                    diagnostic.message, expected.message,
                    "message differs for fixture {index}"
                );
                assert_eq!(
                    location(&test.code, diagnostic.range.start),
                    (expected.line, expected.column),
                    "start location differs for fixture {index}"
                );
                assert_eq!(
                    location(&test.code, diagnostic.range.end),
                    (expected.end_line, expected.end_column),
                    "end location differs for fixture {index}"
                );

                let actual = merged_fix(&test.code, diagnostic);
                match (&expected.fix, actual) {
                    (None, None) => {}
                    (Some(expected), Some(actual)) => {
                        assert_eq!(
                            [actual.range.start, actual.range.end],
                            expected.range,
                            "fix range differs for fixture {index}"
                        );
                        assert_eq!(
                            actual.replacement_text, expected.text,
                            "fix text differs for fixture {index}"
                        );
                    }
                    _ => panic!("fix presence differs for fixture {index}"),
                }
            }
            assert_eq!(
                iterative_fixed_output(&test.code, None, &test.options),
                test.output,
                "iterative output differs for fixture {index}\n{}",
                test.code
            );
        }
    }

    #[test]
    fn recognizes_every_ecmascript_line_terminator() {
        for linebreak in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("condition{linebreak}? yes{linebreak}: no");
            assert!(
                run(&source, None, &json!(["always"])).is_empty(),
                "{linebreak:?}"
            );
            assert_eq!(
                run(&source, None, &json!(["never"]))
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                ["unexpectedTestCons", "unexpectedConsAlt"],
                "{linebreak:?}"
            );
        }
    }

    #[test]
    fn preserves_utf8_byte_ranges_and_utf16_independent_fixes() {
        let source = "const café = 条件 ? はい : いいえ;";
        let diagnostics = run(source, Some("fixture.ts"), &json!(["always"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["expectedTestCons", "expectedConsAlt"]
        );
        let test = source.find("条件").expect("test");
        let consequent = source.find("はい").expect("consequent");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [
                TextRange::new(test as u32, (test + "条件".len()) as u32),
                TextRange::new(consequent as u32, (consequent + "はい".len()) as u32),
            ]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.suggestions[0].fixes[0].replacement_text.as_str())
                .collect::<Vec<_>>(),
            ["\n", "\n"]
        );
    }

    #[test]
    fn suppresses_all_fixes_when_any_comment_is_inside_the_expression() {
        for (source, options, expected) in [
            (
                "condition ? // keep\nconsequent : alternate",
                json!(["always"]),
                vec!["expectedConsAlt"],
            ),
            (
                "condition\n? /* keep */ consequent\n: alternate",
                json!(["never"]),
                vec!["unexpectedTestCons", "unexpectedConsAlt"],
            ),
            (
                "condition ? consequent /* nested */ : alternate",
                json!(["always"]),
                vec!["expectedTestCons", "expectedConsAlt"],
            ),
        ] {
            let diagnostics = run(source, None, &options);
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.suggestions.is_empty())
            );
        }
    }

    #[test]
    fn covers_typescript_tsx_jsx_parentage_and_shared_syntax_boundaries() {
        let typescript = [
            "const typed: string = condition ? yes : no;",
            "const asserted = condition ? (yes as string) : (no satisfies string);",
            "const generic = condition ? factory<T>() : fallback<T>();",
        ]
        .join("\n");
        assert_eq!(
            run(&typescript, Some("fixture.ts"), &json!(["always"])).len(),
            6
        );

        let tsx = [
            "const ignored = <Panel>{condition ? <Yes /> : <No />}</Panel>;",
            "const ignoredParens = <>{(condition ? <Yes /> : <No />)}</>;",
            "const checked = <>{flag && (condition ? <Yes /> : <No />)}</>;",
            "const attribute = <Panel value={condition ? yes : no} />;",
        ]
        .join("\n");
        let diagnostics = run(
            &tsx,
            Some("fixture.tsx"),
            &json!(["always", { "ignoreJSX": true }]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["expectedTestCons", "expectedConsAlt"]
        );
    }

    #[test]
    fn always_multiline_uses_the_conditional_node_span_not_outer_parentheses() {
        assert!(
            run(
                "(\ncondition ? consequent : alternate\n)",
                None,
                &json!(["always-multiline"])
            )
            .is_empty()
        );
        assert_eq!(
            run(
                "condition &&\nother ? consequent : alternate",
                None,
                &json!(["always-multiline"])
            )
            .len(),
            2
        );
    }

    #[test]
    fn never_preserves_two_atomic_removals_as_one_upstream_fix() {
        let source = "condition\n?\nconsequent : alternate";
        let diagnostics = run(source, None, &json!(["never"]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].suggestions[0].fixes.len(), 2);
        let merged = merged_fix(source, &diagnostics[0]).expect("merged fix");
        assert_eq!(
            merged,
            LintFix::replace_range(
                TextRange::new("condition".len() as u32, "condition\n?\n".len() as u32),
                "?"
            )
        );
    }

    #[test]
    fn nested_diagnostics_follow_upstream_parent_before_child_order() {
        let diagnostics = run("a ? b ? c : d : e", None, &json!(["always"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                "expectedTestCons",
                "expectedConsAlt",
                "expectedTestCons",
                "expectedConsAlt",
            ]
        );
        assert!(diagnostics[1].range.end > diagnostics[2].range.end);
    }

    #[test]
    fn malformed_and_textual_lookalikes_do_not_report() {
        for source in [
            "condition ?",
            "condition ? consequent",
            "const text = 'condition ? yes : no';",
            "const template = `condition ? yes : no`;",
            "const regex = /condition ? yes : no/;",
            "// condition ? yes : no",
            "type Value = Condition extends true ? Yes : No;",
        ] {
            assert!(
                run(source, Some("fixture.ts"), &json!(["always"])).is_empty(),
                "{source}"
            );
        }
    }
}
