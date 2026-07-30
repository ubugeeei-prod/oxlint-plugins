//! AST-backed port of Playwright's `expect-expect` rule.

use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier,
    MemberExpression, ModuleExportName, Program, VariableDeclarator, match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use regress::Regex;

use crate::{
    thresholds::{PlaywrightCall, classify_call, is_expect_assertion},
    types::{Diagnostic, DiagnosticData, LineIndex, PlaywrightOptions},
};

const RULE_NAME: &str = "expect-expect";

pub(crate) fn scan_expect_expect<'ast>(
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

    let mut expect_names = SmallVec::<[CompactString; 8]>::new();
    expect_names.push(CompactString::from("expect"));
    for alias in &options.expect_aliases {
        push_unique(&mut expect_names, alias.as_str());
    }

    let mut extend_declarations = SmallVec::<[(CompactString, CompactString); 16]>::new();
    NameCollector {
        expect_names: &mut expect_names,
        extend_declarations: &mut extend_declarations,
        test_names: &mut test_names,
    }
    .visit_program(program);
    resolve_extend_aliases(&mut test_names, &extend_declarations);

    let patterns = options
        .assert_function_patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern.as_str()).ok())
        .collect();
    let mut visitor = ExpectExpectVisitor {
        active_tests: SmallVec::new(),
        assert_function_names: &options.assert_function_names,
        assert_function_patterns: patterns,
        expect_names: &expect_names,
        pending_tests: SmallVec::new(),
        test_names: &test_names,
    };
    visitor.visit_program(program);

    for test in visitor.pending_tests {
        if !test.asserted {
            diagnostics.push(Diagnostic {
                rule_name: RULE_NAME,
                message_id: "noAssertions",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, test.callee_span),
                fix: None,
            });
        }
    }
}

struct NameCollector<'storage> {
    expect_names: &'storage mut SmallVec<[CompactString; 8]>,
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
            match module_export_name(&specifier.imported) {
                Some("test") => push_unique(self.test_names, specifier.local.name.as_str()),
                Some("expect") => push_unique(self.expect_names, specifier.local.name.as_str()),
                _ => {}
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

struct PendingTest {
    asserted: bool,
    callee_span: Span,
}

struct ExpectExpectVisitor<'options, 'names> {
    active_tests: SmallVec<[usize; 8]>,
    assert_function_names: &'options [CompactString],
    assert_function_patterns: SmallVec<[Regex; 4]>,
    expect_names: &'names [CompactString],
    pending_tests: SmallVec<[PendingTest; 16]>,
    test_names: &'names [CompactString],
}

impl<'ast> Visit<'ast> for ExpectExpectVisitor<'_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        if matches!(
            classify_call(call, self.test_names),
            Some(PlaywrightCall::Test)
        ) {
            let index = self.pending_tests.len();
            self.pending_tests.push(PendingTest {
                asserted: false,
                callee_span: call.callee.span(),
            });
            self.active_tests.push(index);
            walk::walk_call_expression(self, call);
            self.active_tests.pop();
            return;
        }

        if (is_expect_assertion(call, self.expect_names, self.test_names)
            || self.is_custom_assertion(call))
            && let Some(index) = self.active_tests.first().copied()
        {
            self.pending_tests[index].asserted = true;
        }
        walk::walk_call_expression(self, call);
    }
}

impl ExpectExpectVisitor<'_, '_> {
    fn is_custom_assertion(&self, call: &CallExpression<'_>) -> bool {
        let Some(name) = terminal_callee_identifier(&call.callee) else {
            return false;
        };
        self.assert_function_names
            .iter()
            .any(|configured| configured == name)
            || self
                .assert_function_patterns
                .iter()
                .any(|pattern| pattern.find(name).is_some())
    }
}

fn terminal_callee_identifier<'ast>(expression: &'ast Expression<'ast>) -> Option<&'ast str> {
    match expression.get_inner_expression() {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        Expression::CallExpression(call) => terminal_callee_identifier(&call.callee),
        member @ match_member_expression!(Expression) => {
            terminal_member_identifier(member.to_member_expression())
        }
        _ => None,
    }
}

fn terminal_member_identifier<'ast>(member: &'ast MemberExpression<'ast>) -> Option<&'ast str> {
    match member {
        MemberExpression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        MemberExpression::ComputedMemberExpression(member) => {
            match member.expression.get_inner_expression() {
                Expression::Identifier(identifier) => Some(identifier.name.as_str()),
                _ => None,
            }
        }
        MemberExpression::PrivateFieldExpression(_) => None,
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
