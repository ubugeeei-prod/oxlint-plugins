//! Native implementation of `@stylistic/indent-binary-ops`.
//!
//! Oxc supplies the binary/logical and TypeScript union/intersection trees.
//! The shared stylistic lexer supplies ESLint-compatible tokens and comments,
//! which are required for the upstream rule's line-sensitive indentation
//! heuristics.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BinaryExpression, LogicalExpression, TSIntersectionType, TSTypeAliasDeclaration, TSUnionType,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::{
    context::Scan,
    lexer::{Token, TokenKind},
};

const RULE_NAME: &str = "indent-binary-ops";
const MESSAGE_ID: &str = "wrongIndentation";
const ASSIGNMENT_OPERATORS: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "<<=", ">>=", ">>>=", "|=", "^=", "&=", "**=", "||=", "&&=",
    "??=",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndentOption {
    Spaces(usize),
    Tab,
}

impl IndentOption {
    fn from_options(options: &Value) -> Self {
        let option = match options {
            Value::Array(items) => items.first(),
            Value::Null => None,
            value => Some(value),
        };

        match option {
            Some(Value::String(value)) if value == "tab" => Self::Tab,
            Some(Value::Number(value)) => value
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .map_or(Self::Spaces(2), Self::Spaces),
            _ => Self::Spaces(2),
        }
    }

    fn unit(self) -> String {
        match self {
            Self::Spaces(count) => " ".repeat(count),
            Self::Tab => "\t".to_owned(),
        }
    }

    fn subtract(self, indent: &str) -> String {
        let remove_utf16 = match self {
            Self::Spaces(count) => count,
            Self::Tab => 1,
        };
        if remove_utf16 == 0 {
            return indent.to_owned();
        }

        let mut consumed_utf16 = 0;
        let mut byte_offset = 0;
        for (index, character) in indent.char_indices() {
            consumed_utf16 += character.len_utf16();
            byte_offset = index + character.len_utf8();
            if consumed_utf16 >= remove_utf16 {
                break;
            }
        }
        indent.get(byte_offset..).unwrap_or_default().to_owned()
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "the N-API diagnostic data contract requires an owned decimal string"
    )]
    fn label(self, indent: &str) -> String {
        let length = indent.chars().map(char::len_utf16).sum::<usize>();
        let unit = match self {
            Self::Spaces(_) => "space",
            Self::Tab => "tab",
        };
        let mut label = length.to_string();
        label.push(' ');
        label.push_str(unit);
        if length != 1 {
            label.push('s');
        }
        label
    }
}

pub(crate) fn check_indent_binary_ops(
    scan: &Scan<'_>,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|filename| SourceType::from_path(filename).ok()) {
        let _ = parse_and_check(scan, source_type, options, diagnostics, true);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(scan, source_type, options, diagnostics, false) {
            return;
        }
    }
}

fn parse_and_check(
    scan: &Scan<'_>,
    source_type: SourceType,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
    allow_recoverable_errors: bool,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
    if !allow_recoverable_errors && !parsed.errors.is_empty() {
        return false;
    }

    let option = IndentOption::from_options(options);
    let mut checker = IndentBinaryOps {
        scan,
        lines: LineTable::new(scan.source()),
        option,
        indent_unit: option.unit(),
        indent_cache: BTreeMap::new(),
        type_alias_keyword: None,
        diagnostics,
    };
    checker.visit_program(&parsed.program);
    true
}

struct IndentBinaryOps<'scan, 'diagnostics> {
    scan: &'scan Scan<'scan>,
    lines: LineTable<'scan>,
    option: IndentOption,
    indent_unit: String,
    indent_cache: BTreeMap<usize, String>,
    type_alias_keyword: Option<usize>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl IndentBinaryOps<'_, '_> {
    fn check_pair(&mut self, node: Span, right: Span) {
        let Ok(node_start) = usize::try_from(node.start) else {
            return;
        };
        let Ok(node_end) = usize::try_from(node.end) else {
            return;
        };
        let Ok(right_start) = usize::try_from(right.start) else {
            return;
        };
        let Ok(right_end) = usize::try_from(right.end) else {
            return;
        };

        if self.lines.line_of(node_start) == self.lines.line_of(node_end) {
            return;
        }

        let Some(mut right_index) = self.first_significant_in(right_start, right_end) else {
            return;
        };
        let Some(mut operator_index) = self.scan.prev_significant(right_index) else {
            return;
        };

        while self.token_text(operator_index) == "(" {
            right_index = operator_index;
            let Some(previous) = self.scan.prev_significant(right_index) else {
                return;
            };
            operator_index = previous;
            if self.scan.tokens()[operator_index].start <= node_start {
                return;
            }
        }

        let Some(left_index) = self.scan.prev_significant(operator_index) else {
            return;
        };
        let left_line = self.lines.line_of(self.scan.tokens()[left_index].start);
        let right_line = self.lines.line_of(self.scan.tokens()[right_index].start);
        if left_line == right_line {
            return;
        }

        let token_before_all = self.significant_before(node_start);
        let first_left = self.first_token_starting_on(left_line);
        let last_left = self.last_token_ending_on(left_line);
        let need_addition = self.needs_addition(left_line, token_before_all, first_left, last_left);
        let need_subtraction = self.needs_subtraction(left_line, first_left, last_left);

        let indent_left = self.indent_for_line(left_line);
        let indent_right = self.indent_for_line(right_line);
        let indent_target = match (need_addition, need_subtraction) {
            (true, false) => {
                let mut target = indent_left;
                target.push_str(&self.indent_unit);
                target
            }
            (false, true) => self.option.subtract(&indent_left),
            _ => indent_left,
        };
        if indent_target == indent_right {
            return;
        }

        let Some((start, end)) = self.lines.indent_range(right_line) else {
            return;
        };
        let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
            return;
        };
        let range = TextRange::new(start, end);
        let expected = self.option.label(&indent_target);
        let mut message = String::from("Expected indentation of ");
        message.push_str(&expected);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: message.clone(),
            data: BTreeMap::from([("expected".to_owned(), expected)]),
            range,
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message,
                fixes: std::iter::once(LintFix::replace_range(range, indent_target.clone()))
                    .collect(),
            })
            .collect(),
        });
        self.indent_cache.insert(right_line, indent_target);
    }

    fn needs_addition(
        &self,
        left_line: usize,
        token_before_all: Option<usize>,
        first_left: Option<usize>,
        last_left: Option<usize>,
    ) -> bool {
        let first_is_keyword = first_left.is_some_and(|index| {
            is_keyword_token(&self.scan.tokens()[index], self.token_text(index))
                && !matches!(self.token_text(index), "typeof" | "instanceof" | "this")
        });
        let first_is_type_alias_keyword = first_left == self.type_alias_keyword
            && first_left.is_some_and(|index| {
                self.scan.tokens()[index].kind == TokenKind::Identifier
                    && self.token_text(index) == "type"
            });
        let last_requires_indent = last_left.is_some_and(|index| {
            let text = self.token_text(index);
            matches!(text, ":" | "[" | "(" | "<") || ASSIGNMENT_OPERATORS.contains(&text)
        });
        let before_requires_indent = token_before_all.is_some_and(|index| {
            let text = self.token_text(index);
            let token_line = self.lines.line_of(self.scan.tokens()[index].start);
            (matches!(text, "[" | "(" | "{" | "=>" | ":") || ASSIGNMENT_OPERATORS.contains(&text))
                && token_line == left_line
        });

        first_is_keyword
            || first_is_type_alias_keyword
            || last_requires_indent
            || before_requires_indent
    }

    fn needs_subtraction(
        &self,
        left_line: usize,
        first_left: Option<usize>,
        last_left: Option<usize>,
    ) -> bool {
        let last_is_close_paren = last_left.is_some_and(|index| self.token_text(index) == ")");
        let first_is_close =
            first_left.is_some_and(|index| matches!(self.token_text(index), "]" | ")" | "}"));
        last_is_close_paren && self.has_more_close_parens(left_line) && !first_is_close
    }

    fn has_more_close_parens(&self, line: usize) -> bool {
        let mut opens = 0usize;
        let mut closes = 0usize;
        for (index, token) in self.scan.tokens().iter().enumerate() {
            if self.lines.line_of(token.start) != line {
                continue;
            }
            match self.token_text(index) {
                "(" => opens += 1,
                ")" => closes += 1,
                _ => {}
            }
        }
        opens < closes
    }

    fn first_significant_in(&self, start: usize, end: usize) -> Option<usize> {
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, token)| !token.kind.is_comment() && token.start >= start && token.end <= end)
            .map(|(index, _)| index)
    }

    fn significant_before(&self, position: usize) -> Option<usize> {
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .rev()
            .find(|(_, token)| !token.kind.is_comment() && token.end <= position)
            .map(|(index, _)| index)
    }

    fn first_token_starting_on(&self, line: usize) -> Option<usize> {
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, token)| self.lines.line_of(token.start) == line)
            .map(|(index, _)| index)
    }

    fn last_token_ending_on(&self, line: usize) -> Option<usize> {
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .rev()
            .find(|(_, token)| self.lines.line_of(token.end) == line)
            .map(|(index, _)| index)
    }

    fn indent_for_line(&self, line: usize) -> String {
        self.indent_cache
            .get(&line)
            .cloned()
            .or_else(|| self.lines.indent(line).map(ToOwned::to_owned))
            .unwrap_or_default()
    }

    fn token_text(&self, index: usize) -> &str {
        self.scan.token_text(index)
    }
}

impl<'ast> Visit<'ast> for IndentBinaryOps<'_, '_> {
    fn visit_binary_expression(&mut self, expression: &BinaryExpression<'ast>) {
        self.check_pair(expression.span, expression.right.span());
        walk::walk_binary_expression(self, expression);
    }

    fn visit_logical_expression(&mut self, expression: &LogicalExpression<'ast>) {
        self.check_pair(expression.span, expression.right.span());
        walk::walk_logical_expression(self, expression);
    }

    fn visit_ts_union_type(&mut self, union: &TSUnionType<'ast>) {
        if union.types.len() > 1 {
            for member in &union.types {
                self.check_pair(union.span, member.span());
            }
        }
        walk::walk_ts_union_type(self, union);
    }

    fn visit_ts_intersection_type(&mut self, intersection: &TSIntersectionType<'ast>) {
        if intersection.types.len() > 1 {
            for member in &intersection.types {
                self.check_pair(intersection.span, member.span());
            }
        }
        walk::walk_ts_intersection_type(self, intersection);
    }

    fn visit_ts_type_alias_declaration(&mut self, declaration: &TSTypeAliasDeclaration<'ast>) {
        let previous_keyword = self.type_alias_keyword;
        self.type_alias_keyword = usize::try_from(declaration.id.span.start)
            .ok()
            .and_then(|identifier_start| self.significant_before(identifier_start));
        walk::walk_ts_type_alias_declaration(self, declaration);
        self.type_alias_keyword = previous_keyword;
    }
}

fn is_keyword_token(token: &Token, text: &str) -> bool {
    token.kind == TokenKind::Identifier
        && matches!(
            text,
            "as" | "async"
                | "await"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "enum"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "from"
                | "function"
                | "if"
                | "implements"
                | "import"
                | "in"
                | "instanceof"
                | "interface"
                | "let"
                | "new"
                | "null"
                | "of"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "static"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "undefined"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
        )
}

struct LineTable<'source> {
    source: &'source str,
    starts: Vec<usize>,
    ends: Vec<usize>,
}

impl<'source> LineTable<'source> {
    fn new(source: &'source str) -> Self {
        let bytes = source.as_bytes();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        starts.push(0);

        let mut index = 0;
        while index < bytes.len() {
            let width = match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => 2,
                b'\r' | b'\n' => 1,
                0xE2 if bytes.get(index + 1) == Some(&0x80)
                    && matches!(bytes.get(index + 2), Some(0xA8 | 0xA9)) =>
                {
                    3
                }
                _ => {
                    index += 1;
                    continue;
                }
            };
            ends.push(index);
            index += width;
            starts.push(index);
        }
        ends.push(source.len());
        Self {
            source,
            starts,
            ends,
        }
    }

    fn line_of(&self, offset: usize) -> usize {
        let offset = offset.min(self.source.len());
        self.starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1)
    }

    fn indent(&self, line: usize) -> Option<&'source str> {
        let start = *self.starts.get(line)?;
        let end = *self.ends.get(line)?;
        let line_source = self.source.get(start..end)?;
        let indent_end = line_source
            .char_indices()
            .find(|(_, character)| !is_ecmascript_whitespace(*character))
            .map_or(line_source.len(), |(index, _)| index);
        line_source.get(..indent_end)
    }

    fn indent_range(&self, line: usize) -> Option<(usize, usize)> {
        let start = *self.starts.get(line)?;
        let indent = self.indent(line)?;
        Some((start, start.saturating_add(indent.len())))
    }
}

fn is_ecmascript_whitespace(character: char) -> bool {
    character.is_whitespace() || character == '\u{feff}'
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "JSON options and exhaustive compatibility sources stay readable in tests"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;
    use crate::native_stylistic::{StylisticRuleConfig, StylisticRunConfig, run_stylistic_lint};

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    struct FixtureCase {
        code: String,
        output: Option<String>,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/indent-binary-ops.json"
        ))
        .expect("generated upstream fixture is valid JSON")
    }

    fn run(source: &str, options: Value, filename: &str) -> Vec<LintDiagnostic> {
        run_stylistic_lint(
            source,
            &StylisticRunConfig {
                filename: Some(filename.to_owned()),
                rules: vec![StylisticRuleConfig {
                    name: RULE_NAME.to_owned(),
                    options,
                }],
            },
        )
        .expect("indent-binary-ops is registered")
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

    fn apply_recursively(source: &str, options: Value, filename: &str) -> String {
        let mut output = source.to_owned();
        for _ in 0..10 {
            let diagnostics = run(&output, options.clone(), filename);
            if diagnostics.is_empty() {
                return output;
            }
            output = apply_fixes(&output, &diagnostics);
        }
        output
    }

    #[test]
    fn replays_every_upstream_valid_case() {
        let fixture = fixture();
        assert_eq!(fixture.valid.len(), 19);
        for case in fixture.valid {
            assert!(
                run(&case.code, Value::Null, "fixture.ts").is_empty(),
                "upstream valid case reported:\n{}",
                case.code
            );
        }
    }

    #[test]
    fn replays_every_upstream_invalid_case_and_recursive_output() {
        let fixture = fixture();
        assert_eq!(fixture.invalid.len(), 29);
        for case in fixture.invalid {
            let expected = case.output.expect("upstream invalid case has output");
            let first_pass = run(&case.code, Value::Null, "fixture.ts");
            assert!(
                !first_pass.is_empty(),
                "upstream invalid case did not report:\n{}",
                case.code
            );
            assert!(
                first_pass.iter().all(|diagnostic| {
                    diagnostic.message_id == MESSAGE_ID
                        && diagnostic.message.starts_with("Expected indentation of ")
                        && diagnostic.data.contains_key("expected")
                        && diagnostic.suggestions.len() == 1
                        && diagnostic.suggestions[0].fixes.len() == 1
                        && diagnostic.suggestions[0].fixes[0].range == diagnostic.range
                }),
                "diagnostic contract mismatch:\n{:#?}",
                first_pass
            );
            assert_eq!(
                apply_recursively(&case.code, Value::Null, "fixture.ts"),
                expected,
                "recursive output mismatch for:\n{}",
                case.code
            );
        }
    }

    #[test]
    fn reports_exact_messages_data_ranges_and_fixes_for_each_option() {
        let source = "const total = first\n    + second";
        let diagnostic = &run(source, Value::Null, "fixture.js")[0];
        let line_start = source.find("    +").expect("second line");
        assert_eq!(
            diagnostic.range,
            TextRange::new(line_start as u32, (line_start + 4) as u32)
        );
        assert_eq!(diagnostic.message, "Expected indentation of 2 spaces");
        assert_eq!(
            diagnostic.data,
            BTreeMap::from([("expected".to_owned(), "2 spaces".to_owned())])
        );
        assert_eq!(diagnostic.suggestions[0].fixes[0].replacement_text, "  ");

        let tabbed = &run(source, json!(["tab"]), "fixture.js")[0];
        assert_eq!(tabbed.message, "Expected indentation of 1 tab");
        assert_eq!(tabbed.suggestions[0].fixes[0].replacement_text, "\t");

        let no_indent = &run(source, json!([0]), "fixture.js")[0];
        assert_eq!(no_indent.message, "Expected indentation of 0 spaces");
        assert_eq!(no_indent.suggestions[0].fixes[0].replacement_text, "");
    }

    #[test]
    fn covers_javascript_typescript_tsx_comments_and_utf8_offsets() {
        let cases = [
            (
                "const total = first\n+ second",
                Value::Null,
                "fixture.js",
                "const total = first\n  + second",
            ),
            (
                "type Value =\n| A\n    | B",
                Value::Null,
                "fixture.ts",
                "type Value =\n  | A\n  | B",
            ),
            (
                "type Value =\n& A\n    & B",
                Value::Null,
                "fixture.ts",
                "type Value =\n  & A\n  & B",
            ),
            (
                "const view = <Box value={first\n+ second} />",
                Value::Null,
                "fixture.tsx",
                "const view = <Box value={first\n  + second} />",
            ),
            (
                "const 日本語 = first // keep\n+ second",
                Value::Null,
                "fixture.js",
                "const 日本語 = first // keep\n  + second",
            ),
        ];

        for (source, options, filename, expected) in cases {
            assert_eq!(
                apply_recursively(source, options, filename),
                expected,
                "{filename}: {source}"
            );
        }
    }

    #[test]
    fn handles_crlf_unicode_separators_tabs_and_subtraction_paths() {
        assert_eq!(
            apply_recursively(
                "if (\r\n\ta\r\n\t\t&& b\r\n) {}",
                json!(["tab"]),
                "fixture.js"
            ),
            "if (\r\n\ta\r\n\t&& b\r\n) {}"
        );
        for separator in ["\u{2028}", "\u{2029}"] {
            let source = format!("const total = first{separator}+ second");
            let expected = format!("const total = first{separator}  + second");
            assert_eq!(
                apply_recursively(&source, Value::Null, "fixture.js"),
                expected
            );
        }

        let source = "const total = (\n  (first\n      && second)\n    || third\n)";
        let expected = "const total = (\n  (first\n    && second)\n  || third\n)";
        assert_eq!(
            apply_recursively(source, Value::Null, "fixture.js"),
            expected
        );
    }

    #[test]
    fn ignores_single_line_and_non_target_syntax_and_handles_invalid_input() {
        for source in [
            "const value = first + second;",
            "const value = condition ? first : second;",
            "const values = [first,\nsecond];",
            "const object = { first,\nsecond };",
            "type Value = Array<\nstring\n>;",
            "type Wrapper =\n  type\n  | Other;",
            "const value = !first;",
        ] {
            assert!(
                run(source, Value::Null, "fixture.ts").is_empty(),
                "false positive for {source}"
            );
        }
        assert!(run("const value = first +", Value::Null, "fixture.ts").is_empty());
        assert_eq!(
            apply_recursively(
                "const value = first\n+ second",
                json!(["invalid"]),
                "fixture.js"
            ),
            "const value = first\n  + second"
        );
    }
}
