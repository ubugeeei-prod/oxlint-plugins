//! Native implementation of `@stylistic/member-delimiter-style`.
//!
//! TypeScript's parsed interface/type-literal members provide the structural
//! boundary. The shared stylistic lexer supplies exact delimiter and trailing
//! comment ranges, including the upstream rule's deliberately conservative
//! no-delimiter fix policy.

use oxc_allocator::Allocator;
use oxc_ast::ast::{TSInterfaceBody, TSSignature, TSTypeLiteral};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;
use super::lexer::{Token, tokenize};

const RULE: &str = "member-delimiter-style";
const UNEXPECTED_COMMA: &str = "Unexpected separator (,).";
const UNEXPECTED_SEMI: &str = "Unexpected separator (;).";
const EXPECTED_COMMA: &str = "Expected a comma.";
const EXPECTED_SEMI: &str = "Expected a semicolon.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Delimiter {
    None,
    Semi,
    Comma,
}

#[derive(Clone, Copy, Debug)]
struct MemberPolicy {
    delimiter: Delimiter,
    require_last: bool,
}

#[derive(Clone, Copy, Debug)]
struct ContainerPolicy {
    multiline: MemberPolicy,
    singleline: MemberPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MultilineDetection {
    Brackets,
    LastMember,
}

#[derive(Clone, Copy, Debug)]
struct Options {
    interface: ContainerPolicy,
    type_literal: ContainerPolicy,
    multiline_detection: MultilineDetection,
}

impl Options {
    fn from_value(options: &Value) -> Self {
        let default = ContainerPolicy {
            multiline: MemberPolicy {
                delimiter: Delimiter::Semi,
                require_last: true,
            },
            singleline: MemberPolicy {
                delimiter: Delimiter::Semi,
                require_last: false,
            },
        };
        let root = first_option(options);
        let base = root.map_or(default, |value| merge_container(default, value));
        let overrides = root.and_then(|value| value.get("overrides"));
        let interface = overrides
            .and_then(|value| value.get("interface"))
            .map_or(base, |value| merge_container(base, value));
        let type_literal = overrides
            .and_then(|value| value.get("typeLiteral"))
            .map_or(base, |value| merge_container(base, value));
        let multiline_detection = if root
            .and_then(|value| value.get("multilineDetection"))
            .and_then(Value::as_str)
            == Some("last-member")
        {
            MultilineDetection::LastMember
        } else {
            MultilineDetection::Brackets
        };
        Self {
            interface,
            type_literal,
            multiline_detection,
        }
    }
}

fn merge_container(base: ContainerPolicy, value: &Value) -> ContainerPolicy {
    ContainerPolicy {
        multiline: value
            .get("multiline")
            .map_or(base.multiline, |value| merge_member(base.multiline, value)),
        singleline: value.get("singleline").map_or(base.singleline, |value| {
            merge_member(base.singleline, value)
        }),
    }
}

fn merge_member(base: MemberPolicy, value: &Value) -> MemberPolicy {
    let delimiter = match value.get("delimiter").and_then(Value::as_str) {
        Some("none") => Delimiter::None,
        Some("comma") => Delimiter::Comma,
        Some("semi") => Delimiter::Semi,
        _ => base.delimiter,
    };
    MemberPolicy {
        delimiter,
        require_last: value
            .get("requireLast")
            .and_then(Value::as_bool)
            .unwrap_or(base.require_last),
    }
}

pub(crate) fn check_member_delimiter_style(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let options = Options::from_value(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, &tokens, options, diagnostics);
    } else {
        for source_type in [SourceType::ts(), SourceType::tsx()] {
            if parse_and_check(source, source_type, &tokens, options, diagnostics) {
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

    let mut visitor = MemberDelimiterStyle {
        source,
        tokens,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct MemberDelimiterStyle<'source, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for MemberDelimiterStyle<'_, '_> {
    fn visit_ts_interface_body(&mut self, body: &TSInterfaceBody<'ast>) {
        self.check_container(body.span, &body.body, self.options.interface);
        walk::walk_ts_interface_body(self, body);
    }

    fn visit_ts_type_literal(&mut self, literal: &TSTypeLiteral<'ast>) {
        self.check_container(literal.span, &literal.members, self.options.type_literal);
        walk::walk_ts_type_literal(self, literal);
    }
}

impl MemberDelimiterStyle<'_, '_> {
    fn check_container(
        &mut self,
        span: Span,
        members: &[TSSignature<'_>],
        policy: ContainerPolicy,
    ) {
        let mut singleline = !has_line_terminator(slice_u32(self.source, span.start, span.end));
        if self.options.multiline_detection == MultilineDetection::LastMember
            && !singleline
            && let Some(last_member) = members.last()
        {
            singleline = line_at(self.source, last_member.span().end as usize)
                == line_at(self.source, span.end as usize);
        }
        let member_policy = if singleline {
            policy.singleline
        } else {
            policy.multiline
        };
        let last_index = members.len().saturating_sub(1);
        let close = self
            .tokens
            .iter()
            .rev()
            .find(|token| {
                token.start >= span.start as usize
                    && token.end <= span.end as usize
                    && token.text(self.source) == "}"
            })
            .map_or(span.end as usize, |token| token.start);
        for (index, member) in members.iter().enumerate() {
            let boundary = members
                .get(index + 1)
                .map_or(close, |next| next.span().start as usize);
            self.check_member(
                member.span(),
                boundary,
                member_policy,
                index == last_index,
                singleline,
            );
        }
    }

    fn check_member(
        &mut self,
        span: Span,
        boundary: usize,
        policy: MemberPolicy,
        is_last: bool,
        singleline: bool,
    ) {
        let Some(last_index) = self.last_significant_token(span, boundary) else {
            return;
        };
        let last = self.tokens[last_index];
        let actual = match last.text(self.source) {
            ";" => Some(Delimiter::Semi),
            "," => Some(Delimiter::Comma),
            _ => None,
        };
        let expected = if is_last && !policy.require_last {
            Delimiter::None
        } else {
            policy.delimiter
        };

        let mismatch = match (actual, expected) {
            (Some(Delimiter::Semi), Delimiter::Comma) => {
                Some(("expectedComma", EXPECTED_COMMA, FixKind::Replace(",")))
            }
            (Some(Delimiter::Semi), Delimiter::None) => {
                Some(("unexpectedSemi", UNEXPECTED_SEMI, FixKind::Remove))
            }
            (Some(Delimiter::Comma), Delimiter::Semi) => {
                Some(("expectedSemi", EXPECTED_SEMI, FixKind::Replace(";")))
            }
            (Some(Delimiter::Comma), Delimiter::None) => {
                Some(("unexpectedComma", UNEXPECTED_COMMA, FixKind::Remove))
            }
            (None, Delimiter::Semi) => Some(("expectedSemi", EXPECTED_SEMI, FixKind::Insert(";"))),
            (None, Delimiter::Comma) => {
                Some(("expectedComma", EXPECTED_COMMA, FixKind::Insert(",")))
            }
            _ => None,
        };
        let Some((message_id, message, fix_kind)) = mismatch else {
            return;
        };

        let fix = match fix_kind {
            FixKind::Insert(text) => {
                Some(LintFix::replace_range(byte_range(last.end, last.end), text))
            }
            FixKind::Replace(text) => Some(LintFix::replace_range(
                byte_range(last.start, last.end),
                text,
            )),
            FixKind::Remove => self
                .removal_is_safe(last_index, singleline)
                .then(|| LintFix::remove_range(byte_range(last.start, last.end))),
        };
        self.report(last.end, message_id, message, fix);
    }

    fn last_significant_token(&self, span: Span, boundary: usize) -> Option<usize> {
        let start = usize::try_from(span.start).ok()?;
        self.tokens
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, token)| {
                (!token.kind.is_comment() && token.start >= start && token.end <= boundary)
                    .then_some(index)
            })
    }

    fn removal_is_safe(&self, last_index: usize, singleline: bool) -> bool {
        if singleline {
            return true;
        }
        let last = self.tokens[last_index];
        if line_content_end(self.source, last.end) == last.end {
            return true;
        }

        let trailing_comment = self.tokens[last_index + 1..]
            .iter()
            .take_while(|token| token.kind.is_comment())
            .last();
        trailing_comment.is_some_and(|comment| {
            line_at(self.source, comment.end) > line_at(self.source, last.end)
                || line_content_end(self.source, comment.end) == comment.end
        })
    }

    fn report(
        &mut self,
        offset: usize,
        message_id: &'static str,
        message: &'static str,
        fix: Option<LintFix>,
    ) {
        let Ok(offset) = u32::try_from(offset) else {
            return;
        };
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: Default::default(),
            range: TextRange::new(offset, offset),
            suggestions: fix
                .map(|fix| LintSuggestion {
                    message_id: message_id.to_owned(),
                    message: message.to_owned(),
                    fixes: std::iter::once(fix).collect(),
                })
                .into_iter()
                .collect(),
        });
    }
}

#[derive(Clone, Copy)]
enum FixKind {
    Insert(&'static str),
    Replace(&'static str),
    Remove,
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

fn slice_u32(source: &str, start: u32, end: u32) -> &str {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return "";
    };
    source.get(start..end).unwrap_or("")
}

fn has_line_terminator(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn line_at(source: &str, offset: usize) -> usize {
    let end = offset.min(source.len());
    let mut lines = 0;
    let mut characters = source[..end].chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            lines += 1;
        } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
            lines += 1;
        }
    }
    lines
}

fn line_content_end(source: &str, offset: usize) -> usize {
    source
        .get(offset..)
        .and_then(|suffix| {
            suffix
                .char_indices()
                .find(|(_, character)| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
                .map(|(index, _)| offset + index)
        })
        .unwrap_or(source.len())
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the option and boundary regression matrix readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;
    use crate::{StylisticRuleConfig, StylisticRunConfig, run_stylistic_lint};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
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
        source_commit: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        unfixable_invalid: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCase {
        code: String,
        #[serde(default)]
        options: Value,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        recursive_output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        range: [u32; 2],
        loc: ExpectedLocation,
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedLocation {
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/member-delimiter-style-v5.10.0.json"
        ))
        .expect("generated upstream fixture is valid JSON")
    }

    fn run(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        run_stylistic_lint(
            source,
            &StylisticRunConfig {
                filename: Some("fixture.ts".to_owned()),
                rules: vec![StylisticRuleConfig {
                    name: RULE.to_owned(),
                    options: options.clone(),
                }],
            },
        )
        .expect("member-delimiter-style runs")
    }

    fn fixes(diagnostics: &[LintDiagnostic]) -> Vec<&LintFix> {
        diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect()
    }

    fn fixed_output(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = fixes(diagnostics);
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

    fn recursive_output(source: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, options);
            let Some(next) = fixed_output(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            assert_ne!(next, output, "fix pass must make progress");
            output = next;
            changed = true;
        }
        panic!("member-delimiter-style fixes did not converge");
    }

    fn position_at(source: &str, offset: u32) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        let mut bytes = 0;
        for character in source.chars() {
            if bytes >= offset as usize {
                break;
            }
            bytes += character.len_utf8();
            if matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }

    #[test]
    fn fixture_is_the_complete_pinned_stable_inventory() {
        let fixture = fixture();
        assert_eq!(fixture.generated.version, "5.10.0");
        assert_eq!(
            fixture.generated.source_commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.valid, 61);
        assert_eq!(fixture.generated.inventory.invalid, 99);
        assert_eq!(fixture.generated.inventory.diagnostics, 153);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 1);
    }

    #[test]
    fn replays_every_upstream_valid_case() {
        let fixture = fixture();
        for case in fixture.valid {
            assert!(
                run(&case.code, &case.options).is_empty(),
                "upstream valid case reported:\n{}",
                case.code
            );
        }
    }

    #[test]
    fn replays_every_upstream_invalid_diagnostic_fix_and_recursive_output() {
        let fixture = fixture();
        for case in fixture.invalid {
            let diagnostics = run(&case.code, &case.options);
            assert_eq!(
                diagnostics.len(),
                case.expected_diagnostics.len(),
                "diagnostic count mismatch:\n{}",
                case.code
            );
            for (actual, expected) in diagnostics.iter().zip(&case.expected_diagnostics) {
                assert_eq!(actual.message_id, expected.message_id, "{}", case.code);
                assert_eq!(actual.message, expected.message, "{}", case.code);
                assert_eq!(
                    [actual.range.start, actual.range.end],
                    expected.range,
                    "{}",
                    case.code
                );
                assert_eq!(
                    position_at(&case.code, actual.range.start),
                    (expected.loc.line, expected.loc.column),
                    "{}",
                    case.code
                );
                assert_eq!(expected.loc.line, expected.loc.end_line, "{}", case.code);
                assert_eq!(
                    expected.loc.column, expected.loc.end_column,
                    "{}",
                    case.code
                );
                let actual_fix = actual
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first());
                match (actual_fix, &expected.fix) {
                    (Some(actual), Some(expected)) => {
                        assert_eq!(
                            [actual.range.start, actual.range.end],
                            expected.range,
                            "{}",
                            case.code
                        );
                        assert_eq!(actual.replacement_text, expected.text, "{}", case.code);
                    }
                    (None, None) => {}
                    _ => panic!("fix availability mismatch:\n{}", case.code),
                }
            }
            assert_eq!(
                fixed_output(&case.code, &diagnostics),
                case.output,
                "single-pass output mismatch:\n{}",
                case.code
            );
            assert_eq!(
                recursive_output(&case.code, &case.options),
                case.recursive_output,
                "recursive output mismatch:\n{}",
                case.code
            );
        }
    }

    #[test]
    fn covers_all_signature_kinds_nested_literals_and_container_overrides() {
        let source = concat!(
            "interface 日本語 {\r\n",
            "  property: string,\r\n",
            "  method(): void,\r\n",
            "  (value: string): number,\r\n",
            "  new (value: string): 日本語,\r\n",
            "  [key: string]: unknown,\r\n",
            "  nested: { value: string; other(): void; },\r\n",
            "  nestedMultiline: {\r\n",
            "    value: string;\r\n",
            "    other(): void;\r\n",
            "  },\r\n",
            "}\r\n",
            "type Inline = { value: string, other(): void, };\r\n",
        );
        let options = json!([{
            "multiline": { "delimiter": "semi", "requireLast": true },
            "singleline": { "delimiter": "comma", "requireLast": false },
            "overrides": {
                "interface": {
                    "multiline": { "delimiter": "comma", "requireLast": true }
                },
                "typeLiteral": {
                    "multiline": { "delimiter": "none", "requireLast": true }
                }
            }
        }]);
        let diagnostics = run(source, &options);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                "expectedComma",
                "unexpectedSemi",
                "unexpectedSemi",
                "unexpectedSemi",
                "unexpectedComma"
            ]
        );
    }

    #[test]
    fn supports_last_member_detection_comments_and_unicode_line_terminators() {
        let source = concat!(
            "interface X {\u{2028}",
            "  première: string; /** fin */ deuxième: number;\u{2029}",
            "}\n",
            "type T = {\n  value: string; // end\n}\n",
        );
        let options = json!([{
            "multiline": { "delimiter": "none", "requireLast": true },
            "singleline": { "delimiter": "comma", "requireLast": false },
            "multilineDetection": "last-member"
        }]);
        let diagnostics = run(source, &options);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["unexpectedSemi", "unexpectedSemi", "unexpectedSemi"]
        );
        assert!(
            diagnostics.iter().any(
                |diagnostic| diagnostic.range.start as usize > source.find("première").unwrap()
            )
        );
        assert_eq!(fixes(&diagnostics).len(), 2);
    }

    #[test]
    fn last_member_detection_can_select_singleline_policy_for_multiline_brackets() {
        let source = "interface Last {\n  first: string; second: number; }";
        let diagnostics = run(
            source,
            &json!([{
                "multiline": { "delimiter": "semi", "requireLast": true },
                "singleline": { "delimiter": "comma", "requireLast": false },
                "multilineDetection": "last-member"
            }]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["expectedComma", "unexpectedSemi"]
        );
        assert_eq!(
            fixed_output(source, &diagnostics).as_deref(),
            Some("interface Last {\n  first: string, second: number }")
        );
    }

    #[test]
    fn leaves_malformed_and_non_typescript_sources_alone() {
        for source in [
            "const object = { value: 1, other: 2 };",
            "class Example { field = 1; method() {} }",
            "interface Broken { value:",
            "const view = <div>{';,'}</div>;",
        ] {
            assert!(run(source, &Value::Null).is_empty(), "{source}");
        }
    }
}
