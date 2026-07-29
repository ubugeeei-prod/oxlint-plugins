//! Native implementation of stable `@stylistic/jsx-first-prop-new-line`.
//!
//! Oxc supplies exact JSX opening-element, type-argument, and attribute
//! boundaries. This rule intentionally preserves upstream's raw replacement
//! ranges, including replacements that span comments between the tag name and
//! first property.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::JSXOpeningElement;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;

const RULE: &str = "jsx-first-prop-new-line";
const PROP_ON_NEW_LINE: &str = "propOnNewLine";
const PROP_ON_SAME_LINE: &str = "propOnSameLine";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Always,
    Never,
    Multiline,
    MultilineMultiprop,
    Multiprop,
}

impl Mode {
    fn from_options(options: &Value) -> Self {
        match first_option(options).and_then(Value::as_str) {
            Some("always") => Self::Always,
            Some("never") => Self::Never,
            Some("multiline") => Self::Multiline,
            Some("multiprop") => Self::Multiprop,
            _ => Self::MultilineMultiprop,
        }
    }
}

pub(crate) fn check_jsx_first_prop_new_line(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let mode = Mode::from_options(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, mode, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, mode, diagnostics) {
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
    mode: Mode,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = FirstPropVisitor {
        source,
        mode,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct FirstPropVisitor<'source, 'diagnostics> {
    source: &'source str,
    mode: Mode,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for FirstPropVisitor<'_, '_> {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'ast>) {
        self.check(element);
        walk::walk_jsx_opening_element(self, element);
    }
}

impl FirstPropVisitor<'_, '_> {
    fn check(&mut self, element: &JSXOpeningElement<'_>) {
        let Some(first_attribute) = element.attributes.first() else {
            return;
        };
        let element_span = element.span;
        let attribute_span = first_attribute.span();
        let multiline = contains_line_terminator(
            self.source,
            usize::try_from(element_span.start).unwrap_or(usize::MAX),
            usize::try_from(element_span.end).unwrap_or(usize::MAX),
        );
        let multiple_properties = element.attributes.len() > 1;
        let require_new_line = match self.mode {
            Mode::Always => true,
            Mode::Multiline => multiline,
            Mode::MultilineMultiprop => multiline && multiple_properties,
            Mode::Multiprop => multiple_properties,
            Mode::Never => false,
        };

        if require_new_line {
            if same_line(
                self.source,
                usize::try_from(element_span.start).unwrap_or(usize::MAX),
                usize::try_from(attribute_span.start).unwrap_or(usize::MAX),
            ) {
                let boundary = element
                    .type_arguments
                    .as_ref()
                    .map_or_else(|| element.name.span().end, |arguments| arguments.span.end);
                self.report(
                    attribute_span,
                    PROP_ON_NEW_LINE,
                    "Property should be placed on a new line",
                    TextRange::new(boundary, attribute_span.start),
                    "\n",
                );
            }
            return;
        }

        let require_same_line = self.mode == Mode::Never
            || (self.mode == Mode::Multiprop && multiline && !multiple_properties);
        if require_same_line
            && !same_line(
                self.source,
                usize::try_from(element_span.start).unwrap_or(usize::MAX),
                usize::try_from(attribute_span.start).unwrap_or(usize::MAX),
            )
        {
            self.report(
                attribute_span,
                PROP_ON_SAME_LINE,
                "Property should be placed on the same line as the component declaration",
                TextRange::new(element.name.span().end, attribute_span.start),
                " ",
            );
        }
    }

    fn report(
        &mut self,
        span: oxc_span::Span,
        message_id: &'static str,
        message: &'static str,
        fix_range: TextRange,
        replacement: &'static str,
    ) {
        let range = TextRange::new(span.start, span.end);
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range,
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(fix_range, replacement)).collect(),
            })
            .collect(),
        });
    }
}

fn same_line(source: &str, start: usize, end: usize) -> bool {
    !contains_line_terminator(source, start, end)
}

fn contains_line_terminator(source: &str, start: usize, end: usize) -> bool {
    source
        .get(start.min(source.len())..end.min(source.len()))
        .is_some_and(|text| {
            text.chars()
                .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        })
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the stable upstream option matrix readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_first_prop_new_line(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(source: &str, filename: Option<&str>, options: Value) -> Vec<String> {
        run(source, filename, options)
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    fn apply(source: &str, diagnostics: &[LintDiagnostic]) -> String {
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
    fn covers_every_stable_mode_and_default() {
        assert_eq!(
            ids(
                "<Foo first second />",
                Some("fixture.tsx"),
                json!(["always"])
            ),
            [PROP_ON_NEW_LINE]
        );
        assert_eq!(
            ids("<Foo\nfirst />", Some("fixture.tsx"), json!(["never"])),
            [PROP_ON_SAME_LINE]
        );
        assert_eq!(
            ids(
                "<Foo first={{\nvalue: 1\n}} />",
                Some("fixture.tsx"),
                json!(["multiline"])
            ),
            [PROP_ON_NEW_LINE]
        );
        assert_eq!(
            ids(
                "<Foo first={{\nvalue: 1\n}} second />",
                Some("fixture.tsx"),
                json!(["multiline-multiprop"])
            ),
            [PROP_ON_NEW_LINE]
        );
        assert_eq!(
            ids(
                "<Foo first second />",
                Some("fixture.tsx"),
                json!(["multiprop"])
            ),
            [PROP_ON_NEW_LINE]
        );
        assert_eq!(
            ids(
                "<Foo first={{\nvalue: 1\n}} second />",
                Some("fixture.tsx"),
                Value::Null
            ),
            [PROP_ON_NEW_LINE]
        );
    }

    #[test]
    fn reproduces_every_base_upstream_valid_case() {
        let cases = [
            ("<Foo />", "never"),
            ("<Foo prop=\"bar\" />", "never"),
            ("<Foo {...this.props} />", "never"),
            ("<Foo a a a />", "never"),
            ("\n<Foo a\n  b\n/>\n", "never"),
            ("<Foo />", "multiline"),
            ("<Foo prop=\"one\" />", "multiline"),
            ("<Foo {...this.props} />", "multiline"),
            ("<Foo a a a />", "multiline"),
            (
                "\n<Foo\n  propOne=\"one\"\n  propTwo=\"two\"\n/>\n",
                "multiline",
            ),
            (
                "\n<Foo\n  {...this.props}\n  propTwo=\"two\"\n/>\n",
                "multiline",
            ),
            ("\n<Foo bar />\n", "multiline-multiprop"),
            ("\n<Foo bar baz />\n", "multiline-multiprop"),
            ("\n<Foo prop={{\n}} />\n", "multiline-multiprop"),
            ("\n<Foo\n  foo={{\n  }}\n  bar\n/>\n", "multiline-multiprop"),
            ("<Foo />", "always"),
            (
                "\n<Foo\n  propOne=\"one\"\n  propTwo=\"two\"\n/>\n",
                "always",
            ),
            (
                "\n<Foo\n  {...this.props}\n  propTwo=\"two\"\n/>\n",
                "always",
            ),
            ("\n<Foo />\n", "multiprop"),
            ("\n<Foo bar />\n", "multiprop"),
            ("\n<Foo {...this.props} />\n", "multiprop"),
        ];
        for (source, option) in cases {
            assert!(
                run(source, Some("fixture.tsx"), json!([option])).is_empty(),
                "{option}: {source}"
            );
        }
    }

    #[test]
    fn reports_full_attribute_ranges_and_exact_replacement_boundaries() {
        let source = "<UI.Root first=\"one\" second />;";
        let diagnostics = run(source, Some("fixture.tsx"), json!(["always"]));
        assert_eq!(
            ids(source, Some("fixture.tsx"), json!(["always"])),
            [PROP_ON_NEW_LINE]
        );
        assert_eq!(diagnostics[0].range, TextRange::new(9, 20));
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].range,
            TextRange::new(8, 9)
        );
        assert_eq!(
            apply(source, &diagnostics),
            "<UI.Root\nfirst=\"one\" second />;"
        );

        let source = "<svg:path\nxml:lang=\"en\" />;";
        let diagnostics = run(source, Some("fixture.tsx"), json!(["never"]));
        assert_eq!(diagnostics[0].range, TextRange::new(10, 23));
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].range,
            TextRange::new(9, 10)
        );
        assert_eq!(apply(source, &diagnostics), "<svg:path xml:lang=\"en\" />;");
    }

    #[test]
    fn handles_spreads_boolean_properties_and_nested_opening_elements() {
        let source = concat!(
            "<Outer first second>",
            "<Inner {...props} value={1} />",
            "<Leaf\nflag />",
            "</Outer>"
        );
        let diagnostics = run(source, Some("fixture.tsx"), json!(["multiprop"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [PROP_ON_NEW_LINE, PROP_ON_NEW_LINE, PROP_ON_SAME_LINE]
        );
        assert_eq!(
            apply(source, &diagnostics),
            concat!(
                "<Outer\nfirst second>",
                "<Inner\n{...props} value={1} />",
                "<Leaf flag />",
                "</Outer>"
            )
        );
    }

    #[test]
    fn preserves_typescript_generic_type_arguments_for_newline_fixes() {
        let source = "<DataTable<Items> fullscreen keyField=\"id\" items={items} />";
        let diagnostics = run(source, Some("fixture.tsx"), json!(["multiline"]));
        assert!(
            diagnostics.is_empty(),
            "single-line opening element is valid"
        );

        let source = "<DataTable<Items> fullscreen items={{\nvalue: 1\n}} />";
        let diagnostics = run(source, Some("fixture.tsx"), json!(["multiline"]));
        assert_eq!(diagnostics.len(), 1);
        let fix = &diagnostics[0].suggestions[0].fixes[0];
        assert_eq!(
            &source[fix.range.start as usize..fix.range.end as usize],
            " "
        );
        assert_eq!(
            apply(source, &diagnostics),
            "<DataTable<Items>\nfullscreen items={{\nvalue: 1\n}} />"
        );
    }

    #[test]
    fn matches_upstream_comment_replacement_semantics() {
        let source = "<Foo /* displaced */ first second />";
        let diagnostics = run(source, Some("fixture.tsx"), json!(["always"]));
        assert_eq!(
            apply(source, &diagnostics),
            "<Foo\nfirst second />",
            "upstream replaces the whole name-to-property gap"
        );

        let source = "<Foo\n/* displaced */\nfirst />";
        let diagnostics = run(source, Some("fixture.tsx"), json!(["never"]));
        assert_eq!(apply(source, &diagnostics), "<Foo first />");
    }

    #[test]
    fn supports_unicode_crlf_and_all_ecmascript_line_terminators() {
        let source = concat!(
            "const 日本語 = <部品\r\n値=\"😀\" />;\r\n",
            "const solo = <Solo\rprop />;\r",
            "const café = <Élément\u{2028}nom=\"été\" />;\u{2028}",
            "const τέλος = <Στοιχείο\u{2029}τιμή=\"κόσμος\" />;\u{2029}",
            "const nested = <Outer><内側\n属性 /></Outer>;\n"
        );
        let diagnostics = run(source, Some("fixture.tsx"), json!(["never"]));
        assert_eq!(diagnostics.len(), 5);
        assert_eq!(
            apply(source, &diagnostics),
            concat!(
                "const 日本語 = <部品 値=\"😀\" />;\r\n",
                "const solo = <Solo prop />;\r",
                "const café = <Élément nom=\"été\" />;\u{2028}",
                "const τέλος = <Στοιχείο τιμή=\"κόσμος\" />;\u{2029}",
                "const nested = <Outer><内側 属性 /></Outer>;\n"
            )
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.suggestions.len() == 1)
        );
    }

    #[test]
    fn ignores_invalid_jsx_and_non_jsx_and_falls_back_for_bad_options() {
        for (source, filename) in [
            ("const object = { first: 1, second: 2 };", "fixture.js"),
            ("type Props = { first: string };", "fixture.ts"),
            ("const view = <Foo first={value />;", "fixture.tsx"),
            ("const view = <Foo first></Bar>;", "fixture.jsx"),
        ] {
            assert!(
                run(source, Some(filename), json!(["always"])).is_empty(),
                "{source}"
            );
        }

        let source = "<Foo first={{\nvalue: 1\n}} second />";
        for options in [
            json!(["sideways"]),
            json!([42]),
            json!([{ "mode": "always" }]),
            Value::Null,
        ] {
            assert_eq!(
                ids(source, Some("fixture.tsx"), options),
                [PROP_ON_NEW_LINE],
                "invalid options use multiline-multiprop default"
            );
        }
    }
}
