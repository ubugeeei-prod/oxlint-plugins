//! Native implementation of stable `@stylistic/jsx-wrap-multilines`.
//!
//! Oxc identifies each JSX expression and its syntactic owner. The shared
//! stylistic token scan supplies the surrounding parentheses, operators, and
//! comments that upstream uses for its deliberately context-sensitive fixes.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, ConditionalExpression, Expression, JSXAttribute,
    JSXAttributeValue, JSXExpression, LogicalExpression, ObjectExpression, ObjectPropertyKind,
    ReturnStatement, Statement, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::{Scan, first_option};
use super::lexer::Token;

const RULE: &str = "jsx-wrap-multilines";
const MISSING_PARENS: &str = "Missing parentheses around multilines JSX";
const PARENS_ON_NEW_LINES: &str = "Parentheses around JSX should be on separate lines";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Ignore,
    Parens,
    ParensNewLine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Options {
    declaration: Mode,
    assignment: Mode,
    r#return: Mode,
    arrow: Mode,
    condition: Mode,
    logical: Mode,
    prop: Mode,
    property_value: Mode,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            declaration: Mode::Parens,
            assignment: Mode::Parens,
            r#return: Mode::Parens,
            arrow: Mode::Parens,
            condition: Mode::Ignore,
            logical: Mode::Ignore,
            prop: Mode::Ignore,
            property_value: Mode::Ignore,
        }
    }
}

impl Options {
    fn from_json(value: &Value) -> Self {
        let defaults = Self::default();
        let object = first_option(value).and_then(Value::as_object);
        Self {
            declaration: option_mode(object, "declaration", defaults.declaration),
            assignment: option_mode(object, "assignment", defaults.assignment),
            r#return: option_mode(object, "return", defaults.r#return),
            arrow: option_mode(object, "arrow", defaults.arrow),
            condition: option_mode(object, "condition", defaults.condition),
            logical: option_mode(object, "logical", defaults.logical),
            prop: option_mode(object, "prop", defaults.prop),
            property_value: option_mode(object, "propertyValue", defaults.property_value),
        }
    }
}

fn option_mode(object: Option<&serde_json::Map<String, Value>>, key: &str, default: Mode) -> Mode {
    match object.and_then(|object| object.get(key)) {
        Some(Value::Bool(true)) => Mode::Parens,
        Some(Value::Bool(false)) => Mode::Ignore,
        Some(Value::String(value)) if value == "parens" => Mode::Parens,
        Some(Value::String(value)) if value == "parens-new-line" => Mode::ParensNewLine,
        Some(Value::String(value)) if value == "ignore" => Mode::Ignore,
        _ => default,
    }
}

pub(crate) fn check_jsx_wrap_multilines(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let scan = Scan::new(source);
    let options = Options::from_json(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(&scan, source_type, options, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(&scan, source_type, options, diagnostics) {
                break;
            }
        }
    }

    diagnostics[first_diagnostic..]
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
}

fn parse_and_check(
    scan: &Scan<'_>,
    source_type: SourceType,
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, scan.source(), source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = WrapMultilinesVisitor {
        scan,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct WrapMultilinesVisitor<'scan, 'diagnostics> {
    scan: &'scan Scan<'scan>,
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for WrapMultilinesVisitor<'_, '_> {
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'ast>) {
        if self.options.declaration != Mode::Ignore
            && let Some(initializer) = &declarator.init
        {
            if self.options.condition == Mode::Ignore
                && let Some(conditional) = conditional_expression(initializer)
            {
                self.check_expression(&conditional.consequent, self.options.declaration);
                self.check_expression(&conditional.alternate, self.options.declaration);
            } else {
                self.check_expression(initializer, self.options.declaration);
            }
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'ast>) {
        if self.options.assignment != Mode::Ignore {
            if self.options.condition == Mode::Ignore
                && let Some(conditional) = conditional_expression(&assignment.right)
            {
                self.check_expression(&conditional.consequent, self.options.assignment);
                self.check_expression(&conditional.alternate, self.options.assignment);
            } else {
                self.check_expression(&assignment.right, self.options.assignment);
            }
        }
        walk::walk_assignment_expression(self, assignment);
    }

    fn visit_return_statement(&mut self, statement: &ReturnStatement<'ast>) {
        if self.options.r#return != Mode::Ignore
            && let Some(argument) = &statement.argument
        {
            self.check_expression(argument, self.options.r#return);
        }
        walk::walk_return_statement(self, statement);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'ast>) {
        // Upstream's listener runs on exit. Sorting diagnostics by source span
        // below preserves the observable report order while retaining the same
        // owner selection.
        walk::walk_arrow_function_expression(self, arrow);
        if self.options.arrow != Mode::Ignore
            && arrow.expression
            && let Some(body) = arrow_body_expression(arrow)
        {
            self.check_expression(body, self.options.arrow);
        }
    }

    fn visit_conditional_expression(&mut self, conditional: &ConditionalExpression<'ast>) {
        if self.options.condition != Mode::Ignore {
            self.check_expression(&conditional.consequent, self.options.condition);
            self.check_expression(&conditional.alternate, self.options.condition);
        }
        walk::walk_conditional_expression(self, conditional);
    }

    fn visit_logical_expression(&mut self, logical: &LogicalExpression<'ast>) {
        if self.options.logical != Mode::Ignore {
            self.check_expression(&logical.right, self.options.logical);
        }
        walk::walk_logical_expression(self, logical);
    }

    fn visit_jsx_attribute(&mut self, attribute: &JSXAttribute<'ast>) {
        if self.options.prop != Mode::Ignore
            && let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value
        {
            self.check_jsx_expression(&container.expression, self.options.prop);
        }
        walk::walk_jsx_attribute(self, attribute);
    }

    fn visit_object_expression(&mut self, object: &ObjectExpression<'ast>) {
        if self.options.property_value != Mode::Ignore {
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    continue;
                };
                // Upstream intentionally checks JSXElement only here, not
                // JSXFragment.
                if let Some(span) = jsx_element_span(&property.value) {
                    self.check(span, self.options.property_value);
                }
            }
        }
        walk::walk_object_expression(self, object);
    }
}

impl WrapMultilinesVisitor<'_, '_> {
    fn check_expression(&mut self, expression: &Expression<'_>, mode: Mode) {
        if let Some(span) = jsx_span(expression) {
            self.check(span, mode);
        }
    }

    fn check_jsx_expression(&mut self, expression: &JSXExpression<'_>, mode: Mode) {
        if let Some(span) = jsx_expression_span(expression) {
            self.check(span, mode);
        }
    }

    fn check(&mut self, span: Span, mode: Mode) {
        if mode == Mode::Ignore || is_single_line(self.scan.source(), span) {
            return;
        }

        let before = significant_before(self.scan, span.start);
        let after = significant_after(self.scan, span.end);
        let parenthesized = before.zip(after).is_some_and(|(before, after)| {
            before.text(self.scan.source()) == "(" && after.text(self.scan.source()) == ")"
        });

        match mode {
            Mode::Ignore => {}
            Mode::Parens if !parenthesized => {
                let Some(text) = source_text(self.scan.source(), span) else {
                    return;
                };
                let mut replacement = String::with_capacity(text.len() + 2);
                replacement.push('(');
                replacement.push_str(text);
                replacement.push(')');
                self.report(
                    span,
                    "missingParens",
                    MISSING_PARENS,
                    LintFix::replace_range(TextRange::new(span.start, span.end), replacement),
                );
            }
            Mode::Parens => {}
            Mode::ParensNewLine if parenthesized => {
                let Some((before, after)) = before.zip(after) else {
                    return;
                };
                let needs_opening =
                    same_line(self.scan.source(), u32_from_usize(before.end), span.start);
                let needs_closing =
                    same_line(self.scan.source(), span.end, u32_from_usize(after.start));
                if !needs_opening && !needs_closing {
                    return;
                }
                let Some(text) = source_text(self.scan.source(), span) else {
                    return;
                };
                let mut replacement = String::with_capacity(
                    text.len() + usize::from(needs_opening) + usize::from(needs_closing),
                );
                if needs_opening {
                    replacement.push('\n');
                }
                replacement.push_str(text);
                if needs_closing {
                    replacement.push('\n');
                }
                self.report(
                    span,
                    "parensOnNewLines",
                    PARENS_ON_NEW_LINES,
                    LintFix::replace_range(TextRange::new(span.start, span.end), replacement),
                );
            }
            Mode::ParensNewLine => {
                let Some(fix) = missing_newline_parens_fix(self.scan, span, before, after) else {
                    return;
                };
                self.report(span, "missingParens", MISSING_PARENS, fix);
            }
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

fn missing_newline_parens_fix(
    scan: &Scan<'_>,
    span: Span,
    before: Option<&Token>,
    after: Option<&Token>,
) -> Option<LintFix> {
    let source = scan.source();
    let text = source_text(source, span)?;
    let before = before?;
    let before_end = u32::try_from(before.end).ok()?;

    if same_line(source, before_end, span.start) {
        let mut replacement = String::with_capacity(text.len() + 4);
        replacement.push_str("(\n");
        replacement.push_str(text);
        replacement.push_str("\n)");
        return Some(LintFix::replace_range(
            TextRange::new(span.start, span.end),
            replacement,
        ));
    }

    let (line_start, column) = line_start_and_column(source, usize::try_from(span.start).ok()?)?;
    let tab_indented = source.get(line_start..)?.starts_with('\t');
    let indent_unit = if tab_indented { '\t' } else { ' ' };
    let indent_before = repeated(indent_unit, column);
    let indent_after = repeated(
        indent_unit,
        column.saturating_sub(if tab_indented { 1 } else { 2 }),
    );
    let text_before = source
        .get(before.end..usize::try_from(span.start).ok()?)?
        .trim();
    let before_text = before.text(source).trim();
    let mut replacement = String::new();
    replacement.push_str(before_text);
    if !matches!(before_text, "{" | "[") {
        replacement.push(' ');
    }
    replacement.push_str("(\n");
    replacement.push_str(&indent_before);
    replacement.push_str(text_before);
    if !text_before.is_empty() {
        replacement.push('\n');
        replacement.push_str(&indent_before);
    }
    replacement.push_str(text);
    replacement.push('\n');
    replacement.push_str(&indent_after);
    replacement.push(')');

    let end = after
        .filter(|token| matches!(token.text(source), ";" | "}"))
        .and_then(|token| u32::try_from(token.start).ok())
        .unwrap_or(span.end);
    Some(LintFix::replace_range(
        TextRange::new(u32::try_from(before.start).ok()?, end),
        replacement,
    ))
}

fn jsx_span(expression: &Expression<'_>) -> Option<Span> {
    match unwrap_parentheses(expression) {
        Expression::JSXElement(element) => Some(element.span),
        Expression::JSXFragment(fragment) => Some(fragment.span),
        _ => None,
    }
}

fn jsx_element_span(expression: &Expression<'_>) -> Option<Span> {
    match unwrap_parentheses(expression) {
        Expression::JSXElement(element) => Some(element.span),
        _ => None,
    }
}

fn jsx_expression_span(expression: &JSXExpression<'_>) -> Option<Span> {
    match expression {
        JSXExpression::JSXElement(element) => Some(element.span),
        JSXExpression::JSXFragment(fragment) => Some(fragment.span),
        JSXExpression::ParenthesizedExpression(parenthesized) => {
            jsx_span(&parenthesized.expression)
        }
        _ => None,
    }
}

fn conditional_expression<'ast>(
    expression: &'ast Expression<'ast>,
) -> Option<&'ast ConditionalExpression<'ast>> {
    match unwrap_parentheses(expression) {
        Expression::ConditionalExpression(conditional) => Some(conditional),
        _ => None,
    }
}

fn unwrap_parentheses<'ast>(mut expression: &'ast Expression<'ast>) -> &'ast Expression<'ast> {
    while let Expression::ParenthesizedExpression(parenthesized) = expression {
        expression = &parenthesized.expression;
    }
    expression
}

fn arrow_body_expression<'ast>(
    arrow: &'ast ArrowFunctionExpression<'ast>,
) -> Option<&'ast Expression<'ast>> {
    let Statement::ExpressionStatement(statement) = arrow.body.statements.first()? else {
        return None;
    };
    Some(&statement.expression)
}

fn significant_before<'tokens>(scan: &'tokens Scan<'_>, start: u32) -> Option<&'tokens Token> {
    let start = usize::try_from(start).ok()?;
    scan.tokens()
        .iter()
        .rev()
        .find(|token| token.end <= start && !token.kind.is_comment())
}

fn significant_after<'tokens>(scan: &'tokens Scan<'_>, end: u32) -> Option<&'tokens Token> {
    let end = usize::try_from(end).ok()?;
    scan.tokens()
        .iter()
        .find(|token| token.start >= end && !token.kind.is_comment())
}

fn source_text(source: &str, span: Span) -> Option<&str> {
    source.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
}

fn is_single_line(source: &str, span: Span) -> bool {
    source_text(source, span).is_some_and(|text| !contains_line_terminator(text))
}

fn same_line(source: &str, start: u32, end: u32) -> bool {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return false;
    };
    source
        .get(start..end)
        .is_some_and(|text| !contains_line_terminator(text))
}

fn contains_line_terminator(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn line_start_and_column(source: &str, offset: usize) -> Option<(usize, usize)> {
    let prefix = source.get(..offset)?;
    let mut line_start = 0;
    let mut cursor = 0;
    while cursor < prefix.len() {
        let character = prefix.get(cursor..)?.chars().next()?;
        let length = character.len_utf8();
        match character {
            '\r' => {
                cursor += length;
                if prefix.as_bytes().get(cursor) == Some(&b'\n') {
                    cursor += 1;
                }
                line_start = cursor;
                continue;
            }
            '\n' | '\u{2028}' | '\u{2029}' => line_start = cursor + length,
            _ => {}
        }
        cursor += length;
    }
    let column = prefix.get(line_start..)?.encode_utf16().count();
    Some((line_start, column))
}

fn repeated(character: char, count: usize) -> String {
    std::iter::repeat_n(character, count).collect()
}

fn u32_from_usize(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json fixtures and formatted diagnostics keep exhaustive upstream tests readable"
)]
mod tests {
    use serde_json::Value;

    use super::check_jsx_wrap_multilines;
    use crate::{LintDiagnostic, LintFix};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-wrap-multilines-v5.10.0.json"
        ))
        .expect("fixture must be valid JSON")
    }

    fn diagnostics(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_wrap_multilines(source, Some("fixture.tsx"), options, &mut diagnostics);
        diagnostics
    }

    fn fixed_source(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<&LintFix>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| {
            (
                std::cmp::Reverse(fix.range.start),
                std::cmp::Reverse(fix.range.end),
            )
        });
        let mut output = source.to_owned();
        for fix in fixes {
            let start = usize::try_from(fix.range.start).expect("start fits usize");
            let end = usize::try_from(fix.range.end).expect("end fits usize");
            output.replace_range(start..end, &fix.replacement_text);
        }
        Some(output)
    }

    #[test]
    fn accepts_all_71_authored_stable_valid_cases() {
        let fixture = fixture();
        let valid = fixture["valid"].as_array().expect("valid cases");
        assert_eq!(valid.len(), 71);
        for (index, case) in valid.iter().enumerate() {
            let source = case["code"].as_str().expect("source");
            let options = case.get("options").unwrap_or(&Value::Null);
            assert!(
                diagnostics(source, options).is_empty(),
                "valid case {index}: {source}"
            );
        }
    }

    #[test]
    fn replays_all_75_invalid_cases_and_93_diagnostics_exactly() {
        let fixture = fixture();
        let invalid = fixture["invalid"].as_array().expect("invalid cases");
        assert_eq!(invalid.len(), 75);
        let mut diagnostic_count = 0;
        for (index, case) in invalid.iter().enumerate() {
            let source = case["code"].as_str().expect("source");
            let options = case.get("options").unwrap_or(&Value::Null);
            let actual = diagnostics(source, options);
            let expected = case["errors"].as_array().expect("errors");
            diagnostic_count += actual.len();
            assert_eq!(
                actual
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|error| error["messageId"].as_str().expect("message id"))
                    .collect::<Vec<_>>(),
                "message ids for invalid case {index}: {source}",
            );
            assert_eq!(
                actual
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|error| error["message"].as_str().expect("message"))
                    .collect::<Vec<_>>(),
                "messages for invalid case {index}: {source}",
            );
            assert_eq!(
                fixed_source(source, &actual).as_deref(),
                case["output"].as_str(),
                "output for invalid case {index}: {source}",
            );
        }
        assert_eq!(diagnostic_count, 93);
    }

    #[test]
    fn fixes_every_owner_mode_and_converges() {
        let source = concat!(
            "const declaration = <A>one\n</A>;\n",
            "let assignment; assignment = <B>two\n</B>;\n",
            "const arrow = () => <C>three\n</C>;\n",
            "function value() { return <D>four\n</D>; }\n",
            "const condition = ready ? <E>five\n</E> : <F>six\n</F>;\n",
            "const logical = ready && <G>seven\n</G>;\n",
            "const prop = <View child={<H>eight\n</H>} />;\n",
            "const object = { value: <I>nine\n</I> };\n",
        );
        let options = serde_json::json!([{
            "declaration": "parens",
            "assignment": "parens",
            "return": "parens",
            "arrow": "parens",
            "condition": "parens",
            "logical": "parens",
            "prop": "parens",
            "propertyValue": "parens"
        }]);
        let first = diagnostics(source, &options);
        assert_eq!(first.len(), 10);
        let output = fixed_source(source, &first).expect("all reports fix");
        assert!(diagnostics(&output, &options).is_empty());
    }

    #[test]
    fn preserves_unicode_tsx_comments_and_all_line_terminators() {
        for newline in ["\r\n", "\r", "\n", "\u{2028}", "\u{2029}"] {
            let source = format!("const 印 = <部品<Item>>日本語{newline}<子 /></部品>;");
            let diagnostics = diagnostics(&source, &Value::Null);
            assert_eq!(diagnostics.len(), 1, "{newline:?}");
            let start = source.find("<部品").expect("JSX start");
            assert_eq!(
                diagnostics[0].range,
                crate::TextRange::new(
                    u32::try_from(start).expect("start"),
                    u32::try_from(source.len() - 1).expect("end"),
                ),
            );
        }

        let source = "const C = () =>\n  // 説明 😀\n  <View>\n    <Child />\n  </View>;";
        let options = serde_json::json!([{ "arrow": "parens-new-line" }]);
        let output = fixed_source(source, &diagnostics(source, &options)).expect("fix");
        assert_eq!(
            output,
            "const C = () => (\n  // 説明 😀\n  <View>\n    <Child />\n  </View>\n);",
        );
    }

    #[test]
    fn ignores_disabled_single_line_and_malformed_inputs_safely() {
        assert!(diagnostics("const value = <A />;", &Value::Null).is_empty());
        assert!(
            diagnostics(
                "const value = <A>\n<B />\n</A>;",
                &serde_json::json!([{ "declaration": false }]),
            )
            .is_empty(),
        );
        assert!(
            diagnostics(
                "const value = <A>\n<B />\n</A>;",
                &serde_json::json!([{ "declaration": "ignore" }]),
            )
            .is_empty(),
        );
        assert!(
            diagnostics(
                "const value = <A>\n<B />\n</A>;",
                &serde_json::json!([{ "declaration": "unknown" }]),
            )
            .len()
                == 1,
        );
        assert!(diagnostics("const broken = <A>", &Value::Null).is_empty());
    }
}
