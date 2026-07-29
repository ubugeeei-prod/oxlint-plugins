//! Stylistic rules for JSX syntax.
//!
//! The shared lexer deliberately stays parser-free, so this module recognizes
//! JSX opening tags conservatively. It only inspects top-level `=` tokens whose
//! right-hand side is a valid JSX attribute value (`"…"`, `'…'`, or `{…}`).
//! Braced expressions are skipped through the shared bracket map, keeping
//! assignments and comparisons inside attribute expressions out of scope.

use serde_json::Value;

use crate::{LintDiagnostic, LintFix, TextRange};

use super::context::{Scan, is_whitespace, option_keyword, punct_is, push};
use super::lexer::TokenKind;

/// Enforces spacing around `=` in JSX attributes.
pub(crate) fn check_jsx_equals_spacing(
    scan: &Scan,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let always = option_keyword(options, "never") == "always";
    let tokens = scan.tokens();
    let mut jsx_depth = 0_usize;
    let mut index = 0_usize;

    while index < tokens.len() {
        if !punct_is(&tokens[index], scan.source(), "<") {
            index += 1;
            continue;
        }

        if is_jsx_closing_tag(scan, index) {
            if let Some(close) = find_tag_close(scan, index + 1) {
                jsx_depth = jsx_depth.saturating_sub(1);
                index = close + 1;
                continue;
            }
        }

        if is_jsx_fragment_open(scan, index) {
            jsx_depth += 1;
            index += 2;
            continue;
        }

        let Some(name_index) = scan.next_significant(index) else {
            break;
        };
        if tokens[name_index].kind != TokenKind::Identifier
            || (jsx_depth == 0 && !can_start_jsx_root(scan, index))
        {
            index += 1;
            continue;
        }

        let Some(close) = find_tag_close(scan, name_index + 1) else {
            index += 1;
            continue;
        };
        if jsx_depth == 0 && looks_like_type_parameter_list(scan, close) {
            index = close + 1;
            continue;
        }

        let tag_name = jsx_tag_name_end(scan, name_index, close);
        check_opening_tag_attributes(scan, tag_name, name_index + 1, close, always, diagnostics);

        let self_closing = scan
            .prev_significant(close)
            .is_some_and(|previous| punct_is(&tokens[previous], scan.source(), "/"));
        if !self_closing {
            jsx_depth += 1;
        }
        index = close + 1;
    }
}

fn jsx_tag_name_end(scan: &Scan, start: usize, close: usize) -> usize {
    let tokens = scan.tokens();
    let mut end = start;
    let mut cursor = start + 1;

    while cursor + 1 < close {
        let separator = &tokens[cursor];
        let segment = &tokens[cursor + 1];
        if !scan.gap(&tokens[end], separator).is_empty()
            || !scan.gap(separator, segment).is_empty()
            || !matches!(scan.token_text(cursor), "." | ":" | "-")
            || segment.kind != TokenKind::Identifier
        {
            break;
        }
        end = cursor + 1;
        cursor += 2;
    }

    end
}

fn check_opening_tag_attributes(
    scan: &Scan,
    tag_name: usize,
    start: usize,
    close: usize,
    always: bool,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = scan.tokens();
    let mut index = start;

    while index < close {
        if punct_is(&tokens[index], scan.source(), "{") {
            if let Some(partner) = scan.partner(index) {
                index = partner + 1;
                continue;
            }
        }

        if !punct_is(&tokens[index], scan.source(), "=") {
            index += 1;
            continue;
        }

        let Some(name) = scan.prev_significant(index) else {
            index += 1;
            continue;
        };
        let Some(value) = scan.next_significant(index) else {
            break;
        };
        if name == tag_name
            || tokens[name].kind != TokenKind::Identifier
            || !is_jsx_attribute_value(scan, value, close)
            || tokens[name + 1..index]
                .iter()
                .any(|token| token.kind.is_comment())
            || tokens[index + 1..value]
                .iter()
                .any(|token| token.kind.is_comment())
        {
            index += 1;
            continue;
        }

        let before = scan.gap(&tokens[name], &tokens[index]);
        let after = scan.gap(&tokens[index], &tokens[value]);
        if always {
            if before.is_empty() {
                report_spacing(
                    diagnostics,
                    "needSpaceBefore",
                    "A space is required before '='",
                    &tokens[index],
                    "insertSpace",
                    "Insert a space.",
                    LintFix::replace_range(
                        TextRange::new(
                            u32::try_from(tokens[index].start).unwrap_or(u32::MAX),
                            u32::try_from(tokens[index].start).unwrap_or(u32::MAX),
                        ),
                        " ",
                    ),
                );
            }
            if after.is_empty() {
                report_spacing(
                    diagnostics,
                    "needSpaceAfter",
                    "A space is required after '='",
                    &tokens[index],
                    "insertSpace",
                    "Insert a space.",
                    LintFix::replace_range(
                        TextRange::new(
                            u32::try_from(tokens[index].end).unwrap_or(u32::MAX),
                            u32::try_from(tokens[index].end).unwrap_or(u32::MAX),
                        ),
                        " ",
                    ),
                );
            }
        } else {
            if is_whitespace(before) {
                report_spacing(
                    diagnostics,
                    "noSpaceBefore",
                    "There should be no space before '='",
                    &tokens[index],
                    "removeSpace",
                    "Remove the whitespace.",
                    LintFix::remove_range(TextRange::new(
                        u32::try_from(tokens[name].end).unwrap_or(u32::MAX),
                        u32::try_from(tokens[index].start).unwrap_or(u32::MAX),
                    )),
                );
            }
            if is_whitespace(after) {
                report_spacing(
                    diagnostics,
                    "noSpaceAfter",
                    "There should be no space after '='",
                    &tokens[index],
                    "removeSpace",
                    "Remove the whitespace.",
                    LintFix::remove_range(TextRange::new(
                        u32::try_from(tokens[index].end).unwrap_or(u32::MAX),
                        u32::try_from(tokens[value].start).unwrap_or(u32::MAX),
                    )),
                );
            }
        }

        index += 1;
    }
}

fn report_spacing(
    diagnostics: &mut Vec<LintDiagnostic>,
    message_id: &'static str,
    message: &'static str,
    equals: &super::lexer::Token,
    suggestion_id: &'static str,
    suggestion_message: &'static str,
    fix: LintFix,
) {
    push(
        diagnostics,
        "jsx-equals-spacing",
        message_id,
        message,
        equals.start,
        equals.end,
        suggestion_id,
        suggestion_message,
        fix,
    );
}

fn is_jsx_attribute_value(scan: &Scan, index: usize, tag_close: usize) -> bool {
    let token = &scan.tokens()[index];
    match token.kind {
        TokenKind::String => true,
        TokenKind::Punctuator if punct_is(token, scan.source(), "{") => scan
            .partner(index)
            .is_some_and(|partner| partner < tag_close),
        _ => false,
    }
}

fn find_tag_close(scan: &Scan, start: usize) -> Option<usize> {
    let tokens = scan.tokens();
    let mut index = start;

    while index < tokens.len() {
        if punct_is(&tokens[index], scan.source(), "{") {
            index = scan.partner(index)?.saturating_add(1);
            continue;
        }
        if punct_is(&tokens[index], scan.source(), ">") {
            return Some(index);
        }
        // A new `<` before the close means the candidate was an ordinary
        // comparison or malformed JSX. Do not consume the nested candidate.
        if punct_is(&tokens[index], scan.source(), "<") {
            return None;
        }
        index += 1;
    }

    None
}

fn is_jsx_closing_tag(scan: &Scan, open: usize) -> bool {
    scan.next_significant(open)
        .is_some_and(|next| punct_is(&scan.tokens()[next], scan.source(), "/"))
}

fn is_jsx_fragment_open(scan: &Scan, open: usize) -> bool {
    scan.next_significant(open)
        .is_some_and(|next| punct_is(&scan.tokens()[next], scan.source(), ">"))
        && (open == 0 || can_start_jsx_root(scan, open))
}

fn can_start_jsx_root(scan: &Scan, open: usize) -> bool {
    let Some(previous) = scan.prev_significant(open) else {
        return true;
    };
    let token = &scan.tokens()[previous];
    if token.kind == TokenKind::Identifier {
        return matches!(scan.token_text(previous), "return" | "yield" | "case");
    }
    token.kind == TokenKind::Punctuator
        && matches!(
            scan.token_text(previous),
            "=" | "=>" | "(" | "[" | "{" | "," | ":" | ";" | "?" | ">"
        )
}

fn looks_like_type_parameter_list(scan: &Scan, close: usize) -> bool {
    scan.next_significant(close).is_some_and(|next| {
        punct_is(&scan.tokens()[next], scan.source(), "(")
            || punct_is(&scan.tokens()[next], scan.source(), "=>")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str, option: Option<&str>) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let options = option.map_or(Value::Null, |value| {
            Value::Array(std::iter::once(Value::String(value.to_owned())).collect())
        });
        let mut diagnostics = Vec::new();
        check_jsx_equals_spacing(&scan, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id.as_str())
            .collect()
    }

    fn fixes(diagnostics: &[LintDiagnostic]) -> Vec<(TextRange, &str)> {
        diagnostics
            .iter()
            .map(|diagnostic| {
                let fix = &diagnostic.suggestions[0].fixes[0];
                (fix.range, fix.replacement_text.as_str())
            })
            .collect()
    }

    #[test]
    fn accepts_all_upstream_default_and_never_valid_cases() {
        for source in [
            "<App />",
            "<App foo />",
            "<App foo=\"bar\" />",
            "<App foo={e => bar(e)} />",
            "<App {...props} />",
        ] {
            assert!(run(source, None).is_empty(), "default rejected {source}");
            assert!(
                run(source, Some("never")).is_empty(),
                "never rejected {source}"
            );
        }
    }

    #[test]
    fn accepts_all_upstream_always_valid_cases() {
        for source in [
            "<App />",
            "<App foo />",
            "<App foo = \"bar\" />",
            "<App foo = {e => bar(e)} />",
            "<App {...props} />",
        ] {
            assert!(
                run(source, Some("always")).is_empty(),
                "always rejected {source}"
            );
        }
    }

    #[test]
    fn ports_upstream_never_invalid_cases_and_exact_fixes() {
        let diagnostics = run("<App foo = {bar} />", None);
        assert_eq!(ids(&diagnostics), ["noSpaceBefore", "noSpaceAfter"]);
        assert_eq!(
            fixes(&diagnostics),
            [(TextRange::new(8, 9), ""), (TextRange::new(10, 11), "")]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [TextRange::new(9, 10), TextRange::new(9, 10)]
        );

        assert_eq!(
            ids(&run("<App foo ={bar} />", Some("never"))),
            ["noSpaceBefore"]
        );
        assert_eq!(
            ids(&run("<App foo= {bar} />", Some("never"))),
            ["noSpaceAfter"]
        );
        assert_eq!(
            ids(&run("<App foo= {bar} bar = {baz} />", Some("never"))),
            ["noSpaceAfter", "noSpaceBefore", "noSpaceAfter"]
        );
    }

    #[test]
    fn ports_upstream_always_invalid_cases_and_exact_fixes() {
        let diagnostics = run("<App foo={bar} />", Some("always"));
        assert_eq!(ids(&diagnostics), ["needSpaceBefore", "needSpaceAfter"]);
        assert_eq!(
            fixes(&diagnostics),
            [(TextRange::new(8, 8), " "), (TextRange::new(9, 9), " ")]
        );

        assert_eq!(
            ids(&run("<App foo ={bar} />", Some("always"))),
            ["needSpaceAfter"]
        );
        assert_eq!(
            ids(&run("<App foo= {bar} />", Some("always"))),
            ["needSpaceBefore"]
        );
        assert_eq!(
            ids(&run("<App foo={bar} bar ={baz} />", Some("always"))),
            ["needSpaceBefore", "needSpaceAfter", "needSpaceAfter"]
        );
    }

    #[test]
    fn handles_nested_member_namespaced_and_hyphenated_jsx() {
        let source = concat!(
            "<UI.Root data-id = \"root\">",
            "text<svg:path xml:lang = {'en'} aria-label= \"label\" />",
            "</UI.Root>"
        );
        assert_eq!(
            ids(&run(source, Some("never"))),
            [
                "noSpaceBefore",
                "noSpaceAfter",
                "noSpaceBefore",
                "noSpaceAfter",
                "noSpaceAfter"
            ]
        );
    }

    #[test]
    fn handles_fragments_and_nested_tags_after_text() {
        let source =
            "<><App foo = \"a\" />text<Other bar= {value}>child<Leaf baz ={x} /></Other></>";
        assert_eq!(
            ids(&run(source, Some("never"))),
            [
                "noSpaceBefore",
                "noSpaceAfter",
                "noSpaceAfter",
                "noSpaceBefore"
            ]
        );
    }

    #[test]
    fn ignores_spreads_boolean_attributes_and_expression_internals() {
        let source = concat!(
            "<App enabled {...props} ",
            "value={fallback = next} ",
            "compare={left <= right && right >= floor} ",
            "nested={{ key: assigned = value }} />"
        );
        assert!(run(source, Some("never")).is_empty());
    }

    #[test]
    fn ignores_non_jsx_assignments_comparisons_and_typescript_generics() {
        for source in [
            "const value = other;",
            "if (left < right && right > floor) {}",
            "type Box<T = 'default'> = { value: T };",
            "function identity<T = string>(value: T): T { return value; }",
            "const identity = <T = string>(value: T) => value;",
            "const literal = '<App foo = \"bar\" />';",
            "const template = `<App foo = \"bar\" />`;",
            "// <App foo = \"bar\" />\nconst value = 1;",
            "/* <App foo = \"bar\" /> */ const value = 1;",
        ] {
            assert!(run(source, None).is_empty(), "false positive for {source}");
        }
    }

    #[test]
    fn supports_single_quoted_multiline_and_unicode_attribute_values() {
        let source = "<日本語 ラベル \n=\n '値' emoji = {'😀'} />";
        let diagnostics = run(source, Some("never"));
        assert_eq!(
            ids(&diagnostics),
            [
                "noSpaceBefore",
                "noSpaceAfter",
                "noSpaceBefore",
                "noSpaceAfter"
            ]
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.range.start > 9)
        );
    }

    #[test]
    fn reports_each_side_independently_for_tabs_and_newlines() {
        assert_eq!(
            ids(&run("<App foo\t=\n{bar} />", Some("never"))),
            ["noSpaceBefore", "noSpaceAfter"]
        );
        assert!(run("<App foo\t=\n{bar} />", Some("always")).is_empty());
    }

    #[test]
    fn does_not_treat_malformed_or_bare_equal_syntax_as_an_attribute() {
        for source in [
            "<App = \"value\" />",
            "<UI.Root = \"value\" />",
            "<App foo = value />",
            "<App foo = />",
            "<App foo /* comment */ = \"value\" />",
            "<App foo = {unterminated />",
        ] {
            assert!(run(source, None).is_empty(), "reported malformed {source}");
        }
    }
}
