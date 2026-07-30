//! AST-backed port of Playwright's `prefer-lowercase-title` rule.

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, ImportDeclaration,
    ImportDeclarationSpecifier, MemberExpression, ModuleExportName, Program, VariableDeclarator,
    match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{
    thresholds::{PlaywrightCall, classify_call},
    types::{Diagnostic, DiagnosticData, DiagnosticFix, LineIndex, PlaywrightOptions},
};

const RULE_NAME: &str = "prefer-lowercase-title";
const TEST_METHOD: &str = "test";
const DESCRIBE_METHOD: &str = "test.describe";

pub(crate) fn scan_prefer_lowercase_title<'ast>(
    program: &Program<'ast>,
    source_text: &str,
    line_index: &LineIndex,
    options: &PlaywrightOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 64]>,
) {
    let mut test_names = SmallVec::<[CompactString; 8]>::new();
    test_names.push(CompactString::from("test"));
    for alias in &options.test_aliases {
        push_unique(&mut test_names, alias.as_str());
    }

    let mut extend_declarations = SmallVec::<[(CompactString, CompactString); 16]>::new();
    NameCollector {
        extend_declarations: &mut extend_declarations,
        test_names: &mut test_names,
    }
    .visit_program(program);
    resolve_extend_aliases(&mut test_names, &extend_declarations);

    PreferLowercaseTitleVisitor {
        describe_depth: 0,
        diagnostics,
        line_index,
        options,
        source_text,
        test_names,
    }
    .visit_program(program);
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

struct PreferLowercaseTitleVisitor<'source, 'options, 'diagnostics> {
    describe_depth: usize,
    diagnostics: &'diagnostics mut SmallVec<[Diagnostic; 64]>,
    line_index: &'source LineIndex,
    options: &'options PlaywrightOptions,
    source_text: &'source str,
    test_names: SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for PreferLowercaseTitleVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        match classify_call(call, &self.test_names) {
            Some(PlaywrightCall::Describe) => {
                self.describe_depth += 1;
                if !self.options.ignore_top_level_describe || self.describe_depth != 1 {
                    self.check_title(call, DESCRIBE_METHOD);
                }
                walk::walk_call_expression(self, call);
                self.describe_depth -= 1;
            }
            Some(PlaywrightCall::Test) => {
                self.check_title(call, TEST_METHOD);
                walk::walk_call_expression(self, call);
            }
            Some(PlaywrightCall::Hook) | None => walk::walk_call_expression(self, call),
        }
    }
}

impl PreferLowercaseTitleVisitor<'_, '_, '_> {
    fn check_title(&mut self, call: &CallExpression<'_>, method: &'static str) {
        if self
            .options
            .lowercase_title_ignored_methods
            .iter()
            .any(|ignored| ignored == method)
        {
            return;
        }

        let Some(title) = call.arguments.first().and_then(title_value) else {
            return;
        };
        if title.description.is_empty()
            || self
                .options
                .allowed_title_prefixes
                .iter()
                .any(|prefix| title.description.starts_with(prefix.as_str()))
        {
            return;
        }

        let Some(replacement) = lowercase_first_utf16_code_unit(title.description) else {
            return;
        };
        self.diagnostics.push(Diagnostic {
            rule_name: RULE_NAME,
            message_id: "unexpectedLowercase",
            data: DiagnosticData {
                method: Some(CompactString::from(method)),
                ..DiagnosticData::default()
            },
            loc: self.line_index.loc_for_span(self.source_text, title.span),
            fix: Some(DiagnosticFix {
                start: title.span.start + 1,
                end: title.span.end.saturating_sub(1),
                replacement,
            }),
        });
    }
}

struct TitleValue<'ast> {
    description: &'ast str,
    span: Span,
}

fn title_value<'ast>(argument: &'ast Argument<'ast>) -> Option<TitleValue<'ast>> {
    match argument {
        Argument::StringLiteral(literal) => Some(TitleValue {
            description: literal.value.as_str(),
            span: literal.span,
        }),
        Argument::TemplateLiteral(template) if template.expressions.is_empty() => {
            template.quasis.first().map(|quasi| TitleValue {
                // Upstream `getStringValue` intentionally uses the raw template
                // value, unlike decoded string literals.
                description: quasi.value.raw.as_str(),
                span: template.span,
            })
        }
        _ => None,
    }
}

fn lowercase_first_utf16_code_unit(description: &str) -> Option<CompactString> {
    let first = description.chars().next()?;

    // JavaScript's `charAt(0)` returns one UTF-16 code unit. An astral scalar
    // therefore becomes an unpaired high surrogate whose lowercase form is
    // unchanged, even if the complete scalar has a Unicode case mapping.
    if first.len_utf16() != 1 {
        return None;
    }

    let lowercase = first.to_lowercase().collect::<CompactString>();
    if lowercase.len() == first.len_utf8() && lowercase.starts_with(first) {
        return None;
    }

    let mut replacement = lowercase;
    replacement.push_str(&description[first.len_utf8()..]);
    Some(replacement)
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
    if member.static_property_name() != Some("extend") {
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
    if member.static_property_name() != Some("extend") {
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
