//! Shared discovery of Playwright `test` imports and `test.extend()` aliases.

use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier,
    MemberExpression, ModuleExportName, Program, VariableDeclarator, match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub(crate) fn collect_test_names<'ast>(
    program: &Program<'ast>,
    configured_aliases: &[CompactString],
) -> SmallVec<[CompactString; 8]> {
    let mut test_names = SmallVec::<[CompactString; 8]>::new();
    test_names.push(CompactString::from("test"));
    for alias in configured_aliases {
        push_unique(&mut test_names, alias.as_str());
    }

    let mut extend_declarations = SmallVec::<[(CompactString, CompactString); 16]>::new();
    NameCollector {
        extend_declarations: &mut extend_declarations,
        test_names: &mut test_names,
    }
    .visit_program(program);
    resolve_extend_aliases(&mut test_names, &extend_declarations);
    test_names
}

struct NameCollector<'storage> {
    extend_declarations: &'storage mut SmallVec<[(CompactString, CompactString); 16]>,
    test_names: &'storage mut SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for NameCollector<'_> {
    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'ast>) {
        let Some(specifiers) = &declaration.specifiers else {
            return;
        };
        for specifier in specifiers {
            let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                continue;
            };
            if module_export_name(&specifier.imported) == Some("test") {
                push_unique(self.test_names, specifier.local.name.as_str());
            }
        }
    }

    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'ast>) {
        if let (BindingPattern::BindingIdentifier(identifier), Some(initializer)) =
            (&declaration.id, &declaration.init)
            && let Some(root) = test_extend_root(initializer)
        {
            self.extend_declarations.push((
                CompactString::from(identifier.name.as_str()),
                CompactString::from(root),
            ));
        }
        walk::walk_variable_declarator(self, declaration);
    }
}

fn resolve_extend_aliases(
    test_names: &mut SmallVec<[CompactString; 8]>,
    declarations: &[(CompactString, CompactString)],
) {
    loop {
        let mut changed = false;
        for (name, root) in declarations {
            if contains_name(test_names, root.as_str()) && !contains_name(test_names, name.as_str())
            {
                test_names.push(name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn test_extend_root<'ast>(expression: &'ast Expression<'ast>) -> Option<&'ast str> {
    let Expression::CallExpression(call) = expression.get_inner_expression() else {
        return None;
    };
    let member = member_from_expression(&call.callee)?;
    if member_name(member) != Some("extend") {
        return None;
    }
    match member.object().get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        Expression::CallExpression(call) => test_extend_root_from_call(call),
        _ => None,
    }
}

fn test_extend_root_from_call<'ast>(call: &'ast CallExpression<'ast>) -> Option<&'ast str> {
    let member = member_from_expression(&call.callee)?;
    if member_name(member) != Some("extend") {
        return None;
    }
    match member.object().get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        Expression::CallExpression(call) => test_extend_root_from_call(call),
        _ => None,
    }
}

fn member_from_expression<'ast>(
    expression: &'ast Expression<'ast>,
) -> Option<&'ast MemberExpression<'ast>> {
    match expression.get_inner_expression() {
        member @ match_member_expression!(Expression) => Some(member.to_member_expression()),
        _ => None,
    }
}

fn member_name<'ast>(member: &'ast MemberExpression<'ast>) -> Option<&'ast str> {
    if let Some(name) = member.static_property_name() {
        return Some(name);
    }
    match member {
        MemberExpression::ComputedMemberExpression(member) => {
            match member.expression.get_inner_expression() {
                Expression::Identifier(identifier) => Some(identifier.name.as_str()),
                Expression::StringLiteral(literal) => Some(literal.value.as_str()),
                Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                    template
                        .quasis
                        .first()
                        .and_then(|quasi| quasi.value.cooked.as_ref())
                        .map(|value| value.as_str())
                }
                _ => None,
            }
        }
        MemberExpression::StaticMemberExpression(_)
        | MemberExpression::PrivateFieldExpression(_) => None,
    }
}

fn module_export_name<'ast>(name: &'ast ModuleExportName<'ast>) -> Option<&'ast str> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::StringLiteral(literal) => Some(literal.value.as_str()),
    }
}

fn contains_name(names: &[CompactString], value: &str) -> bool {
    names.iter().any(|name| name == value)
}

fn push_unique(names: &mut SmallVec<[CompactString; 8]>, value: &str) {
    if !contains_name(names, value) {
        names.push(CompactString::from(value));
    }
}
