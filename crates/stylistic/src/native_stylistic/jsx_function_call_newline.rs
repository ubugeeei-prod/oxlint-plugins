//! Native implementation of stable `@stylistic/jsx-function-call-newline`.
//!
//! Oxc identifies direct JSX element and fragment call arguments. Parentheses
//! are unwrapped because ESLint's ESTree view erases them, while token-adjacent
//! line checks skip comments exactly like `SourceCode#getTokenBefore/After`.

use oxc_allocator::Allocator;
use oxc_ast::{
    Comment,
    ast::{Argument, CallExpression, Expression, NewExpression},
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE_NAME: &str = "jsx-function-call-newline";
const MESSAGE_ID: &str = "missingLineBreak";
const MESSAGE: &str = "Missing line break around JSX";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Always,
    Multiline,
}

impl Mode {
    fn from_options(options: &Value) -> Self {
        let option = match options {
            Value::Array(items) => items.first(),
            Value::Null => None,
            value => Some(value),
        }
        .and_then(Value::as_str);

        if option == Some("always") {
            Self::Always
        } else {
            Self::Multiline
        }
    }
}

pub(crate) fn check_jsx_function_call_newline(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, options, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, options, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = JsxFunctionCallNewline {
        source,
        comments: &parsed.program.comments,
        mode: Mode::from_options(options),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct JsxFunctionCallNewline<'source, 'comments, 'diagnostics> {
    source: &'source str,
    comments: &'comments [Comment],
    mode: Mode,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxFunctionCallNewline<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        self.check_arguments(&call.arguments);
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, new_expression: &NewExpression<'ast>) {
        self.check_arguments(&new_expression.arguments);
        walk::walk_new_expression(self, new_expression);
    }
}

impl JsxFunctionCallNewline<'_, '_, '_> {
    fn check_arguments(&mut self, arguments: &[Argument<'_>]) {
        for argument in arguments {
            let Some(span) = jsx_span(argument) else {
                continue;
            };
            if self.mode == Mode::Multiline && is_same_line(self.source, span.start, span.end) {
                continue;
            }

            let needs_opening = previous_token_end(self.source, self.comments, span.start)
                .is_some_and(|previous_end| is_same_line(self.source, previous_end, span.start));
            let needs_closing =
                next_token(self.source, self.comments, span.end).is_some_and(|next| {
                    next.value != ',' && is_same_line(self.source, span.end, next.end)
                });
            if !needs_opening && !needs_closing {
                continue;
            }

            let Some(text) = source_span(self.source, span) else {
                continue;
            };
            let mut replacement = String::with_capacity(
                text.len() + usize::from(needs_opening) + usize::from(needs_closing),
            );
            if needs_opening {
                replacement.push('\n');
            }
            replacement.push_str(text);
            if needs_closing {
                replacement.push('\n');
            }

            let range = TextRange::new(span.start, span.end);
            self.diagnostics.push(LintDiagnostic {
                rule_name: RULE_NAME.to_owned(),
                message_id: MESSAGE_ID.to_owned(),
                message: MESSAGE.to_owned(),
                data: std::collections::BTreeMap::new(),
                range,
                suggestions: std::iter::once(LintSuggestion {
                    message_id: MESSAGE_ID.to_owned(),
                    message: MESSAGE.to_owned(),
                    fixes: std::iter::once(LintFix::replace_range(range, replacement)).collect(),
                })
                .collect(),
            });
        }
    }
}

fn jsx_span(argument: &Argument<'_>) -> Option<Span> {
    jsx_expression_span(argument.as_expression()?)
}

fn jsx_expression_span(expression: &Expression<'_>) -> Option<Span> {
    match expression {
        Expression::JSXElement(element) => Some(element.span),
        Expression::JSXFragment(fragment) => Some(fragment.span),
        Expression::ParenthesizedExpression(parenthesized) => {
            jsx_expression_span(&parenthesized.expression)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct NextToken {
    value: char,
    end: u32,
}

fn previous_token_end(source: &str, comments: &[Comment], start: u32) -> Option<u32> {
    let mut cursor = usize::try_from(start).ok()?;
    loop {
        cursor = trim_whitespace_back(source, cursor)?;
        if let Some(comment) = comments
            .iter()
            .find(|comment| usize::try_from(comment.span.end).ok() == Some(cursor))
        {
            cursor = usize::try_from(comment.span.start).ok()?;
            continue;
        }
        return (cursor > 0).then(|| u32::try_from(cursor).ok()).flatten();
    }
}

fn next_token(source: &str, comments: &[Comment], end: u32) -> Option<NextToken> {
    let mut cursor = usize::try_from(end).ok()?;
    loop {
        cursor = trim_whitespace_forward(source, cursor)?;
        if let Some(comment) = comments
            .iter()
            .find(|comment| usize::try_from(comment.span.start).ok() == Some(cursor))
        {
            cursor = usize::try_from(comment.span.end).ok()?;
            continue;
        }
        let character = source.get(cursor..)?.chars().next()?;
        let token_end = cursor.checked_add(character.len_utf8())?;
        return Some(NextToken {
            value: character,
            end: u32::try_from(token_end).ok()?,
        });
    }
}

fn trim_whitespace_back(source: &str, mut cursor: usize) -> Option<usize> {
    while cursor > 0 {
        let character = source.get(..cursor)?.chars().next_back()?;
        if !character.is_whitespace() {
            break;
        }
        cursor = cursor.checked_sub(character.len_utf8())?;
    }
    Some(cursor)
}

fn trim_whitespace_forward(source: &str, mut cursor: usize) -> Option<usize> {
    while cursor < source.len() {
        let character = source.get(cursor..)?.chars().next()?;
        if !character.is_whitespace() {
            break;
        }
        cursor = cursor.checked_add(character.len_utf8())?;
    }
    Some(cursor)
}

fn is_same_line(source: &str, start: u32, end: u32) -> bool {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return false;
    };
    source.get(start..end).is_some_and(|between| {
        !between
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

fn source_span(source: &str, span: Span) -> Option<&str> {
    source.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "JSON fixture assertions and generated boundary matrices are clearest with macros"
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
    #[serde(rename_all = "camelCase")]
    struct Generated {
        version: String,
        commit: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        fixable_invalid: usize,
        unfixable_invalid: usize,
        total: usize,
    }

    #[derive(Deserialize)]
    struct FixtureCase {
        code: String,
        #[serde(default)]
        options: Value,
        #[serde(default)]
        errors: Vec<ExpectedError>,
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedError {
        message_id: String,
        message: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-function-call-newline-v5.10.0.json"
        ))
        .expect("generated jsx-function-call-newline fixture is valid JSON")
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_function_call_newline(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            let start = usize::try_from(fix.range.start).expect("fix start fits usize");
            let end = usize::try_from(fix.range.end).expect("fix end fits usize");
            output.replace_range(start..end, &fix.replacement_text);
        }
        Some(output)
    }

    #[test]
    fn pinned_inventory_is_complete_and_deterministic() {
        let fixture = fixture();
        assert_eq!(fixture.generated.version, "v5.10.0");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.valid, 19);
        assert_eq!(fixture.generated.inventory.invalid, 8);
        assert_eq!(fixture.generated.inventory.diagnostics, 13);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 8);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 0);
        assert_eq!(fixture.generated.inventory.total, 27);
    }

    #[test]
    fn accepts_every_upstream_valid_case_in_jsx_and_tsx() {
        for (index, test) in fixture().valid.iter().enumerate() {
            for filename in ["fixture.jsx", "fixture.tsx"] {
                let diagnostics = run(&test.code, Some(filename), &test.options);
                assert!(
                    diagnostics.is_empty(),
                    "upstream valid case {index} reported for {filename}:\n{}\n{diagnostics:#?}",
                    test.code
                );
            }
        }
    }

    #[test]
    fn replays_every_upstream_invalid_case_with_exact_fixes_and_convergence() {
        for (index, test) in fixture().invalid.iter().enumerate() {
            for filename in ["fixture.jsx", "fixture.tsx"] {
                let diagnostics = run(&test.code, Some(filename), &test.options);
                assert_eq!(
                    diagnostics.len(),
                    test.errors.len(),
                    "diagnostic count mismatch for case {index}, {filename}:\n{}",
                    test.code
                );
                for (diagnostic, expected) in diagnostics.iter().zip(&test.errors) {
                    assert_eq!(diagnostic.rule_name, RULE_NAME);
                    assert_eq!(diagnostic.message_id, expected.message_id);
                    assert_eq!(diagnostic.message, expected.message);
                    assert!(diagnostic.data.is_empty());
                    assert_eq!(diagnostic.suggestions.len(), 1);
                    assert_eq!(diagnostic.suggestions[0].message_id, expected.message_id);
                    assert_eq!(diagnostic.suggestions[0].message, expected.message);
                    assert_eq!(diagnostic.suggestions[0].fixes.len(), 1);
                    let text = source_span(
                        &test.code,
                        Span::new(diagnostic.range.start, diagnostic.range.end),
                    )
                    .expect("diagnostic range is a source boundary");
                    assert!(
                        (text.starts_with('<') && text.ends_with('>'))
                            || (text.starts_with("<>") && text.ends_with("</>")),
                        "diagnostic range must cover exactly one JSX argument: {text:?}"
                    );
                    assert_eq!(diagnostic.suggestions[0].fixes[0].range, diagnostic.range);
                }
                let expected_output = test.output.as_deref();
                assert_eq!(
                    apply_fixes(&test.code, &diagnostics).as_deref(),
                    expected_output,
                    "first-pass output mismatch for case {index}, {filename}"
                );
                assert!(
                    run(
                        expected_output.expect("all pinned invalid cases are fixable"),
                        Some(filename),
                        &test.options
                    )
                    .is_empty(),
                    "fixed case {index} did not converge for {filename}"
                );
            }
        }
    }

    #[test]
    fn preserves_unicode_byte_ranges_crlf_and_tsx_syntax() {
        let source = concat!(
            "const prefix = \"😀\";\r\n",
            "declare const value: unknown;\r\n",
            "consume(<外側<T> value={value satisfies T}\r\n",
            "  label=\"日本語\" />);"
        );
        let diagnostics = run(source, Some("fixture.tsx"), &json!([]));
        assert_eq!(diagnostics.len(), 1);
        let start = u32::try_from(source.find("<外側").expect("JSX start")).expect("start fits");
        let end = u32::try_from(source.find("/>);").expect("JSX end") + 2).expect("end fits");
        assert_eq!(diagnostics[0].range, TextRange::new(start, end));
        assert_eq!(
            apply_fixes(source, &diagnostics).as_deref(),
            Some(concat!(
                "const prefix = \"😀\";\r\n",
                "declare const value: unknown;\r\n",
                "consume(\n<外側<T> value={value satisfies T}\r\n",
                "  label=\"日本語\" />\n);"
            ))
        );
    }

    #[test]
    fn unknown_and_malformed_options_fall_back_to_multiline() {
        let single_line = "fn(<div />)";
        let multiline = "fn(<div\n />)";
        for options in [
            Value::Null,
            json!([]),
            json!(["unknown"]),
            json!([false]),
            json!([{}]),
            json!("invalid"),
        ] {
            assert!(run(single_line, Some("fixture.jsx"), &options).is_empty());
            assert_eq!(
                run(multiline, Some("fixture.jsx"), &options)
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                [MESSAGE_ID]
            );
        }
    }

    #[test]
    fn handles_fragments_comments_parentheses_and_ecmascript_line_terminators() {
        let source = "fn(/* before */(<>日本語</>)/* after */)";
        let diagnostics = run(source, Some("fixture.tsx"), &json!(["always"]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            source_span(
                source,
                Span::new(diagnostics[0].range.start, diagnostics[0].range.end)
            ),
            Some("<>日本語</>")
        );
        assert_eq!(
            apply_fixes(source, &diagnostics).as_deref(),
            Some("fn(/* before */(\n<>日本語</>\n)/* after */)")
        );

        for terminator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("fn({terminator}<div />{terminator})");
            assert!(
                run(&source, Some("fixture.jsx"), &json!(["always"])).is_empty(),
                "terminator {terminator:?} was not recognized"
            );
        }
    }

    #[test]
    fn preserves_outer_before_nested_call_report_order() {
        let source = "outer(<A>{inner(<B />)}</A>)";
        let diagnostics = run(source, Some("fixture.tsx"), &json!(["always"]));
        assert_eq!(diagnostics.len(), 2);
        let ranges = diagnostics
            .iter()
            .map(|diagnostic| {
                source_span(
                    source,
                    Span::new(diagnostic.range.start, diagnostic.range.end),
                )
                .expect("diagnostic source")
            })
            .collect::<Vec<_>>();
        assert_eq!(ranges, ["<A>{inner(<B />)}</A>", "<B />"]);
    }

    #[test]
    fn ignores_non_direct_jsx_arguments_and_parse_failures() {
        for source in [
            "fn(() => <div />)",
            "fn(condition ? <A /> : <B />)",
            "fn({ value: <div /> })",
            "fn(...[<div />])",
            "fn(<div)",
        ] {
            assert!(
                run(source, Some("fixture.tsx"), &json!(["always"])).is_empty(),
                "unexpected diagnostic for {source}"
            );
        }
    }
}
