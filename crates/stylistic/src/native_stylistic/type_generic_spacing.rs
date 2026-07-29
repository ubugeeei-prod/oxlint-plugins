//! Native implementation of stable `@stylistic/type-generic-spacing`.
//!
//! The upstream rule listens to TypeScript's generic declaration,
//! instantiation, and parameter nodes. Oxc provides those exact boundaries;
//! the checks below intentionally preserve the upstream rule's unusual
//! comment and newline behavior so diagnostics and whitespace fixes stay
//! compatible with v5.10.0.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{
        ClassType, FunctionType, TSType, TSTypeParameter, TSTypeParameterDeclaration,
        TSTypeParameterInstantiation,
    },
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::lexer::tokenize;

const RULE: &str = "type-generic-spacing";
const MESSAGE_ID: &str = "genericSpacingMismatch";
const MESSAGE: &str = "Generic spaces mismatch";

/// Checks generic bracket, declaration-prefix, and default-value spacing.
pub(crate) fn check_type_generic_spacing(
    source: &str,
    filename: Option<&str>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, diagnostics);
        return;
    }

    for source_type in [
        SourceType::ts(),
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, diagnostics) {
            return;
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

    let mut visitor = TypeGenericSpacing {
        source,
        preserve_prefix_stack: Vec::new(),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct TypeGenericSpacing<'source, 'diagnostics> {
    source: &'source str,
    preserve_prefix_stack: Vec<bool>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for TypeGenericSpacing<'_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        let preserve_prefix = match kind {
            AstKind::TSCallSignatureDeclaration(_)
            | AstKind::ArrowFunctionExpression(_)
            | AstKind::TSFunctionType(_)
            | AstKind::TSConstructorType(_) => true,
            AstKind::Function(function) => function.r#type == FunctionType::FunctionExpression,
            AstKind::Class(class) => class.r#type == ClassType::ClassExpression,
            _ => false,
        };
        self.preserve_prefix_stack.push(preserve_prefix);
    }

    fn leave_node(&mut self, _kind: AstKind<'ast>) {
        self.preserve_prefix_stack.pop();
    }

    fn visit_ts_type_parameter_instantiation(&mut self, node: &TSTypeParameterInstantiation<'ast>) {
        if let (Some(first), Some(last)) = (node.params.first(), node.params.last()) {
            self.check_bracket_spacing(node.span, first.span(), last.span());
        }
        walk::walk_ts_type_parameter_instantiation(self, node);
    }

    fn visit_ts_type_parameter_declaration(&mut self, node: &TSTypeParameterDeclaration<'ast>) {
        if !self.preserve_prefix_stack.last().copied().unwrap_or(false) {
            self.check_declaration_prefix(node.span);
        }

        if let (Some(first), Some(last)) = (node.params.first(), node.params.last()) {
            self.check_bracket_spacing(node.span, first.span, last.span);
        }
        walk::walk_ts_type_parameter_declaration(self, node);
    }

    fn visit_ts_type_parameter(&mut self, node: &TSTypeParameter<'ast>) {
        self.check_default_spacing(node);
        walk::walk_ts_type_parameter(self, node);
    }
}

impl TypeGenericSpacing<'_, '_> {
    fn check_declaration_prefix(&mut self, generic: Span) {
        let generic_start = generic.start as usize;
        let whitespace_start = trailing_js_whitespace_start(self.source, generic_start);
        if whitespace_start == generic_start {
            return;
        }

        self.report(
            generic.start as usize,
            generic.end as usize,
            whitespace_start,
            generic_start,
            "",
        );
    }

    fn check_bracket_spacing(&mut self, generic: Span, first: Span, last: Span) {
        let open_end = (generic.start as usize).saturating_add(1);
        let first_start = first.start as usize;
        self.remove_space_between(open_end, first_start);

        let mut last_end = last.end as usize;
        let close_start = (generic.end as usize).saturating_sub(1);
        if let Some(trailing) = slice(self.source, last_end, close_start)
            && let Some(token) = tokenize(trailing)
                .into_iter()
                .rev()
                .find(|token| !token.kind.is_comment())
        {
            // A trailing comma belongs to the generic list but not the final
            // parameter span. SourceCode#getTokenBefore(closeToken) sees it.
            last_end = last_end.saturating_add(token.end);
        }
        self.remove_space_between(last_end, close_start);
    }

    fn remove_space_between(&mut self, left_end: usize, right_start: usize) {
        let Some(gap) = slice(self.source, left_end, right_start) else {
            return;
        };

        // Stable upstream behavior: a gap beginning with CR/LF is preserved,
        // while a leading space before a later newline is still reported.
        if gap.chars().any(is_js_whitespace)
            && !gap
                .as_bytes()
                .first()
                .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
        {
            self.report(left_end, right_start, left_end, right_start, "");
        }
    }

    fn check_default_spacing(&mut self, node: &TSTypeParameter<'_>) {
        let Some(default) = &node.default else {
            return;
        };
        let left = node.constraint.as_ref().map_or_else(
            || node.name.span(),
            |constraint| inner_type_span(constraint),
        );
        let from = left.end as usize;
        let to = inner_type_span(default).start as usize;
        let Some(gap) = slice(self.source, from, to) else {
            return;
        };
        if valid_default_gap(gap) {
            return;
        }

        let replacement = normalize_default_gap(gap);
        self.report(from, to, from, to, replacement);
    }

    fn report(
        &mut self,
        report_start: usize,
        report_end: usize,
        fix_start: usize,
        fix_end: usize,
        replacement: impl Into<String>,
    ) {
        let (Ok(report_start), Ok(report_end), Ok(fix_start), Ok(fix_end)) = (
            u32::try_from(report_start),
            u32::try_from(report_end),
            u32::try_from(fix_start),
            u32::try_from(fix_end),
        ) else {
            return;
        };
        let fix = LintFix::replace_range(TextRange::new(fix_start, fix_end), replacement.into());
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: MESSAGE.to_owned(),
            data: BTreeMap::new(),
            range: TextRange::new(report_start, report_end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message: MESSAGE.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

/// Mirrors `/(?:^|[^ ]) = (?:$|[^ ])/` without allocating a regex.
fn valid_default_gap(gap: &str) -> bool {
    gap.as_bytes()
        .windows(3)
        .enumerate()
        .any(|(index, window)| {
            window == b" = "
                && (index == 0 || gap.as_bytes()[index - 1] != b' ')
                && (index + 3 == gap.len() || gap.as_bytes()[index + 3] != b' ')
        })
}

fn inner_type_span(mut type_annotation: &TSType<'_>) -> Span {
    while let TSType::TSParenthesizedType(parenthesized) = type_annotation {
        type_annotation = &parenthesized.type_annotation;
    }
    type_annotation.span()
}

/// Mirrors the first-only `/\s*=\s*/` replacement used by upstream.
fn normalize_default_gap(gap: &str) -> String {
    let Some(equals) = gap.find('=') else {
        return gap.to_owned();
    };
    let before = trailing_js_whitespace_start(gap, equals);
    let after = leading_js_whitespace_end(gap, equals + 1);
    let mut replacement =
        String::with_capacity(gap.len().saturating_sub(after - before).saturating_add(3));
    replacement.push_str(&gap[..before]);
    replacement.push_str(" = ");
    replacement.push_str(&gap[after..]);
    replacement
}

fn trailing_js_whitespace_start(source: &str, end: usize) -> usize {
    let end = end.min(source.len());
    let mut start = end;
    for (offset, character) in source[..end].char_indices().rev() {
        if !is_js_whitespace(character) {
            break;
        }
        start = offset;
    }
    start
}

fn leading_js_whitespace_end(source: &str, start: usize) -> usize {
    let start = start.min(source.len());
    let mut end = start;
    for character in source[start..].chars() {
        if !is_js_whitespace(character) {
            break;
        }
        end += character.len_utf8();
    }
    end
}

fn is_js_whitespace(character: char) -> bool {
    character.is_whitespace() || matches!(character, '\u{feff}')
}

fn slice(source: &str, start: usize, end: usize) -> Option<&str> {
    (start <= end && end <= source.len()).then(|| &source[start..end])
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::Value;

    use super::*;

    const FIXTURE: &str =
        include_str!("../../../../npm/stylistic/test/fixtures/type-generic-spacing.json");

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
    }

    #[derive(Deserialize)]
    struct TestCase {
        code: String,
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
        range: [u32; 2],
        fix: ExpectedFix,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn run(source: &str, filename: Option<&str>) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_type_generic_spacing(source, filename, &mut diagnostics);
        diagnostics
    }

    fn message_ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id.as_str())
            .collect()
    }

    fn first_fix(diagnostic: &LintDiagnostic) -> &LintFix {
        &diagnostic.suggestions[0].fixes[0]
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut fixes = diagnostics.iter().map(first_fix).collect::<Vec<_>>();
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

    fn upstream_fixture() -> Fixture {
        serde_json::from_str(FIXTURE).expect("generated upstream fixture is valid JSON")
    }

    #[test]
    fn replays_every_stable_upstream_valid_case() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.valid.len(), 15);
        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, Some("fixture.ts")).is_empty(),
                "upstream valid case {index} reported:\n{}",
                test_case.code
            );
        }
    }

    #[test]
    fn replays_every_stable_upstream_diagnostic_range_and_fix_exactly() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.invalid.len(), 18);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .flat_map(|test_case| &test_case.errors)
                .count(),
            28
        );

        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, Some("fixture.ts"));
            assert_eq!(
                message_ids(&diagnostics),
                test_case
                    .errors
                    .iter()
                    .map(|error| error.message_id.as_str())
                    .collect::<Vec<_>>(),
                "message mismatch for upstream invalid case {index}:\n{}",
                test_case.code
            );
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| [diagnostic.range.start, diagnostic.range.end])
                    .collect::<Vec<_>>(),
                test_case
                    .errors
                    .iter()
                    .map(|error| error.range)
                    .collect::<Vec<_>>(),
                "range mismatch for upstream invalid case {index}:\n{}",
                test_case.code
            );
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| {
                        let fix = first_fix(diagnostic);
                        (
                            [fix.range.start, fix.range.end],
                            fix.replacement_text.as_str(),
                        )
                    })
                    .collect::<Vec<_>>(),
                test_case
                    .errors
                    .iter()
                    .map(|error| (error.fix.range, error.fix.text.as_str()))
                    .collect::<Vec<_>>(),
                "fix mismatch for upstream invalid case {index}:\n{}",
                test_case.code
            );
            assert_eq!(
                apply_fixes(&test_case.code, &diagnostics),
                test_case
                    .output
                    .as_deref()
                    .expect("every upstream invalid case is fixable"),
                "output mismatch for upstream invalid case {index}:\n{}",
                test_case.code
            );
            assert!(
                test_case.options.is_null()
                    || test_case.options.as_array().is_some_and(Vec::is_empty),
                "stable rule has no options"
            );
        }
    }

    #[test]
    fn covers_nested_references_calls_constructors_and_instantiation_expressions() {
        let cases = [
            (
                "type Box = Outer< Inner< string >, Promise< number > >;",
                "type Box = Outer<Inner<string>, Promise<number>>;",
            ),
            (
                "const value = factory< Map< string, Set< number > > >();",
                "const value = factory<Map<string, Set<number>>>();",
            ),
            (
                "const value = new Factory< Array< string > >();",
                "const value = new Factory<Array<string>>();",
            ),
            (
                "const ctor = Factory< string >;",
                "const ctor = Factory<string>;",
            ),
        ];
        for (source, expected) in cases {
            let diagnostics = run(source, Some("fixture.ts"));
            assert!(!diagnostics.is_empty(), "expected diagnostics for {source}");
            assert_eq!(apply_fixes(source, &diagnostics), expected);
        }
    }

    #[test]
    fn preserves_prefix_spaces_for_the_six_stable_parent_kinds() {
        for source in [
            "interface Log { <T>(name: T): void }",
            "const arrow = <T>(name: T) => name;",
            "type FunctionType = <T>(name: T) => T;",
            "type ConstructorType = new <T>(name: T) => T;",
            "const expression = function <T>(name: T) {};",
            "const expression = class <T> {};",
        ] {
            assert!(
                run(source, Some("fixture.ts")).is_empty(),
                "preserved parent spacing reported for {source}"
            );
        }
    }

    #[test]
    fn strips_prefix_spaces_for_declarations_methods_and_signatures() {
        let cases = [
            ("function named <T>() {}", "function named<T>() {}"),
            ("class Named <T> {}", "class Named<T> {}"),
            (
                "interface Api { method <T>(value: T): T }",
                "interface Api { method<T>(value: T): T }",
            ),
            (
                "interface Api { new <T>(value: T): T }",
                "interface Api { new<T>(value: T): T }",
            ),
        ];
        for (source, expected) in cases {
            let diagnostics = run(source, Some("fixture.ts"));
            assert_eq!(diagnostics.len(), 1, "unexpected diagnostics for {source}");
            assert_eq!(apply_fixes(source, &diagnostics), expected);
        }
    }

    #[test]
    fn matches_comment_and_newline_quirks_in_bracket_gaps() {
        let valid = [
            "type A = Box<\n string>;",
            "type B = Box<string\n >;",
            "type C<\r\n T> = T;",
        ];
        for source in valid {
            assert!(
                run(source, Some("fixture.ts")).is_empty(),
                "leading CR/LF gap must be preserved: {source:?}"
            );
        }

        let cases = [
            ("type A = Box< \n string>;", "type A = Box<string>;"),
            ("type B = Box</* spaced */string>;", "type B = Box<string>;"),
            ("type C = Box<string/* spaced */>;", "type C = Box<string>;"),
            ("type D = Box<\u{2028}string>;", "type D = Box<string>;"),
        ];
        for (source, expected) in cases {
            let diagnostics = run(source, Some("fixture.ts"));
            assert_eq!(
                diagnostics.len(),
                1,
                "unexpected diagnostics for {source:?}"
            );
            assert_eq!(apply_fixes(source, &diagnostics), expected);
        }
    }

    #[test]
    fn normalizes_defaults_with_constraints_comments_tabs_and_linebreaks() {
        let cases = [
            ("type A<T=true> = T;", "type A<T = true> = T;"),
            (
                "type B<T extends string=Array<number>> = T;",
                "type B<T extends string = Array<number>> = T;",
            ),
            (
                "type C<T/* keep */=/* keep */string> = T;",
                "type C<T/* keep */ = /* keep */string> = T;",
            ),
            ("type D<T\t=\tstring> = T;", "type D<T = string> = T;"),
            ("type E<T\n=\nstring> = T;", "type E<T = string> = T;"),
        ];
        for (source, expected) in cases {
            let diagnostics = run(source, Some("fixture.ts"));
            assert_eq!(
                diagnostics.len(),
                1,
                "unexpected diagnostics for {source:?}"
            );
            assert_eq!(apply_fixes(source, &diagnostics), expected);
        }
    }

    #[test]
    fn keeps_unicode_byte_ranges_and_crlf_fixes_exact() {
        let source = "type 日本語< 値 = string > = 値;\r\nconst 結果 = factory< 数字 >();";
        let diagnostics = run(source, Some("fixture.ts"));
        assert_eq!(diagnostics.len(), 4);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [
                TextRange::new("type 日本語<".len() as u32, "type 日本語< ".len() as u32,),
                TextRange::new(
                    "type 日本語< 値 = string".len() as u32,
                    "type 日本語< 値 = string ".len() as u32,
                ),
                TextRange::new(
                    "type 日本語< 値 = string > = 値;\r\nconst 結果 = factory<".len() as u32,
                    "type 日本語< 値 = string > = 値;\r\nconst 結果 = factory< ".len() as u32,
                ),
                TextRange::new(
                    "type 日本語< 値 = string > = 値;\r\nconst 結果 = factory< 数字".len() as u32,
                    "type 日本語< 値 = string > = 値;\r\nconst 結果 = factory< 数字 ".len() as u32,
                ),
            ]
        );
        assert_eq!(
            apply_fixes(source, &diagnostics),
            "type 日本語<値 = string> = 値;\r\nconst 結果 = factory<数字>();"
        );
    }

    #[test]
    fn supports_tsx_generic_arrows_and_ignores_jsx_angle_brackets() {
        let source = concat!(
            "const identity = <T,>(value: T) => <Panel value={value} />;\n",
            "const nested = <T extends Box< string >,>(value: T) => value;\n",
        );
        let diagnostics = run(source, Some("fixture.tsx"));
        assert_eq!(message_ids(&diagnostics), [MESSAGE_ID, MESSAGE_ID]);
        assert_eq!(
            apply_fixes(source, &diagnostics),
            concat!(
                "const identity = <T,>(value: T) => <Panel value={value} />;\n",
                "const nested = <T extends Box<string>,>(value: T) => value;\n",
            )
        );
    }

    #[test]
    fn parse_errors_are_silent() {
        for source in [
            "type Broken<T = > = T;",
            "const value = factory<;",
            "const jsx = <Panel value={;",
        ] {
            assert!(
                run(
                    source,
                    Some(if source.contains("jsx") {
                        "fixture.tsx"
                    } else {
                        "fixture.ts"
                    })
                )
                .is_empty()
            );
        }
    }
}
