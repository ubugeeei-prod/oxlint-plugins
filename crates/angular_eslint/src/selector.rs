use oxc_ast::ast::{Class, Expression, ObjectExpression, ObjectPropertyKind, PropertyKey};
use oxc_ast_visit::{Visit, walk};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::{Diagnostic, DiagnosticDatum};

const COMPONENT_SELECTOR: &str = "component-selector";
const DIRECTIVE_SELECTOR: &str = "directive-selector";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectorType {
    Attribute,
    Element,
}

impl SelectorType {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "attribute" => Some(Self::Attribute),
            "element" => Some(Self::Element),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Attribute => "attribute",
            Self::Element => "element",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectorStyle {
    CamelCase,
    KebabCase,
}

impl SelectorStyle {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "camelCase" => Some(Self::CamelCase),
            "kebab-case" => Some(Self::KebabCase),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::CamelCase => "camelCase",
            Self::KebabCase => "kebab-case",
        }
    }
}

#[derive(Clone, Debug)]
struct SelectorConfig {
    selector_type: SelectorType,
    prefixes: Option<SmallVec<[CompactString; 4]>>,
    style: SelectorStyle,
}

#[derive(Clone, Debug, Default)]
struct ParsedSelector {
    element: Option<CompactString>,
    attributes: SmallVec<[CompactString; 4]>,
}

impl Visit<'_> for Scanner<'_> {
    fn visit_class(&mut self, class: &Class<'_>) {
        self.check_class_suffix_decorators(class);
        self.check_component_inline_declarations(class);
        self.check_input_rename(class);
        self.check_prefix_rules(class);
        self.check_selector_decorators(class);
        walk::walk_class(self, class);
    }
}

impl Scanner<'_> {
    fn check_selector_decorators(&mut self, class: &Class<'_>) {
        let options = &self.options.options;
        for decorator in &class.decorators {
            let Expression::CallExpression(call) = decorator.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                continue;
            };
            let (rule_name, is_component) = match callee.name.as_str() {
                "Component" if self.options.is_enabled(COMPONENT_SELECTOR) => {
                    (COMPONENT_SELECTOR, true)
                }
                "Directive" if self.options.is_enabled(DIRECTIVE_SELECTOR) => {
                    (DIRECTIVE_SELECTOR, false)
                }
                _ => continue,
            };
            let Some(Expression::ObjectExpression(metadata)) = call
                .arguments
                .first()
                .and_then(|argument| argument.as_expression())
            else {
                continue;
            };
            let Some((selector, span)) = selector_value(metadata) else {
                continue;
            };
            let parsed = parse_selectors(selector);
            if parsed.is_empty() {
                continue;
            }
            let Some(config) = selector_config(options, &parsed) else {
                continue;
            };
            self.check_selector(rule_name, is_component, metadata, &parsed, &config, span);
        }
    }

    fn check_selector(
        &mut self,
        rule_name: &'static str,
        is_component: bool,
        metadata: &ObjectExpression<'_>,
        parsed: &[ParsedSelector],
        config: &SelectorConfig,
        span: Span,
    ) {
        let shadow_dom = is_component
            && config.style != SelectorStyle::KebabCase
            && has_shadow_dom_encapsulation(metadata);
        let effective_style = if shadow_dom {
            SelectorStyle::KebabCase
        } else {
            config.style
        };
        let valid_selectors = selector_values(parsed, config.selector_type);
        let prefixes = config.prefixes.as_deref().unwrap_or_default();
        let prefix_required =
            !prefixes.is_empty() && prefixes.iter().any(|prefix| !prefix.is_empty());
        let has_expected_type = !valid_selectors.is_empty();
        let has_expected_prefix = !prefix_required
            || valid_selectors.iter().any(|selector| {
                prefixes.iter().any(|prefix| {
                    prefix_matches(selector, prefix, effective_style, config.selector_type)
                })
            });
        let has_expected_style = valid_selectors
            .iter()
            .any(|selector| style_matches(selector, effective_style));
        let has_selector_after_prefix = !prefix_required
            || valid_selectors.iter().any(|selector| {
                prefixes
                    .iter()
                    .any(|prefix| selector_after_prefix(selector, prefix))
            });

        if shadow_dom
            && !parsed.iter().any(|selector| {
                selector
                    .element
                    .as_deref()
                    .is_some_and(|value| value.contains('-'))
            })
        {
            self.report_selector(
                rule_name,
                "shadowDomEncapsulatedStyleFailure",
                SmallVec::new(),
                span,
            );
        } else if !has_expected_type {
            self.report_selector(
                rule_name,
                "typeFailure",
                data("type", config.selector_type.as_str()),
                span,
            );
        } else if !has_selector_after_prefix && prefix_required {
            self.report_selector(
                rule_name,
                "selectorAfterPrefixFailure",
                data("prefix", &human_readable_prefixes(prefixes)),
                span,
            );
        } else if !has_expected_style {
            if shadow_dom {
                self.report_selector(
                    rule_name,
                    "shadowDomEncapsulatedStyleFailure",
                    SmallVec::new(),
                    span,
                );
            } else if is_component && !has_expected_prefix && prefix_required {
                self.report_selector(
                    rule_name,
                    "styleAndPrefixFailure",
                    data2(
                        "style",
                        config.style.as_str(),
                        "prefix",
                        &human_readable_prefixes(prefixes),
                    ),
                    span,
                );
            } else {
                self.report_selector(
                    rule_name,
                    "styleFailure",
                    data("style", config.style.as_str()),
                    span,
                );
            }
        } else if !has_expected_prefix && prefix_required {
            self.report_selector(
                rule_name,
                "prefixFailure",
                data("prefix", &human_readable_prefixes(prefixes)),
                span,
            );
        }
    }

    fn report_selector(
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

fn selector_value<'a>(metadata: &'a ObjectExpression<'a>) -> Option<(&'a str, Span)> {
    for property in &metadata.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        if property_name(&property.key) != Some("selector") {
            continue;
        }
        return match property.value.get_inner_expression() {
            Expression::StringLiteral(literal) => Some((literal.value.as_str(), literal.span)),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
                .quasis
                .first()
                .map(|quasi| (quasi.value.raw.as_str(), template.span)),
            _ => None,
        };
    }
    None
}

fn property_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

fn selector_config(options: &Value, parsed: &[ParsedSelector]) -> Option<SelectorConfig> {
    let first = options.as_array()?.first()?;
    let mut configs: SmallVec<[SelectorConfig; 2]> = SmallVec::new();
    if let Some(items) = first.as_array() {
        for item in items {
            configs.extend(configs_from_object(item));
        }
    } else {
        configs.extend(configs_from_object(first));
    }
    if configs.is_empty() {
        return None;
    }
    if configs.len() == 1 {
        return configs.into_iter().next();
    }
    let actual_type = if parsed
        .first()
        .is_some_and(|selector| !selector.attributes.is_empty())
    {
        SelectorType::Attribute
    } else if parsed
        .first()
        .and_then(|selector| selector.element.as_deref())
        .is_some_and(|element| !element.is_empty() && element != "*")
    {
        SelectorType::Element
    } else {
        return None;
    };
    configs
        .into_iter()
        .find(|config| config.selector_type == actual_type)
}

fn configs_from_object(value: &Value) -> SmallVec<[SelectorConfig; 2]> {
    let Some(object) = value.as_object() else {
        return SmallVec::new();
    };
    let Some(style) = object
        .get("style")
        .and_then(Value::as_str)
        .and_then(SelectorStyle::parse)
    else {
        return SmallVec::new();
    };
    let prefixes = match object.get("prefix") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let mut prefixes = SmallVec::new();
            prefixes.push(CompactString::from(value.as_str()));
            Some(prefixes)
        }
        Some(Value::Array(values)) => Some(
            values
                .iter()
                .filter_map(Value::as_str)
                .map(CompactString::from)
                .collect(),
        ),
        _ => return SmallVec::new(),
    };
    let types: SmallVec<[SelectorType; 2]> = match object.get("type") {
        Some(Value::String(value)) => SelectorType::parse(value).into_iter().collect(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .filter_map(SelectorType::parse)
            .collect(),
        _ => SmallVec::new(),
    };
    types
        .into_iter()
        .map(|selector_type| SelectorConfig {
            selector_type,
            prefixes: prefixes.clone(),
            style,
        })
        .collect()
}

fn parse_selectors(source: &str) -> SmallVec<[ParsedSelector; 4]> {
    source
        .split(',')
        .filter_map(|part| {
            let selector = part.trim();
            if selector.is_empty() {
                return None;
            }
            let element_end = selector
                .find(|character: char| {
                    character == '['
                        || character == '.'
                        || character == '#'
                        || character == ':'
                        || character.is_whitespace()
                })
                .unwrap_or(selector.len());
            let element = selector.get(..element_end).and_then(|value| {
                (!value.is_empty() && value != "*").then(|| CompactString::from(value))
            });
            let mut attributes = SmallVec::new();
            let mut remaining = selector;
            while let Some(open) = remaining.find('[') {
                remaining = &remaining[open + 1..];
                let Some(close) = remaining.find(']') else {
                    break;
                };
                let content = remaining[..close].trim();
                let name_end = content
                    .find(|character: char| {
                        character.is_whitespace()
                            || matches!(character, '=' | '~' | '|' | '^' | '$' | '*' | '!')
                    })
                    .unwrap_or(content.len());
                if let Some(name) = content.get(..name_end).filter(|name| !name.is_empty()) {
                    attributes.push(CompactString::from(name));
                }
                remaining = &remaining[close + 1..];
            }
            Some(ParsedSelector {
                element,
                attributes,
            })
        })
        .collect()
}

fn selector_values(parsed: &[ParsedSelector], selector_type: SelectorType) -> SmallVec<[&str; 8]> {
    match selector_type {
        SelectorType::Attribute => parsed
            .iter()
            .flat_map(|selector| selector.attributes.iter().map(CompactString::as_str))
            .collect(),
        SelectorType::Element => parsed
            .iter()
            .filter_map(|selector| selector.element.as_deref())
            .collect(),
    }
}

fn style_matches(selector: &str, style: SelectorStyle) -> bool {
    match style {
        SelectorStyle::CamelCase => {
            !selector.is_empty()
                && selector
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        }
        SelectorStyle::KebabCase => {
            !selector.is_empty()
                && selector.split('-').all(|part| {
                    !part.is_empty()
                        && part.chars().all(|character| {
                            character.is_ascii_lowercase() || character.is_ascii_digit()
                        })
                })
        }
    }
}

fn prefix_regex(prefix: &str) -> Option<Regex> {
    let mut pattern = CompactString::new("^(?:");
    pattern.push_str(prefix);
    pattern.push(')');
    Regex::new(pattern.as_str()).ok()
}

fn prefix_matches(
    selector: &str,
    prefix: &str,
    style: SelectorStyle,
    selector_type: SelectorType,
) -> bool {
    if prefix.is_empty() {
        return true;
    }
    let Some(found) = prefix_regex(prefix).and_then(|regex| regex.find(selector)) else {
        return false;
    };
    if selector_type == SelectorType::Attribute {
        return true;
    }
    let rest = &selector[found.end()..];
    match style {
        SelectorStyle::CamelCase => rest
            .chars()
            .next()
            .is_none_or(|character| character.is_ascii_uppercase()),
        SelectorStyle::KebabCase => rest.is_empty() || rest.starts_with('-'),
    }
}

fn selector_after_prefix(selector: &str, prefix: &str) -> bool {
    let Some(regex) = prefix_regex(prefix) else {
        return true;
    };
    let replaced = regex.replace(selector, "");
    !replaced.is_empty()
}

fn has_shadow_dom_encapsulation(metadata: &ObjectExpression<'_>) -> bool {
    metadata.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };
        if property_name(&property.key) != Some("encapsulation") {
            return false;
        }
        let Expression::StaticMemberExpression(member) = property.value.get_inner_expression()
        else {
            return false;
        };
        matches!(
            member.object.get_inner_expression(),
            Expression::Identifier(identifier) if identifier.name == "ViewEncapsulation"
        ) && member.property.name == "ShadowDom"
    })
}

fn human_readable_prefixes(prefixes: &[CompactString]) -> CompactString {
    let mut output = CompactString::new("");
    for (index, prefix) in prefixes.iter().enumerate() {
        if index > 0 {
            if index + 1 == prefixes.len() {
                output.push_str(" or ");
            } else {
                output.push_str(", ");
            }
        }
        output.push('"');
        output.push_str(prefix);
        output.push('"');
    }
    output
}

fn data(key: &str, value: &str) -> SmallVec<[DiagnosticDatum; 2]> {
    let mut output = SmallVec::new();
    output.push(DiagnosticDatum {
        key: CompactString::from(key),
        value: CompactString::from(value),
    });
    output
}

fn data2(
    first_key: &str,
    first_value: &str,
    second_key: &str,
    second_value: &str,
) -> SmallVec<[DiagnosticDatum; 2]> {
    SmallVec::from_buf([
        DiagnosticDatum {
            key: CompactString::from(first_key),
            value: CompactString::from(first_value),
        },
        DiagnosticDatum {
            key: CompactString::from(second_key),
            value: CompactString::from(second_value),
        },
    ])
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Authored option fixtures use serde_json::json arrays and Vec-shaped assertions to mirror the JavaScript ABI exactly."
)]
mod tests {
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use crate::{ScanOptions, scan_angular_eslint_with_options};

    fn scan(rule_name: &str, source: &str, option: Value) -> Vec<crate::Diagnostic> {
        let mut rule_names = SmallVec::new();
        rule_names.push(CompactString::from(rule_name));
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names,
                options: json!([option]),
            },
        )
        .into_vec()
    }

    #[test]
    fn accepts_the_upstream_selector_option_families() {
        let valid = [
            (
                "component-selector",
                "@Component({ selector: 'sg-foo-bar' }) class Test {}",
                json!({"type":"element","prefix":"sg","style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: '[ng-foo-bar]' }) class Test {}",
                json!({"type":"attribute","prefix":["app","ng"],"style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: 'app-foo-bar[baz].app' }) class Test {}",
                json!({"type":"element","prefix":["app","cd","ng"],"style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: 'appBarFoo' }) class Test {}",
                json!({"type":"element","prefix":"app","style":"camelCase"}),
            ),
            (
                "component-selector",
                "@Component({ selector: 'app1-foo-bar' }) class Test {}",
                json!({"type":"element","prefix":"app1","style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: '[appFooBar]' }) class Test {}",
                json!({"type":["attribute","element"],"prefix":["app","ng"],"style":"camelCase"}),
            ),
            (
                "component-selector",
                "@Component({ selector: `[appFooBar], [appBarFoo]` }) class Test {}",
                json!({"type":["attribute","element"],"prefix":["app","ng"],"style":"camelCase"}),
            ),
            (
                "component-selector",
                "@Component({ selector: `button[appFooBar]` }) class Test {}",
                json!({"type":["attribute","element"],"prefix":["app","ng"],"style":"camelCase"}),
            ),
            (
                "component-selector",
                "@Component({ selector: 'singleword' }) class Test {}",
                json!({"type":"element","prefix":"","style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: 'foo-bar' }) class Test {}",
                json!({"type":"element","prefix":[],"style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: 'app-foo-bar' }) class Test {}",
                json!([{"type":"element","prefix":"app","style":"kebab-case"}]),
            ),
            (
                "component-selector",
                "@Component({ selector: '[appFooBar]' }) class Test {}",
                json!([
                    {"type":"element","prefix":"app","style":"kebab-case"},
                    {"type":"attribute","prefix":"app","style":"camelCase"}
                ]),
            ),
            (
                "component-selector",
                "@Component({ selector: 'lib-foo-bar' }) class Test {}",
                json!([
                    {"type":"element","prefix":["app","lib"],"style":"kebab-case"},
                    {"type":"attribute","prefix":"app","style":"camelCase"}
                ]),
            ),
            (
                "component-selector",
                "@Component({ selector: 'app-foo-bar', encapsulation: ViewEncapsulation.ShadowDom }) class Test {}",
                json!({"type":"element","prefix":"app","style":"camelCase"}),
            ),
            (
                "directive-selector",
                "@Directive({ selector: '[appHighlight]' }) class Test {}",
                json!({"type":"attribute","prefix":"app","style":"camelCase"}),
            ),
            (
                "directive-selector",
                "@Directive({ selector: '[lib-highlight]' }) class Test {}",
                json!({"type":"attribute","prefix":["app","lib"],"style":"kebab-case"}),
            ),
            (
                "directive-selector",
                "@Directive({ selector: 'app-highlight' }) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
            ),
            (
                "directive-selector",
                "@Directive({ selector: '[highlight]' }) class Test {}",
                json!({"type":"attribute","style":"camelCase"}),
            ),
            (
                "component-selector",
                "@Directive({ selector: 'wrong-kind' }) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
            ),
            (
                "component-selector",
                "const selector = 'app-x'; @Component({ selector }) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({ selector: `app-${suffix}` }) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
            ),
            (
                "component-selector",
                "@Component({}) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
            ),
        ];

        for (rule_name, source, option) in valid {
            assert!(
                scan(rule_name, source, option).is_empty(),
                "expected valid {rule_name}: {source}",
            );
        }
    }

    #[test]
    fn reports_exact_selector_failures_and_message_data() {
        let invalid = [
            (
                "component-selector",
                "@Component({ selector: 'foo-bar' }) class Test {}",
                json!({"type":"element","prefix":"sg","style":"kebab-case"}),
                "prefixFailure",
                vec![("prefix", "\"sg\"")],
            ),
            (
                "component-selector",
                "@Component({ selector: '[app-foo-bar]' }) class Test {}",
                json!({"type":"attribute","prefix":["cd","ng"],"style":"kebab-case"}),
                "prefixFailure",
                vec![("prefix", "\"cd\" or \"ng\"")],
            ),
            (
                "component-selector",
                "@Component({ selector: 'app-foo-bar[baz].app' }) class Test {}",
                json!({"type":"element","prefix":["foo","cd","ng"],"style":"kebab-case"}),
                "prefixFailure",
                vec![("prefix", "\"foo\", \"cd\" or \"ng\"")],
            ),
            (
                "component-selector",
                "@Component({ selector: '[ng-bar-foo]' }) class Test {}",
                json!({"type":"attribute","prefix":"ng","style":"camelCase"}),
                "styleFailure",
                vec![("style", "camelCase")],
            ),
            (
                "component-selector",
                "@Component({ selector: 'appFooBar' }) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
                "styleAndPrefixFailure",
                vec![("style", "kebab-case"), ("prefix", "\"app\"")],
            ),
            (
                "component-selector",
                "@Component({ selector: 'app' }) class Test {}",
                json!({"type":"element","prefix":"app","style":"kebab-case"}),
                "selectorAfterPrefixFailure",
                vec![("prefix", "\"app\"")],
            ),
            (
                "component-selector",
                "@Component({ selector: '[appFooBar]' }) class Test {}",
                json!({"type":"element","prefix":["app","ng"],"style":"camelCase"}),
                "typeFailure",
                vec![("type", "element")],
            ),
            (
                "component-selector",
                "@Component({ selector: 'app-foo-bar' }) class Test {}",
                json!({"type":"attribute","prefix":["app","ng"],"style":"kebab-case"}),
                "typeFailure",
                vec![("type", "attribute")],
            ),
            (
                "component-selector",
                "@Component({ selector: 'appFooBar', encapsulation: ViewEncapsulation.ShadowDom }) class Test {}",
                json!({"type":"element","prefix":"app","style":"camelCase"}),
                "shadowDomEncapsulatedStyleFailure",
                vec![],
            ),
            (
                "directive-selector",
                "@Directive({ selector: '[fooHighlight]' }) class Test {}",
                json!({"type":"attribute","prefix":"app","style":"camelCase"}),
                "prefixFailure",
                vec![("prefix", "\"app\"")],
            ),
            (
                "directive-selector",
                "@Directive({ selector: '[app-highlight]' }) class Test {}",
                json!({"type":"attribute","prefix":"app","style":"camelCase"}),
                "styleFailure",
                vec![("style", "camelCase")],
            ),
            (
                "directive-selector",
                "@Directive({ selector: 'app-highlight' }) class Test {}",
                json!({"type":"attribute","prefix":"app","style":"kebab-case"}),
                "typeFailure",
                vec![("type", "attribute")],
            ),
            (
                "directive-selector",
                "@Directive({ selector: '[app]' }) class Test {}",
                json!({"type":"attribute","prefix":"app","style":"camelCase"}),
                "selectorAfterPrefixFailure",
                vec![("prefix", "\"app\"")],
            ),
        ];

        for (rule_name, source, option, message_id, expected_data) in invalid {
            let diagnostics = scan(rule_name, source, option);
            assert_eq!(diagnostics.len(), 1, "{rule_name}: {source}");
            let diagnostic = &diagnostics[0];
            assert_eq!(diagnostic.message_id, message_id, "{rule_name}: {source}");
            let actual_data: Vec<(&str, &str)> = diagnostic
                .data
                .iter()
                .map(|datum| (datum.key.as_str(), datum.value.as_str()))
                .collect();
            assert_eq!(actual_data, expected_data, "{rule_name}: {source}");
        }
    }

    #[test]
    fn keeps_rule_selection_locations_and_malformed_input_safe() {
        let option = json!({"type":"element","prefix":"app","style":"kebab-case"});
        assert!(
            scan(
                "directive-selector",
                "@Component({ selector: 'wrong' }) class Test {}",
                option.clone(),
            )
            .is_empty(),
        );
        assert!(
            scan(
                "component-selector",
                "@Component({ selector: 'wrong' ",
                option.clone(),
            )
            .is_empty(),
        );
        assert!(scan("component-selector", "const plain = 1;", option.clone()).is_empty(),);

        let diagnostics = scan(
            "component-selector",
            "const emoji = '😀';\n@Component({ selector: 'wrong-name' }) class Test {}",
            option,
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].loc.start_line, 2);
        assert_eq!(diagnostics[0].loc.start_column, 23);
        assert_eq!(diagnostics[0].loc.end_column, 35);
    }
}
