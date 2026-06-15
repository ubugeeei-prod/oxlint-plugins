//! Rule `function-name` (SonarJS key S100).
//!
//! Clean-room port. Function-like declarations should follow the configured
//! naming convention. The default SonarJS convention is
//! `^[_a-z][a-zA-Z0-9]*$`: a function name starts with an underscore or ASCII
//! lowercase letter, followed by ASCII letters or digits.
//!
//! Observed behaviour reproduced here:
//! - function declarations are checked by their declared identifier;
//! - variable declarators initialized with a function or arrow are checked by
//!   the variable binding name;
//! - class/object methods are checked by their identifier key;
//! - object properties whose value is a function or arrow are checked by their
//!   identifier key;
//! - assignments, IIFEs, private names, and string/numeric keys are ignored.
//!
//! Behaviour is reproduced from public RSPEC documentation and black-box
//! observation only; no upstream source, tests, fixtures, or messages were
//! copied.

use compact_str::ToCompactString;
use oxc_ast::ast::{
    Class, ClassElement, Expression, Function, FunctionType, ObjectProperty, PropertyKey,
    VariableDeclarator,
};
use regex::Regex;

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "function-name";

const DEFAULT_FORMAT: &str = "^[_a-z][a-zA-Z0-9]*$";

impl Scanner<'_> {
    pub(crate) fn check_function_name_declaration(&mut self, function: &Function<'_>) {
        if !matches!(
            function.r#type,
            FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction
        ) {
            return;
        }
        let Some(id) = &function.id else {
            return;
        };
        self.report_bad_function_name(id.name.as_str(), id.span);
    }

    pub(crate) fn check_function_name_variable(&mut self, declarator: &VariableDeclarator<'_>) {
        let Some(init) = &declarator.init else {
            return;
        };
        if !is_function_like_expression(init) {
            return;
        }
        let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        self.report_bad_function_name(id.name.as_str(), id.span);
    }

    pub(crate) fn check_function_name_class(&mut self, class: &Class<'_>) {
        for element in &class.body.body {
            let ClassElement::MethodDefinition(method) = element else {
                continue;
            };
            self.check_function_name_property_key(&method.key);
        }
    }

    pub(crate) fn check_function_name_object_property(&mut self, property: &ObjectProperty<'_>) {
        if property.method || is_function_like_expression(&property.value) {
            self.check_function_name_property_key(&property.key);
        }
    }

    fn check_function_name_property_key(&mut self, key: &PropertyKey<'_>) {
        match key {
            PropertyKey::StaticIdentifier(id) => {
                self.report_bad_function_name(id.name.as_str(), id.span);
            }
            PropertyKey::Identifier(id) => {
                self.report_bad_function_name(id.name.as_str(), id.span);
            }
            _ => {}
        }
    }

    fn report_bad_function_name(&mut self, name: &str, span: oxc_span::Span) {
        let format = self.options.function_name_format.as_str();
        if matches_format(name, format) {
            return;
        }
        let data = DiagnosticData {
            value: Some(name.to_compact_string()),
            format: Some(format.to_compact_string()),
        };
        self.report_with_data(RULE_NAME, "renameFunction", data, span, None);
    }
}

fn is_function_like_expression(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
    )
}

fn matches_format(name: &str, format: &str) -> bool {
    if format == DEFAULT_FORMAT {
        return matches_default_format(name);
    }
    Regex::new(format).is_ok_and(|regex| regex.is_match(name))
}

fn matches_default_format(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_lowercase()) && chars.all(|ch| ch.is_ascii_alphanumeric())
}
