use oxc_ast::ast::{
    Class, Expression, ObjectExpression, ObjectPropertyKind, PropertyKey, TSTypeName,
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::{Diagnostic, DiagnosticDatum};

const COMPONENT_CLASS_SUFFIX: &str = "component-class-suffix";
const DIRECTIVE_CLASS_SUFFIX: &str = "directive-class-suffix";
const VALIDATOR_SUFFIX: &str = "Validator";

impl Scanner<'_> {
    pub(crate) fn check_class_suffix_decorators(&mut self, class: &Class<'_>) {
        for decorator in &class.decorators {
            let Expression::CallExpression(call) = decorator.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                continue;
            };
            match callee.name.as_str() {
                "Component" if self.options.is_enabled(COMPONENT_CLASS_SUFFIX) => {
                    let suffixes = configured_suffixes(&self.options.options, "Component");
                    self.check_class_suffix(
                        class,
                        COMPONENT_CLASS_SUFFIX,
                        "componentClassSuffix",
                        suffixes,
                    );
                }
                "Directive"
                    if self.options.is_enabled(DIRECTIVE_CLASS_SUFFIX)
                        && call
                            .arguments
                            .first()
                            .and_then(|argument| argument.as_expression())
                            .is_some_and(|expression| {
                                let Expression::ObjectExpression(metadata) =
                                    expression.get_inner_expression()
                                else {
                                    return false;
                                };
                                has_selector_property(metadata)
                            }) =>
                {
                    let mut suffixes = configured_suffixes(&self.options.options, "Directive");
                    if implements_validator(class) {
                        suffixes.push(CompactString::from(VALIDATOR_SUFFIX));
                    }
                    self.check_class_suffix(
                        class,
                        DIRECTIVE_CLASS_SUFFIX,
                        "directiveClassSuffix",
                        suffixes,
                    );
                }
                _ => {}
            }
        }
    }

    fn check_class_suffix(
        &mut self,
        class: &Class<'_>,
        rule_name: &'static str,
        message_id: &'static str,
        suffixes: SmallVec<[CompactString; 4]>,
    ) {
        let class_name = class.id.as_ref().map(|identifier| identifier.name.as_str());
        if class_name.is_some_and(|name| {
            suffixes
                .iter()
                .any(|suffix| name.ends_with(suffix.as_str()))
        }) {
            return;
        }
        let span = class
            .id
            .as_ref()
            .map_or(class.span, |identifier| identifier.span);
        let mut data = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("suffixes"),
            value: human_readable_suffixes(&suffixes),
        });
        self.report_class_suffix(rule_name, message_id, data, span);
    }

    fn report_class_suffix(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: SmallVec<[DiagnosticDatum; 2]>,
        span: Span,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

fn configured_suffixes(options: &Value, default_suffix: &str) -> SmallVec<[CompactString; 4]> {
    let configured = options
        .as_array()
        .and_then(|options| options.first())
        .and_then(Value::as_object)
        .and_then(|option| option.get("suffixes"))
        .and_then(Value::as_array);
    match configured {
        Some(suffixes) => suffixes
            .iter()
            .filter_map(Value::as_str)
            .map(CompactString::from)
            .collect(),
        None => {
            let mut suffixes = SmallVec::new();
            suffixes.push(CompactString::from(default_suffix));
            suffixes
        }
    }
}

fn has_selector_property(metadata: &ObjectExpression<'_>) -> bool {
    metadata.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        property_name(&property.key) == Some("selector")
    })
}

fn property_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

fn implements_validator(class: &Class<'_>) -> bool {
    class.implements.iter().any(|implementation| {
        type_name(&implementation.expression).is_some_and(|name| name.ends_with(VALIDATOR_SUFFIX))
    })
}

fn type_name<'a>(name: &'a TSTypeName<'a>) -> Option<&'a str> {
    match name {
        TSTypeName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        TSTypeName::QualifiedName(qualified) => Some(qualified.right.name.as_str()),
        TSTypeName::ThisExpression(_) => None,
    }
}

fn human_readable_suffixes(suffixes: &[CompactString]) -> CompactString {
    let mut output = CompactString::new("");
    for (index, suffix) in suffixes.iter().enumerate() {
        if index > 0 {
            if index + 1 == suffixes.len() {
                output.push_str(" or ");
            } else {
                output.push_str(", ");
            }
        }
        output.push('"');
        output.push_str(suffix);
        output.push('"');
    }
    output
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Pinned upstream option cases use serde_json::json and Vec-shaped assertions to mirror the JavaScript ABI exactly."
)]
mod tests {
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use super::{COMPONENT_CLASS_SUFFIX, DIRECTIVE_CLASS_SUFFIX};
    use crate::{Diagnostic, ScanOptions, scan_angular_eslint_with_options};

    // Pinned from angular-eslint v22.0.0, commit
    // 7ee4556badebf8c140ffdefdd0b07b02820d5e96.
    fn scan(rule_name: &str, source: &str, options: Value) -> Vec<Diagnostic> {
        let mut rule_names = SmallVec::new();
        rule_names.push(CompactString::from(rule_name));
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names,
                options,
            },
        )
        .into_vec()
    }

    #[test]
    fn ports_the_upstream_component_class_suffix_cases() {
        let valid = [
            "@Component({ selector: 'sg-foo-bar' }) class TestComponent {}",
            "@Directive({ selector: '[myHighlight]' }) class TestDirective {}",
            "@Pipe({ name: 'sg-test-pipe' }) class TestPipe {}",
            "@Injectable() class TestService {}",
            "class TestEmpty {}",
        ];
        for source in valid {
            assert!(scan(COMPONENT_CLASS_SUFFIX, source, Value::Null).is_empty());
        }
        for (source, option) in [
            (
                "@Component({ selector: 'sgBarFoo' }) class TestPage {}",
                json!([{"suffixes":["Page"]}]),
            ),
            (
                "@Component({ selector: 'sgBarFoo' }) class TestView {}",
                json!([{"suffixes":["Page","View"]}]),
            ),
        ] {
            assert!(scan(COMPONENT_CLASS_SUFFIX, source, option).is_empty());
        }

        let invalid = [
            (
                "@Component({ selector: 'sg-foo-bar' }) class Test {}",
                Value::Null,
                "\"Component\"",
            ),
            (
                "@Component({ selector: 'sgBarFoo' }) class TestPage {}",
                json!([{"suffixes":["Component","View"]}]),
                "\"Component\" or \"View\"",
            ),
            (
                "@Component({ selector: 'sgBarFoo' }) class TestPage {}",
                json!([{"suffixes":["Component"]}]),
                "\"Component\"",
            ),
            (
                "@Component({ selector: 'sgBarFoo' }) class TestDirective {}",
                json!([{"suffixes":["Page"]}]),
                "\"Page\"",
            ),
        ];
        for (source, option, suffixes) in invalid {
            let diagnostics = scan(COMPONENT_CLASS_SUFFIX, source, option);
            assert_eq!(diagnostics.len(), 1, "{source}");
            assert_eq!(diagnostics[0].message_id, "componentClassSuffix");
            assert_eq!(diagnostics[0].data[0].key, "suffixes");
            assert_eq!(diagnostics[0].data[0].value, suffixes);
        }
    }

    #[test]
    fn ports_the_upstream_directive_class_suffix_cases() {
        let valid = [
            "@Directive({ selector: 'sgBarFoo' }) class TestDirective {}",
            "@Directive({ selector: 'sgBarFoo' }) class TestValidator implements Validator {}",
            "@Directive({ selector: 'sgBarFoo' }) class TestValidator implements AsyncValidator {}",
            "@Directive class Test {}",
            "@Directive() class Test {}",
            "@Component({ selector: 'sg-bar-foo' }) class TestComponent {}",
            "@Pipe({ name: 'sgPipe' }) class TestPipe {}",
            "@Injectable() class TestService {}",
            "class TestEmpty {}",
        ];
        for source in valid {
            assert!(scan(DIRECTIVE_CLASS_SUFFIX, source, Value::Null).is_empty());
        }
        for (source, option) in [
            (
                "@Directive({ selector: 'sgBarFoo' }) class TestDir {}",
                json!([{"suffixes":["Dir"]}]),
            ),
            (
                "@Directive({ selector: 'sgBarFoo' }) class TestView {}",
                json!([{"suffixes":["Page","View"]}]),
            ),
        ] {
            assert!(scan(DIRECTIVE_CLASS_SUFFIX, source, option).is_empty());
        }

        let invalid = [
            (
                "@Directive({ selector: 'sg-foo-bar' }) class Test {}",
                Value::Null,
                "\"Directive\"",
            ),
            (
                "@Directive({ selector: 'sg-foo-bar' }) class TestDirectivePage implements AsyncValidator {}",
                Value::Null,
                "\"Directive\" or \"Validator\"",
            ),
            (
                "@Directive({ selector: 'sgBarFoo' }) class TestPageDirective {}",
                json!([{"suffixes":["Page"]}]),
                "\"Page\"",
            ),
        ];
        for (source, option, suffixes) in invalid {
            let diagnostics = scan(DIRECTIVE_CLASS_SUFFIX, source, option);
            assert_eq!(diagnostics.len(), 1, "{source}");
            assert_eq!(diagnostics[0].message_id, "directiveClassSuffix");
            assert_eq!(diagnostics[0].data[0].key, "suffixes");
            assert_eq!(diagnostics[0].data[0].value, suffixes);
        }
    }

    #[test]
    fn covers_multiple_classes_rule_selection_utf16_and_edge_options() {
        let source = concat!(
            "const emoji = '😀';\n",
            "@Component({}) class First {}\n",
            "@Component({}) class SecondPage {}\n",
            "@Directive({ selector: dynamicSelector }) class ThirdValidator implements forms.AsyncValidator {}\n",
            "@Directive({ other: true }) class Ignored {}\n",
        );
        let component_diagnostics = scan(
            COMPONENT_CLASS_SUFFIX,
            source,
            json!([{"suffixes":["Page"]}]),
        );
        assert_eq!(component_diagnostics.len(), 1);
        assert_eq!(component_diagnostics[0].loc.start_line, 2);
        assert_eq!(component_diagnostics[0].loc.start_column, 21);
        assert_eq!(component_diagnostics[0].loc.end_column, 26);

        let directive_diagnostics = scan(DIRECTIVE_CLASS_SUFFIX, source, Value::Null);
        assert!(directive_diagnostics.is_empty());
        assert!(scan(COMPONENT_CLASS_SUFFIX, source, json!([{"suffixes":[]}])).len() == 2);
        assert!(scan(COMPONENT_CLASS_SUFFIX, "class 😀 {}", Value::Null).is_empty());
        assert!(
            scan(
                DIRECTIVE_CLASS_SUFFIX,
                "@Directive({ selector: 'x' }) class Wrong {}",
                json!([{"suffixes":["Wrong"]}]),
            )
            .is_empty()
        );
    }
}
