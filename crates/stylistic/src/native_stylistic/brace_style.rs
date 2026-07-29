//! Native implementation of stable `@stylistic/brace-style`.
//!
//! Oxc supplies the exact ESTree node set visited by the upstream rule. The
//! shared lexer supplies ESLint-compatible token/comment boundaries so the
//! whitespace-only fixes retain the upstream comment-safety behavior.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind, AstType,
    ast::{IfStatement, Statement, TryStatement},
};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::{
    context::first_option,
    lexer::{Token, TokenKind, tokenize},
};

const RULE: &str = "brace-style";
const NEXT_LINE_OPEN: &str =
    "Opening curly brace does not appear on the same line as controlling statement.";
const SAME_LINE_OPEN: &str =
    "Opening curly brace appears on the same line as controlling statement.";
const BLOCK_SAME_LINE: &str = "Statement inside of curly braces should be on next line.";
const NEXT_LINE_CLOSE: &str =
    "Closing curly brace does not appear on the same line as the subsequent block.";
const SINGLE_LINE_CLOSE: &str = "Closing curly brace should be on the same line as opening curly brace or on the line after the previous block.";
const SAME_LINE_CLOSE: &str =
    "Closing curly brace appears on the same line as the subsequent block.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    OneTrueBrace,
    Stroustrup,
    Allman,
}

#[derive(Clone, Copy)]
struct Options {
    style: Style,
    allow_single_line: bool,
}

impl Options {
    fn from_json(value: &Value) -> Self {
        let style = match first_option(value).and_then(Value::as_str) {
            Some("stroustrup") => Style::Stroustrup,
            Some("allman") => Style::Allman,
            _ => Style::OneTrueBrace,
        };
        let allow_single_line = value
            .as_array()
            .and_then(|items| items.get(1))
            .and_then(|option| option.get("allowSingleLine"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Self {
            style,
            allow_single_line,
        }
    }
}

pub(crate) fn check_brace_style(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let normalized = Options::from_json(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
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

    let mut visitor = BraceStyle {
        source,
        tokens,
        options,
        parents: Vec::new(),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct BraceStyle<'source, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: Options,
    parents: Vec<AstType>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for BraceStyle<'_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        let parent = self.parents.last().copied();
        match kind {
            AstKind::BlockStatement(block)
                if !matches!(
                    parent,
                    Some(
                        AstType::Program
                            | AstType::BlockStatement
                            | AstType::StaticBlock
                            | AstType::SwitchCase
                    )
                ) =>
            {
                self.validate_curly_pair(block.span, false);
            }
            AstKind::FunctionBody(body) => self.validate_curly_pair(body.span, false),
            AstKind::StaticBlock(block) => self.validate_curly_pair(block.span, true),
            AstKind::ClassBody(body) => self.validate_curly_pair(body.span, false),
            AstKind::SwitchStatement(statement) => {
                self.validate_curly_pair(statement.span, true);
            }
            AstKind::TSModuleBlock(block) => self.validate_curly_pair(block.span, false),
            AstKind::IfStatement(statement) => self.validate_if(statement),
            AstKind::TryStatement(statement) => self.validate_try(statement),
            _ => {}
        }
        self.parents.push(kind.ty());
    }

    fn leave_node(&mut self, _kind: AstKind<'ast>) {
        self.parents.pop();
    }
}

impl BraceStyle<'_, '_> {
    fn validate_if(&mut self, statement: &IfStatement<'_>) {
        if statement.alternate.is_none() {
            return;
        }
        if let Statement::BlockStatement(consequent) = &statement.consequent {
            self.validate_curly_before_keyword(consequent.span);
        }
    }

    fn validate_try(&mut self, statement: &TryStatement<'_>) {
        self.validate_curly_before_keyword(statement.block.span);
        if statement.finalizer.is_some()
            && let Some(handler) = &statement.handler
        {
            self.validate_curly_before_keyword(handler.body.span);
        }
    }

    fn validate_curly_pair(&mut self, span: Span, search_open: bool) {
        let Some((open, close)) = self.find_curly_pair(span, search_open) else {
            return;
        };
        let Some(before_open) = previous_significant(self.tokens, open) else {
            return;
        };
        let Some(after_open) = next_significant(self.tokens, open) else {
            return;
        };
        let Some(before_close) = previous_significant(self.tokens, close) else {
            return;
        };
        let single_line_exception = self.options.allow_single_line
            && same_line(
                self.source,
                self.tokens[open].start,
                self.tokens[close].start,
            );

        if self.options.style != Style::Allman
            && !same_line(
                self.source,
                self.tokens[before_open].start,
                self.tokens[open].start,
            )
        {
            let fix = self.safe_replacement(before_open, open, " ");
            self.report(open, "nextLineOpen", NEXT_LINE_OPEN, fix);
        }

        if self.options.style == Style::Allman
            && same_line(
                self.source,
                self.tokens[before_open].start,
                self.tokens[open].start,
            )
            && !single_line_exception
        {
            self.report(
                open,
                "sameLineOpen",
                SAME_LINE_OPEN,
                Some(LintFix::replace_range(
                    byte_range(self.tokens[open].start, self.tokens[open].start),
                    "\n",
                )),
            );
        }

        if after_open != close
            && same_line(
                self.source,
                self.tokens[open].start,
                self.tokens[after_open].start,
            )
            && !single_line_exception
        {
            self.report(
                open,
                "blockSameLine",
                BLOCK_SAME_LINE,
                Some(LintFix::replace_range(
                    byte_range(self.tokens[open].end, self.tokens[open].end),
                    "\n",
                )),
            );
        }

        if before_close != open
            && same_line(
                self.source,
                self.tokens[before_close].start,
                self.tokens[close].start,
            )
            && !single_line_exception
        {
            self.report(
                close,
                "singleLineClose",
                SINGLE_LINE_CLOSE,
                Some(LintFix::replace_range(
                    byte_range(self.tokens[close].start, self.tokens[close].start),
                    "\n",
                )),
            );
        }
    }

    fn validate_curly_before_keyword(&mut self, span: Span) {
        let Some((_, close)) = self.find_curly_pair(span, false) else {
            return;
        };
        let Some(keyword) = next_significant(self.tokens, close) else {
            return;
        };

        if self.options.style == Style::OneTrueBrace
            && !same_line(
                self.source,
                self.tokens[close].start,
                self.tokens[keyword].start,
            )
        {
            let fix = self.safe_replacement(close, keyword, " ");
            self.report(close, "nextLineClose", NEXT_LINE_CLOSE, fix);
        }

        if self.options.style != Style::OneTrueBrace
            && same_line(
                self.source,
                self.tokens[close].start,
                self.tokens[keyword].start,
            )
        {
            self.report(
                close,
                "sameLineClose",
                SAME_LINE_CLOSE,
                Some(LintFix::replace_range(
                    byte_range(self.tokens[close].end, self.tokens[close].end),
                    "\n",
                )),
            );
        }
    }

    fn find_curly_pair(&self, span: Span, search_open: bool) -> Option<(usize, usize)> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let open = self.tokens.iter().position(|token| {
            (search_open && token.start >= start || !search_open && token.start == start)
                && token.end <= end
                && token.kind == TokenKind::Punctuator
                && token.text(self.source) == "{"
        })?;
        let close = self.tokens.iter().rposition(|token| {
            token.start >= start
                && token.end <= end
                && token.kind == TokenKind::Punctuator
                && token.text(self.source) == "}"
        })?;
        (open < close).then_some((open, close))
    }

    fn safe_replacement(
        &self,
        left: usize,
        right: usize,
        replacement: &'static str,
    ) -> Option<LintFix> {
        if self.tokens[left + 1..right]
            .iter()
            .any(|token| token.kind.is_comment())
        {
            return None;
        }
        Some(LintFix::replace_range(
            byte_range(self.tokens[left].end, self.tokens[right].start),
            replacement,
        ))
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

fn previous_significant(tokens: &[Token], index: usize) -> Option<usize> {
    (0..index)
        .rev()
        .find(|&candidate| !tokens[candidate].kind.is_comment())
}

fn next_significant(tokens: &[Token], index: usize) -> Option<usize> {
    (index + 1..tokens.len()).find(|&candidate| !tokens[candidate].kind.is_comment())
}

fn same_line(source: &str, left: usize, right: usize) -> bool {
    let (start, end) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    !source[start.min(source.len())..end.min(source.len())]
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}'))
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

fn message_order(message_id: &str) -> u8 {
    match message_id {
        "nextLineOpen" | "sameLineOpen" => 0,
        "blockSameLine" => 1,
        "nextLineClose" | "sameLineClose" => 2,
        "singleLineClose" => 3,
        _ => 4,
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the stable option matrix concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    const FIXTURE: &str = include_str!("../../../../npm/stylistic/test/fixtures/brace-style.json");

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TestCase {
        code: String,
        language: String,
        #[serde(default)]
        options: Value,
        output: Option<String>,
        #[serde(default)]
        errors: Vec<ExpectedError>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedError {
        message_id: String,
        line: Option<usize>,
    }

    fn run(source: &str, filename: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_brace_style(source, Some(filename), options, &mut diagnostics);
        diagnostics
    }

    fn filename(test_case: &TestCase) -> &'static str {
        if test_case.language == "js" {
            "fixture.js"
        } else {
            "fixture.ts"
        }
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first())
            })
            .collect::<Option<Vec<_>>>()?;
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            let start = usize::try_from(fix.range.start).expect("start fits usize");
            let end = usize::try_from(fix.range.end).expect("end fits usize");
            output.replace_range(start..end, &fix.replacement_text);
        }
        Some(output)
    }

    fn line_at(source: &str, byte_offset: u32) -> usize {
        let end = usize::try_from(byte_offset).expect("offset fits usize");
        let mut line = 1;
        let mut chars = source[..end].chars().peekable();
        while let Some(character) = chars.next() {
            match character {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    line += 1;
                }
                '\n' | '\u{2028}' | '\u{2029}' => line += 1,
                _ => {}
            }
        }
        line
    }

    #[test]
    fn accepts_every_stable_upstream_valid_case_individually() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.valid.len(), 89);
        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, filename(test_case), &test_case.options).is_empty(),
                "upstream valid case {index} reported diagnostics:\n{}",
                test_case.code
            );
        }
    }

    #[test]
    fn replays_every_stable_upstream_invalid_case_and_exact_output() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.invalid.len(), 91);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.errors)
                .count(),
            130
        );
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, filename(test_case), &test_case.options);
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                test_case
                    .errors
                    .iter()
                    .map(|error| error.message_id.as_str())
                    .collect::<Vec<_>>(),
                "message IDs differ for upstream invalid case {index}:\n{}",
                test_case.code
            );
            for (diagnostic, expected) in diagnostics.iter().zip(&test_case.errors) {
                if let Some(line) = expected.line {
                    assert_eq!(
                        line_at(&test_case.code, diagnostic.range.start),
                        line,
                        "line differs for upstream invalid case {index}"
                    );
                }
            }
            assert_eq!(
                apply_fixes(&test_case.code, &diagnostics),
                test_case.output,
                "fix output differs for upstream invalid case {index}:\n{}",
                test_case.code
            );
        }
    }

    #[test]
    fn reports_all_six_messages_with_exact_token_ranges_data_and_fixes() {
        let one_tbs = "if (a)\n{\nb();\n}\nelse {\nc(); }\n";
        let diagnostics = run(one_tbs, "fixture.js", &json!([]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| (
                    diagnostic.message_id.as_str(),
                    diagnostic.range,
                    diagnostic.message.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("nextLineOpen", byte_range(7, 8), NEXT_LINE_OPEN),
                ("nextLineClose", byte_range(14, 15), NEXT_LINE_CLOSE),
                ("singleLineClose", byte_range(28, 29), SINGLE_LINE_CLOSE),
            ]
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.data.is_empty())
        );
        assert_eq!(
            apply_fixes(one_tbs, &diagnostics).as_deref(),
            Some("if (a) {\nb();\n} else {\nc(); \n}\n")
        );

        let allman = "if (a) { b();\n} else\n{\nc();\n}\n";
        let diagnostics = run(allman, "fixture.js", &json!(["allman"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["sameLineOpen", "blockSameLine", "sameLineClose"]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [byte_range(7, 8), byte_range(7, 8), byte_range(14, 15)]
        );
    }

    #[test]
    fn honors_allow_single_line_for_functions_classes_switches_and_static_blocks() {
        let source = "function f() { return 1; }\nclass C { static { value; } }\nswitch (x) { case 1: break; }\n";
        assert!(
            run(
                source,
                "fixture.js",
                &json!(["allman", { "allowSingleLine": true }])
            )
            .is_empty()
        );
        assert_eq!(run(source, "fixture.js", &json!(["allman"])).len(), 12);
    }

    #[test]
    fn preserves_comment_safety_and_ignores_statement_list_object_and_jsx_braces() {
        let source = "const View = () => <Panel value={{ nested: true }} />;\nif (foo) // keep\n{\nbar();\n}\n{}\n{\n  {}\n}\nconst value = { nested: {} };\n";
        let diagnostics = run(source, "fixture.tsx", &json!([]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["nextLineOpen"]
        );
        assert!(diagnostics[0].suggestions.is_empty());
        assert_eq!(apply_fixes(source, &diagnostics), None);
    }

    #[test]
    fn handles_unicode_crlf_ls_ps_typescript_and_tsx_without_offset_drift() {
        let unicode = "const 名 = 1;\r\nif (名)\r\n{\r\n名++;\r\n}\r\n";
        let brace = unicode.find('{').expect("brace exists");
        let diagnostics = run(unicode, "fixture.ts", &json!([]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range, byte_range(brace, brace + 1));

        for separator in ["\u{2028}", "\u{2029}"] {
            let source = [
                "if (ok)", separator, "{", separator, "work();", separator, "}",
            ]
            .concat();
            assert_eq!(
                run(&source, "fixture.js", &json!([]))
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                ["nextLineOpen"]
            );
        }

        let typescript = "namespace Foo\n{\n}\nmodule \"Bar\" { value(); }\n";
        assert_eq!(
            run(typescript, "fixture.ts", &json!([]))
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["nextLineOpen", "blockSameLine", "singleLineClose"]
        );
        let tsx =
            "const View = () => <section>{value}</section>;\nif (ok)\n{\nrender(<View />);\n}\n";
        assert_eq!(
            run(tsx, "fixture.tsx", &json!([]))
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["nextLineOpen"]
        );
    }

    #[test]
    fn invalid_syntax_and_non_schema_options_do_not_create_false_positives() {
        assert!(run("if (x)\n{", "fixture.js", &json!([])).is_empty());
        assert!(
            run(
                "if (x) {\ny();\n}\n",
                "fixture.js",
                &json!(["unknown", { "allowSingleLine": "yes" }])
            )
            .is_empty()
        );
    }
}
