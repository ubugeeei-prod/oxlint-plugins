use oxc_ast::ast::{
    CallExpression, Class, ClassElement, Decorator, Expression, MethodDefinitionKind,
    ObjectExpression, ObjectPropertyKind, PropertyKey,
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::{Diagnostic, DiagnosticDatum};

const NO_INPUT_PREFIX: &str = "no-input-prefix";
const PIPE_PREFIX: &str = "pipe-prefix";

impl Scanner<'_> {
    pub(crate) fn check_prefix_rules(&mut self, class: &Class<'_>) {
        if self.options.is_enabled(NO_INPUT_PREFIX) {
            let prefixes = configured_prefixes(&self.options.options);
            if !prefixes.is_empty() {
                self.check_input_members(class, &prefixes);
                self.check_inputs_metadata(class, &prefixes);
            }
        }
        if self.options.is_enabled(PIPE_PREFIX) {
            let prefixes = configured_prefixes(&self.options.options);
            if !prefixes.is_empty() {
                self.check_pipe_prefix(class, &prefixes);
            }
        }
    }

    fn check_input_members(&mut self, class: &Class<'_>, prefixes: &[CompactString]) {
        for element in &class.body.body {
            match element {
                ClassElement::PropertyDefinition(property) => {
                    self.check_input_member(&property.decorators, &property.key, prefixes);
                }
                ClassElement::MethodDefinition(method)
                    if method.kind == MethodDefinitionKind::Set =>
                {
                    self.check_input_member(&method.decorators, &method.key, prefixes);
                }
                ClassElement::AccessorProperty(property) => {
                    self.check_input_member(&property.decorators, &property.key, prefixes);
                }
                _ => {}
            }
        }
    }

    fn check_input_member(
        &mut self,
        decorators: &[Decorator<'_>],
        key: &PropertyKey<'_>,
        prefixes: &[CompactString],
    ) {
        let Some(input_call) = input_decorator(decorators) else {
            return;
        };
        if let Some((property_name, property_span)) = property_key_value(key)
            && has_disallowed_prefix(prefixes, property_name)
        {
            self.report_prefix(NO_INPUT_PREFIX, "noInputPrefix", prefixes, property_span);
        }
        let Some(call) = input_call else {
            return;
        };
        let Some((alias, alias_span)) = input_alias(call) else {
            return;
        };
        if has_disallowed_prefix(prefixes, alias) {
            self.report_prefix(NO_INPUT_PREFIX, "noInputPrefix", prefixes, alias_span);
        }
    }

    fn check_inputs_metadata(&mut self, class: &Class<'_>, prefixes: &[CompactString]) {
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
                if property_name(&property.key) != Some("inputs") {
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
                        .filter(|character| !character.is_whitespace())
                        .collect();
                    let mut names = normalized.split(':');
                    let property_name = names.next().unwrap_or_default();
                    let alias_name = names.next().unwrap_or_default();
                    if has_disallowed_prefix(prefixes, property_name)
                        || has_disallowed_prefix(prefixes, alias_name)
                    {
                        self.report_prefix(NO_INPUT_PREFIX, "noInputPrefix", prefixes, span);
                    }
                }
            }
        }
    }

    fn check_pipe_prefix(&mut self, class: &Class<'_>, prefixes: &[CompactString]) {
        for decorator in &class.decorators {
            let Some(call) = called_decorator(decorator, &["Pipe"]) else {
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
            let Some((name, span)) = object_static_string(metadata, "name") else {
                continue;
            };
            let matches: SmallVec<[usize; 4]> = prefixes
                .iter()
                .filter_map(|prefix| pipe_prefix_end(name, prefix))
                .collect();
            if matches.is_empty() {
                self.report_prefix(PIPE_PREFIX, "pipePrefix", prefixes, span);
            } else if matches.iter().all(|end| *end == name.len()) {
                self.report_prefix(PIPE_PREFIX, "selectorAfterPrefixFailure", prefixes, span);
            }
        }
    }

    fn report_prefix(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        prefixes: &[CompactString],
        span: Span,
    ) {
        let mut data = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("prefixes"),
            value: human_readable_prefixes(prefixes),
        });
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

fn configured_prefixes(options: &Value) -> SmallVec<[CompactString; 4]> {
    options
        .as_array()
        .and_then(|options| options.first())
        .and_then(Value::as_object)
        .and_then(|option| option.get("prefixes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(CompactString::from)
        .collect()
}

fn input_decorator<'a>(decorators: &'a [Decorator<'a>]) -> Option<Option<&'a CallExpression<'a>>> {
    decorators.iter().find_map(
        |decorator| match decorator.expression.get_inner_expression() {
            Expression::Identifier(identifier) if identifier.name == "Input" => Some(None),
            Expression::CallExpression(call)
                if matches!(
                    call.callee.get_inner_expression(),
                    Expression::Identifier(identifier) if identifier.name == "Input"
                ) =>
            {
                Some(Some(call.as_ref()))
            }
            _ => None,
        },
    )
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

fn input_alias<'a>(call: &'a CallExpression<'a>) -> Option<(&'a str, Span)> {
    let argument = call.arguments.first()?.as_expression()?;
    if let Some(value) = static_string(argument) {
        return Some(value);
    }
    let Expression::ObjectExpression(metadata) = argument.get_inner_expression() else {
        return None;
    };
    object_static_string(metadata, "alias")
}

fn object_static_string<'a>(
    object: &'a ObjectExpression<'a>,
    name: &str,
) -> Option<(&'a str, Span)> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        (property_name(&property.key) == Some(name))
            .then(|| static_string(&property.value))
            .flatten()
    })
}

fn property_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        PropertyKey::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .map(|quasi| quasi.value.raw.as_str()),
        _ => None,
    }
}

fn property_key_value<'a>(key: &'a PropertyKey<'a>) -> Option<(&'a str, Span)> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => {
            Some((identifier.name.as_str(), identifier.span))
        }
        PropertyKey::StringLiteral(literal) => Some((literal.value.as_str(), literal.span)),
        PropertyKey::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .map(|quasi| (quasi.value.raw.as_str(), template.span)),
        _ => None,
    }
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

fn has_disallowed_prefix(prefixes: &[CompactString], name: &str) -> bool {
    prefixes
        .iter()
        .any(|prefix| is_disallowed_prefix(prefix, name))
}

fn is_disallowed_prefix(prefix: &str, name: &str) -> bool {
    let mut pattern = CompactString::new("^(?:");
    pattern.push_str(prefix);
    pattern.push_str(")(?:[^a-z]|$)");
    Regex::new(pattern.as_str())
        .ok()
        .is_some_and(|regex| regex.is_match(name))
}

fn pipe_prefix_end(name: &str, prefix: &str) -> Option<usize> {
    let mut pattern = CompactString::new("^(?:");
    pattern.push_str(prefix);
    pattern.push(')');
    let found = Regex::new(pattern.as_str()).ok()?.find(name)?;
    let rest = &name[found.end()..];
    (rest
        .chars()
        .next()
        .is_none_or(|character| character.is_ascii_uppercase()))
    .then_some(found.end())
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

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Pinned upstream option cases use serde_json::json and Vec-shaped assertions to mirror the JavaScript ABI exactly."
)]
mod tests {
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use super::{NO_INPUT_PREFIX, PIPE_PREFIX};
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
    fn ports_the_upstream_no_input_prefix_valid_cases() {
        let options = json!([{"prefixes":["on"]}]);
        for source in [
            "class Test {}",
            "@Page({ inputs: ['on', onChange, `onLine`, 'on: on2', 'offline: on', ...onCheck, onInput()] }) class Test {}",
            "@Component() class Test { on = new EventEmitter(); }",
            "@Directive() class Test { @Input() buttonChange = new EventEmitter<'on'>(); }",
            "@Component() class Test { @Input() On = new EventEmitter<{ on: onType }>(); }",
            "@Directive() class Test { @Input(`one`) ontype = new EventEmitter(); }",
            "@Directive() class Test { @Input({ alias: `one` }) ontype = new EventEmitter(); }",
            "@Component() class Test { @Input('oneProp') common = new EventEmitter(); }",
            "@Component() class Test { @Input({ alias: 'oneProp' }) common = new EventEmitter(); }",
            "@Directive() class Test<On> { @Input() ON = new EventEmitter<On>(); }",
            "const on = 'on'; @Component() class Test { @Input(on) touchMove = new EventEmitter(); }",
            "const test = 'on'; const on = 'on'; @Directive() class Test { @Input(test) [on]: EventEmitter<OnTest>; }",
            "@Component() class Test { @Input() notOn: string = 'on'; }",
            "@Component({ selector:'foo', 'inputs': [`test: foo`] }) class Test {}",
            "@Directive({ selector:'foo', ['inputs']: [`test: foo`] }) class Test {}",
            "@Component({ selector:'foo', [`inputs`]: [`test: foo`] }) class Test {}",
            "@Directive({ selector:'foo' }) class Test { @Input() set 'setter'(_) {} }",
        ] {
            assert!(
                scan(NO_INPUT_PREFIX, source, options.clone()).is_empty(),
                "{source}"
            );
        }
        assert!(scan(NO_INPUT_PREFIX, "@Input() on: string;", Value::Null).is_empty());
    }

    #[test]
    fn ports_the_upstream_no_input_prefix_invalid_cases() {
        let cases = [
            (
                "@Component({ inputs: ['on'] }) class Test {}",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Directive({ 'inputs': [onLevel, `test: on`, onFunction()] }) class Test {}",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Component({ ['inputs']: ['onTest: test'] }) class Test {}",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Directive({ [`inputs`]: ['onTest: test'] }) class Test {}",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Component() class Test { @Input() on: EventEmitter<any>; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Directive() class Test { @Input() @Custom('on') 'onPrefix' = value; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Component() class Test { @Custom() @Input(`on`) _on = value; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Component() class Test { @Input({ required:true, alias:`on` }) _on = value; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Directive() class Test { @Input('onPrefix') _on = value; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Directive() class Test { @Input({ alias:'onPrefix', required:true }) _on = value; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Component() class Test { @Input('setter') set 'on-setter'(_) {} }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Directive() class Test { @Input(`onSetter`) set setter(_) {} }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Injectable() class Test { @Input('on') isPrefix = value; }",
                json!([{"prefixes":["on","is","should"]}]),
                2,
                "\"on\", \"is\" or \"should\"",
            ),
            (
                "@Component() class Test { @Input() on: string = 'on'; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
            (
                "@Injectable() class Test { @Input('on') isPrefix = `on`; }",
                json!([{"prefixes":["on"]}]),
                1,
                "\"on\"",
            ),
        ];
        for (source, options, count, prefixes) in cases {
            let diagnostics = scan(NO_INPUT_PREFIX, source, options);
            assert_eq!(diagnostics.len(), count, "{source}");
            assert!(diagnostics.iter().all(|diagnostic| {
                diagnostic.message_id == "noInputPrefix"
                    && diagnostic.data[0].key == "prefixes"
                    && diagnostic.data[0].value == prefixes
            }));
        }
    }

    #[test]
    fn ports_all_upstream_pipe_prefix_cases_and_edges() {
        let valid = [
            ("@Pipe class Test {}", json!([{"prefixes":["ng"]}])),
            ("@Pipe({}) class Test {}", json!([{"prefixes":["ng"]}])),
            (
                "@Pipe({ name }) class MockPipe {}",
                json!([{"prefixes":["ng"]}]),
            ),
            (
                "@Pipe({ name:'ngBarFoo' }) class Test {}",
                json!([{"prefixes":[]}]),
            ),
            (
                "@Pipe({ name:'ngBarFoo' }) class Test {}",
                json!([{"prefixes":["ng"]}]),
            ),
            (
                "@Pipe({ name:'ngBarFoo' }) class Test {}",
                json!([{"prefixes":["ng","sg","mg"]}]),
            ),
            (
                "@Pipe({ name:`ngBarFoo` }) class Test {}",
                json!([{"prefixes":["ng","sg","mg"]}]),
            ),
            ("class Test {}", json!([{"prefixes":["ng"]}])),
        ];
        for (source, options) in valid {
            assert!(scan(PIPE_PREFIX, source, options).is_empty(), "{source}");
        }
        let invalid = [
            (
                "@Pipe({ name:'foo-bar' }) class Test {}",
                json!([{"prefixes":["ng"]}]),
                "pipePrefix",
                "\"ng\"",
            ),
            (
                "@Pipe({ name:'ng' }) class Test {}",
                json!([{"prefixes":["ng"]}]),
                "selectorAfterPrefixFailure",
                "\"ng\"",
            ),
            (
                "@Pipe({ name:'foo-bar' }) class Test {}",
                json!([{"prefixes":["ng","mg","sg"]}]),
                "pipePrefix",
                "\"ng\", \"mg\" or \"sg\"",
            ),
        ];
        for (source, options, message_id, prefixes) in invalid {
            let diagnostics = scan(PIPE_PREFIX, source, options);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message_id, message_id);
            assert_eq!(diagnostics[0].data[0].value, prefixes);
        }

        let utf16 = scan(
            PIPE_PREFIX,
            "const emoji='😀';\n@Pipe({ name:'bad' }) class Test {}",
            json!([{"prefixes":["ng"]}]),
        );
        assert_eq!(utf16[0].loc.start_line, 2);
        assert_eq!(utf16[0].loc.start_column, 13);
        assert!(
            scan(
                PIPE_PREFIX,
                "@Pipe({ name:'bad' }) class Test {}",
                Value::Null
            )
            .is_empty()
        );
    }
}
