use oxc_ast::ast::{
    CallExpression, Class, ClassElement, Decorator, Expression, MethodDefinitionKind,
    ObjectExpression, ObjectProperty, ObjectPropertyKind, PropertyDefinition, PropertyKey,
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::Diagnostic;

const NO_INPUT_RENAME: &str = "no-input-rename";

#[derive(Debug, Default)]
struct SelectorContext {
    selectors: SmallVec<[CompactString; 4]>,
    directive_name: Option<CompactString>,
}

impl Scanner<'_> {
    pub(crate) fn check_input_rename(&mut self, class: &Class<'_>) {
        if !self.options.is_enabled(NO_INPUT_RENAME) {
            return;
        }

        let allowed_names = configured_allowed_names(&self.options.options);
        let selector_context = selector_context(class);
        self.check_input_metadata(class, &allowed_names, &selector_context);

        for element in &class.body.body {
            match element {
                ClassElement::PropertyDefinition(property) if !property.computed => {
                    let Some(property_name) = identifier_property_name(&property.key) else {
                        continue;
                    };
                    if let Some((alias, span)) = decorator_alias(&property.decorators) {
                        self.check_alias(
                            property_name,
                            alias,
                            span,
                            &allowed_names,
                            &selector_context,
                        );
                    }
                    if let Some((alias, span)) = signal_input_alias(property) {
                        self.check_alias(
                            property_name,
                            alias,
                            span,
                            &allowed_names,
                            &selector_context,
                        );
                    }
                }
                ClassElement::MethodDefinition(method)
                    if method.kind == MethodDefinitionKind::Set && !method.computed =>
                {
                    let Some(property_name) = identifier_property_name(&method.key) else {
                        continue;
                    };
                    if let Some((alias, span)) = decorator_alias(&method.decorators) {
                        self.check_alias(
                            property_name,
                            alias,
                            span,
                            &allowed_names,
                            &selector_context,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn check_input_metadata(
        &mut self,
        class: &Class<'_>,
        allowed_names: &[CompactString],
        selector_context: &SelectorContext,
    ) {
        for decorator in &class.decorators {
            let Some(call) = called_decorator(decorator, &["Component", "Directive"]) else {
                continue;
            };
            let Some(Expression::ObjectExpression(metadata)) = call
                .arguments
                .first()
                .and_then(|argument| argument.as_expression())
                .map(Expression::get_inner_expression)
            else {
                continue;
            };
            for property in &metadata.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    continue;
                };
                if metadata_property_name(property) != Some("inputs") {
                    continue;
                }
                let Expression::ArrayExpression(inputs) = property.value.get_inner_expression()
                else {
                    continue;
                };
                for input in &inputs.elements {
                    let Some(expression) = input.as_expression() else {
                        continue;
                    };
                    let Some((binding, span)) = static_string(expression) else {
                        continue;
                    };
                    let normalized: CompactString = binding
                        .chars()
                        .filter(|character| {
                            !character.is_whitespace() && !matches!(character, '[' | ']')
                        })
                        .collect();
                    let mut names = normalized.split(':');
                    let property_name = names.next().unwrap_or_default();
                    let Some(alias_name) = names.next() else {
                        continue;
                    };
                    self.check_alias(
                        property_name,
                        alias_name,
                        span,
                        allowed_names,
                        selector_context,
                    );
                }
            }
        }
    }

    fn check_alias(
        &mut self,
        property_name: &str,
        alias_name: &str,
        span: Span,
        allowed_names: &[CompactString],
        selector_context: &SelectorContext,
    ) {
        if allowed_names.iter().any(|name| name == alias_name)
            || aria_alias_matches_property(property_name, alias_name)
            || alias_allowed_by_selector(selector_context, property_name, alias_name)
        {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name: NO_INPUT_RENAME,
            message_id: "noInputRename",
            data: SmallVec::new(),
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

fn configured_allowed_names(options: &Value) -> SmallVec<[CompactString; 4]> {
    options
        .as_array()
        .and_then(|options| options.first())
        .and_then(Value::as_object)
        .and_then(|option| option.get("allowedNames"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(CompactString::from)
        .collect()
}

fn selector_context(class: &Class<'_>) -> SelectorContext {
    let mut context = SelectorContext::default();
    for decorator in &class.decorators {
        let Some(call) = called_decorator(decorator, &["Component", "Directive"]) else {
            continue;
        };
        let Some(Expression::ObjectExpression(metadata)) = call
            .arguments
            .first()
            .and_then(|argument| argument.as_expression())
            .map(Expression::get_inner_expression)
        else {
            continue;
        };
        let Some((selector, _)) = object_static_string(metadata, "selector") else {
            continue;
        };
        context.directive_name = bracketed_directive_name(selector).map(CompactString::from);
        context.selectors = selector
            .split(',')
            .map(|selector| {
                selector
                    .chars()
                    .filter(|character| {
                        !character.is_whitespace() && !matches!(character, '[' | ']')
                    })
                    .collect()
            })
            .collect();
    }
    context
}

fn bracketed_directive_name(selector: &str) -> Option<&str> {
    let start = selector.find('[')? + 1;
    let end = selector[start..].find(']')? + start;
    Some(&selector[start..end])
}

fn alias_allowed_by_selector(
    context: &SelectorContext,
    property_name: &str,
    alias_name: &str,
) -> bool {
    context.directive_name.as_deref() == Some(alias_name)
        || context.selectors.iter().any(|selector| {
            selector == alias_name || composed_name(selector, property_name).as_str() == alias_name
        })
}

fn composed_name(selector: &str, property_name: &str) -> CompactString {
    let mut name = CompactString::from(selector);
    let mut chars = property_name.chars();
    if let Some(first) = chars.next() {
        name.extend(first.to_uppercase());
    }
    name.extend(chars);
    name
}

fn decorator_alias<'a>(decorators: &'a [Decorator<'a>]) -> Option<(&'a str, Span)> {
    let call = decorators.iter().find_map(|decorator| {
        let Expression::CallExpression(call) = decorator.expression.get_inner_expression() else {
            return None;
        };
        matches!(
            call.callee.get_inner_expression(),
            Expression::Identifier(identifier) if identifier.name == "Input"
        )
        .then_some(call.as_ref())
    })?;
    let argument = call.arguments.first()?.as_expression()?;
    if let Some(value) = static_string(argument) {
        return Some(value);
    }
    let Expression::ObjectExpression(metadata) = argument.get_inner_expression() else {
        return None;
    };
    object_alias_static_string(metadata)
}

fn signal_input_alias<'a>(property: &'a PropertyDefinition<'a>) -> Option<(&'a str, Span)> {
    let Expression::CallExpression(call) = property.value.as_ref()?.get_inner_expression() else {
        return None;
    };
    if !is_input_call(call) {
        return None;
    }
    call.arguments.iter().find_map(|argument| {
        let Expression::ObjectExpression(options) =
            argument.as_expression()?.get_inner_expression()
        else {
            return None;
        };
        object_alias_static_string(options)
    })
}

fn is_input_call(call: &CallExpression<'_>) -> bool {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => identifier.name == "input",
        Expression::StaticMemberExpression(member) => {
            member.property.name == "required"
                && matches!(
                    member.object.get_inner_expression(),
                    Expression::Identifier(identifier) if identifier.name == "input"
                )
        }
        _ => false,
    }
}

fn called_decorator<'a>(
    decorator: &'a Decorator<'a>,
    names: &[&str],
) -> Option<&'a CallExpression<'a>> {
    let Expression::CallExpression(call) = decorator.expression.get_inner_expression() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
        return None;
    };
    names
        .contains(&callee.name.as_str())
        .then_some(call.as_ref())
}

fn object_static_string<'a>(
    object: &'a ObjectExpression<'a>,
    name: &str,
) -> Option<(&'a str, Span)> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        (metadata_property_name(property) == Some(name))
            .then(|| static_string(&property.value))
            .flatten()
    })
}

fn object_alias_static_string<'a>(object: &'a ObjectExpression<'a>) -> Option<(&'a str, Span)> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        matches!(
            &property.key,
            PropertyKey::StaticIdentifier(identifier)
                if !property.computed && identifier.name == "alias"
        )
        .then(|| static_string(&property.value))
        .flatten()
    })
}

fn metadata_property_name<'a>(property: &'a ObjectProperty<'a>) -> Option<&'a str> {
    match &property.key {
        PropertyKey::StaticIdentifier(identifier) if !property.computed => {
            Some(identifier.name.as_str())
        }
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        PropertyKey::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .map(|quasi| quasi.value.raw.as_str()),
        _ => None,
    }
}

fn identifier_property_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    let PropertyKey::StaticIdentifier(identifier) = key else {
        return None;
    };
    Some(identifier.name.as_str())
}

fn static_string<'a>(expression: &'a Expression<'a>) -> Option<(&'a str, Span)> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Some((literal.value.as_str(), literal.span)),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .map(|quasi| (quasi.value.raw.as_str(), template.span)),
        _ => None,
    }
}

fn aria_alias_matches_property(property_name: &str, alias_name: &str) -> bool {
    is_aria_attribute(alias_name) && kebab_to_camel_case(alias_name) == property_name
}

fn kebab_to_camel_case(value: &str) -> CompactString {
    let mut output = CompactString::new("");
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '-' && chars.peek().is_some_and(|next| next.is_ascii_alphabetic()) {
            if let Some(next) = chars.next() {
                output.push(next.to_ascii_uppercase());
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn is_aria_attribute(value: &str) -> bool {
    matches!(
        value,
        "aria-activedescendant"
            | "aria-atomic"
            | "aria-autocomplete"
            | "aria-busy"
            | "aria-checked"
            | "aria-colcount"
            | "aria-colindex"
            | "aria-colspan"
            | "aria-controls"
            | "aria-current"
            | "aria-describedby"
            | "aria-details"
            | "aria-disabled"
            | "aria-dragged"
            | "aria-dropeffect"
            | "aria-errormessage"
            | "aria-expanded"
            | "aria-flowto"
            | "aria-haspopup"
            | "aria-hidden"
            | "aria-invalid"
            | "aria-label"
            | "aria-labelledby"
            | "aria-level"
            | "aria-live"
            | "aria-modal"
            | "aria-multiline"
            | "aria-multiselectable"
            | "aria-orientation"
            | "aria-owns"
            | "aria-placeholder"
            | "aria-posinset"
            | "aria-pressed"
            | "aria-readonly"
            | "aria-relevant"
            | "aria-required"
            | "aria-rowcount"
            | "aria-rowindex"
            | "aria-rowspan"
            | "aria-selected"
            | "aria-setsize"
            | "aria-sort"
            | "aria-valuemax"
            | "aria-valuemin"
            | "aria-valuenow"
            | "aria-valuetext"
    )
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

    use super::NO_INPUT_RENAME;
    use crate::{Diagnostic, ScanOptions, scan_angular_eslint_with_options};

    const UPSTREAM_FIXTURE: &str =
        include_str!("../../../npm/angular-eslint/test/fixtures/no-input-rename-v22.0.0.json");

    fn scan(source: &str, options: Value) -> Vec<Diagnostic> {
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from(NO_INPUT_RENAME)]),
                options,
            },
        )
        .into_vec()
    }

    #[test]
    fn replays_every_upstream_authored_valid_case() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid no-input-rename fixture");
        let valid = fixture["valid"]
            .as_array()
            .expect("fixture has valid cases");
        assert_eq!(valid.len(), 46);
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
    fn replays_every_upstream_authored_invalid_location() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid no-input-rename fixture");
        let invalid = fixture["invalid"]
            .as_array()
            .expect("fixture has invalid cases");
        assert_eq!(invalid.len(), 35);
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
                assert_eq!(diagnostic.message_id, "noInputRename");
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
    fn reports_every_supported_alias_shape_in_source_order() {
        let source = r#"
            @Component({ inputs: ['first: firstAlias', `second: secondAlias`] })
            class Test {
                @Input('thirdAlias') third: string;
                fourth = input(0, { alias: 'fourthAlias' });
                fifth = input.required<string>({ alias: `fifthAlias` });
                @Input({ required: true, alias: 'sixthAlias' }) set sixth(value: string) {}
            }
        "#;
        let diagnostics = scan(source, json!([]));
        assert_eq!(diagnostics.len(), 6);
        assert!(diagnostics.windows(2).all(|pair| {
            (pair[0].loc.start_line, pair[0].loc.start_column)
                < (pair[1].loc.start_line, pair[1].loc.start_column)
        }));
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message_id == "noInputRename")
        );
    }

    #[test]
    fn honors_allowed_names_selectors_composition_and_aria_semantics() {
        let source = r#"
            @Directive({
                selector: 'img[fooDirective], foo',
                inputs: ['metadata: foo', 'host: fooDirective', 'ariaLabel: aria-label']
            })
            class Test {
                @Input('fooMyColor') myColor: string;
                bySelector = input(0, { alias: 'foo' });
                byDirective = input.required<string>({ alias: 'fooDirective' });
                ariaLabel = input(0, { alias: 'aria-label' });
                allowed = input(0, { alias: 'migration-name' });
            }
        "#;
        assert!(scan(source, json!([{"allowedNames":["migration-name"]}])).is_empty());
    }

    #[test]
    fn rejects_near_miss_selector_and_aria_aliases() {
        let source = r#"
            @Directive({ selector: 'foo' })
            class Test {
                @Input('foocolor') color: string;
                ariaBusy = input(0, { alias: 'aria-invalid' });
                wrongAria = input.required<string>({ alias: 'aria-madeup' });
            }
        "#;
        assert_eq!(scan(source, json!([])).len(), 3);
    }

    #[test]
    fn ignores_host_directive_renames_and_dynamic_or_computed_shapes() {
        let source = r#"
            const alias = 'external';
            const dynamicMetadataKey = 'inputs';
            @Component({
                hostDirectives: [{ inputs: ['internal: external'] }],
                inputs: [dynamic(), alias, ...spread],
                [dynamicMetadataKey]: ['internal: external']
            })
            class Test {
                @Input(alias) dynamicDecorator: string;
                [computed] = input(0, { alias: 'computedAlias' });
                ordinary = customInput(0, { alias: 'notAngular' });
                required = input['required']({ alias: 'computedRequired' });
                dynamicOptions = input(0, { [alias]: 'dynamicAlias' });
                quotedOptions = input(0, { 'alias': 'quotedAlias' });
            }
        "#;
        assert!(scan(source, json!([])).is_empty());
    }

    #[test]
    fn does_not_match_comments_strings_or_unrelated_decorators() {
        let source = r#"
            const text = "@Input('renamed') name";
            // @Input("commented") property: string;
            class Test {
                @OtherInput('renamed') name: string;
                value = other.input({ alias: 'renamed' });
            }
        "#;
        assert!(scan(source, json!([])).is_empty());
    }

    #[test]
    fn does_not_treat_computed_identifier_metadata_keys_as_static() {
        let source = r#"
            const selector = 'selector';
            const inputs = 'inputs';
            @Directive({
                [selector]: 'publicName',
                [inputs]: ['internal: publicName']
            })
            class Test {
                @Input('publicName') internal: string;
            }
        "#;
        let diagnostics = scan(source, json!([]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].loc.start_line, 9);
    }

    #[test]
    fn preserves_utf16_columns_and_rule_isolation() {
        let source = "class Test { emoji = '😀'; @Input('renamed') name: string; }";
        let diagnostics = scan(source, json!([]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].loc.start_column, 34);
        assert_eq!(diagnostics[0].loc.end_column, 43);

        let selected_other_rule = scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from("no-output-rename")]),
                options: Value::Null,
            },
        );
        assert!(
            selected_other_rule
                .iter()
                .all(|diagnostic| diagnostic.rule_name != NO_INPUT_RENAME)
        );
    }

    #[test]
    fn malformed_source_fails_closed() {
        assert!(scan("class Test { @Input('rename'", json!([])).is_empty());
        assert!(scan("@Component({ inputs: ['a: b'] }) class {", json!([])).is_empty());
    }
}
