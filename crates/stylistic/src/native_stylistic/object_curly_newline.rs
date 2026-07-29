//! AST-backed implementation of `@stylistic/object-curly-newline`.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ExportNamedDeclaration, ImportDeclaration, ImportDeclarationSpecifier, ObjectExpression,
    ObjectPattern, TSEnumBody, TSInterfaceBody, TSTypeLiteral,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::Scan;

const RULE: &str = "object-curly-newline";
const UNEXPECTED_BEFORE_ID: &str = "unexpectedLinebreakBeforeClosingBrace";
const UNEXPECTED_BEFORE_MESSAGE: &str = "Unexpected line break before this closing brace.";
const UNEXPECTED_AFTER_ID: &str = "unexpectedLinebreakAfterOpeningBrace";
const UNEXPECTED_AFTER_MESSAGE: &str = "Unexpected line break after this opening brace.";
const EXPECTED_BEFORE_ID: &str = "expectedLinebreakBeforeClosingBrace";
const EXPECTED_BEFORE_MESSAGE: &str = "Expected a line break before this closing brace.";
const EXPECTED_AFTER_ID: &str = "expectedLinebreakAfterOpeningBrace";
const EXPECTED_AFTER_MESSAGE: &str = "Expected a line break after this opening brace.";

#[derive(Clone, Copy)]
struct NormalizedOption {
    multiline: bool,
    min_properties: Option<usize>,
    consistent: bool,
}

#[derive(Clone, Copy)]
struct NormalizedOptions {
    object_expression: NormalizedOption,
    object_pattern: NormalizedOption,
    import_declaration: NormalizedOption,
    export_named_declaration: NormalizedOption,
    ts_type_literal: NormalizedOption,
    ts_interface_body: NormalizedOption,
    ts_enum_body: NormalizedOption,
}

/// Enforces newline symmetry and configured thresholds around object-like braces.
pub(crate) fn check_object_curly_newline(
    scan: &Scan<'_>,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
        if parsed.errors.is_empty() {
            let mut visitor = ObjectCurlyNewlineVisitor {
                scan,
                options: normalize_options(options),
                include_typescript_nodes: filename.is_none_or(is_typescript_filename),
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            return;
        }
    }
}

struct ObjectCurlyNewlineVisitor<'source, 'diagnostics> {
    scan: &'source Scan<'source>,
    options: NormalizedOptions,
    include_typescript_nodes: bool,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for ObjectCurlyNewlineVisitor<'_, '_> {
    fn visit_object_expression(&mut self, node: &ObjectExpression<'ast>) {
        self.check(
            node.span,
            node.properties.len(),
            self.options.object_expression,
        );
        walk::walk_object_expression(self, node);
    }

    fn visit_object_pattern(&mut self, node: &ObjectPattern<'ast>) {
        self.check(
            node.span,
            node.properties.len() + usize::from(node.rest.is_some()),
            self.options.object_pattern,
        );
        walk::walk_object_pattern(self, node);
    }

    fn visit_import_declaration(&mut self, node: &ImportDeclaration<'ast>) {
        let properties = node
            .specifiers
            .as_ref()
            .map(|specifiers| {
                specifiers
                    .iter()
                    .filter(|specifier| {
                        matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(_))
                    })
                    .count()
            })
            .unwrap_or(0);
        if properties > 0 {
            self.check(node.span, properties, self.options.import_declaration);
        }
        walk::walk_import_declaration(self, node);
    }

    fn visit_export_named_declaration(&mut self, node: &ExportNamedDeclaration<'ast>) {
        if !node.specifiers.is_empty() {
            self.check(
                node.span,
                node.specifiers.len(),
                self.options.export_named_declaration,
            );
        }
        walk::walk_export_named_declaration(self, node);
    }

    fn visit_ts_type_literal(&mut self, node: &TSTypeLiteral<'ast>) {
        if self.include_typescript_nodes {
            self.check(node.span, node.members.len(), self.options.ts_type_literal);
        }
        walk::walk_ts_type_literal(self, node);
    }

    fn visit_ts_interface_body(&mut self, node: &TSInterfaceBody<'ast>) {
        if self.include_typescript_nodes {
            self.check(node.span, node.body.len(), self.options.ts_interface_body);
        }
        walk::walk_ts_interface_body(self, node);
    }

    fn visit_ts_enum_body(&mut self, node: &TSEnumBody<'ast>) {
        if self.include_typescript_nodes {
            self.check(node.span, node.members.len(), self.options.ts_enum_body);
        }
        walk::walk_ts_enum_body(self, node);
    }
}

impl ObjectCurlyNewlineVisitor<'_, '_> {
    fn check(&mut self, span: Span, property_count: usize, options: NormalizedOption) {
        let Some((open, close)) = self.braces(span) else {
            return;
        };
        let tokens = self.scan.tokens();
        let Some(first_with_comments) = open.checked_add(1).filter(|index| *index <= close) else {
            return;
        };
        let Some(last_with_comments) = close.checked_sub(1).filter(|index| *index >= open) else {
            return;
        };
        let Some(first) = self.scan.next_significant(open) else {
            return;
        };
        let Some(last) = self.scan.prev_significant(close) else {
            return;
        };
        if first > close || last < open {
            return;
        }

        let multiline_contents = property_count > 0
            && !tokens_on_same_line(self.scan, last_with_comments, first_with_comments);
        let requires_linebreaks = options
            .min_properties
            .is_some_and(|minimum| property_count >= minimum)
            || (options.multiline && multiline_contents);
        let comment_after_open = tokens[first_with_comments].kind.is_comment();
        let comment_before_close = tokens[last_with_comments].kind.is_comment();

        if requires_linebreaks {
            if tokens_on_same_line(self.scan, open, first) {
                self.push_diagnostic(
                    EXPECTED_AFTER_ID,
                    EXPECTED_AFTER_MESSAGE,
                    open,
                    (!comment_after_open).then_some((tokens[open].end..tokens[open].end, "\n")),
                );
            }
            if tokens_on_same_line(self.scan, last, close) {
                self.push_diagnostic(
                    EXPECTED_BEFORE_ID,
                    EXPECTED_BEFORE_MESSAGE,
                    close,
                    (!comment_before_close)
                        .then_some((tokens[close].start..tokens[close].start, "\n")),
                );
            }
            return;
        }

        let break_after_open = !tokens_on_same_line(self.scan, open, first);
        let break_before_close = !tokens_on_same_line(self.scan, last, close);
        if break_after_open && (!options.consistent || !break_before_close) {
            self.push_diagnostic(
                UNEXPECTED_AFTER_ID,
                UNEXPECTED_AFTER_MESSAGE,
                open,
                (!comment_after_open).then_some((tokens[open].end..tokens[first].start, "")),
            );
        }
        if break_before_close && (!options.consistent || !break_after_open) {
            self.push_diagnostic(
                UNEXPECTED_BEFORE_ID,
                UNEXPECTED_BEFORE_MESSAGE,
                close,
                (!comment_before_close).then_some((tokens[last].end..tokens[close].start, "")),
            );
        }
    }

    fn braces(&self, span: Span) -> Option<(usize, usize)> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let open = self
            .scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, token)| {
                token.start >= start
                    && token.end <= end
                    && self.scan.slice(token.start, token.end) == "{"
            })
            .map(|(index, _)| index)?;
        let close = self.scan.partner(open)?;
        (self.scan.tokens()[close].end <= end).then_some((open, close))
    }

    fn push_diagnostic(
        &mut self,
        message_id: &str,
        message: &str,
        brace: usize,
        fix: Option<(std::ops::Range<usize>, &str)>,
    ) {
        let token = self.scan.tokens()[brace];
        let (Ok(start), Ok(end)) = (u32::try_from(token.start), u32::try_from(token.end)) else {
            return;
        };
        let suggestions = fix
            .and_then(|(range, replacement)| {
                Some(LintSuggestion {
                    message_id: message_id.to_owned(),
                    message: message.to_owned(),
                    fixes: std::iter::once(LintFix::replace_range(
                        TextRange::new(
                            u32::try_from(range.start).ok()?,
                            u32::try_from(range.end).ok()?,
                        ),
                        replacement,
                    ))
                    .collect(),
                })
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            range: TextRange::new(start, end),
            suggestions,
            data: BTreeMap::new(),
        });
    }
}

fn normalize_options(options: &Value) -> NormalizedOptions {
    let provided = match options {
        Value::Array(values) => values.first(),
        Value::Null => None,
        value => Some(value),
    };
    if let Some(Value::Object(object)) = provided
        && object.values().any(is_node_specific_option)
    {
        return NormalizedOptions {
            object_expression: normalize_option(object.get("ObjectExpression")),
            object_pattern: normalize_option(object.get("ObjectPattern")),
            import_declaration: normalize_option(object.get("ImportDeclaration")),
            export_named_declaration: normalize_option(object.get("ExportDeclaration")),
            ts_type_literal: normalize_option(object.get("TSTypeLiteral")),
            ts_interface_body: normalize_option(object.get("TSInterfaceBody")),
            ts_enum_body: normalize_option(object.get("TSEnumBody")),
        };
    }

    let option = normalize_option(provided);
    NormalizedOptions {
        object_expression: option,
        object_pattern: option,
        import_declaration: option,
        export_named_declaration: option,
        ts_type_literal: option,
        ts_interface_body: option,
        ts_enum_body: option,
    }
}

fn normalize_option(value: Option<&Value>) -> NormalizedOption {
    let Some(value) = value.filter(|value| js_truthy(value)) else {
        return NormalizedOption {
            multiline: false,
            min_properties: None,
            consistent: true,
        };
    };
    if value.as_str() == Some("always") {
        return NormalizedOption {
            multiline: false,
            min_properties: Some(0),
            consistent: false,
        };
    }
    if value.as_str() == Some("never") {
        return NormalizedOption {
            multiline: false,
            min_properties: None,
            consistent: false,
        };
    }
    NormalizedOption {
        multiline: value
            .get("multiline")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        min_properties: value
            .get("minProperties")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .and_then(|value| usize::try_from(value).ok()),
        consistent: value
            .get("consistent")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn is_node_specific_option(value: &Value) -> bool {
    matches!(value, Value::Object(_) | Value::String(_))
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn tokens_on_same_line(scan: &Scan<'_>, left: usize, right: usize) -> bool {
    if left == right {
        return true;
    }
    let (start, end) = if scan.tokens()[left].end <= scan.tokens()[right].start {
        (scan.tokens()[left].end, scan.tokens()[right].start)
    } else if scan.tokens()[right].end <= scan.tokens()[left].start {
        (scan.tokens()[right].end, scan.tokens()[left].start)
    } else {
        return true;
    };
    !has_linebreak(scan.slice(start, end))
}

fn has_linebreak(text: &str) -> bool {
    text.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
}

fn is_typescript_filename(filename: &str) -> bool {
    let lowercase = filename.to_ascii_lowercase();
    lowercase.ends_with(".ts")
        || lowercase.ends_with(".tsx")
        || lowercase.ends_with(".mts")
        || lowercase.ends_with(".cts")
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the option matrix readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        run_with_filename(source, options, Some("file.tsx"))
    }

    fn run_with_filename(
        source: &str,
        options: Value,
        filename: Option<&str>,
    ) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_object_curly_newline(&scan, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn fixed(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| suggestion.fixes.iter())
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| (fix.range.start, fix.range.end));
        let mut accepted = Vec::with_capacity(fixes.len());
        let mut last_end = None;
        for fix in fixes {
            if last_end.is_some_and(|end| end >= fix.range.start) {
                continue;
            }
            last_end = Some(fix.range.end);
            accepted.push(fix);
        }
        accepted.sort_by_key(|fix| std::cmp::Reverse(fix.range.start));
        let mut output = source.to_owned();
        for fix in accepted {
            output.replace_range(
                usize::try_from(fix.range.start).ok()?..usize::try_from(fix.range.end).ok()?,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    #[test]
    fn covers_default_always_never_multiline_minimum_and_consistency() {
        assert!(run("const value = {\na: 1\n};", json!([])).is_empty());
        assert_eq!(
            fixed(
                "const value = {a: 1};",
                &run("const value = {a: 1};", json!(["always"]))
            )
            .as_deref(),
            Some("const value = {\na: 1\n};")
        );
        assert_eq!(
            fixed(
                "const value = {\na: 1\n};",
                &run("const value = {\na: 1\n};", json!(["never"]))
            )
            .as_deref(),
            Some("const value = {a: 1};")
        );
        assert_eq!(
            run(
                "const value = {a: {\n  nested: true\n}};",
                json!([{ "multiline": true }])
            )
            .len(),
            4
        );
        assert_eq!(
            run(
                "const value = {a: 1, b: 2};",
                json!([{ "minProperties": 2 }])
            )
            .len(),
            2
        );
        assert!(run("const value = {\na: 1\n};", json!([{ "consistent": true }])).is_empty());
    }

    #[test]
    fn handles_every_supported_node_category_and_specific_options() {
        let source = r#"
const expression = { a: 1 };
const { b } = expression;
import { c } from "source";
export { c };
type Shape = { d: string };
interface Box { e: number }
enum Kind { F }
"#;
        let diagnostics = run(
            source,
            json!([{
                "ObjectExpression": "always",
                "ObjectPattern": "always",
                "ImportDeclaration": "always",
                "ExportDeclaration": "always",
                "TSTypeLiteral": "always",
                "TSInterfaceBody": "always",
                "TSEnumBody": "always"
            }]),
        );
        assert_eq!(diagnostics.len(), 14);
        assert!(
            diagnostics
                .chunks_exact(2)
                .all(|pair| pair[0].message_id == EXPECTED_AFTER_ID
                    && pair[1].message_id == EXPECTED_BEFORE_ID)
        );
    }

    #[test]
    fn ignores_blocks_classes_tuple_types_and_non_named_imports() {
        let source = r#"
if (ready) { run(); }
class Box { method() { return true; } }
type Tuple = [string, number];
import fallback from "source";
import * as namespace from "source";
export const value = { nested: true };
"#;
        let diagnostics = run(source, json!([{ "ImportDeclaration": "always" }]));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn comment_boundaries_match_upstream_unfixable_behavior() {
        let after_open = "const value = {\n // keep\n a: 1};";
        let diagnostics = run(after_open, json!(["never"]));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggestions.is_empty());

        let before_close = "const value = {a: 1\n // keep\n};";
        let diagnostics = run(before_close, json!(["never"]));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggestions.is_empty());
    }

    #[test]
    fn supports_crlf_cr_and_ecmascript_unicode_line_terminators() {
        for separator in ["\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("const value = {{{separator}a: 1{separator}}};");
            assert!(run(&source, json!([])).is_empty(), "{separator:?}");
            assert_eq!(run(&source, json!(["never"])).len(), 2, "{separator:?}");
        }
    }

    #[test]
    fn preserves_utf8_byte_ranges_and_fixes() {
        let source = "const 日本語 = {値: 1};";
        let diagnostics = run(source, json!(["always"]));
        assert_eq!(diagnostics.len(), 2);
        let open = source.find('{').unwrap();
        let close = source.find('}').unwrap();
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(open as u32, (open + 1) as u32)
        );
        assert_eq!(
            diagnostics[1].range,
            TextRange::new(close as u32, (close + 1) as u32)
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("const 日本語 = {\n値: 1\n};")
        );
    }

    #[test]
    fn malformed_sources_do_not_produce_heuristic_diagnostics() {
        assert!(run("const value = { a: 1", json!(["always"])).is_empty());
        assert!(run("const view = <Panel value={{ a: 1 }}", json!(["always"])).is_empty());
    }

    #[test]
    fn replays_every_pinned_upstream_fixture_exactly() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../npm/stylistic/test/fixtures/object-curly-newline.json"
        )))
        .expect("valid committed object-curly-newline fixture");
        let suites = fixture["suites"].as_array().expect("fixture suites");
        let mut valid_count = 0;
        let mut invalid_count = 0;
        let mut diagnostic_count = 0;

        for suite in suites {
            let language = suite["language"].as_str().expect("suite language");
            for case in suite["valid"].as_array().expect("valid cases") {
                valid_count += 1;
                let source = case["code"].as_str().expect("valid source");
                let options = case.get("options").unwrap_or(&Value::Null);
                assert!(
                    run_with_filename(
                        source,
                        options.clone(),
                        Some(if language == "flow" {
                            "fixture.js"
                        } else {
                            "fixture.tsx"
                        }),
                    )
                    .is_empty(),
                    "{language} valid case failed:\n{source}\noptions: {options}"
                );
            }

            for case in suite["invalid"].as_array().expect("invalid cases") {
                invalid_count += 1;
                let source = case["code"].as_str().expect("invalid source");
                let options = case.get("options").unwrap_or(&Value::Null);
                let diagnostics = run_with_filename(
                    source,
                    options.clone(),
                    Some(if language == "flow" {
                        "fixture.js"
                    } else {
                        "fixture.tsx"
                    }),
                );
                let expected = case["expectedDiagnostics"]
                    .as_array()
                    .expect("expected diagnostics");
                diagnostic_count += expected.len();
                assert_eq!(
                    diagnostics.len(),
                    expected.len(),
                    "{language} invalid diagnostic count:\n{source}\noptions: {options}"
                );

                for (index, (actual, expected)) in
                    diagnostics.iter().zip(expected.iter()).enumerate()
                {
                    assert_eq!(
                        actual.message_id, expected["messageId"],
                        "{language} invalid message id {index}:\n{source}"
                    );
                    assert_eq!(
                        actual.message, expected["message"],
                        "{language} invalid message {index}:\n{source}"
                    );
                    assert!(
                        actual.data.is_empty(),
                        "{language} invalid data {index}:\n{source}"
                    );
                    let start = byte_to_utf16(source, usize::try_from(actual.range.start).unwrap());
                    let end = byte_to_utf16(source, usize::try_from(actual.range.end).unwrap());
                    assert_eq!(
                        json!([start, end]),
                        expected["range"],
                        "{language} invalid range {index}:\n{source}"
                    );
                    assert_eq!(
                        location_json(source, start, end),
                        expected["loc"],
                        "{language} invalid location {index}:\n{source}"
                    );
                }

                let expected_output = &case["output"];
                if expected_output.is_null() {
                    assert!(
                        fixed(source, &diagnostics).is_none(),
                        "{language} case must remain unfixable:\n{source}"
                    );
                    assert!(
                        diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.suggestions.is_empty()),
                        "{language} unfixable case has no suppressed fix:\n{source}"
                    );
                } else {
                    let output = fixed(source, &diagnostics).unwrap_or_else(|| {
                        panic!(
                            "{language} expected fixable diagnostics:\n{source}\noptions: {options}"
                        )
                    });
                    assert_eq!(
                        output,
                        expected_output.as_str().expect("string output"),
                        "{language} invalid output:\n{source}\noptions: {options}"
                    );
                    assert_fix_convergence(&output, options, language);
                }
            }
        }

        assert_eq!(valid_count, 256);
        assert_eq!(invalid_count, 223);
        assert_eq!(diagnostic_count, 387);
    }

    fn assert_fix_convergence(first_output: &str, options: &Value, language: &str) {
        let mut output = first_output.to_owned();
        for _ in 0..8 {
            let diagnostics = run_with_filename(
                &output,
                options.clone(),
                Some(if language == "flow" {
                    "fixture.js"
                } else {
                    "fixture.tsx"
                }),
            );
            if diagnostics.is_empty() {
                return;
            }
            let Some(next) = fixed(&output, &diagnostics) else {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.suggestions.is_empty()),
                    "{language} stable diagnostics contain an unexpected fix"
                );
                return;
            };
            assert_ne!(next, output, "{language} fixes made no progress:\n{output}");
            output = next;
        }
        panic!("{language} fixes did not converge:\n{output}");
    }

    fn byte_to_utf16(source: &str, byte_offset: usize) -> usize {
        source[..byte_offset].encode_utf16().count()
    }

    fn location_json(source: &str, start: usize, end: usize) -> Value {
        let (line, column) = position_at_utf16(source, start);
        let (end_line, end_column) = position_at_utf16(source, end);
        json!({
            "line": line,
            "column": column,
            "endLine": end_line,
            "endColumn": end_column,
        })
    }

    fn position_at_utf16(source: &str, target: usize) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        let mut offset = 0;
        let mut chars = source.chars().peekable();
        while offset < target {
            let Some(character) = chars.next() else {
                break;
            };
            let width = character.len_utf16();
            offset += width;
            if character == '\r' {
                if chars.peek() == Some(&'\n') && offset < target {
                    chars.next();
                    offset += 1;
                }
                line += 1;
                column = 1;
            } else if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
                line += 1;
                column = 1;
            } else {
                column += width;
            }
        }
        (line, column)
    }
}
