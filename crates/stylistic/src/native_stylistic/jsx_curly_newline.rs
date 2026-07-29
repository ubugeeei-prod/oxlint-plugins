//! Native implementation of `@stylistic/jsx-curly-newline`.
//!
//! Oxc identifies JSX expression containers and their contained expressions.
//! The shared lexer supplies the significant tokens immediately inside the
//! braces so removal fixes can reproduce upstream's comment-safe behaviour.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::JSXExpressionContainer;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;
use super::lexer::{Token, tokenize};

const RULE: &str = "jsx-curly-newline";

const EXPECTED_BEFORE: &str = "expectedBefore";
const EXPECTED_AFTER: &str = "expectedAfter";
const UNEXPECTED_BEFORE: &str = "unexpectedBefore";
const UNEXPECTED_AFTER: &str = "unexpectedAfter";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinebreakMode {
    Consistent,
    Require,
    Forbid,
}

impl LinebreakMode {
    fn from_value(value: Option<&Value>) -> Self {
        match value.and_then(Value::as_str) {
            Some("require") => Self::Require,
            Some("forbid") => Self::Forbid,
            _ => Self::Consistent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Options {
    singleline: LinebreakMode,
    multiline: LinebreakMode,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let Some(option) = first_option(options) else {
            return Self::consistent();
        };
        if option.as_str() == Some("never") {
            return Self {
                singleline: LinebreakMode::Forbid,
                multiline: LinebreakMode::Forbid,
            };
        }
        let Some(object) = option.as_object() else {
            return Self::consistent();
        };
        Self {
            singleline: LinebreakMode::from_value(object.get("singleline")),
            multiline: LinebreakMode::from_value(object.get("multiline")),
        }
    }

    const fn consistent() -> Self {
        Self {
            singleline: LinebreakMode::Consistent,
            multiline: LinebreakMode::Consistent,
        }
    }

    const fn mode_for(self, multiline: bool) -> LinebreakMode {
        if multiline {
            self.multiline
        } else {
            self.singleline
        }
    }
}

pub(crate) fn check_jsx_curly_newline(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let options = Options::from_json(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|value| SourceType::from_path(value).ok()) {
        let _ = parse_and_check(source, source_type, &tokens, options, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, &tokens, options, diagnostics) {
                break;
            }
        }
    }

    diagnostics[first_diagnostic..].sort_by_key(|diagnostic| {
        (
            diagnostic.range.start,
            diagnostic.range.end,
            message_order(&diagnostic.message_id),
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

    let mut visitor = JsxCurlyNewlineVisitor {
        source,
        tokens,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct JsxCurlyNewlineVisitor<'source, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxCurlyNewlineVisitor<'_, '_> {
    fn visit_jsx_expression_container(&mut self, container: &JSXExpressionContainer<'ast>) {
        self.check(container);
        walk::walk_jsx_expression_container(self, container);
    }
}

impl JsxCurlyNewlineVisitor<'_, '_> {
    fn check(&mut self, container: &JSXExpressionContainer<'_>) {
        let Some(curlys) = CurlyTokens::find(self.tokens, container.span, self.source) else {
            return;
        };
        let first = (curlys.left + 1..=curlys.right)
            .find(|&index| !self.tokens[index].kind.is_comment())
            .unwrap_or(curlys.right);
        let last = (curlys.left..curlys.right)
            .rev()
            .find(|&index| !self.tokens[index].kind.is_comment())
            .unwrap_or(curlys.left);
        let has_left_newline =
            !same_line(self.source, &self.tokens[curlys.left], &self.tokens[first]);
        let has_right_newline =
            !same_line(self.source, &self.tokens[last], &self.tokens[curlys.right]);
        let expression_span = container.expression.span();
        let expression_multiline = contains_line_terminator(
            self.source,
            usize::try_from(expression_span.start).unwrap_or(usize::MAX),
            usize::try_from(expression_span.end).unwrap_or(usize::MAX),
        );
        let needs_newlines = match self.options.mode_for(expression_multiline) {
            LinebreakMode::Require => true,
            LinebreakMode::Forbid => false,
            LinebreakMode::Consistent => has_left_newline,
        };

        if has_left_newline && !needs_newlines {
            let fix = (!has_comment_between(self.tokens, curlys.left, first)).then(|| {
                LintFix::remove_range(byte_range(
                    self.tokens[curlys.left].end,
                    self.tokens[first].start,
                ))
            });
            self.report(
                curlys.left,
                UNEXPECTED_AFTER,
                "Unexpected newline after '{'.",
                fix,
            );
        } else if !has_left_newline && needs_newlines {
            self.report(
                curlys.left,
                EXPECTED_AFTER,
                "Expected newline after '{'.",
                Some(LintFix::replace_range(
                    byte_range(self.tokens[curlys.left].end, self.tokens[curlys.left].end),
                    "\n",
                )),
            );
        }

        if has_right_newline && !needs_newlines {
            let fix = (!has_comment_between(self.tokens, last, curlys.right)).then(|| {
                LintFix::remove_range(byte_range(
                    self.tokens[last].end,
                    self.tokens[curlys.right].start,
                ))
            });
            self.report(
                curlys.right,
                UNEXPECTED_BEFORE,
                "Unexpected newline before '}'.",
                fix,
            );
        } else if !has_right_newline && needs_newlines {
            self.report(
                curlys.right,
                EXPECTED_BEFORE,
                "Expected newline before '}'.",
                Some(LintFix::replace_range(
                    byte_range(
                        self.tokens[curlys.right].start,
                        self.tokens[curlys.right].start,
                    ),
                    "\n",
                )),
            );
        }
    }

    fn report(
        &mut self,
        token_index: usize,
        message_id: &'static str,
        message: &'static str,
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
struct CurlyTokens {
    left: usize,
    right: usize,
}

impl CurlyTokens {
    fn find(tokens: &[Token], span: Span, source: &str) -> Option<Self> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let left = tokens
            .iter()
            .position(|token| token.start == start && token.text(source) == "{")?;
        let right = tokens
            .iter()
            .rposition(|token| token.end == end && token.text(source) == "}")?;
        (left < right).then_some(Self { left, right })
    }
}

fn has_comment_between(tokens: &[Token], left: usize, right: usize) -> bool {
    tokens[left.saturating_add(1)..right]
        .iter()
        .any(|token| token.kind.is_comment())
}

fn same_line(source: &str, left: &Token, right: &Token) -> bool {
    !contains_line_terminator(source, left.end, right.start)
}

fn contains_line_terminator(source: &str, start: usize, end: usize) -> bool {
    source
        .get(start.min(source.len())..end.min(source.len()))
        .is_some_and(|text| {
            text.chars()
                .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        })
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

const fn message_order(message_id: &str) -> u8 {
    match message_id.as_bytes() {
        b"unexpectedAfter" | b"expectedAfter" => 0,
        _ => 1,
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the upstream option matrix readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_curly_newline(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(source: &str, filename: Option<&str>, options: Value) -> Vec<String> {
        run(source, filename, options)
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    fn apply(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    #[test]
    fn covers_consistent_default_never_and_complete_object_modes() {
        assert!(run("<div>{foo}</div>", None, Value::Null).is_empty());
        assert!(run("<div>{\nfoo\n}</div>", None, json!(["consistent"])).is_empty());
        assert_eq!(
            ids("<div>{\nfoo\n}</div>", None, json!(["never"])),
            [UNEXPECTED_AFTER, UNEXPECTED_BEFORE]
        );
        assert_eq!(
            ids(
                "<div>{foo}</div>",
                None,
                json!([{ "singleline": "require", "multiline": "forbid" }])
            ),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert_eq!(
            ids(
                "<div>{\nfoo &&\nbar\n}</div>",
                None,
                json!([{ "singleline": "forbid", "multiline": "require" }])
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn reproduces_every_base_upstream_valid_case() {
        let cases = [
            ("<div>{foo}</div>", json!(["consistent"])),
            (
                "\n<div>\n  {\n    foo\n  }\n</div>\n",
                json!(["consistent"]),
            ),
            (
                "\n<div>\n  { foo &&\n    foo.bar }\n</div>\n",
                json!(["consistent"]),
            ),
            (
                "\n<div>\n  {\n    foo &&\n    foo.bar\n  }\n</div>\n",
                json!(["consistent"]),
            ),
            ("\n<div foo={\n  bar\n} />\n", json!(["consistent"])),
            (
                "<div>{foo}</div>",
                json!([{ "singleline": "consistent", "multiline": "require" }]),
            ),
            (
                "<div foo={bar} />",
                json!([{ "singleline": "consistent", "multiline": "require" }]),
            ),
            (
                "\n<div>\n  {\n    foo &&\n    foo.bar\n  }\n</div>\n",
                json!([{ "singleline": "consistent", "multiline": "require" }]),
            ),
            (
                "\n<div>\n  {\n    foo\n  }\n</div>\n",
                json!([{ "singleline": "consistent", "multiline": "require" }]),
            ),
            ("<div>{foo}</div>", json!(["never"])),
            ("<div foo={bar} />", json!(["never"])),
            (
                "\n<div>\n  { foo &&\n    foo.bar }\n</div>\n",
                json!(["never"]),
            ),
        ];
        for (source, options) in cases {
            assert!(
                run(source, Some("fixture.tsx"), options).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn matches_all_four_messages_ranges_and_exact_fixes() {
        let source = "<div>{foo &&\nbar}</div>";
        let diagnostics = run(
            source,
            Some("fixture.tsx"),
            json!([{ "singleline": "consistent", "multiline": "require" }]),
        );
        assert_eq!(
            ids(
                source,
                Some("fixture.tsx"),
                json!([{
                    "singleline": "consistent",
                    "multiline": "require"
                }])
            ),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [TextRange::new(5, 6), TextRange::new(16, 17)]
        );
        assert_eq!(
            apply(source, &diagnostics).as_deref(),
            Some("<div>{\nfoo &&\nbar\n}</div>")
        );

        let never = "<div>{\nfoo\n}</div>";
        let diagnostics = run(never, Some("fixture.jsx"), json!(["never"]));
        assert_eq!(
            ids(never, Some("fixture.jsx"), json!(["never"])),
            [UNEXPECTED_AFTER, UNEXPECTED_BEFORE]
        );
        assert_eq!(
            apply(never, &diagnostics).as_deref(),
            Some("<div>{foo}</div>")
        );
    }

    #[test]
    fn keeps_comment_removals_unfixable_but_insertions_fixable() {
        for (source, expected_id) in [
            ("<div>{ /* comment */\nfoo }</div>", UNEXPECTED_AFTER),
            ("<div>{ foo\n/* comment */ }</div>", UNEXPECTED_BEFORE),
        ] {
            let diagnostics = run(source, Some("fixture.tsx"), json!(["never"]));
            assert_eq!(diagnostics.len(), 1, "{source}");
            assert_eq!(diagnostics[0].message_id, expected_id, "{source}");
            assert!(diagnostics[0].suggestions.is_empty(), "{source}");
        }

        let source = "<div>{/* comment */ foo}</div>";
        let diagnostics = run(
            source,
            Some("fixture.tsx"),
            json!([{ "singleline": "require" }]),
        );
        assert_eq!(
            ids(
                source,
                Some("fixture.tsx"),
                json!([{ "singleline": "require" }])
            ),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert_eq!(
            apply(source, &diagnostics).as_deref(),
            Some("<div>{\n/* comment */ foo\n}</div>")
        );
    }

    #[test]
    fn handles_nested_children_attributes_fragments_spreads_and_empty_expressions() {
        let source = concat!(
            "<>",
            "<Outer value={\nfoo\n}>",
            "{\ncondition ? <Inner data={\nbar\n} /> : null\n}",
            "{/* comment-only */}",
            "</Outer>",
            "</>"
        );
        let diagnostics = run(source, Some("fixture.tsx"), json!(["never"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                UNEXPECTED_AFTER,
                UNEXPECTED_BEFORE,
                UNEXPECTED_AFTER,
                UNEXPECTED_AFTER,
                UNEXPECTED_BEFORE,
                UNEXPECTED_BEFORE,
            ]
        );
        assert!(
            run(
                "<App enabled {...props}>{}</App>",
                Some("fixture.tsx"),
                Value::Null
            )
            .is_empty()
        );
    }

    #[test]
    fn uses_expression_multiline_status_and_left_consistency_exactly() {
        let options = json!([{
            "singleline": "forbid",
            "multiline": "require"
        }]);
        assert_eq!(
            ids(
                "<div>{foo &&\nbar}</div>",
                Some("fixture.tsx"),
                options.clone()
            ),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert_eq!(
            ids(
                "<div>{\nfoo\n}</div>",
                Some("fixture.tsx"),
                json!([{
                    "singleline": "forbid",
                    "multiline": "forbid"
                }])
            ),
            [UNEXPECTED_AFTER, UNEXPECTED_BEFORE]
        );
        assert!(
            !run(
                "<div>{foo\n}</div>",
                Some("fixture.tsx"),
                json!(["consistent"])
            )
            .is_empty(),
            "consistent follows only the left boundary"
        );
        assert_eq!(
            ids(
                "<div>{foo\n}</div>",
                Some("fixture.tsx"),
                json!(["consistent"])
            ),
            [UNEXPECTED_BEFORE]
        );
    }

    #[test]
    fn supports_typescript_tsx_unicode_crlf_and_all_ecmascript_line_terminators() {
        let source = concat!(
            "const 日本語: JSX.Element = <div>{\r\n値\r\n}</div>;\r\n",
            "const café = <Comp value={\u{2028}élément\u{2028}} />;\u{2028}",
            "const τέλος = <>{\u{2029}κόσμος\u{2029}}</>;\u{2029}",
            "const emoji = <span>{\n'😀'\n}</span>;\n"
        );
        let diagnostics = run(source, Some("fixture.tsx"), json!(["never"]));
        assert_eq!(diagnostics.len(), 8);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.suggestions.len() == 1)
        );
        let first = &diagnostics[0];
        assert_eq!(
            &source[first.range.start as usize..first.range.end as usize],
            "{"
        );
        assert_eq!(
            apply(source, &diagnostics).as_deref(),
            Some(
                "const 日本語: JSX.Element = <div>{値}</div>;\r\nconst café = <Comp value={élément} />;\u{2028}const τέλος = <>{κόσμος}</>;\u{2029}const emoji = <span>{'😀'}</span>;\n"
            )
        );
    }

    #[test]
    fn ignores_plain_javascript_types_and_invalid_jsx_and_falls_back_for_bad_options() {
        for (source, filename) in [
            ("const value = {foo: '\\n'};", Some("fixture.js")),
            ("type Box<T> = { value: T };", Some("fixture.ts")),
            ("const value = <div>{foo</div>;", Some("fixture.tsx")),
            ("const value = <div>{foo}</span>;", Some("fixture.jsx")),
        ] {
            assert!(
                run(source, filename, json!(["never"])).is_empty(),
                "{source}"
            );
        }

        let source = "<div>{\nfoo\n}</div>";
        for options in [
            json!(["sideways"]),
            json!([42]),
            json!([{ "singleline": 12, "multiline": false }]),
            Value::Null,
        ] {
            assert!(
                run(source, Some("fixture.tsx"), options).is_empty(),
                "invalid options should fall back to consistent"
            );
        }
    }
}
