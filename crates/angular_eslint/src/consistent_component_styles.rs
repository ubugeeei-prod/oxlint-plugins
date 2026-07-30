use oxc_ast::ast::{
    ArrayExpression, BigIntLiteral, BooleanLiteral, Class, ClassType, Decorator, Expression,
    NullLiteral, NumericLiteral, ObjectProperty, PropertyKey, RegExpLiteral, StringLiteral,
    TemplateLiteral,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::Diagnostic;

const CONSISTENT_COMPONENT_STYLES: &str = "consistent-component-styles";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Array,
    String,
}

impl Scanner<'_> {
    pub(crate) fn check_consistent_component_styles(&mut self, class: &Class<'_>) {
        if !self.options.is_enabled(CONSISTENT_COMPONENT_STYLES)
            || class.r#type != ClassType::ClassDeclaration
        {
            return;
        }

        let mode = configured_mode(&self.options.options);
        for decorator in &class.decorators {
            let Some(component) = component_decorator(decorator) else {
                continue;
            };
            let mut collector = MetadataDiagnosticCollector {
                mode,
                diagnostics: SmallVec::new(),
            };
            collector.visit_call_expression(component);
            for (message_id, span) in collector.diagnostics {
                self.diagnostics.push(Diagnostic {
                    rule_name: CONSISTENT_COMPONENT_STYLES,
                    message_id,
                    data: SmallVec::new(),
                    loc: self.line_index.loc_for_span(self.source_text, span),
                });
            }
        }
    }
}

fn configured_mode(options: &Value) -> Mode {
    match options
        .as_array()
        .and_then(|options| options.first())
        .and_then(Value::as_str)
    {
        Some("array") => Mode::Array,
        _ => Mode::String,
    }
}

fn component_decorator<'a>(
    decorator: &'a Decorator<'a>,
) -> Option<&'a oxc_ast::ast::CallExpression<'a>> {
    let Expression::CallExpression(call) = decorator.expression.get_inner_expression() else {
        return None;
    };
    matches!(
        call.callee.get_inner_expression(),
        Expression::Identifier(identifier) if identifier.name == "Component"
    )
    .then_some(call.as_ref())
}

struct MetadataDiagnosticCollector {
    mode: Mode,
    diagnostics: SmallVec<[(&'static str, Span); 8]>,
}

impl<'a> Visit<'a> for MetadataDiagnosticCollector {
    fn visit_object_property(&mut self, property: &ObjectProperty<'a>) {
        let diagnostic = match (self.mode, metadata_property_name(property)) {
            (Mode::Array, Some(MetadataPropertyName::Styles))
                if is_literal_or_template(&property.value) =>
            {
                Some(("useStylesArray", property.value.span()))
            }
            (Mode::Array, Some(MetadataPropertyName::StyleUrl))
                if property_contains_literal(property) =>
            {
                Some(("useStyleUrls", property.span))
            }
            (Mode::String, Some(MetadataPropertyName::Styles)) => {
                let Expression::ArrayExpression(array) = &property.value else {
                    walk::walk_object_property(self, property);
                    return;
                };
                is_single_array_with_literal(array).then_some(("useStylesString", array.span))
            }
            (Mode::String, Some(MetadataPropertyName::StyleUrls))
                if property_contains_single_array_with_literal(property) =>
            {
                Some(("useStyleUrl", property.span))
            }
            _ => None,
        };
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
        walk::walk_object_property(self, property);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetadataPropertyName {
    Styles,
    StyleUrl,
    StyleUrls,
}

fn metadata_property_name(property: &ObjectProperty<'_>) -> Option<MetadataPropertyName> {
    let name = match &property.key {
        PropertyKey::StaticIdentifier(identifier) if !property.computed => identifier.name.as_str(),
        PropertyKey::StringLiteral(literal) => literal.value.as_str(),
        PropertyKey::TemplateLiteral(template) => template.quasis.first()?.value.raw.as_str(),
        _ => return None,
    };
    match name {
        "styles" => Some(MetadataPropertyName::Styles),
        "styleUrl" => Some(MetadataPropertyName::StyleUrl),
        "styleUrls" => Some(MetadataPropertyName::StyleUrls),
        _ => None,
    }
}

fn is_literal_or_template(expression: &Expression<'_>) -> bool {
    matches!(
        expression,
        Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
    )
}

fn is_single_array_with_literal(array: &ArrayExpression<'_>) -> bool {
    if array.elements.len() != 1 {
        return false;
    }
    let mut finder = LiteralFinder::default();
    finder.visit_array_expression(array);
    finder.found
}

fn property_contains_literal(property: &ObjectProperty<'_>) -> bool {
    let mut finder = LiteralFinder::default();
    finder.visit_object_property(property);
    finder.found
}

#[derive(Default)]
struct LiteralFinder {
    found: bool,
}

impl<'a> Visit<'a> for LiteralFinder {
    fn visit_boolean_literal(&mut self, _literal: &BooleanLiteral) {
        self.found = true;
    }

    fn visit_null_literal(&mut self, _literal: &NullLiteral) {
        self.found = true;
    }

    fn visit_numeric_literal(&mut self, _literal: &NumericLiteral<'a>) {
        self.found = true;
    }

    fn visit_string_literal(&mut self, _literal: &StringLiteral<'a>) {
        self.found = true;
    }

    fn visit_big_int_literal(&mut self, _literal: &BigIntLiteral<'a>) {
        self.found = true;
    }

    fn visit_reg_exp_literal(&mut self, _literal: &RegExpLiteral<'a>) {
        self.found = true;
    }

    fn visit_template_literal(&mut self, _template: &TemplateLiteral<'a>) {
        self.found = true;
    }
}

fn property_contains_single_array_with_literal(property: &ObjectProperty<'_>) -> bool {
    let mut finder = SingleArrayLiteralFinder::default();
    finder.visit_object_property(property);
    finder.found
}

#[derive(Default)]
struct SingleArrayLiteralFinder {
    found: bool,
}

impl<'a> Visit<'a> for SingleArrayLiteralFinder {
    fn visit_array_expression(&mut self, array: &ArrayExpression<'a>) {
        if is_single_array_with_literal(array) {
            self.found = true;
            return;
        }
        walk::walk_array_expression(self, array);
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Pinned upstream fixtures use serde_json values and Vec assertions to mirror the JavaScript ABI."
)]
mod tests {
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use super::CONSISTENT_COMPONENT_STYLES;
    use crate::{Diagnostic, ScanOptions, scan_angular_eslint_with_options};

    const UPSTREAM_FIXTURE: &str = include_str!(
        "../../../npm/angular-eslint/test/fixtures/consistent-component-styles-v22.0.0.json"
    );

    fn scan(source: &str, options: Value) -> Vec<Diagnostic> {
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from(
                    CONSISTENT_COMPONENT_STYLES,
                )]),
                options,
            },
        )
        .into_vec()
    }

    #[test]
    fn replays_every_upstream_authored_valid_case() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid consistent styles fixture");
        let valid = fixture["valid"]
            .as_array()
            .expect("fixture has valid cases");
        assert_eq!(valid.len(), 21);
        for test_case in valid {
            let source = test_case["code"]
                .as_str()
                .expect("valid fixture case has source");
            let diagnostics = scan(source, test_case["options"].clone());
            assert!(
                diagnostics.is_empty(),
                "{}: {diagnostics:#?}",
                test_case["name"]
                    .as_str()
                    .expect("valid fixture case has name")
            );
        }
    }

    #[test]
    fn replays_every_upstream_authored_invalid_location_and_message() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid consistent styles fixture");
        let invalid = fixture["invalid"]
            .as_array()
            .expect("fixture has invalid cases");
        assert_eq!(invalid.len(), 20);
        for test_case in invalid {
            let source = test_case["code"]
                .as_str()
                .expect("invalid fixture case has source");
            let diagnostics = scan(source, test_case["options"].clone());
            let errors = test_case["errors"]
                .as_array()
                .expect("invalid fixture case has errors");
            assert_eq!(
                diagnostics.len(),
                errors.len(),
                "{}: {diagnostics:#?}",
                test_case["name"]
                    .as_str()
                    .expect("invalid fixture case has name")
            );
            for (diagnostic, error) in diagnostics.iter().zip(errors) {
                assert_eq!(
                    diagnostic.message_id,
                    error["messageId"].as_str().expect("error has a message id")
                );
                assert!(diagnostic.data.is_empty());
                assert_eq!(
                    diagnostic.loc.start_line,
                    error["line"].as_u64().expect("error has line") as u32
                );
                assert_eq!(
                    diagnostic.loc.start_column + 1,
                    error["column"].as_u64().expect("error has column") as u32
                );
                assert_eq!(
                    diagnostic.loc.end_line,
                    error["endLine"].as_u64().expect("error has end line") as u32
                );
                assert_eq!(
                    diagnostic.loc.end_column + 1,
                    error["endColumn"].as_u64().expect("error has end column") as u32
                );
            }
        }
    }

    #[test]
    fn reports_all_four_messages_in_source_order() {
        let source = r#"
            @Component({
                styles: ['first'],
                styleUrls: [`first.css`],
            })
            class StringMode {}
            @Component({
                styles: `second`,
                styleUrl: 'second.css',
            })
            class ArrayMode {}
        "#;
        let string_diagnostics = scan(source, json!(["string"]));
        assert_eq!(
            string_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            ["useStylesString", "useStyleUrl"]
        );

        let array_diagnostics = scan(source, json!(["array"]));
        assert_eq!(
            array_diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            ["useStylesArray", "useStyleUrls"]
        );
        assert!(array_diagnostics.windows(2).all(|pair| {
            (pair[0].loc.start_line, pair[0].loc.start_column)
                < (pair[1].loc.start_line, pair[1].loc.start_column)
        }));
    }

    #[test]
    fn defaults_unknown_or_missing_modes_to_string() {
        let source = "@Component({ styles: ['x'] }) class Test {}";
        for options in [
            json!([]),
            json!(["string"]),
            json!(["invalid"]),
            json!([{}]),
        ] {
            let diagnostics = scan(source, options);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message_id, "useStylesString");
        }
    }

    #[test]
    fn matches_upstream_literal_and_descendant_selector_semantics() {
        let array_source = r#"
            @Component({
                styles: 1,
                styleUrl: choose(url, 'fallback.css'),
            })
            class Test {}
        "#;
        assert_eq!(
            scan(array_source, json!(["array"]))
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            ["useStylesArray", "useStyleUrls"]
        );

        let string_source = r#"
            @Component({
                styles: [resolve('inline')],
                styleUrls: select(['nested.css']),
            })
            class Test {}
        "#;
        assert_eq!(
            scan(string_source, json!(["string"]))
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            ["useStylesString", "useStyleUrl"]
        );
    }

    #[test]
    fn supports_upstream_static_and_template_metadata_key_shapes() {
        let source = r#"
            const suffix = 'ignored-by-upstream-selector';
            @Component({
                'styles': ['one'],
                ['styleUrls']: ['two.css'],
                [`styles${suffix}`]: ['three'],
                [styleUrls]: ['ignored.css'],
            })
            class Test {}
        "#;
        let diagnostics = scan(source, json!(["string"]));
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            ["useStylesString", "useStyleUrl", "useStylesString"]
        );
    }

    #[test]
    fn ignores_non_component_and_non_declaration_contexts() {
        let source = r#"
            const text = "@Component({ styleUrls: ['fake.css'] })";
            const metadata = { styleUrls: ['plain.css'] };
            @Directive({ styleUrls: ['directive.css'] })
            class DirectiveTest {}
            @Other({ styleUrls: ['other.css'] })
            class OtherTest {}
            const Expression = @Component({ styleUrls: ['expression.css'] }) class {};
        "#;
        assert!(scan(source, json!([])).is_empty());
    }

    #[test]
    fn traverses_component_decorator_metadata_without_regex_false_positives() {
        let source = r#"
            @Component(factory({
                styles: ['nested'],
                styleUrls: ['nested.css'],
            }))
            class Test {}
        "#;
        assert_eq!(scan(source, json!([])).len(), 2);
    }

    #[test]
    fn handles_empty_sparse_spread_and_multiple_arrays_like_upstream() {
        let source = r#"
            @Component({
                empty: [],
                styles: [],
                styleUrls: [],
                otherStyles: ['x'],
            })
            class Empty {}
            @Component({
                styles: ['one', 'two'],
                styleUrls: ['one.css', 'two.css'],
            })
            class Multiple {}
            @Component({
                styles: [...['spread']],
                styleUrls: [[`nested.css`]],
            })
            class Descendants {}
        "#;
        let diagnostics = scan(source, json!([]));
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            ["useStylesString", "useStyleUrl"]
        );
    }

    #[test]
    fn preserves_utf16_columns_and_exact_report_spans() {
        let diagnostics = scan(
            "@Component({ marker: '😀', styles: ['x'], styleUrls: ['x.css'] }) class Test {}",
            json!([]),
        );
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            (
                diagnostics[0].loc.start_column,
                diagnostics[0].loc.end_column
            ),
            (35, 40)
        );
        assert_eq!(
            (
                diagnostics[1].loc.start_column,
                diagnostics[1].loc.end_column
            ),
            (42, 62)
        );
    }

    #[test]
    fn is_rule_isolated_and_parse_errors_fail_closed() {
        let source = "@Component({ styleUrls: ['x.css'] }) class Test {}";
        let diagnostics = scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from("no-output-rename")]),
                options: json!([]),
            },
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule_name != CONSISTENT_COMPONENT_STYLES)
        );
        assert!(scan("@Component({ styleUrls: ['x.css']", json!([])).is_empty());
    }
}
