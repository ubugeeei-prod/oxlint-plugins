//! React-component and HOC recognition heuristics for the react-refresh
//! scanner.

use oxc_ast::ast::*;

use crate::helpers::{component_check_for_name, function_param_count};
use crate::scanner::Scanner;
use crate::{ComponentCheck, DEFAULT_HOCS};

impl Scanner<'_> {
    pub(crate) fn is_expression_react_component(
        &self,
        expression: &Expression<'_>,
    ) -> ComponentCheck {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => {
                component_check_for_name(identifier.name.as_str())
            }
            Expression::ArrowFunctionExpression(function) => {
                if function_param_count(&function.params) > 2 {
                    ComponentCheck::No
                } else {
                    ComponentCheck::NeedName
                }
            }
            Expression::FunctionExpression(function) => {
                if function_param_count(&function.params) > 2 {
                    ComponentCheck::No
                } else if let Some(id) = &function.id {
                    component_check_for_name(id.name.as_str())
                } else {
                    ComponentCheck::NeedName
                }
            }
            Expression::ConditionalExpression(expression) => {
                let consequent = self.is_expression_react_component(&expression.consequent);
                let alternate = self.is_expression_react_component(&expression.alternate);
                if consequent == ComponentCheck::No || alternate == ComponentCheck::No {
                    ComponentCheck::No
                } else if consequent == ComponentCheck::NeedName
                    || alternate == ComponentCheck::NeedName
                {
                    ComponentCheck::NeedName
                } else {
                    ComponentCheck::Yes
                }
            }
            Expression::CallExpression(call) => self.is_call_expression_react_component(call),
            Expression::TaggedTemplateExpression(tagged) => {
                if self.get_tagged_template_hoc_name(tagged).is_some() {
                    ComponentCheck::NeedName
                } else {
                    ComponentCheck::No
                }
            }
            _ => ComponentCheck::No,
        }
    }

    pub(crate) fn is_call_expression_react_component(
        &self,
        call: &CallExpression<'_>,
    ) -> ComponentCheck {
        let Some(hoc_name) = self.get_call_hoc_name(call) else {
            return ComponentCheck::No;
        };
        if !self.is_valid_hoc(hoc_name) {
            return ComponentCheck::No;
        }

        if hoc_name != "memo" && hoc_name != "forwardRef" {
            return ComponentCheck::Yes;
        }

        let Some(argument) = call.arguments.first() else {
            return ComponentCheck::No;
        };

        self.is_argument_react_component(argument)
    }

    fn is_argument_react_component(&self, argument: &Argument<'_>) -> ComponentCheck {
        match argument {
            Argument::Identifier(identifier) => component_check_for_name(identifier.name.as_str()),
            Argument::FunctionExpression(function) => {
                if let Some(id) = &function.id {
                    component_check_for_name(id.name.as_str())
                } else {
                    ComponentCheck::NeedName
                }
            }
            Argument::ArrowFunctionExpression(function) => {
                if function_param_count(&function.params) > 2 {
                    ComponentCheck::No
                } else {
                    ComponentCheck::NeedName
                }
            }
            Argument::CallExpression(call) => self.is_call_expression_react_component(call),
            Argument::TSAsExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSSatisfiesExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSTypeAssertion(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSNonNullExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            Argument::TSInstantiationExpression(expression) => {
                self.is_expression_react_component(&expression.expression)
            }
            _ => ComponentCheck::No,
        }
    }

    fn get_call_hoc_name<'a>(&self, call: &'a CallExpression<'a>) -> Option<&'a str> {
        self.get_hoc_name_from_expression(&call.callee)
    }

    fn get_tagged_template_hoc_name<'a>(
        &self,
        tagged: &'a TaggedTemplateExpression<'a>,
    ) -> Option<&'a str> {
        let name = self.get_hoc_name_from_expression(&tagged.tag)?;
        self.is_valid_hoc(name).then_some(name)
    }

    fn get_hoc_name_from_expression<'a>(&self, expression: &'a Expression<'a>) -> Option<&'a str> {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => self.get_call_hoc_name(call),
            Expression::StaticMemberExpression(member) => {
                let property_name = member.property.name.as_str();
                if self.is_valid_hoc(property_name) {
                    return Some(property_name);
                }
                if let Expression::Identifier(object) = member.object.get_inner_expression() {
                    let object_name = object.name.as_str();
                    if self.is_valid_hoc(object_name) {
                        return Some(object_name);
                    }
                }
                if let Expression::CallExpression(call) = member.object.get_inner_expression() {
                    return self.get_call_hoc_name(call);
                }
                None
            }
            Expression::Identifier(identifier) => Some(identifier.name.as_str()),
            _ => None,
        }
    }

    fn is_valid_hoc(&self, name: &str) -> bool {
        DEFAULT_HOCS.contains(&name) || self.options.extra_hocs.iter().any(|hoc| hoc == name)
    }
}
