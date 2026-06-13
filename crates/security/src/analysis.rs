//! Static-expression / import-access-path analysis and binding-pattern walks
//! for the security scanner.

use oxc_ast::ast::*;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{is_import_meta_url, property_key_name};
use crate::scanner::Scanner;
use crate::{
    AccessPath, Binding, PATH_CONSTRUCTION_METHODS, PATH_PACKAGES, PATH_STATIC_MEMBERS,
    URL_PACKAGES,
};

impl<'a> Scanner<'a> {
    pub(crate) fn require_package_name(&self, call: &'a CallExpression<'a>) -> Option<&'a str> {
        if !call.callee.is_specific_id("require") {
            return None;
        }
        let Some(Expression::StringLiteral(literal)) =
            call.arguments.first().and_then(Argument::as_expression)
        else {
            return None;
        };
        Some(literal.value.as_str())
    }

    pub(crate) fn import_access_path(
        &self,
        expression: &'a Expression<'a>,
        package_names: &[&str],
    ) -> Option<AccessPath> {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => match self.lookup(identifier.name.as_str()) {
                Some(Binding::Import(path))
                    if package_names.contains(&path.package_name.as_str()) =>
                {
                    Some(path.clone())
                }
                _ => None,
            },
            Expression::StaticMemberExpression(member) => {
                let mut path = self.import_access_path(&member.object, package_names)?;
                path.path
                    .push(CompactString::from(member.property.name.as_str()));
                Some(path)
            }
            Expression::CallExpression(call) => {
                let package_name = self.require_package_name(call)?;
                if !package_names.contains(&package_name) {
                    return None;
                }
                Some(AccessPath {
                    package_name: CompactString::from(package_name),
                    path: SmallVec::new(),
                })
            }
            _ => None,
        }
    }

    pub(crate) fn is_static_expression(
        &self,
        expression: &'a Expression<'a>,
        depth: usize,
    ) -> bool {
        if depth > 32 {
            return false;
        }

        match expression.get_inner_expression() {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(template) => template
                .expressions
                .iter()
                .all(|expression| self.is_static_expression(expression, depth + 1)),
            Expression::BinaryExpression(binary) => {
                self.is_static_expression(&binary.left, depth + 1)
                    && self.is_static_expression(&binary.right, depth + 1)
            }
            Expression::Identifier(identifier) => match identifier.name.as_str() {
                "__dirname" | "__filename" => true,
                name => matches!(self.lookup(name), Some(Binding::Static)),
            },
            Expression::StaticMemberExpression(member) => {
                is_import_meta_url(expression)
                    || self.is_static_path_member(member.property.name.as_str(), &member.object)
            }
            Expression::CallExpression(call) => {
                self.is_static_path_call(call, depth)
                    || self.is_static_file_url_to_path(call, depth)
                    || self.is_static_require_resolve(call, depth)
                    || self.is_static_process_cwd(call)
            }
            _ => false,
        }
    }

    fn is_static_path_member(&self, name: &str, object: &'a Expression<'a>) -> bool {
        if !PATH_STATIC_MEMBERS.contains(&name) {
            return false;
        }
        self.import_access_path(object, &PATH_PACKAGES).is_some()
    }

    fn is_static_path_call(&self, call: &'a CallExpression<'a>, depth: usize) -> bool {
        let Some(path) = self.import_access_path(&call.callee, &PATH_PACKAGES) else {
            return false;
        };
        let method = match path.path.as_slice() {
            [name] => name.as_str(),
            [namespace, name] if namespace.as_str() == "posix" => name.as_str(),
            _ => return false,
        };
        PATH_CONSTRUCTION_METHODS.contains(&method)
            && !call.arguments.is_empty()
            && call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expression| self.is_static_expression(expression, depth + 1))
            })
    }

    fn is_static_file_url_to_path(&self, call: &'a CallExpression<'a>, depth: usize) -> bool {
        let Some(path) = self.import_access_path(&call.callee, &URL_PACKAGES) else {
            return false;
        };
        matches!(path.path.as_slice(), [name] if name.as_str() == "fileURLToPath")
            && !call.arguments.is_empty()
            && call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expression| self.is_static_expression(expression, depth + 1))
            })
    }

    fn is_static_require_resolve(&self, call: &'a CallExpression<'a>, depth: usize) -> bool {
        if !call.callee.is_specific_member_access("require", "resolve") {
            return false;
        }
        if matches!(
            self.lookup("require"),
            Some(Binding::Unknown | Binding::Import(_))
        ) {
            return false;
        }
        !call.arguments.is_empty()
            && call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expression| self.is_static_expression(expression, depth + 1))
            })
    }

    fn is_static_process_cwd(&self, call: &'a CallExpression<'a>) -> bool {
        call.callee.is_specific_member_access("process", "cwd")
            && !matches!(
                self.lookup("process"),
                Some(Binding::Unknown | Binding::Import(_))
            )
    }

    pub(crate) fn bind_pattern_from_import(
        &mut self,
        pattern: &'a BindingPattern<'a>,
        path: &AccessPath,
    ) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                self.bind(identifier.name.as_str(), Binding::Import(path.clone()));
            }
            BindingPattern::ObjectPattern(pattern) => {
                for property in &pattern.properties {
                    self.bind_object_property_from_import(property, path);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::ArrayPattern(pattern) => {
                for element in pattern.elements.iter().flatten() {
                    self.bind_pattern_unknown(element);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::AssignmentPattern(pattern) => {
                self.bind_pattern_from_import(&pattern.left, path);
            }
        }
    }

    fn bind_object_property_from_import(
        &mut self,
        property: &'a BindingProperty<'a>,
        base: &AccessPath,
    ) {
        let Some(key_name) = property_key_name(&property.key) else {
            self.bind_pattern_unknown(&property.value);
            return;
        };
        let mut path = base.clone();
        path.path.push(CompactString::from(key_name));
        self.bind_pattern_from_import(&property.value, &path);
    }

    pub(crate) fn bind_pattern_static_or_unknown(
        &mut self,
        pattern: &'a BindingPattern<'a>,
        is_static: bool,
    ) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                self.bind(
                    identifier.name.as_str(),
                    if is_static {
                        Binding::Static
                    } else {
                        Binding::Unknown
                    },
                );
            }
            BindingPattern::ObjectPattern(pattern) => {
                for property in &pattern.properties {
                    self.bind_pattern_static_or_unknown(&property.value, false);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::ArrayPattern(pattern) => {
                for element in pattern.elements.iter().flatten() {
                    self.bind_pattern_static_or_unknown(element, false);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::AssignmentPattern(pattern) => {
                self.bind_pattern_static_or_unknown(&pattern.left, false);
            }
        }
    }

    pub(crate) fn bind_pattern_unknown(&mut self, pattern: &'a BindingPattern<'a>) {
        self.bind_pattern_static_or_unknown(pattern, false);
    }
}
