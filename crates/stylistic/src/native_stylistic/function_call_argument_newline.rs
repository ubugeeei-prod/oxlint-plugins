//! Native implementation of `@stylistic/function-call-argument-newline`.
//!
//! Argument boundaries come from Oxc's AST, while comments between a comma and
//! the following argument come from parser trivia. This reproduces upstream's
//! token-based locations and its rule that an immediately preceding line
//! comment makes a diagnostic intentionally unfixable.

use oxc_allocator::Allocator;
use oxc_ast::{
    Comment,
    ast::{Argument, CallExpression, ImportExpression, NewExpression},
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE_NAME: &str = "function-call-argument-newline";
const MISSING_ID: &str = "missingLineBreak";
const MISSING_MESSAGE: &str = "There should be a line break after this argument.";
const UNEXPECTED_ID: &str = "unexpectedLineBreak";
const UNEXPECTED_MESSAGE: &str = "There should be no line break here.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Always,
    Never,
    Consistent,
}

impl Mode {
    fn from_options(options: &Value) -> Self {
        let option = match options {
            Value::Array(items) => items.first(),
            Value::Null => None,
            value => Some(value),
        }
        .and_then(Value::as_str);

        match option {
            Some("never") => Self::Never,
            Some("consistent") => Self::Consistent,
            _ => Self::Always,
        }
    }
}

#[derive(Clone, Copy)]
enum Expectation {
    SameLine,
    NewLine,
}

pub(crate) fn check_function_call_argument_newline(
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
            let mut visitor = FunctionCallArgumentNewlineVisitor {
                source_text,
                comments: &parsed.program.comments,
                mode: Mode::from_options(options),
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            return;
        }
    }
}

struct FunctionCallArgumentNewlineVisitor<'source, 'comments, 'diagnostics> {
    source_text: &'source str,
    comments: &'comments [Comment],
    mode: Mode,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for FunctionCallArgumentNewlineVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        self.check_arguments(call.arguments.iter().map(Argument::span));
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, new_expression: &NewExpression<'ast>) {
        self.check_arguments(new_expression.arguments.iter().map(Argument::span));
        walk::walk_new_expression(self, new_expression);
    }

    fn visit_import_expression(&mut self, import_expression: &ImportExpression<'ast>) {
        if let Some(options) = &import_expression.options {
            self.check_arguments([import_expression.source.span(), options.span()]);
        }
        walk::walk_import_expression(self, import_expression);
    }
}

impl FunctionCallArgumentNewlineVisitor<'_, '_, '_> {
    fn check_arguments(&mut self, arguments: impl IntoIterator<Item = Span>) {
        let mut arguments = arguments.into_iter();
        let Some(mut previous) = arguments.next() else {
            return;
        };
        let Some(mut current) = arguments.next() else {
            return;
        };

        let expectation = match self.mode {
            Mode::Always => Expectation::NewLine,
            Mode::Never => Expectation::SameLine,
            Mode::Consistent => {
                if is_same_line(self.source_text, previous.end, current.start) {
                    Expectation::SameLine
                } else {
                    Expectation::NewLine
                }
            }
        };

        self.check_pair(previous, current, expectation);
        previous = current;
        for next in arguments {
            current = next;
            self.check_pair(previous, current, expectation);
            previous = current;
        }
    }

    fn check_pair(&mut self, previous: Span, current: Span, expectation: Expectation) {
        let same_line = is_same_line(self.source_text, previous.end, current.start);
        let (message_id, message, replacement) = match expectation {
            Expectation::NewLine if same_line => (MISSING_ID, MISSING_MESSAGE, "\n"),
            Expectation::SameLine if !same_line => (UNEXPECTED_ID, UNEXPECTED_MESSAGE, " "),
            Expectation::NewLine | Expectation::SameLine => return,
        };

        let Some(token_before) =
            token_before_current(self.source_text, self.comments, previous.end, current.start)
        else {
            return;
        };
        let range = TextRange::new(token_before.end, current.start);
        let suggestions = if token_before.is_line_comment {
            Vec::new()
        } else {
            std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(range, replacement)).collect(),
            })
            .collect()
        };

        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: std::collections::BTreeMap::new(),
            range,
            suggestions,
        });
    }
}

#[derive(Clone, Copy)]
struct TokenBefore {
    end: u32,
    is_line_comment: bool,
}

fn token_before_current(
    source_text: &str,
    comments: &[Comment],
    previous_end: u32,
    current_start: u32,
) -> Option<TokenBefore> {
    let start = usize::try_from(previous_end).ok()?;
    let end = usize::try_from(current_start).ok()?;
    let between = source_text.get(start..end)?;
    let mut candidate = between
        .match_indices(',')
        .filter_map(|(relative, _)| {
            let offset = start.saturating_add(relative);
            let offset = u32::try_from(offset).ok()?;
            let inside_comment = comments
                .iter()
                .any(|comment| comment.span.start <= offset && offset < comment.span.end);
            (!inside_comment).then_some(TokenBefore {
                end: offset.saturating_add(1),
                is_line_comment: false,
            })
        })
        .max_by_key(|token| token.end);

    for comment in comments {
        if comment.span.start < previous_end || comment.span.end > current_start {
            continue;
        }
        let comment_token = TokenBefore {
            end: comment.span.end,
            is_line_comment: comment.is_line(),
        };
        if candidate.is_none_or(|token| comment_token.end > token.end) {
            candidate = Some(comment_token);
        }
    }

    candidate
}

fn is_same_line(source_text: &str, start: u32, end: u32) -> bool {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return false;
    };
    let Some(between) = source_text.get(start..end) else {
        return false;
    };
    !between
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "JSON options and generated line-terminator sources keep the compatibility matrix readable"
)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
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
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
    }

    fn upstream_fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/function-call-argument-newline.json"
        ))
        .expect("generated upstream fixture is valid JSON")
    }

    fn run(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_function_call_argument_newline(source, options, &mut diagnostics);
        diagnostics
    }

    fn location_at(source: &str, offset: u32) -> (usize, usize) {
        let offset = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(source.len());
        let prefix = &source[..offset];
        let line = prefix
            .chars()
            .filter(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
            .count()
            + 1;
        let line_start = prefix
            .char_indices()
            .rev()
            .find(|(_, character)| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
            .map_or(0, |(index, character)| index + character.len_utf8());
        let column = source[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>()
            + 1;
        (line, column)
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));

        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        output
    }

    #[test]
    fn replays_all_upstream_valid_cases() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.valid.len(), 32);

        for case in fixture.valid {
            let diagnostics = run(&case.code, &case.options);
            assert!(
                diagnostics.is_empty(),
                "upstream valid case reported:\n{}\n{diagnostics:#?}",
                case.code
            );
        }
    }

    #[test]
    fn replays_all_upstream_invalid_cases_with_exact_order_locations_and_fixes() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.invalid.len(), 32);
        assert_eq!(
            fixture.invalid.iter().flat_map(|case| &case.errors).count(),
            42
        );

        for case in fixture.invalid {
            let diagnostics = run(&case.code, &case.options);
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                case.errors
                    .iter()
                    .map(|error| error.message_id.as_str())
                    .collect::<Vec<_>>(),
                "message order mismatch for:\n{}",
                case.code
            );
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| (
                        location_at(&case.code, diagnostic.range.start),
                        location_at(&case.code, diagnostic.range.end),
                    ))
                    .collect::<Vec<_>>(),
                case.errors
                    .iter()
                    .map(|error| (
                        (error.line, error.column),
                        (error.end_line, error.end_column),
                    ))
                    .collect::<Vec<_>>(),
                "location mismatch for:\n{}",
                case.code
            );

            if let Some(output) = case.output {
                assert_eq!(
                    apply_fixes(&case.code, &diagnostics),
                    output,
                    "fix mismatch for:\n{}",
                    case.code
                );
            } else {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.suggestions.is_empty()),
                    "line-comment case must remain unfixable:\n{}",
                    case.code
                );
            }
        }
    }

    #[test]
    fn covers_typescript_tsx_scripts_and_each_supported_expression_kind() {
        let cases = [
            ("const value = fn<string>(first, second);", 1),
            ("const node = render(<One />, <Two />);", 1),
            ("with (context) { fn(first, second); }", 1),
            ("const value = object?.method(first, second);", 1),
            ("const value = object[key](first, second);", 1),
            (
                "class Child extends Base { constructor() { super(first, second); } }",
                1,
            ),
            ("const value = new Factory(first, second);", 1),
            (
                "const value = import('data.json', { with: { type: 'json' } });",
                1,
            ),
        ];

        for (source, expected) in cases {
            assert_eq!(
                run(source, &Value::Null).len(),
                expected,
                "wrong diagnostic count for {source}"
            );
        }
    }

    #[test]
    fn preserves_utf8_ranges_comments_and_ecmascript_line_terminators() {
        let utf8 = "fn('日本語', value)";
        let utf8_diagnostic = &run(utf8, &Value::Null)[0];
        let comma_end = utf8.find(',').expect("comma") + 1;
        let value_start = utf8.find("value").expect("value");
        assert_eq!(
            utf8_diagnostic.range,
            TextRange::new(comma_end as u32, value_start as u32)
        );

        let block = "fn(first, /* keep */\nsecond)";
        let block_diagnostic = &run(block, &serde_json::json!(["never"]))[0];
        assert_eq!(
            apply_fixes(block, std::slice::from_ref(block_diagnostic)),
            "fn(first, /* keep */ second)"
        );

        for separator in ["\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("fn(first,{separator}second)");
            let diagnostics = run(&source, &serde_json::json!(["never"]));
            assert_eq!(diagnostics.len(), 1, "separator {separator:?}");
            assert_eq!(apply_fixes(&source, &diagnostics), "fn(first, second)");
        }
    }

    #[test]
    fn preserves_preorder_and_avoids_non_call_false_positives() {
        let source = "outer(inner(a, b), middle(c, d), last)";
        let diagnostics = run(source, &Value::Null);
        assert_eq!(diagnostics.len(), 4);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range.start)
                .collect::<Vec<_>>(),
            [
                (source.find("), middle").expect("outer first comma") + 2) as u32,
                (source.find("), last").expect("outer second comma") + 2) as u32,
                (source.find("a, b").expect("inner comma") + 2) as u32,
                (source.find("c, d").expect("middle comma") + 2) as u32,
            ]
        );

        for source in [
            "function declaration(first, second) {}",
            "const array = [first, second];",
            "const object = { first, second };",
            "const tuple: [string, number] = ['first', 2];",
            "type Callback = (first: string, second: number) => void;",
            "const grouped = (first, second);",
        ] {
            assert!(
                run(source, &Value::Null).is_empty(),
                "false positive for {source}"
            );
        }
    }

    #[test]
    fn invalid_options_fall_back_to_always_and_parse_failures_are_silent() {
        for options in [
            Value::Null,
            serde_json::json!([]),
            serde_json::json!(["unknown"]),
            serde_json::json!([42]),
        ] {
            assert_eq!(run("fn(first, second)", &options).len(), 1);
        }
        assert!(run("fn(first,", &Value::Null).is_empty());
    }
}
