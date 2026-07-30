//! Option-aware ports of Playwright's three `no-restricted-*` rules.

use oxc_ast::ast::{
    Argument, CallExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier,
    MemberExpression, ModuleExportName, Program, match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::types::{Diagnostic, DiagnosticData, LineIndex, PlaywrightOptions, Restriction};

const LOCATORS_RULE: &str = "no-restricted-locators";
const MATCHERS_RULE: &str = "no-restricted-matchers";
const ROLES_RULE: &str = "no-restricted-roles";

pub(crate) fn scan_restricted_rules<'ast>(
    program: &Program<'ast>,
    source_text: &str,
    line_index: &LineIndex,
    options: &PlaywrightOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 64]>,
) {
    if options.restricted_locators.is_empty()
        && options.restricted_matchers.is_empty()
        && options.restricted_roles.is_empty()
    {
        return;
    }

    let mut expect_names = SmallVec::<[CompactString; 8]>::new();
    expect_names.push(CompactString::from("expect"));
    for alias in &options.expect_aliases {
        push_unique(&mut expect_names, alias.as_str());
    }
    ExpectImportCollector {
        expect_names: &mut expect_names,
    }
    .visit_program(program);
    let mut visitor = RestrictedVisitor {
        diagnostics,
        expect_names,
        line_index,
        options,
        source_text,
    };
    visitor.visit_program(program);
}

struct RestrictedVisitor<'source, 'options, 'diagnostics> {
    source_text: &'source str,
    line_index: &'source LineIndex,
    options: &'options PlaywrightOptions,
    diagnostics: &'diagnostics mut SmallVec<[Diagnostic; 64]>,
    expect_names: SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for RestrictedVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        self.check_restricted_locator(call);
        self.check_restricted_matchers(call);
        self.check_restricted_role(call);
        walk::walk_call_expression(self, call);
    }
}

struct ExpectImportCollector<'names> {
    expect_names: &'names mut SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for ExpectImportCollector<'_> {
    fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'ast>) {
        if declaration.source.value != "@playwright/test" {
            return;
        }
        let Some(specifiers) = &declaration.specifiers else {
            return;
        };
        for specifier in specifiers {
            let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                continue;
            };
            if module_export_name(&specifier.imported) == Some("expect") {
                push_unique(self.expect_names, specifier.local.name.as_str());
            }
        }
    }
}

impl RestrictedVisitor<'_, '_, '_> {
    fn check_restricted_locator(&mut self, call: &CallExpression<'_>) {
        let Some(member) = member_from_expression(&call.callee) else {
            return;
        };
        let Some(method) = member.static_property_name() else {
            return;
        };
        let Some(restriction) = last_restriction(&self.options.restricted_locators, method) else {
            return;
        };
        self.report(
            LOCATORS_RULE,
            restriction,
            call.span,
            DiagnosticSubject::Method(method),
        );
    }

    fn check_restricted_role(&mut self, call: &CallExpression<'_>) {
        let Some(member) = member_from_expression(&call.callee) else {
            return;
        };
        if member.static_property_name() != Some("getByRole") {
            return;
        }
        let Some(role) = call.arguments.first().and_then(static_argument_value) else {
            return;
        };
        let Some(restriction) = last_restriction(&self.options.restricted_roles, role) else {
            return;
        };
        self.report(
            ROLES_RULE,
            restriction,
            call.span,
            DiagnosticSubject::Role(role),
        );
    }

    fn check_restricted_matchers(&mut self, call: &CallExpression<'_>) {
        let Some(member) = member_from_expression(&call.callee) else {
            return;
        };
        let Some(chain) = matcher_chain(member, &self.expect_names) else {
            return;
        };

        for restriction in &self.options.restricted_matchers {
            let links = restriction
                .value
                .split('.')
                .filter(|link| !link.is_empty())
                .collect::<SmallVec<[&str; 4]>>();
            if links.is_empty() || links.len() > chain.len() {
                continue;
            }
            let Some(start) = chain.windows(links.len()).position(|window| {
                window
                    .iter()
                    .map(|link| link.name)
                    .eq(links.iter().copied())
            }) else {
                continue;
            };
            let matched = &chain[start..start + links.len()];
            let span = Span::new(matched[0].span.start, matched[matched.len() - 1].span.end);
            self.report(
                MATCHERS_RULE,
                restriction,
                span,
                DiagnosticSubject::Restriction(restriction.value.as_str()),
            );
        }
    }

    fn report(
        &mut self,
        rule_name: &'static str,
        restriction: &Restriction,
        span: Span,
        subject: DiagnosticSubject<'_>,
    ) {
        let custom_message = restriction
            .message
            .as_ref()
            .filter(|message| !message.is_empty());
        let mut data = DiagnosticData {
            message: custom_message.cloned().unwrap_or_default(),
            ..DiagnosticData::default()
        };
        match subject {
            DiagnosticSubject::Method(method) => {
                data.method = Some(CompactString::from(method));
            }
            DiagnosticSubject::Restriction(value) => {
                data.restriction = Some(CompactString::from(value));
            }
            DiagnosticSubject::Role(role) => {
                data.role = Some(CompactString::from(role));
            }
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id: if custom_message.is_some() {
                "restrictedWithMessage"
            } else {
                "restricted"
            },
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

enum DiagnosticSubject<'a> {
    Method(&'a str),
    Restriction(&'a str),
    Role(&'a str),
}

#[derive(Clone, Copy)]
struct ChainLink<'a> {
    name: &'a str,
    span: Span,
}

fn matcher_chain<'ast>(
    member: &'ast MemberExpression<'ast>,
    expect_names: &[CompactString],
) -> Option<SmallVec<[ChainLink<'ast>; 8]>> {
    let mut reversed = SmallVec::<[ChainLink<'ast>; 8]>::new();
    let mut current = member;
    loop {
        reversed.push(member_link(current)?);
        match current.object().get_inner_expression() {
            Expression::CallExpression(call) => {
                if is_expect_call(call, expect_names) {
                    break;
                }
                current = member_from_expression(&call.callee)?;
            }
            expression => {
                current = member_from_expression(expression)?;
            }
        }
    }
    reversed.reverse();
    Some(reversed)
}

fn is_expect_call(call: &CallExpression<'_>, expect_names: &[CompactString]) -> bool {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) => contains_name(expect_names, identifier.name.as_str()),
        member @ match_member_expression!(Expression) => {
            let member = member.to_member_expression();
            matches!(member.static_property_name(), Some("poll" | "soft"))
                && matches!(
                    member.object().get_inner_expression(),
                    Expression::Identifier(identifier)
                        if contains_name(expect_names, identifier.name.as_str())
                )
        }
        _ => false,
    }
}

fn member_link<'ast>(member: &'ast MemberExpression<'ast>) -> Option<ChainLink<'ast>> {
    match member {
        MemberExpression::StaticMemberExpression(member) => Some(ChainLink {
            name: member.property.name.as_str(),
            span: member.property.span,
        }),
        MemberExpression::ComputedMemberExpression(member) => {
            let expression = member.expression.get_inner_expression();
            Some(ChainLink {
                name: static_expression_value(expression)?,
                span: expression.span(),
            })
        }
        MemberExpression::PrivateFieldExpression(_) => None,
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

fn static_argument_value<'ast>(argument: &'ast Argument<'ast>) -> Option<&'ast str> {
    static_expression_value(argument.as_expression()?.get_inner_expression())
}

fn static_expression_value<'ast>(expression: &'ast Expression<'ast>) -> Option<&'ast str> {
    match expression {
        Expression::StringLiteral(literal) => Some(literal.value.as_str()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .map(|value| value.as_str()),
        _ => None,
    }
}

fn module_export_name<'ast>(name: &'ast ModuleExportName<'ast>) -> Option<&'ast str> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::StringLiteral(literal) => Some(literal.value.as_str()),
    }
}

fn last_restriction<'a>(restrictions: &'a [Restriction], value: &str) -> Option<&'a Restriction> {
    restrictions
        .iter()
        .rev()
        .find(|restriction| restriction.value == value)
}

fn contains_name(names: &[CompactString], value: &str) -> bool {
    names.iter().any(|name| name == value)
}

fn push_unique(names: &mut SmallVec<[CompactString; 8]>, value: &str) {
    if !contains_name(names, value) {
        names.push(CompactString::from(value));
    }
}
