//! AST-backed array layout rules.
//!
//! `array-element-newline` needs the exact `ArrayExpression` / `ArrayPattern`
//! element list. A flat bracket scan cannot distinguish array literals from
//! computed members or TypeScript tuple types, and it cannot model sparse
//! element slots. Oxc supplies that structure; the shared stylistic token scan
//! supplies the comment-inclusive token gaps and fix ranges used by upstream.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrayExpression, ArrayExpressionElement, ArrayPattern};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::Scan;

const RULE_NAME: &str = "array-element-newline";
const MISSING_ID: &str = "missingLineBreak";
const MISSING_MESSAGE: &str = "There should be a linebreak after this element.";
const UNEXPECTED_ID: &str = "unexpectedLineBreak";
const UNEXPECTED_MESSAGE: &str = "There should be no linebreak here.";

#[derive(Clone, Copy)]
struct NormalizedOption {
    consistent: bool,
    multiline: bool,
    /// `None` represents JavaScript's `Number.POSITIVE_INFINITY`.
    min_items: Option<usize>,
}

#[derive(Clone, Copy)]
struct NormalizedOptions {
    expression: Option<NormalizedOption>,
    pattern: Option<NormalizedOption>,
}

/// Enforces element separators for array literals and destructuring patterns.
pub(crate) fn check_array_element_newline(
    scan: &Scan<'_>,
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
            let mut visitor = ArrayElementNewlineVisitor {
                scan,
                options: normalize_options(options),
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            return;
        }
    }
}

struct ArrayElementNewlineVisitor<'source, 'diagnostics> {
    scan: &'source Scan<'source>,
    options: NormalizedOptions,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for ArrayElementNewlineVisitor<'_, '_> {
    fn visit_array_expression(&mut self, array: &ArrayExpression<'ast>) {
        if let Some(options) = self.options.expression {
            let elements = array
                .elements
                .iter()
                .map(|element| {
                    (!matches!(element, ArrayExpressionElement::Elision(_))).then(|| element.span())
                })
                .collect::<Vec<_>>();
            self.check_elements(&elements, options);
        }
        walk::walk_array_expression(self, array);
    }

    fn visit_array_pattern(&mut self, array: &ArrayPattern<'ast>) {
        if let Some(options) = self.options.pattern {
            let mut elements = array
                .elements
                .iter()
                .map(|element| element.as_ref().map(GetSpan::span))
                .collect::<Vec<_>>();
            if let Some(rest) = &array.rest {
                elements.push(Some(rest.span()));
            }
            self.check_elements(&elements, options);
        }
        walk::walk_array_pattern(self, array);
    }
}

impl ArrayElementNewlineVisitor<'_, '_> {
    fn check_elements(&mut self, elements: &[Option<Span>], options: NormalizedOption) {
        let element_break = options.multiline
            && elements
                .iter()
                .flatten()
                .any(|span| has_linebreak(self.scan.slice(span.start as usize, span.end as usize)));

        let mut linebreaks_count = 0;
        for pair in elements.windows(2) {
            let [Some(previous), Some(current)] = pair else {
                continue;
            };
            let Some((last_previous, first_current, _)) =
                self.separator_tokens(*previous, *current)
            else {
                continue;
            };
            if !tokens_on_same_line(self.scan, last_previous, first_current) {
                linebreaks_count += 1;
            }
        }

        let needs_linebreaks = options
            .min_items
            .is_some_and(|minimum| elements.len() >= minimum)
            || element_break
            || (options.consistent && linebreaks_count > 0 && linebreaks_count < elements.len());

        for pair in elements.windows(2) {
            let [Some(previous), Some(current)] = pair else {
                continue;
            };
            let Some((last_previous, first_current, comma)) =
                self.separator_tokens(*previous, *current)
            else {
                continue;
            };
            let same_line = tokens_on_same_line(self.scan, last_previous, first_current);
            if needs_linebreaks && same_line {
                self.report_missing(first_current);
            } else if !needs_linebreaks && !same_line {
                self.report_unexpected(first_current, comma);
            }
        }
    }

    fn separator_tokens(&self, previous: Span, current: Span) -> Option<(usize, usize, usize)> {
        let previous_end = usize::try_from(previous.end).ok()?;
        let current_start = usize::try_from(current.start).ok()?;
        let comma = self
            .scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, token)| {
                token.start >= previous_end
                    && token.end <= current_start
                    && self.scan.slice(token.start, token.end) == ","
            })
            .map(|(index, _)| index)?;
        let last_previous = self.scan.prev_significant(comma)?;
        let first_current = self.scan.next_significant(comma)?;
        Some((last_previous, first_current, comma))
    }

    fn report_missing(&mut self, first_current: usize) {
        let Some(token_before) = first_current.checked_sub(1) else {
            return;
        };
        let tokens = self.scan.tokens();
        let range = tokens[token_before].end..tokens[first_current].start;
        self.push_diagnostic(
            MISSING_ID,
            MISSING_MESSAGE,
            range.clone(),
            Some((range, "\n")),
        );
    }

    fn report_unexpected(&mut self, first_current: usize, comma: usize) {
        let Some(token_before) = first_current.checked_sub(1) else {
            return;
        };
        let tokens = self.scan.tokens();
        let report_range = tokens[token_before].end..tokens[first_current].start;
        let fix = if tokens[token_before].kind.is_comment() {
            None
        } else if !tokens_on_same_line(self.scan, token_before, first_current) {
            Some((report_range.clone(), " "))
        } else {
            let Some(two_tokens_before) = token_before.checked_sub(1) else {
                return self.push_diagnostic(UNEXPECTED_ID, UNEXPECTED_MESSAGE, report_range, None);
            };
            if tokens[two_tokens_before].kind.is_comment() {
                None
            } else {
                Some((tokens[two_tokens_before].end..tokens[comma].start, ""))
            }
        };
        self.push_diagnostic(UNEXPECTED_ID, UNEXPECTED_MESSAGE, report_range, fix);
    }

    fn push_diagnostic(
        &mut self,
        message_id: &str,
        message: &str,
        range: std::ops::Range<usize>,
        fix: Option<(std::ops::Range<usize>, &str)>,
    ) {
        let (Ok(start), Ok(end)) = (u32::try_from(range.start), u32::try_from(range.end)) else {
            return;
        };
        let suggestions = fix
            .and_then(|(fix_range, replacement)| {
                Some(LintSuggestion {
                    message_id: message_id.to_owned(),
                    message: message.to_owned(),
                    fixes: std::iter::once(LintFix::replace_range(
                        TextRange::new(
                            u32::try_from(fix_range.start).ok()?,
                            u32::try_from(fix_range.end).ok()?,
                        ),
                        replacement,
                    ))
                    .collect(),
                })
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
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

    if let Some(Value::Object(object)) = provided {
        let expression = object
            .get("ArrayExpression")
            .filter(|value| js_truthy(value));
        let pattern = object.get("ArrayPattern").filter(|value| js_truthy(value));
        if expression.is_some() || pattern.is_some() {
            return NormalizedOptions {
                expression: expression.map(normalize_option),
                pattern: pattern.map(normalize_option),
            };
        }
    }

    let normalized = normalize_option(provided.unwrap_or(&Value::Null));
    NormalizedOptions {
        expression: Some(normalized),
        pattern: Some(normalized),
    }
}

fn normalize_option(option: &Value) -> NormalizedOption {
    if !js_truthy(option) || option.as_str() == Some("always") {
        return NormalizedOption {
            consistent: false,
            multiline: false,
            min_items: Some(0),
        };
    }
    if option.as_str() == Some("never") {
        return NormalizedOption {
            consistent: false,
            multiline: false,
            min_items: None,
        };
    }
    if option.as_str() == Some("consistent") {
        return NormalizedOption {
            consistent: true,
            multiline: false,
            min_items: None,
        };
    }

    let consistent = option
        .get("consistent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let multiline = option
        .get("multiline")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let min_items = option
        .get("minItems")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    NormalizedOption {
        consistent,
        multiline,
        min_items,
    }
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
    !has_linebreak(scan.slice(scan.tokens()[left].end, scan.tokens()[right].start))
}

fn has_linebreak(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the upstream option matrix readable in focused tests"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, options: Value) -> Vec<LintDiagnostic> {
        let scan = Scan::new(source);
        let mut diagnostics = Vec::new();
        check_array_element_newline(&scan, &options, &mut diagnostics);
        diagnostics
    }

    fn fixed(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| suggestion.fixes.iter())
            .collect::<Vec<_>>();
        if fixes.len() != diagnostics.len() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse(fix.range.start));
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
    fn defaults_to_always_for_expressions_and_patterns() {
        let expression = run("const value = [1, 2, 3];", json!([]));
        assert_eq!(
            expression
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [MISSING_ID, MISSING_ID]
        );
        assert_eq!(
            fixed("const value = [1, 2, 3];", &expression).as_deref(),
            Some("const value = [1,\n2,\n3];")
        );

        let pattern = run("const [a, b] = value;", json!([]));
        assert_eq!(
            fixed("const [a, b] = value;", &pattern).as_deref(),
            Some("const [a,\nb] = value;")
        );
    }

    #[test]
    fn handles_never_and_comma_on_next_line_fix_shape() {
        let source = "const value = [1\n, 2\n, 3];";
        let diagnostics = run(source, json!(["never"]));
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("const value = [1, 2, 3];")
        );
    }

    #[test]
    fn comment_before_element_can_suppress_the_upstream_fix() {
        let source = "const value = [1,\n/* keep */ 2];";
        let diagnostics = run(source, json!(["never"]));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggestions.is_empty());
    }

    #[test]
    fn skips_sparse_array_pairs_on_both_sides_of_a_hole() {
        assert!(run("const value = [1, , 3];", json!([])).is_empty());
        assert!(run("const [a, , c] = value;", json!([])).is_empty());
    }

    #[test]
    fn supports_consistent_multiline_and_min_items() {
        assert_eq!(
            run("const value = [1,\n2, 3];", json!(["consistent"])).len(),
            1
        );
        assert_eq!(
            run(
                "const value = [() => {\n  return 1;\n}, next];",
                json!([{ "multiline": true }])
            )
            .len(),
            1
        );
        assert!(run("const value = [1, 2];", json!([{ "minItems": 3 }])).is_empty());
        assert_eq!(
            run("const value = [1, 2, 3];", json!([{ "minItems": 3 }])).len(),
            2
        );
    }

    #[test]
    fn separates_array_expression_and_pattern_configuration() {
        let source = "const [a,\nb] = [1, 2];";
        let diagnostics = run(
            source,
            json!([{ "ArrayExpression": "always", "ArrayPattern": "never" }]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [UNEXPECTED_ID, MISSING_ID]
        );
    }

    #[test]
    fn handles_typescript_tsx_spreads_defaults_and_nested_arrays() {
        let type_script = "
type Tuple = [string, number];
const value = <number[]>[1, 2];
const nested = [[1, 2], [3, 4]];
const [head = [1, 2], ...tail] = value;
";
        let diagnostics = run(type_script, json!([]));
        assert_eq!(diagnostics.len(), 6);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message_id == MISSING_ID)
        );

        let tsx = "const view = <Panel values={[first, ...rest]} />;";
        assert_eq!(run(tsx, json!([])).len(), 1);
    }

    #[test]
    fn ignores_non_runtime_brackets_comments_strings_regexes_and_templates() {
        let source = r#"
// [1, 2]
const text = "[1, 2]";
const regex = /[1, 2]/;
const template = `[1, 2]`;
type Tuple = [1, 2];
interface Box { value: [1, 2] }
const member = object[key, fallback];
"#;
        assert!(run(source, json!([])).is_empty());
    }

    #[test]
    fn preserves_utf8_ranges_and_multiline_comment_boundaries() {
        let source = "const 日本語 = [値, /* 🦀 */ 次];";
        let diagnostics = run(source, json!([]));
        assert_eq!(diagnostics.len(), 1);
        let start = source.find("*/").expect("comment terminator") + 2;
        let end = source.find('次').expect("next element");
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(start as u32, end as u32)
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("const 日本語 = [値, /* 🦀 */\n次];")
        );
    }

    #[test]
    fn reports_outer_arrays_before_nested_arrays() {
        let source = "const value = [[1, 2], [3, 4]];";
        let diagnostics = run(source, json!([]));
        let starts = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.range.start)
            .collect::<Vec<_>>();
        assert_eq!(starts, [22, 18, 26]);
    }

    #[test]
    fn parse_failures_do_not_create_heuristic_diagnostics() {
        assert!(run("const value = [1, 2", json!([])).is_empty());
        assert!(run("const view = <Panel values={[1, 2]} >", json!([])).is_empty());
    }
}
