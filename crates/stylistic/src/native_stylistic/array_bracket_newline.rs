//! Native implementation of `@stylistic/array-bracket-newline`.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrayExpression, ArrayPattern};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;
use super::lexer::{Token, TokenKind, tokenize};

const RULE: &str = "array-bracket-newline";

#[derive(Clone, Copy)]
struct Options {
    consistent: bool,
    multiline: bool,
    min_items: usize,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let Some(option) = first_option(options) else {
            return Self {
                consistent: false,
                multiline: true,
                min_items: usize::MAX,
            };
        };

        if let Some(keyword) = option.as_str() {
            return match keyword {
                "always" => Self {
                    consistent: false,
                    multiline: false,
                    min_items: 0,
                },
                "consistent" => Self {
                    consistent: true,
                    multiline: false,
                    min_items: usize::MAX,
                },
                _ => Self {
                    consistent: false,
                    multiline: false,
                    min_items: usize::MAX,
                },
            };
        }

        let min_items = option.get("minItems").and_then(Value::as_u64);
        if min_items == Some(0) {
            return Self {
                consistent: false,
                multiline: false,
                min_items: 0,
            };
        }
        Self {
            consistent: false,
            multiline: option
                .get("multiline")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            min_items: min_items
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(usize::MAX),
        }
    }
}

pub(crate) fn check_array_bracket_newline(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let normalized = Options::from_json(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|value| SourceType::from_path(value).ok()) {
        let _ = parse_and_check(source, source_type, &tokens, normalized, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::ts(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, &tokens, normalized, diagnostics) {
                break;
            }
        }
    }

    diagnostics[first_diagnostic..].sort_by_key(|diagnostic| {
        (
            diagnostic.range.start,
            diagnostic.range.end,
            diagnostic.message_id.clone(),
        )
    });
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    tokens: &[Token],
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = ArrayBracketNewline {
        source,
        tokens,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct ArrayBracketNewline<'source, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for ArrayBracketNewline<'_, '_> {
    fn visit_array_expression(&mut self, array: &ArrayExpression<'ast>) {
        self.check(array.span, array.elements.len());
        walk::walk_array_expression(self, array);
    }

    fn visit_array_pattern(&mut self, array: &ArrayPattern<'ast>) {
        self.check(
            array.span,
            array.elements.len() + usize::from(array.rest.is_some()),
        );
        walk::walk_array_pattern(self, array);
    }
}

impl ArrayBracketNewline<'_, '_> {
    fn check(&mut self, span: Span, element_count: usize) {
        let Some(array) = ArrayTokens::find(self.tokens, span, self.source) else {
            return;
        };
        let needs_linebreaks = element_count >= self.options.min_items
            || (self.options.multiline
                && element_count > 0
                && !same_line(
                    self.source,
                    self.tokens,
                    array.last_including_comment,
                    array.first_including_comment,
                ))
            || (element_count == 0
                && array.first_including_comment == array.last_including_comment
                && self.tokens[array.first_including_comment].kind == TokenKind::BlockComment
                && !same_line(
                    self.source,
                    self.tokens,
                    array.last_including_comment,
                    array.first_including_comment,
                ))
            || (self.options.consistent
                && !same_line(
                    self.source,
                    self.tokens,
                    array.open,
                    array.first_significant,
                ));

        if needs_linebreaks {
            if same_line(
                self.source,
                self.tokens,
                array.open,
                array.first_significant,
            ) {
                self.report(
                    array.open,
                    "missingOpeningLinebreak",
                    "A linebreak is required after '['.",
                    Some(LintFix::replace_range(
                        byte_range(self.tokens[array.open].end, self.tokens[array.open].end),
                        "\n",
                    )),
                );
            }
            if same_line(
                self.source,
                self.tokens,
                array.last_significant,
                array.close,
            ) {
                self.report(
                    array.close,
                    "missingClosingLinebreak",
                    "A linebreak is required before ']'.",
                    Some(LintFix::replace_range(
                        byte_range(
                            self.tokens[array.close].start,
                            self.tokens[array.close].start,
                        ),
                        "\n",
                    )),
                );
            }
        } else {
            if !same_line(
                self.source,
                self.tokens,
                array.open,
                array.first_significant,
            ) {
                let next = array.open + 1;
                let fix = (!self.tokens[next].kind.is_comment()).then(|| {
                    LintFix::remove_range(byte_range(
                        self.tokens[array.open].end,
                        self.tokens[next].start,
                    ))
                });
                self.report(
                    array.open,
                    "unexpectedOpeningLinebreak",
                    "There should be no linebreak after '['.",
                    fix,
                );
            }
            if !same_line(
                self.source,
                self.tokens,
                array.last_significant,
                array.close,
            ) {
                let previous = array.close - 1;
                let fix = (!self.tokens[previous].kind.is_comment()).then(|| {
                    LintFix::remove_range(byte_range(
                        self.tokens[previous].end,
                        self.tokens[array.close].start,
                    ))
                });
                self.report(
                    array.close,
                    "unexpectedClosingLinebreak",
                    "There should be no linebreak before ']'.",
                    fix,
                );
            }
        }
    }

    fn report(
        &mut self,
        token_index: usize,
        message_id: &str,
        message: &str,
        fix: Option<LintFix>,
    ) {
        let token = &self.tokens[token_index];
        let suggestions = fix
            .map(|fix| LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: byte_range(token.start, token.end),
            suggestions,
        });
    }
}

#[derive(Clone, Copy)]
struct ArrayTokens {
    open: usize,
    close: usize,
    first_including_comment: usize,
    last_including_comment: usize,
    first_significant: usize,
    last_significant: usize,
}

impl ArrayTokens {
    fn find(tokens: &[Token], span: Span, source: &str) -> Option<Self> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let open = tokens
            .iter()
            .position(|token| token.start == start && token.text(source) == "[")?;
        let close = tokens
            .iter()
            .rposition(|token| token.end == end && token.text(source) == "]")?;
        if open >= close {
            return None;
        }
        let first_significant =
            (open + 1..=close).find(|&index| !tokens[index].kind.is_comment())?;
        let last_significant = (open..close)
            .rev()
            .find(|&index| !tokens[index].kind.is_comment())?;
        Some(Self {
            open,
            close,
            first_including_comment: open + 1,
            last_including_comment: close - 1,
            first_significant,
            last_significant,
        })
    }
}

fn same_line(source: &str, tokens: &[Token], left: usize, right: usize) -> bool {
    let left_end = tokens[left].end;
    let right_start = tokens[right].start;
    line_number(source, left_end) == line_number(source, right_start)
}

fn line_number(source: &str, end: usize) -> usize {
    source[..end.min(source.len())]
        .chars()
        .filter(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        .count()
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the upstream option matrix readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_array_bracket_newline(source, None, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(source: &str, options: Value) -> Vec<String> {
        run(source, options)
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    #[test]
    fn covers_default_always_never_and_consistent_modes() {
        assert!(run("const value = [1, 2];", Value::Null).is_empty());
        assert_eq!(
            ids("const value = [1, 2];", json!(["always"])),
            ["missingOpeningLinebreak", "missingClosingLinebreak"]
        );
        assert_eq!(
            ids("const value = [\n1,\n2\n];", json!(["never"])),
            ["unexpectedOpeningLinebreak", "unexpectedClosingLinebreak"]
        );
        assert!(run("const value = [\n1,\n2\n];", json!(["consistent"])).is_empty());
    }

    #[test]
    fn covers_multiline_min_items_patterns_comments_and_nested_arrays() {
        assert_eq!(
            ids("const value = [1,\n2];", Value::Null),
            ["missingOpeningLinebreak", "missingClosingLinebreak"]
        );
        assert_eq!(
            ids("const value = [1, 2];", json!([{ "minItems": 2 }])),
            ["missingOpeningLinebreak", "missingClosingLinebreak"]
        );
        assert_eq!(
            ids("const [a, b] = value;", json!([{ "minItems": 2 }])),
            ["missingOpeningLinebreak", "missingClosingLinebreak"]
        );
        assert!(run("const value = [/*\ncomment\n*/];", Value::Null).is_empty());
        assert_eq!(ids("const value = [[1, 2]];", json!(["always"])).len(), 4);
    }

    #[test]
    fn preserves_comment_no_fix_cases_and_utf8_ranges() {
        let diagnostics = run("const 日本語 = [\n// comment\n1];", json!(["never"]));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggestions.is_empty());
        assert_eq!(
            diagnostics[0].range.start,
            u32::try_from("const 日本語 = ".len()).unwrap()
        );
    }

    #[test]
    fn ignores_member_access_types_and_parse_failures() {
        for source in [
            "const value = object[\nkey\n];",
            "type Tuple = [\nstring,\nnumber\n];",
            "interface Box { [\nkey: string\n]: number }",
            "const broken = [",
        ] {
            assert!(run(source, json!(["never"])).is_empty(), "{source}");
        }
    }

    #[test]
    fn accepts_script_javascript_typescript_and_tsx() {
        assert_eq!(
            ids("with (scope) { value = [1, 2]; }", json!(["always"])).len(),
            2
        );
        assert_eq!(
            ids("const value: number[] = [1, 2];", json!(["always"])).len(),
            2
        );
        assert_eq!(
            ids("const view = <Box value={[1, 2]} />;", json!(["always"])).len(),
            2
        );
    }
}
