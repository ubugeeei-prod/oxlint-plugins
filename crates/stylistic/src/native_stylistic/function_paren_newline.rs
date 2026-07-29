//! Native implementation of `@stylistic/function-paren-newline`.
//!
//! The rule is driven by parsed call/function/import/new nodes because a plain
//! parenthesis scan cannot distinguish calls and parameter lists from control
//! statements or grouping expressions. The shared lexer still supplies exact
//! token/comment boundaries, so diagnostics and safe whitespace fixes match
//! ESLint's token-oriented behavior.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionExpression, CallExpression, FormalParameters, Function, FunctionType,
    ImportExpression, NewExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::scope::ScopeFlags;
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::{Scan, punct_is};

const RULE_NAME: &str = "function-paren-newline";
const EXPECTED_BEFORE: &str = "Expected newline before ')'.";
const EXPECTED_AFTER: &str = "Expected newline after '('.";
const EXPECTED_BETWEEN: &str = "Expected newline between arguments/params.";
const UNEXPECTED_BEFORE: &str = "Unexpected newline before ')'.";
const UNEXPECTED_AFTER: &str = "Unexpected newline after '('.";

/// Checks every stable node kind visited by the upstream rule.
pub(crate) fn check_function_paren_newline(
    source_text: &str,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    let first_diagnostic = diagnostics.len();
    for source_type in [
        SourceType::ts(),
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, source_text, source_type).parse();
        if parsed.errors.is_empty() {
            let scan = Scan::new(source_text);
            let mut visitor = FunctionParenNewlineVisitor {
                scan: &scan,
                option: RuleOption::from_value(options),
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            diagnostics[first_diagnostic..]
                .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
            return;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuleOption {
    Multiline,
    MultilineArguments,
    Consistent,
    MinItems(usize),
    Never,
}

impl RuleOption {
    fn from_value(options: &Value) -> Self {
        let Some(raw) = options.as_array().and_then(|values| values.first()) else {
            return Self::Multiline;
        };
        match raw.as_str() {
            Some("multiline") => Self::Multiline,
            Some("multiline-arguments") => Self::MultilineArguments,
            Some("consistent") => Self::Consistent,
            Some("always") => Self::MinItems(0),
            Some("never") => Self::Never,
            _ => raw
                .as_object()
                .and_then(|object| object.get("minItems"))
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .map_or(Self::MinItems(0), Self::MinItems),
        }
    }

    fn needs_newlines(self, elements: &[Span], has_left_newline: bool, source: &str) -> bool {
        match self {
            Self::MultilineArguments if elements.len() == 1 => has_left_newline,
            Self::Multiline | Self::MultilineArguments => elements
                .windows(2)
                .any(|pair| has_line_terminator(slice(source, pair[0].end, pair[1].start))),
            Self::Consistent => has_left_newline,
            Self::MinItems(minimum) => elements.len() >= minimum,
            Self::Never => false,
        }
    }
}

#[derive(Clone, Copy)]
struct ParenPair {
    left: usize,
    right: usize,
}

struct FunctionParenNewlineVisitor<'scan, 'diagnostics> {
    scan: &'scan Scan<'scan>,
    option: RuleOption,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for FunctionParenNewlineVisitor<'_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        let elements = call.arguments.iter().map(GetSpan::span).collect::<Vec<_>>();
        if let Some(pair) = self.trailing_paren_pair(call.span) {
            self.validate(pair, &elements);
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, expression: &NewExpression<'ast>) {
        let elements = expression
            .arguments
            .iter()
            .map(GetSpan::span)
            .collect::<Vec<_>>();
        if let Some(pair) = self.trailing_paren_pair(expression.span)
            && (expression.callee.span().end <= self.scan.tokens()[pair.left].start as u32)
        {
            self.validate(pair, &elements);
        }
        walk::walk_new_expression(self, expression);
    }

    fn visit_import_expression(&mut self, expression: &ImportExpression<'ast>) {
        let mut elements = Vec::with_capacity(2);
        elements.push(expression.source.span());
        if let Some(options) = &expression.options {
            elements.push(options.span());
        }
        if let Some(pair) = self.trailing_paren_pair(expression.span) {
            self.validate(pair, &elements);
        }
        walk::walk_import_expression(self, expression);
    }

    fn visit_function(&mut self, function: &Function<'ast>, flags: ScopeFlags) {
        if matches!(
            function.r#type,
            FunctionType::FunctionDeclaration | FunctionType::FunctionExpression
        ) {
            self.validate_formal_parameters(
                &function.params,
                function
                    .this_param
                    .as_ref()
                    .map(|parameter| parameter.span()),
            );
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'ast>) {
        self.validate_formal_parameters(&arrow.params, None);
        walk::walk_arrow_function_expression(self, arrow);
    }
}

impl FunctionParenNewlineVisitor<'_, '_> {
    fn validate_formal_parameters(
        &mut self,
        parameters: &FormalParameters<'_>,
        this_parameter: Option<Span>,
    ) {
        let Some(pair) = self.exact_paren_pair(parameters.span) else {
            // A one-parameter arrow such as `value => value` has no parens.
            return;
        };
        let mut elements = Vec::with_capacity(
            parameters
                .items
                .len()
                .saturating_add(usize::from(parameters.rest.is_some()))
                .saturating_add(usize::from(this_parameter.is_some())),
        );
        elements.extend(this_parameter);
        elements.extend(parameters.items.iter().map(GetSpan::span));
        elements.extend(parameters.rest.iter().map(|parameter| parameter.span()));
        elements.sort_unstable_by_key(|span| span.start);
        self.validate(pair, &elements);
    }

    fn validate(&mut self, pair: ParenPair, elements: &[Span]) {
        let tokens = self.scan.tokens();
        let left = &tokens[pair.left];
        let right = &tokens[pair.right];
        let token_after_left = (pair.left + 1..=pair.right)
            .find(|index| !tokens[*index].kind.is_comment())
            .unwrap_or(pair.right);
        let token_before_right = (pair.left..pair.right)
            .rev()
            .find(|index| !tokens[*index].kind.is_comment())
            .unwrap_or(pair.left);
        let after_left = &tokens[token_after_left];
        let before_right = &tokens[token_before_right];
        let has_left_newline = has_line_terminator(self.scan.slice(left.end, after_left.start));
        let has_right_newline = has_line_terminator(self.scan.slice(before_right.end, right.start));
        let needs_newlines =
            self.option
                .needs_newlines(elements, has_left_newline, self.scan.source());

        if has_left_newline && !needs_newlines {
            let fix = (!self.comments_between(pair.left, token_after_left)).then_some((
                left.end,
                after_left.start,
                "",
            ));
            self.report("unexpectedAfter", UNEXPECTED_AFTER, left.span(), fix);
        } else if !has_left_newline && needs_newlines {
            self.report(
                "expectedAfter",
                EXPECTED_AFTER,
                left.span(),
                Some((left.end, left.end, "\n")),
            );
        }

        if has_right_newline && !needs_newlines {
            let fix = (!self.comments_between(token_before_right, pair.right)).then_some((
                before_right.end,
                right.start,
                "",
            ));
            self.report("unexpectedBefore", UNEXPECTED_BEFORE, right.span(), fix);
        } else if !has_right_newline && needs_newlines {
            self.report(
                "expectedBefore",
                EXPECTED_BEFORE,
                right.span(),
                Some((right.start, right.start, "\n")),
            );
        }

        if self.option == RuleOption::MultilineArguments && needs_newlines {
            for pair in elements.windows(2) {
                if !has_line_terminator(slice(self.scan.source(), pair[0].end, pair[1].start)) {
                    let insertion = usize::try_from(pair[1].start).ok();
                    self.report(
                        "expectedBetween",
                        EXPECTED_BETWEEN,
                        pair[0],
                        insertion.map(|offset| (offset, offset, "\n")),
                    );
                }
            }
        }
    }

    fn exact_paren_pair(&self, span: Span) -> Option<ParenPair> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let left =
            self.scan.tokens().iter().position(|token| {
                token.start == start && punct_is(token, self.scan.source(), "(")
            })?;
        let right = self.scan.partner(left)?;
        let token = &self.scan.tokens()[right];
        (token.end == end && punct_is(token, self.scan.source(), ")"))
            .then_some(ParenPair { left, right })
    }

    fn trailing_paren_pair(&self, span: Span) -> Option<ParenPair> {
        let end = usize::try_from(span.end).ok()?;
        let right = self
            .scan
            .tokens()
            .iter()
            .rposition(|token| token.end == end && punct_is(token, self.scan.source(), ")"))?;
        let left = self.scan.partner(right)?;
        punct_is(&self.scan.tokens()[left], self.scan.source(), "(")
            .then_some(ParenPair { left, right })
    }

    fn comments_between(&self, left: usize, right: usize) -> bool {
        self.scan.tokens()[left + 1..right]
            .iter()
            .any(|token| token.kind.is_comment())
    }

    fn report(
        &mut self,
        message_id: &'static str,
        message: &'static str,
        span: Span,
        fix: Option<(usize, usize, &'static str)>,
    ) {
        let suggestion = fix.and_then(|(start, end, replacement)| {
            let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
                return None;
            };
            Some(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(LintFix::replace_range(
                    TextRange::new(start, end),
                    replacement,
                ))
                .collect(),
            })
        });
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: Default::default(),
            range: TextRange::new(span.start, span.end),
            suggestions: suggestion.into_iter().collect(),
        });
    }
}

trait TokenSpan {
    fn span(&self) -> Span;
}

impl TokenSpan for super::lexer::Token {
    fn span(&self) -> Span {
        Span::new(self.start as u32, self.end as u32)
    }
}

fn slice(source: &str, start: u32, end: u32) -> &str {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return "";
    };
    source.get(start..end).unwrap_or("")
}

fn has_line_terminator(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the option regression matrix readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    struct Generated {
        version: String,
        commit: String,
    }

    #[derive(Deserialize)]
    struct FixtureCase {
        code: String,
        options: Value,
        #[serde(default)]
        errors: Vec<FixtureError>,
        #[serde(default)]
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureError {
        message_id: String,
        message: String,
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
        fix: Option<FixtureFix>,
    }

    #[derive(Deserialize)]
    struct FixtureFix {
        range: [u32; 2],
        text: String,
    }

    fn upstream_fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/function-paren-newline-v5.10.0.json"
        ))
        .expect("generated function-paren-newline fixture is valid JSON")
    }

    fn run(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_function_paren_newline(source, options, &mut diagnostics);
        diagnostics
    }

    fn location(source: &str, offset: u32) -> (usize, usize) {
        let offset = usize::try_from(offset).expect("fixture offset fits usize");
        let prefix = source
            .get(..offset)
            .expect("diagnostic offset is a boundary");
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        (line, prefix[line_start..].chars().count() + 1)
    }

    fn iterative_fixed_output(source: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;

        for _ in 0..10 {
            let diagnostics = run(&output, options);
            let mut fixes = diagnostics
                .iter()
                .enumerate()
                .filter_map(|(index, diagnostic)| {
                    diagnostic
                        .suggestions
                        .first()?
                        .fixes
                        .first()
                        .cloned()
                        .map(|fix| (index, fix))
                })
                .collect::<Vec<_>>();
            fixes.sort_by_key(|(index, fix)| (fix.range.start, fix.range.end, *index));
            if fixes.is_empty() {
                break;
            }

            let mut next = String::with_capacity(output.len());
            let mut cursor = 0usize;
            let mut last_end = None;
            let mut applied = false;
            for (_, fix) in fixes {
                if last_end.is_some_and(|end| end >= fix.range.start) {
                    continue;
                }
                let (Ok(start), Ok(end)) = (
                    usize::try_from(fix.range.start),
                    usize::try_from(fix.range.end),
                ) else {
                    continue;
                };
                let (Some(prefix), Some(_replaced)) =
                    (output.get(cursor..start), output.get(start..end))
                else {
                    continue;
                };
                next.push_str(prefix);
                next.push_str(&fix.replacement_text);
                cursor = end;
                last_end = Some(fix.range.end);
                applied = true;
            }
            if !applied {
                break;
            }
            next.push_str(&output[cursor..]);
            output = next;
            changed = true;
        }

        changed.then_some(output)
    }

    #[test]
    fn accepts_every_stable_v5_10_0_valid_fixture() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.generated.version, "v5.10.0");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.valid.len(), 112);

        for (index, test) in fixture.valid.iter().enumerate() {
            let diagnostics = run(&test.code, &test.options);
            assert!(
                diagnostics.is_empty(),
                "upstream valid fixture {index} reported {diagnostics:#?}\n{}",
                test.code
            );
        }
    }

    #[test]
    fn replays_every_stable_v5_10_0_invalid_fixture_exactly() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.invalid.len(), 86);
        assert_eq!(
            fixture
                .invalid
                .iter()
                .map(|test| test.errors.len())
                .sum::<usize>(),
            135
        );

        for (index, test) in fixture.invalid.iter().enumerate() {
            assert!(
                test.code.is_ascii(),
                "fixture locations are UTF-16: {index}"
            );
            let diagnostics = run(&test.code, &test.options);
            assert_eq!(
                diagnostics.len(),
                test.errors.len(),
                "diagnostic count differs for upstream invalid fixture {index}\n{}",
                test.code
            );
            for (diagnostic, expected) in diagnostics.iter().zip(&test.errors) {
                assert_eq!(
                    diagnostic.message_id, expected.message_id,
                    "fixture {index}"
                );
                assert_eq!(diagnostic.message, expected.message, "fixture {index}");
                assert_eq!(
                    location(&test.code, diagnostic.range.start),
                    (expected.line, expected.column),
                    "start location differs for fixture {index}"
                );
                assert_eq!(
                    location(&test.code, diagnostic.range.end),
                    (expected.end_line, expected.end_column),
                    "end location differs for fixture {index}"
                );

                let actual_fix = diagnostic
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first());
                match (&expected.fix, actual_fix) {
                    (None, None) => {}
                    (Some(expected), Some(actual)) => {
                        assert_eq!(
                            [actual.range.start, actual.range.end],
                            expected.range,
                            "fix range differs for fixture {index}"
                        );
                        assert_eq!(
                            actual.replacement_text, expected.text,
                            "fix text differs for fixture {index}"
                        );
                    }
                    _ => panic!("fix presence differs for fixture {index}"),
                }
            }
            assert_eq!(
                iterative_fixed_output(&test.code, &test.options),
                test.output,
                "iterative output differs for fixture {index}\n{}",
                test.code
            );
        }
    }

    #[test]
    fn recognizes_every_ecmascript_line_terminator() {
        for linebreak in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("call({linebreak}first, second);");
            let diagnostics = run(&source, &json!(["consistent"]));
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                ["expectedBefore"],
                "line terminator {linebreak:?}"
            );
        }
    }

    #[test]
    fn preserves_unicode_byte_ranges_and_exact_fixes() {
        let source = "const café = call(α, β);";
        let diagnostics = run(source, &json!(["always"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["expectedAfter", "expectedBefore"]
        );
        let left = source.find('(').expect("opening paren");
        let right = source.rfind(')').expect("closing paren");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [
                TextRange::new(left as u32, left as u32 + 1),
                TextRange::new(right as u32, right as u32 + 1),
            ]
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.suggestions[0].fixes[0].range)
                .collect::<Vec<_>>(),
            [
                TextRange::new(left as u32 + 1, left as u32 + 1),
                TextRange::new(right as u32, right as u32),
            ]
        );
    }

    #[test]
    fn comments_block_only_unsafe_removal_fixes() {
        let source = "function value(/* keep */\nargument\n/* keep */) {}";
        let diagnostics = run(source, &json!(["never"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["unexpectedAfter", "unexpectedBefore"]
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.suggestions.is_empty())
        );
    }

    #[test]
    fn covers_methods_generics_optional_calls_imports_and_constructors() {
        let source = [
            "class Example { method<T>(value: T) {} }",
            "const object = { method(value) {} };",
            "object.method?.<string>('value');",
            "new Example<string>('value');",
            "import('module', { with: { type: 'json' } });",
            "const asserted = <string>object.method('value');",
        ]
        .join("\n");
        let diagnostics = run(&source, &json!(["always"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [
                "expectedAfter",
                "expectedBefore",
                "expectedAfter",
                "expectedBefore",
                "expectedAfter",
                "expectedBefore",
                "expectedAfter",
                "expectedBefore",
                "expectedAfter",
                "expectedBefore",
                "expectedAfter",
                "expectedBefore",
            ]
        );
    }

    #[test]
    fn ignores_grouping_control_headers_literals_and_unparenthesized_arrows() {
        let sources = [
            "if (\ncondition\n) { value(); }",
            "while (\ncondition\n) { break; }",
            "const grouped = (\nvalue\n);",
            "const text = 'call(\\nvalue\\n)';",
            "const regex = /call\\(\\nvalue/;",
            "const template = `call(\\nvalue\\n)`;",
            "const view = <div>call(\nvalue\n)</div>;",
            "const arrow = value => value;",
            "new (Constructor);",
        ];
        for source in sources {
            assert!(
                run(source, &json!(["never"])).is_empty(),
                "false positive: {source}"
            );
        }
    }

    #[test]
    fn malformed_programs_do_not_produce_heuristic_reports() {
        for source in [
            "call(",
            "function broken(",
            "const arrow = (value =>",
            "import(",
        ] {
            assert!(run(source, &json!(["always"])).is_empty(), "{source}");
        }
    }
}
