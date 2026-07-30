//! AST-backed port of Playwright's `no-hooks` rule.

use oxc_ast::ast::{
    CallExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier, MemberExpression,
    ModuleExportName, Program, match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{
    test_names::collect_test_names,
    types::{Diagnostic, DiagnosticData, HookAlias, LineIndex, PlaywrightOptions},
};

const RULE_NAME: &str = "no-hooks";
const HOOK_NAMES: [&str; 4] = ["afterAll", "afterEach", "beforeAll", "beforeEach"];

pub(crate) fn scan_no_hooks<'ast>(
    program: &Program<'ast>,
    source_text: &str,
    line_index: &LineIndex,
    options: &PlaywrightOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 64]>,
) {
    let test_names = collect_test_names(program, &options.test_aliases);
    let mut hook_aliases = options.hook_aliases.clone();
    HookImportCollector {
        hook_aliases: &mut hook_aliases,
    }
    .visit_program(program);

    NoHooksVisitor {
        diagnostics,
        hook_aliases,
        line_index,
        options,
        source_text,
        test_names,
    }
    .visit_program(program);
}

struct HookImportCollector<'storage> {
    hook_aliases: &'storage mut SmallVec<[HookAlias; 8]>,
}

impl<'ast> Visit<'ast> for HookImportCollector<'_> {
    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'ast>) {
        let Some(specifiers) = &declaration.specifiers else {
            return;
        };
        for specifier in specifiers {
            let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                continue;
            };
            let Some(imported) = module_export_name(&specifier.imported) else {
                continue;
            };
            if is_hook_name(imported) {
                push_alias(self.hook_aliases, specifier.local.name.as_str(), imported);
            }
        }
    }
}

struct NoHooksVisitor<'source, 'options, 'diagnostics> {
    diagnostics: &'diagnostics mut SmallVec<[Diagnostic; 64]>,
    hook_aliases: SmallVec<[HookAlias; 8]>,
    line_index: &'source LineIndex,
    options: &'options PlaywrightOptions,
    source_text: &'source str,
    test_names: SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for NoHooksVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        if let Some(hook_name) = hook_name(call, &self.test_names, &self.hook_aliases)
            && !self.options.allowed_hooks.contains(&hook_name)
        {
            self.diagnostics.push(Diagnostic {
                rule_name: RULE_NAME,
                message_id: "unexpectedHook",
                data: DiagnosticData {
                    hook_name: Some(hook_name),
                    ..DiagnosticData::default()
                },
                loc: self.line_index.loc_for_span(self.source_text, call.span),
                fix: None,
            });
        }
        walk::walk_call_expression(self, call);
    }
}

fn hook_name(
    call: &CallExpression<'_>,
    test_names: &[CompactString],
    hook_aliases: &[HookAlias],
) -> Option<CompactString> {
    let mut links = SmallVec::<[&str; 4]>::new();
    let root = callee_chain(&call.callee, &mut links)?;

    if test_names.iter().any(|name| name == root) {
        return match links.as_slice() {
            [name] if is_hook_name(name) => Some(CompactString::from(*name)),
            _ => None,
        };
    }
    if links.is_empty() {
        if is_hook_name(root) {
            return Some(CompactString::from(root));
        }
        return hook_aliases
            .iter()
            .find(|alias| alias.name == root)
            .map(|alias| alias.hook_name.clone());
    }
    None
}

fn callee_chain<'ast>(
    expression: &'ast Expression<'ast>,
    links: &mut SmallVec<[&'ast str; 4]>,
) -> Option<&'ast str> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        member @ match_member_expression!(Expression) => {
            let member = member.to_member_expression();
            let root = callee_chain(member.object(), links)?;
            links.push(member_name(member)?);
            Some(root)
        }
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

fn is_hook_name(name: &str) -> bool {
    HOOK_NAMES.contains(&name)
}

fn push_alias(aliases: &mut SmallVec<[HookAlias; 8]>, name: &str, hook_name: &str) {
    if aliases.iter().all(|alias| alias.name != name) {
        aliases.push(HookAlias {
            name: CompactString::from(name),
            hook_name: CompactString::from(hook_name),
        });
    }
}
