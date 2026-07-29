//! Native implementation of stable `@stylistic/type-named-tuple-spacing`.

use std::{collections::BTreeMap, sync::LazyLock};

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use regex::Regex;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "type-named-tuple-spacing";
const EXPECTED_AFTER_ID: &str = "expectedSpaceAfter";
const EXPECTED_AFTER_MESSAGE: &str = "Expected a space after the ':'.";
const UNEXPECTED_BETWEEN_ID: &str = "unexpectedSpaceBetween";
const UNEXPECTED_BETWEEN_MESSAGE: &str = "Unexpected space between '?' and the ':'.";
const UNEXPECTED_BEFORE_ID: &str = "unexpectedSpaceBefore";
const UNEXPECTED_BEFORE_MESSAGE: &str = "Unexpected space before the ':'.";

static NAMED_TUPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^([A-Za-z0-9_$]+)(\s*)(\?\s*)?:(\s*)(.*)$")
        .expect("type-named-tuple-spacing regex is valid")
});

pub(crate) fn check_type_named_tuple_spacing(
    source: &str,
    filename: Option<&str>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::ts(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, diagnostics) {
                break;
            }
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }
    let mut visitor = TypeNamedTupleSpacing {
        source,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct TypeNamedTupleSpacing<'source, 'diagnostics> {
    source: &'source str,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for TypeNamedTupleSpacing<'_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        if let AstKind::TSNamedTupleMember(member) = kind {
            self.check(member.span());
        }
    }
}

impl TypeNamedTupleSpacing<'_, '_> {
    fn check(&mut self, span: Span) {
        let Ok(start) = usize::try_from(span.start) else {
            return;
        };
        let Ok(end) = usize::try_from(span.end) else {
            return;
        };
        let Some(code) = self.source.get(start..end) else {
            return;
        };
        let Some(captures) = NAMED_TUPLE.captures(code) else {
            return;
        };
        let Some(label) = captures.get(1).map(|capture| capture.as_str()) else {
            return;
        };
        let before_colon = captures.get(2).map_or("", |capture| capture.as_str());
        let optional = captures.get(3).map(|capture| capture.as_str());
        let after_colon = captures.get(4).map_or("", |capture| capture.as_str());
        let element_type = captures.get(5).map_or("", |capture| capture.as_str());

        let mut replacement = String::with_capacity(
            label.len() + element_type.len() + usize::from(optional.is_some()) + 2,
        );
        replacement.push_str(label);
        if optional.is_some() {
            replacement.push('?');
        }
        replacement.push_str(": ");
        replacement.push_str(element_type);

        if optional.is_some_and(|mark| mark.encode_utf16().count() > 1) {
            self.report(
                span,
                UNEXPECTED_BETWEEN_ID,
                UNEXPECTED_BETWEEN_MESSAGE,
                &replacement,
            );
        }
        if !before_colon.is_empty() {
            self.report(
                span,
                UNEXPECTED_BEFORE_ID,
                UNEXPECTED_BEFORE_MESSAGE,
                &replacement,
            );
        }
        if after_colon.encode_utf16().count() != 1 {
            self.report(
                span,
                EXPECTED_AFTER_ID,
                EXPECTED_AFTER_MESSAGE,
                &replacement,
            );
        }
    }

    fn report(
        &mut self,
        span: Span,
        message_id: &'static str,
        message: &'static str,
        replacement: &str,
    ) {
        let range = TextRange::new(span.start, span.end);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            range,
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(range, replacement)).collect(),
            })
            .collect(),
            data: BTreeMap::new(),
        });
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps TypeScript examples concise"
)]
mod tests {
    use super::*;
    use serde::Deserialize;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/type-named-tuple-spacing-v5.10.0.json"
    ));

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    struct TestCase {
        code: String,
        output: Option<String>,
        #[serde(default)]
        errors: Vec<ExpectedError>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedError {
        message_id: String,
    }

    fn run(source: &str, filename: Option<&str>) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_type_named_tuple_spacing(source, filename, &mut diagnostics);
        diagnostics
    }

    fn fixed(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .filter_map(|suggestion| suggestion.fixes.first())
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        fixes.dedup_by_key(|fix| (fix.range.start, fix.range.end));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                usize::try_from(fix.range.start).ok()?..usize::try_from(fix.range.end).ok()?,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    #[test]
    fn replays_every_pinned_upstream_case_with_exact_messages_and_output() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.valid.len(), 5);
        assert_eq!(fixture.invalid.len(), 11);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.errors)
                .count(),
            18
        );
        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, Some("fixture.ts")).is_empty(),
                "valid case {index}: {}",
                test_case.code
            );
        }
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, Some("fixture.ts"));
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| &diagnostic.message_id)
                    .collect::<Vec<_>>(),
                test_case
                    .errors
                    .iter()
                    .map(|error| &error.message_id)
                    .collect::<Vec<_>>(),
                "invalid case {index}: {}",
                test_case.code
            );
            assert_eq!(
                fixed(&test_case.code, &diagnostics),
                test_case.output,
                "invalid case {index}: {}",
                test_case.code
            );
        }
    }

    #[test]
    fn handles_tabs_newlines_crlf_and_all_ecmascript_line_terminators() {
        for whitespace in ["\t", "\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("type T = [name?{whitespace}:{whitespace}{whitespace}number]");
            let diagnostics = run(&source, Some("fixture.ts"));
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                vec![UNEXPECTED_BETWEEN_ID, EXPECTED_AFTER_ID],
                "{whitespace:?}"
            );
            assert_eq!(
                fixed(&source, &diagnostics).as_deref(),
                Some("type T = [name?: number]"),
                "{whitespace:?}"
            );
        }
    }

    #[test]
    fn preserves_utf8_byte_ranges_nested_types_and_tsx() {
        let source = "type 日本語 = [value :  Promise<Array<string>>];\nconst view = <Panel />;";
        let diagnostics = run(source, Some("fixture.tsx"));
        assert_eq!(diagnostics.len(), 2);
        let start = source.find("value").expect("tuple member");
        let end = source.find("];").expect("tuple end");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.range == TextRange::new(start as u32, end as u32))
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("type 日本語 = [value: Promise<Array<string>>];\nconst view = <Panel />;")
        );
    }

    #[test]
    fn ignores_unicode_escaped_and_non_tuple_labels_like_upstream_regex() {
        for source in [
            "type T = [日本語:number]",
            r"type T = [\u0061:number]",
            "type T = [{ value: number }]",
            "const object = { value:number };",
        ] {
            assert!(run(source, Some("fixture.ts")).is_empty(), "{source}");
        }
    }

    #[test]
    fn javascript_and_invalid_typescript_do_not_create_diagnostics() {
        assert!(run("const value = [name, number];", Some("fixture.js")).is_empty());
        assert!(run("type T = [name:", Some("fixture.ts")).is_empty());
    }
}
