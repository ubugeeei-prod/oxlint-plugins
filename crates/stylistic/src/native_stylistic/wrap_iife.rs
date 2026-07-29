//! Native implementation of `@stylistic/wrap-iife`.
//!
//! Oxc supplies the exact call/function/member structure, including optional
//! chains and TypeScript wrappers. The shared token scan supplies the
//! comment-aware parentheses used by the stable rule's three distinct fixes.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    ast::{CallExpression, ChainElement, Expression, Function, MemberExpression},
    match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::{ParenUse, Scan};

const RULE: &str = "wrap-iife";
const WRAP_INVOCATION: &str = "wrapInvocation";
const WRAP_EXPRESSION: &str = "wrapExpression";
const MOVE_INVOCATION: &str = "moveInvocation";

const WRAP_INVOCATION_MESSAGE: &str = "Wrap an immediate function invocation in parentheses.";
const WRAP_EXPRESSION_MESSAGE: &str = "Wrap only the function expression in parens.";
const MOVE_INVOCATION_MESSAGE: &str =
    "Move the invocation into the parens that contain the function.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    Outside,
    Inside,
    Any,
}

#[derive(Clone, Copy, Debug)]
struct Options {
    style: Style,
    function_prototype_methods: bool,
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let items = options.as_array();
        let style = items
            .and_then(|items| items.first())
            .or_else(|| (!options.is_array() && !options.is_null()).then_some(options))
            .and_then(Value::as_str)
            .map_or(Style::Outside, |style| match style {
                "inside" => Style::Inside,
                "any" => Style::Any,
                _ => Style::Outside,
            });
        let function_prototype_methods = items
            .and_then(|items| items.get(1))
            .and_then(|option| option.get("functionPrototypeMethods"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Self {
            style,
            function_prototype_methods,
        }
    }
}

pub(crate) fn check_wrap_iife(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    scan: &Scan<'_>,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_json(options);

    if let Some(source_type) = filename.and_then(|value| SourceType::from_path(value).ok()) {
        let _ = parse_and_check(source, source_type, options, scan, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, options, scan, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    options: Options,
    scan: &Scan<'_>,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    WrapIifeVisitor {
        source,
        scan,
        options,
        diagnostics,
    }
    .visit_program(&parsed.program);
    true
}

struct WrapIifeVisitor<'source, 'scan, 'diagnostics> {
    source: &'source str,
    scan: &'scan Scan<'source>,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for WrapIifeVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        // The upstream listener runs on CallExpression entry, so containing
        // calls must be checked before nested argument calls.
        self.check_call(call);
        walk::walk_call_expression(self, call);
    }
}

impl WrapIifeVisitor<'_, '_, '_> {
    fn check_call(&mut self, call: &CallExpression<'_>) {
        let Some(function) =
            function_from_iife_callee(&call.callee, self.options.function_prototype_methods)
        else {
            return;
        };

        let call_wrapped = wrapped_tokens(self.scan, call.span);
        let function_wrapped = wrapped_tokens(self.scan, function.span);

        if call_wrapped.is_none() && function_wrapped.is_none() {
            let span = if self.options.style == Style::Inside {
                function.span
            } else {
                call.span
            };
            let Some(fix) = parenthesize(self.source, span) else {
                return;
            };
            self.report(call.span, WRAP_INVOCATION, WRAP_INVOCATION_MESSAGE, fix);
            return;
        }

        if self.options.style == Style::Inside && function_wrapped.is_none() {
            let fix = if call_wrapped.is_some_and(|parens| is_grouping_parens(self.scan, parens))
                && !is_callee_of_new_expression(self.scan, call.span)
            {
                move_parens_inside(self.source, self.scan, function.span, call.span)
            } else {
                parenthesize(self.source, function.span)
            };
            let Some(fix) = fix else {
                return;
            };
            self.report(call.span, WRAP_EXPRESSION, WRAP_EXPRESSION_MESSAGE, fix);
            return;
        }

        if self.options.style == Style::Outside && call_wrapped.is_none() {
            let Some(fix) = move_parens_outside(self.source, self.scan, function.span, call.span)
            else {
                return;
            };
            self.report(call.span, MOVE_INVOCATION, MOVE_INVOCATION_MESSAGE, fix);
        }
    }

    fn report(&mut self, span: Span, message_id: &str, message: &str, fix: LintFix) {
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: TextRange::new(span.start, span.end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

fn function_from_iife_callee<'ast>(
    callee: &'ast Expression<'ast>,
    include_prototype_methods: bool,
) -> Option<&'ast Function<'ast>> {
    let callee = skip_parentheses(callee);
    match callee {
        Expression::FunctionExpression(function) => Some(function),
        member @ match_member_expression!(Expression) if include_prototype_methods => {
            function_from_prototype_member(member.to_member_expression())
        }
        Expression::ChainExpression(chain) if include_prototype_methods => {
            match &chain.expression {
                member @ match_member_expression!(ChainElement) => {
                    function_from_prototype_member(member.to_member_expression())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn function_from_prototype_member<'ast>(
    member: &'ast MemberExpression<'ast>,
) -> Option<&'ast Function<'ast>> {
    if !matches!(member.static_property_name(), Some("call" | "apply")) {
        return None;
    }
    match skip_parentheses(member.object()) {
        Expression::FunctionExpression(function) => Some(function),
        _ => None,
    }
}

fn skip_parentheses<'ast>(mut expression: &'ast Expression<'ast>) -> &'ast Expression<'ast> {
    while let Expression::ParenthesizedExpression(parenthesized) = expression {
        expression = &parenthesized.expression;
    }
    expression
}

#[derive(Clone, Copy, Debug)]
struct Parens {
    open: usize,
    close: usize,
}

fn wrapped_tokens(scan: &Scan<'_>, span: Span) -> Option<Parens> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    let open = scan
        .tokens()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, token)| !token.kind.is_comment() && token.end <= start)
        .map(|(index, _)| index)?;
    let close = scan
        .tokens()
        .iter()
        .enumerate()
        .find(|(_, token)| !token.kind.is_comment() && token.start >= end)
        .map(|(index, _)| index)?;
    (scan.token_text(open) == "(" && scan.token_text(close) == ")")
        .then_some(Parens { open, close })
}

fn is_grouping_parens(scan: &Scan<'_>, parens: Parens) -> bool {
    if scan.partner(parens.open) != Some(parens.close) {
        return false;
    }
    if scan.paren_use(parens.open) != ParenUse::Grouping {
        return false;
    }
    scan.prev_significant(parens.open)
        .is_none_or(|previous| scan.token_text(previous) != "import")
}

fn is_callee_of_new_expression(scan: &Scan<'_>, span: Span) -> bool {
    let mut current = span;
    while let Some(parens) = wrapped_tokens(scan, current) {
        if scan.partner(parens.open) != Some(parens.close) {
            return false;
        }
        if scan
            .prev_significant(parens.open)
            .is_some_and(|previous| scan.token_text(previous) == "new")
        {
            return true;
        }
        let (Ok(start), Ok(end)) = (
            u32::try_from(scan.tokens()[parens.open].start),
            u32::try_from(scan.tokens()[parens.close].end),
        ) else {
            return false;
        };
        current = Span::new(start, end);
    }
    false
}

fn parenthesize(source: &str, span: Span) -> Option<LintFix> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    let text = source.get(start..end)?;
    let mut replacement = String::with_capacity(text.len().saturating_add(2));
    replacement.push('(');
    replacement.push_str(text);
    replacement.push(')');
    Some(LintFix::replace_range(
        TextRange::new(span.start, span.end),
        replacement,
    ))
}

fn move_parens_inside(
    source: &str,
    scan: &Scan<'_>,
    function_span: Span,
    call_span: Span,
) -> Option<LintFix> {
    let function_end = usize::try_from(function_span.end).ok()?;
    let close = next_significant_from(scan, usize::try_from(call_span.end).ok()?)?;
    if scan.token_text(close) != ")" {
        return None;
    }
    let close_token = &scan.tokens()[close];
    let between = source.get(function_end..close_token.start)?;
    let mut replacement = String::with_capacity(between.len().saturating_add(1));
    replacement.push(')');
    replacement.push_str(between);
    Some(LintFix::replace_range(
        TextRange::new(function_span.end, u32::try_from(close_token.end).ok()?),
        replacement,
    ))
}

fn move_parens_outside(
    source: &str,
    scan: &Scan<'_>,
    function_span: Span,
    call_span: Span,
) -> Option<LintFix> {
    let close = next_significant_from(scan, usize::try_from(function_span.end).ok()?)?;
    if scan.token_text(close) != ")" {
        return None;
    }
    let close_token = &scan.tokens()[close];
    let call_end = usize::try_from(call_span.end).ok()?;
    let suffix = source.get(close_token.end..call_end)?;
    let mut replacement = String::with_capacity(suffix.len().saturating_add(1));
    replacement.push_str(suffix);
    replacement.push(')');
    Some(LintFix::replace_range(
        TextRange::new(u32::try_from(close_token.start).ok()?, call_span.end),
        replacement,
    ))
}

fn next_significant_from(scan: &Scan<'_>, offset: usize) -> Option<usize> {
    scan.tokens()
        .iter()
        .enumerate()
        .find(|(_, token)| !token.kind.is_comment() && token.start >= offset)
        .map(|(index, _)| index)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the upstream option matrix readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCase {
        code: String,
        #[serde(default)]
        options: Value,
        output: Option<String>,
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
        fix: ExpectedFix,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_wrap_iife(source, filename, &options, &scan, &mut diagnostics);
        diagnostics
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| suggestion.fixes.iter())
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            let start = usize::try_from(fix.range.start).expect("fixture start fits usize");
            let end = usize::try_from(fix.range.end).expect("fixture end fits usize");
            output.replace_range(start..end, &fix.replacement_text);
        }
        Some(output)
    }

    fn recursive_output(source: &str, filename: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, Some(filename), options.clone());
            let Some(next) = apply_fixes(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            assert_ne!(next, output, "fix must make progress");
            output = next;
            changed = true;
        }
        panic!("wrap-iife fixes did not converge");
    }

    #[test]
    fn replays_every_pinned_upstream_valid_and_invalid_case_exactly() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/wrap-iife-v5.10.0.json"
        ))
        .expect("fixture must deserialize");

        assert_eq!(fixture.valid.len(), 86);
        assert_eq!(fixture.invalid.len(), 42);
        for test_case in fixture.valid {
            assert!(
                run(&test_case.code, Some("fixture.js"), test_case.options).is_empty(),
                "upstream valid case reported: {}",
                test_case.code
            );
        }
        for test_case in fixture.invalid {
            let diagnostics = run(
                &test_case.code,
                Some("fixture.js"),
                test_case.options.clone(),
            );
            assert_eq!(
                diagnostics.len(),
                test_case.expected_diagnostics.len(),
                "wrong diagnostic count: {}",
                test_case.code
            );
            for (actual, expected) in diagnostics
                .iter()
                .zip(test_case.expected_diagnostics.iter())
            {
                assert_eq!(actual.message_id, expected.message_id, "{}", test_case.code);
                assert_eq!(actual.message, expected.message, "{}", test_case.code);
                assert_eq!(
                    [actual.range.start, actual.range.end],
                    expected.range,
                    "{}",
                    test_case.code
                );
                let fix = &actual.suggestions[0].fixes[0];
                assert_eq!(
                    [fix.range.start, fix.range.end],
                    expected.fix.range,
                    "{}",
                    test_case.code
                );
                assert_eq!(
                    fix.replacement_text, expected.fix.text,
                    "{}",
                    test_case.code
                );
            }
            assert_eq!(
                apply_fixes(&test_case.code, &diagnostics),
                test_case.output,
                "{}",
                test_case.code
            );
            assert_eq!(
                recursive_output(&test_case.code, "fixture.js", &test_case.options),
                test_case.recursive_output,
                "{}",
                test_case.code
            );
        }
    }

    #[test]
    fn covers_all_styles_and_prototype_method_modes() {
        assert!(run("(function () {})()", None, json!(["inside"])).is_empty());
        assert!(run("(function () {}())", None, json!(["outside"])).is_empty());
        assert!(run("(function () {}())", None, json!(["any"])).is_empty());
        assert!(
            run(
                "function () {}.call()",
                None,
                json!(["inside", { "functionPrototypeMethods": false }])
            )
            .is_empty()
        );
        assert_eq!(
            run(
                "const value = function () {}.apply(null)",
                None,
                json!(["outside", { "functionPrototypeMethods": true }])
            )[0]
            .message_id,
            WRAP_INVOCATION
        );
    }

    #[test]
    fn handles_unicode_crlf_and_all_ecmascript_line_terminators() {
        let source = [
            "const 日本語 = function () { return '😀'; }();\r\n",
            "const café = function () {}();\u{2028}",
            "const τέλος = function () {}();\u{2029}",
        ]
        .concat();
        let diagnostics = run(&source, Some("fixture.ts"), json!(["inside"]));
        assert_eq!(diagnostics.len(), 3);
        for diagnostic in diagnostics {
            assert_eq!(diagnostic.message_id, WRAP_INVOCATION);
            assert!(diagnostic.range.is_valid());
        }

        let asi =
            "const first = function () {}()\r\nconst second = function () {}()\u{2028}void first";
        let asi_diagnostics = run(asi, Some("fixture.js"), json!(["outside"]));
        assert_eq!(asi_diagnostics.len(), 2);
        assert_eq!(
            apply_fixes(asi, &asi_diagnostics).as_deref(),
            Some(
                "const first = (function () {}())\r\nconst second = (function () {}())\u{2028}void first"
            )
        );
    }

    #[test]
    fn supports_typescript_tsx_and_optional_chains() {
        let ts = "const value: number = function (): number { return 1 }();";
        let tsx = "const view = <div>{function (): JSX.Element { return <span /> }()}</div>;";
        let tsx_inside =
            "const view = <div>{(function (): JSX.Element { return <span /> }())}</div>;";
        let tsx_outside =
            "const view = <div>{(function (): JSX.Element { return <span /> })()}</div>;";
        let optional = "const value = function () {}?.call?.(null);";
        assert_eq!(run(ts, Some("fixture.ts"), json!(["outside"])).len(), 1);
        assert_eq!(run(tsx, Some("fixture.tsx"), json!(["inside"])).len(), 1);
        let inside_diagnostics = run(tsx_inside, Some("fixture.tsx"), json!(["inside"]));
        let outside_diagnostics = run(tsx_outside, Some("fixture.tsx"), json!(["outside"]));
        assert_eq!(
            apply_fixes(tsx_inside, &inside_diagnostics).as_deref(),
            Some(tsx_outside)
        );
        assert_eq!(
            apply_fixes(tsx_outside, &outside_diagnostics).as_deref(),
            Some(tsx_inside)
        );
        assert_eq!(
            run(
                optional,
                Some("fixture.js"),
                json!(["inside", { "functionPrototypeMethods": true }])
            )
            .len(),
            1
        );
    }

    #[test]
    fn preserves_comments_when_moving_parentheses_both_directions() {
        let inside = "(function () {} /* function */ () /* invocation */)";
        let outside = "(function () {} /* function */) /* between */ ()";
        let inside_diagnostic = run(inside, None, json!(["inside"]));
        let outside_diagnostic = run(outside, None, json!(["outside"]));
        assert_eq!(
            apply_fixes(inside, &inside_diagnostic).as_deref(),
            Some("(function () {}) /* function */ () /* invocation */")
        );
        assert_eq!(
            apply_fixes(outside, &outside_diagnostic).as_deref(),
            Some("(function () {} /* function */ /* between */ ())")
        );
    }

    #[test]
    fn ignores_arrows_declarations_non_iife_calls_and_dynamic_properties() {
        for source in [
            "(() => {})()",
            "function declaration() {}",
            "const value = function () {};",
            "ordinary.call()",
            "function () {}[method]()",
            "function () {}.bind()",
            "new (function () {})()",
        ] {
            assert!(
                run(
                    source,
                    None,
                    json!(["outside", { "functionPrototypeMethods": true }])
                )
                .is_empty(),
                "must ignore: {source}"
            );
        }
    }

    #[test]
    fn invalid_syntax_and_invalid_options_fail_safely() {
        assert!(run("const = function () {}()", Some("fixture.js"), Value::Null).is_empty());
        assert!(
            run(
                "<div>{function () {}(}</div>",
                Some("fixture.tsx"),
                Value::Null
            )
            .is_empty()
        );

        for options in [
            json!(["sideways"]),
            json!([42, { "functionPrototypeMethods": "yes" }]),
            json!({ "functionPrototypeMethods": true }),
            Value::Null,
        ] {
            let diagnostics = run("const value = function () {}();", None, options);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message_id, WRAP_INVOCATION);
        }
    }

    #[test]
    fn reports_containing_calls_before_nested_iifes() {
        let source = "const value = function () { return function () {}() }()";
        let diagnostics = run(source, None, json!(["outside"]));
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].range.start, 14);
        assert!(diagnostics[0].range.end > diagnostics[1].range.end);
    }
}
