//! Native implementation of stable `@stylistic/jsx-tag-spacing` v5.10.0.
//!
//! Oxc supplies exact JSX element, name, and attribute spans. The remaining
//! punctuation is recovered from the narrow gaps around those spans so the
//! implementation can preserve upstream's point ranges, whitespace fixes,
//! proportional multiline modes, and opening/closing visitor order.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXClosingElement, JSXOpeningElement};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "jsx-tag-spacing";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BasicMode {
    Always,
    Never,
    Allow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BeforeMode {
    Always,
    ProportionalAlways,
    Never,
    Allow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AfterMode {
    Always,
    AllowMultiline,
    Never,
    Allow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Options {
    closing_slash: BasicMode,
    before_self_closing: BeforeMode,
    after_opening: AfterMode,
    before_closing: BeforeMode,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            closing_slash: BasicMode::Never,
            before_self_closing: BeforeMode::Always,
            after_opening: AfterMode::Never,
            before_closing: BeforeMode::Allow,
        }
    }
}

impl Options {
    fn from_value(value: &Value) -> Self {
        let defaults = Self::default();
        let object = match value {
            Value::Array(items) => items.first().and_then(Value::as_object),
            Value::Object(object) => Some(object),
            _ => None,
        };
        let Some(object) = object else {
            return defaults;
        };
        Self {
            closing_slash: object
                .get("closingSlash")
                .and_then(Value::as_str)
                .and_then(parse_basic)
                .unwrap_or(defaults.closing_slash),
            before_self_closing: object
                .get("beforeSelfClosing")
                .and_then(Value::as_str)
                .and_then(parse_before)
                .unwrap_or(defaults.before_self_closing),
            after_opening: object
                .get("afterOpening")
                .and_then(Value::as_str)
                .and_then(parse_after)
                .unwrap_or(defaults.after_opening),
            before_closing: object
                .get("beforeClosing")
                .and_then(Value::as_str)
                .and_then(parse_before)
                .unwrap_or(defaults.before_closing),
        }
    }
}

fn parse_basic(value: &str) -> Option<BasicMode> {
    match value {
        "always" => Some(BasicMode::Always),
        "never" => Some(BasicMode::Never),
        "allow" => Some(BasicMode::Allow),
        _ => None,
    }
}

fn parse_before(value: &str) -> Option<BeforeMode> {
    match value {
        "always" => Some(BeforeMode::Always),
        "proportional-always" => Some(BeforeMode::ProportionalAlways),
        "never" => Some(BeforeMode::Never),
        "allow" => Some(BeforeMode::Allow),
        _ => None,
    }
}

fn parse_after(value: &str) -> Option<AfterMode> {
    match value {
        "always" => Some(AfterMode::Always),
        "allow-multiline" => Some(AfterMode::AllowMultiline),
        "never" => Some(AfterMode::Never),
        "allow" => Some(AfterMode::Allow),
        _ => None,
    }
}

pub(crate) fn check_jsx_tag_spacing(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_value(options);
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
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }
    let mut visitor = TagSpacingVisitor {
        source,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct TagSpacingVisitor<'source, 'diagnostics> {
    source: &'source str,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for TagSpacingVisitor<'_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        self.check_opening(element);
        walk::walk_jsx_opening_element(self, element);
    }

    fn visit_jsx_closing_element(&mut self, element: &JSXClosingElement<'ast>) {
        self.check_closing(element);
        walk::walk_jsx_closing_element(self, element);
    }
}

#[derive(Clone, Copy)]
struct Geometry {
    span: Span,
    name: Span,
    left: Span,
    opening_start: u32,
    slash_start: Option<u32>,
    closing_start: u32,
    self_closing: bool,
    opening_element: bool,
}

impl TagSpacingVisitor<'_, '_> {
    fn check_opening(&mut self, element: &JSXOpeningElement<'_>) {
        let Some(geometry) = self.opening_geometry(element) else {
            return;
        };
        if geometry.self_closing && self.options.closing_slash != BasicMode::Allow {
            self.validate_closing_slash(geometry, self.options.closing_slash);
        }
        if self.options.after_opening != AfterMode::Allow {
            self.validate_after_opening(geometry, self.options.after_opening);
        }
        if geometry.self_closing && self.options.before_self_closing != BeforeMode::Allow {
            self.validate_before_self_closing(geometry, self.options.before_self_closing);
        }
        if self.options.before_closing != BeforeMode::Allow {
            self.validate_before_closing(geometry, self.options.before_closing);
        }
    }

    fn check_closing(&mut self, element: &JSXClosingElement<'_>) {
        let Some(geometry) = self.closing_geometry(element) else {
            return;
        };
        if self.options.after_opening != AfterMode::Allow {
            self.validate_after_opening(geometry, self.options.after_opening);
        }
        if self.options.closing_slash != BasicMode::Allow {
            self.validate_closing_slash(geometry, self.options.closing_slash);
        }
        if self.options.before_closing != BeforeMode::Allow {
            self.validate_before_closing(geometry, self.options.before_closing);
        }
    }

    fn opening_geometry(&self, element: &JSXOpeningElement<'_>) -> Option<Geometry> {
        let span = element.span;
        let name = element.name.span();
        let left = element
            .attributes
            .last()
            .map(GetSpan::span)
            .or_else(|| element.type_arguments.as_deref().map(GetSpan::span))
            .unwrap_or(name);
        let closing_start = span.end.checked_sub(1)?;
        if self.byte(closing_start)? != b'>' {
            return None;
        }
        let slash_start = self
            .previous_non_whitespace(closing_start)
            .filter(|offset| self.byte(*offset) == Some(b'/'));
        let self_closing = slash_start.is_some();
        Some(Geometry {
            span,
            name,
            left,
            opening_start: span.start,
            slash_start,
            closing_start,
            self_closing,
            opening_element: true,
        })
    }

    fn closing_geometry(&self, element: &JSXClosingElement<'_>) -> Option<Geometry> {
        let span = element.span;
        let name = element.name.span();
        let closing_start = span.end.checked_sub(1)?;
        if self.byte(closing_start)? != b'>' {
            return None;
        }
        let slash_start = self.next_non_whitespace(span.start.checked_add(1)?, name.start)?;
        if self.byte(slash_start)? != b'/' {
            return None;
        }
        Some(Geometry {
            span,
            name,
            left: name,
            opening_start: span.start,
            slash_start: Some(slash_start),
            closing_start,
            self_closing: false,
            opening_element: false,
        })
    }

    fn validate_closing_slash(&mut self, geometry: Geometry, option: BasicMode) {
        let Some(slash_start) = geometry.slash_start else {
            return;
        };
        let (left_end, right_start, report_start, report_end, no_space, need_space) =
            if geometry.self_closing {
                (
                    slash_start.saturating_add(1),
                    geometry.closing_start,
                    slash_start,
                    geometry.closing_start.saturating_add(1),
                    (
                        "selfCloseSlashNoSpace",
                        "Whitespace is forbidden between `/` and `>`; write `/>`",
                    ),
                    (
                        "selfCloseSlashNeedSpace",
                        "Whitespace is required between `/` and `>`; write `/ >`",
                    ),
                )
            } else {
                (
                    geometry.opening_start.saturating_add(1),
                    slash_start,
                    geometry.opening_start,
                    slash_start.saturating_add(1),
                    (
                        "closeSlashNoSpace",
                        "Whitespace is forbidden between `<` and `/`; write `</`",
                    ),
                    (
                        "closeSlashNeedSpace",
                        "Whitespace is required between `<` and `/`; write `< /`",
                    ),
                )
            };
        let adjacent = !self.has_space(left_end, right_start);
        match option {
            BasicMode::Never if !adjacent => self.report(
                no_space.0,
                no_space.1,
                TextRange::new(report_start, report_end),
                TextRange::new(left_end, right_start),
                "",
            ),
            BasicMode::Always if adjacent => self.report(
                need_space.0,
                need_space.1,
                TextRange::new(report_start, report_end),
                TextRange::new(right_start, right_start),
                " ",
            ),
            _ => {}
        }
    }

    fn validate_before_self_closing(&mut self, geometry: Geometry, option: BeforeMode) {
        let Some(slash_start) = geometry.slash_start else {
            return;
        };
        if !self.same_line(geometry.span.start, geometry.span.end)
            && option == BeforeMode::ProportionalAlways
            && self.same_line(geometry.left.end, slash_start)
        {
            self.report(
                "beforeSelfCloseNeedNewline",
                "A newline is required before closing bracket",
                TextRange::new(geometry.left.end, geometry.left.end),
                TextRange::new(slash_start, slash_start),
                "\n",
            );
            return;
        }
        if !self.same_line(geometry.left.end, slash_start) {
            return;
        }
        let adjacent = !self.has_space(geometry.left.end, slash_start);
        match option {
            BeforeMode::Always | BeforeMode::ProportionalAlways if adjacent => self.report(
                "beforeSelfCloseNeedSpace",
                "A space is required before closing bracket",
                TextRange::new(slash_start, slash_start),
                TextRange::new(slash_start, slash_start),
                " ",
            ),
            BeforeMode::Never if !adjacent => self.report(
                "beforeSelfCloseNoSpace",
                "A space is forbidden before closing bracket",
                TextRange::new(slash_start, slash_start),
                TextRange::new(geometry.left.end, slash_start),
                "",
            ),
            _ => {}
        }
    }

    fn validate_after_opening(&mut self, geometry: Geometry, option: AfterMode) {
        let opening_start = if geometry.self_closing || geometry.slash_start.is_none() {
            geometry.opening_start
        } else {
            geometry.slash_start.unwrap_or(geometry.opening_start)
        };
        let opening_end = opening_start.saturating_add(1);
        if option == AfterMode::AllowMultiline
            && !self.same_line(opening_start, geometry.name.start)
        {
            return;
        }
        let adjacent = !self.has_space(opening_end, geometry.name.start);
        match option {
            AfterMode::Never | AfterMode::AllowMultiline if !adjacent => self.report(
                "afterOpenNoSpace",
                "A space is forbidden after opening bracket",
                TextRange::new(opening_start, geometry.name.start),
                TextRange::new(opening_end, geometry.name.start),
                "",
            ),
            AfterMode::Always if adjacent => self.report(
                "afterOpenNeedSpace",
                "A space is required after opening bracket",
                TextRange::new(opening_start, geometry.name.start),
                TextRange::new(geometry.name.start, geometry.name.start),
                " ",
            ),
            _ => {}
        }
    }

    fn validate_before_closing(&mut self, geometry: Geometry, option: BeforeMode) {
        if geometry.self_closing {
            return;
        }
        let single_line = self.same_line(geometry.span.start, geometry.span.end);
        if !single_line
            && option == BeforeMode::ProportionalAlways
            && self.same_line(geometry.left.end, geometry.closing_start)
        {
            self.report(
                "beforeCloseNeedNewline",
                "A newline is required before closing bracket",
                TextRange::new(geometry.left.end, geometry.left.end),
                TextRange::new(geometry.closing_start, geometry.closing_start),
                "\n",
            );
            return;
        }
        if !self.same_line(geometry.left.start, geometry.closing_start) {
            return;
        }
        let adjacent = !self.has_space(geometry.left.end, geometry.closing_start);
        match option {
            BeforeMode::Never if !adjacent => self.report(
                "beforeCloseNoSpace",
                "A space is forbidden before closing bracket",
                TextRange::new(geometry.left.end, geometry.closing_start),
                TextRange::new(geometry.left.end, geometry.closing_start),
                "",
            ),
            BeforeMode::Always if adjacent => self.report(
                "beforeCloseNeedSpace",
                "Whitespace is required before closing bracket",
                TextRange::new(geometry.left.end, geometry.closing_start),
                TextRange::new(geometry.closing_start, geometry.closing_start),
                " ",
            ),
            BeforeMode::ProportionalAlways
                if geometry.opening_element && adjacent != single_line =>
            {
                self.report(
                    "beforeCloseNeedSpace",
                    "Whitespace is required before closing bracket",
                    TextRange::new(geometry.left.end, geometry.closing_start),
                    TextRange::new(geometry.closing_start, geometry.closing_start),
                    " ",
                );
            }
            _ => {}
        }
    }

    fn report(
        &mut self,
        message_id: &str,
        message: &str,
        range: TextRange,
        fix_range: TextRange,
        replacement: &str,
    ) {
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range,
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(fix_range, replacement.to_owned()))
                    .collect(),
            })
            .collect(),
        });
    }

    fn byte(&self, offset: u32) -> Option<u8> {
        self.source.as_bytes().get(offset as usize).copied()
    }

    fn previous_non_whitespace(&self, end: u32) -> Option<u32> {
        let mut cursor = end as usize;
        while cursor > 0 {
            let character = self.source.get(..cursor)?.chars().next_back()?;
            cursor -= character.len_utf8();
            if !is_ecmascript_whitespace(character) {
                return u32::try_from(cursor).ok();
            }
        }
        None
    }

    fn next_non_whitespace(&self, start: u32, end: u32) -> Option<u32> {
        let mut cursor = start as usize;
        let end = end as usize;
        while cursor < end {
            let character = self.source.get(cursor..end)?.chars().next()?;
            if !is_ecmascript_whitespace(character) {
                return u32::try_from(cursor).ok();
            }
            cursor += character.len_utf8();
        }
        None
    }

    fn has_space(&self, start: u32, end: u32) -> bool {
        self.source
            .get(start as usize..end as usize)
            .is_some_and(|gap| gap.chars().any(is_ecmascript_whitespace))
    }

    fn same_line(&self, start: u32, end: u32) -> bool {
        self.source
            .get(start as usize..end as usize)
            .is_some_and(|text| !text.chars().any(is_line_terminator))
    }
}

fn is_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\u{000b}' | '\u{000c}' | '\r' | '\n' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the option matrix concise"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<Case>,
        invalid: Vec<Case>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Generated {
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        logical_valid: usize,
        logical_invalid: usize,
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        fixable_invalid: usize,
        total: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        code: String,
        options: Value,
        parser: String,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        recursive_output: Option<String>,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        range: [u32; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-tag-spacing-v5.10.0.json"
        ))
        .expect("generated jsx-tag-spacing fixture is valid JSON")
    }

    fn filename(parser: &str) -> &'static str {
        if parser == "typescript" {
            "fixture.tsx"
        } else {
            "fixture.jsx"
        }
    }

    fn run(source: &str, filename: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_tag_spacing(source, Some(filename), options, &mut diagnostics);
        diagnostics
    }

    fn apply(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
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
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    fn recursive(source: &str, filename: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, filename, options);
            let Some(next) = apply(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            if next == output {
                return changed.then_some(output);
            }
            output = next;
            changed = true;
        }
        panic!("jsx-tag-spacing fixes did not converge");
    }

    #[test]
    fn replays_every_pinned_parser_expanded_case_exactly() {
        let fixture = fixture();
        assert_eq!(fixture.generated.inventory.logical_valid, 38);
        assert_eq!(fixture.generated.inventory.logical_invalid, 36);
        assert_eq!(fixture.generated.inventory.valid, 74);
        assert_eq!(fixture.generated.inventory.invalid, 69);
        assert_eq!(fixture.generated.inventory.diagnostics, 73);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 69);
        assert_eq!(fixture.generated.inventory.total, 143);

        for (index, case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&case.code, filename(&case.parser), &case.options).is_empty(),
                "valid case {index} reported diagnostics"
            );
        }
        for (index, case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&case.code, filename(&case.parser), &case.options);
            assert_eq!(
                diagnostics.len(),
                case.expected_diagnostics.len(),
                "invalid case {index} diagnostic count"
            );
            for (actual, expected) in diagnostics.iter().zip(&case.expected_diagnostics) {
                assert_eq!(
                    actual.message_id, expected.message_id,
                    "invalid case {index}"
                );
                assert_eq!(actual.message, expected.message, "invalid case {index}");
                assert_eq!(
                    actual.range,
                    TextRange::new(expected.range[0], expected.range[1]),
                    "invalid case {index}"
                );
                let actual_fix = actual
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first());
                match (&expected.fix, actual_fix) {
                    (Some(expected), Some(actual)) => {
                        assert_eq!(
                            actual.range,
                            TextRange::new(expected.range[0], expected.range[1]),
                            "invalid case {index}"
                        );
                        assert_eq!(
                            actual.replacement_text, expected.text,
                            "invalid case {index}"
                        );
                    }
                    (None, None) => {}
                    _ => panic!("invalid case {index} fix presence differed"),
                }
            }
            assert_eq!(
                apply(&case.code, &diagnostics),
                case.output,
                "invalid case {index} first-pass output"
            );
            assert_eq!(
                recursive(&case.code, filename(&case.parser), &case.options),
                case.recursive_output,
                "invalid case {index} recursive output"
            );
        }
    }

    #[test]
    fn covers_all_modes_and_preserves_source_order() {
        let source = "const first = < App/ >; const second = <Nested ></Nested >;";
        let diagnostics = run(
            source,
            "fixture.jsx",
            &json!([{
                "closingSlash": "never",
                "beforeSelfClosing": "never",
                "afterOpening": "never",
                "beforeClosing": "never"
            }]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                "selfCloseSlashNoSpace",
                "afterOpenNoSpace",
                "beforeCloseNoSpace",
                "beforeCloseNoSpace"
            ]
        );
    }

    #[test]
    fn handles_unicode_tsx_names_and_exact_byte_ranges() {
        let source = "const 絵: JSX.Element = <外.部<型> 値={1}/ >;";
        let diagnostics = run(
            source,
            "fixture.tsx",
            &json!([{
                "closingSlash": "never",
                "beforeSelfClosing": "allow",
                "afterOpening": "never",
                "beforeClosing": "allow"
            }]),
        );
        assert_eq!(diagnostics.len(), 1);
        let slash = source.find("/ >").expect("self closing slash");
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(slash as u32, (slash + 3) as u32)
        );
        assert_eq!(
            apply(source, &diagnostics).as_deref(),
            Some("const 絵: JSX.Element = <外.部<型> 値={1}/>;")
        );
    }

    #[test]
    fn treats_every_ecmascript_line_terminator_as_multiline() {
        for separator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("<App{separator}prop={{1}}/>");
            let diagnostics = run(
                &source,
                "fixture.tsx",
                &json!([{
                    "closingSlash": "allow",
                    "beforeSelfClosing": "proportional-always",
                    "afterOpening": "allow",
                    "beforeClosing": "allow"
                }]),
            );
            assert_eq!(
                diagnostics[0].message_id, "beforeSelfCloseNeedNewline",
                "{separator:?}"
            );
        }
    }

    #[test]
    fn preserves_fragments_comments_namespaces_members_and_malformed_input() {
        assert!(run("<></>", "fixture.jsx", &Value::Null).is_empty());
        assert!(run("<svg:path />", "fixture.jsx", &Value::Null).is_empty());
        assert!(run("<UI.Button />", "fixture.tsx", &Value::Null).is_empty());
        assert!(run("<App value={/* keep */ 1} />", "fixture.tsx", &Value::Null).is_empty());
        assert!(run("<App>", "fixture.tsx", &Value::Null).is_empty());
    }

    #[test]
    fn malformed_options_fall_back_field_by_field_without_panics() {
        assert_eq!(
            run(
                "<App/>",
                "fixture.tsx",
                &json!([{
                    "closingSlash": 1,
                    "beforeSelfClosing": "invalid",
                    "afterOpening": null,
                    "beforeClosing": false
                }])
            )[0]
            .message_id,
            "beforeSelfCloseNeedSpace"
        );
        assert_eq!(
            run("<App/>", "fixture.tsx", &json!(["invalid"]))[0].message_id,
            "beforeSelfCloseNeedSpace"
        );
    }
}
