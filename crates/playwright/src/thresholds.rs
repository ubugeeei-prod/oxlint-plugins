//! AST-backed ports of Playwright's numeric threshold rules.

use compact_str::format_compact;
use oxc_ast::{
    AstKind,
    ast::{
        Argument, BindingPattern, CallExpression, Expression, ImportDeclaration,
        ImportDeclarationSpecifier, MemberExpression, ModuleExportName, Program,
        VariableDeclarator, match_member_expression,
    },
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::types::{Diagnostic, DiagnosticData, LineIndex, PlaywrightOptions};

const MAX_EXPECTS_RULE: &str = "max-expects";
const MAX_NESTED_DESCRIBE_RULE: &str = "max-nested-describe";
const REQUIRE_TOP_LEVEL_DESCRIBE_RULE: &str = "require-top-level-describe";

pub(crate) fn scan_threshold_rules<'ast>(
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
    ThresholdCollector {
        expect_names: &mut expect_names,
        extend_declarations: &mut extend_declarations,
        test_names: &mut test_names,
    }
    .visit_program(program);

    // Resolve `test.extend()` aliases to a fixed point so chained declarations
    // work regardless of their declaration order.
    loop {
        let mut changed = false;
        for (name, root) in &extend_declarations {
            if contains_name(&test_names, root.as_str())
                && !contains_name(&test_names, name.as_str())
            {
                test_names.push(name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    ThresholdVisitor {
        ancestors: SmallVec::new(),
        describe_depth: 0,
        diagnostics,
        expect_count: 0,
        expect_names,
        function_resets: SmallVec::new(),
        line_index,
        options,
        source_text,
        test_names,
        top_level_describe_count: 0,
    }
    .visit_program(program);
}

struct ThresholdCollector<'storage> {
    expect_names: &'storage mut SmallVec<[CompactString; 8]>,
    extend_declarations: &'storage mut SmallVec<[(CompactString, CompactString); 16]>,
    test_names: &'storage mut SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for ThresholdCollector<'_> {
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

struct ThresholdVisitor<'ast, 'source, 'options, 'diagnostics> {
    ancestors: SmallVec<[AstKind<'ast>; 32]>,
    describe_depth: usize,
    diagnostics: &'diagnostics mut SmallVec<[Diagnostic; 64]>,
    expect_count: usize,
    expect_names: SmallVec<[CompactString; 8]>,
    function_resets: SmallVec<[bool; 8]>,
    line_index: &'source LineIndex,
    options: &'options PlaywrightOptions,
    source_text: &'source str,
    test_names: SmallVec<[CompactString; 8]>,
    top_level_describe_count: usize,
}

impl<'ast> Visit<'ast> for ThresholdVisitor<'ast, '_, '_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        if matches!(
            kind,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
        ) {
            let reset = match self.ancestors.last() {
                Some(AstKind::CallExpression(parent)) => {
                    matches!(
                        classify_call(parent, &self.test_names),
                        Some(PlaywrightCall::Test)
                    )
                }
                _ => true,
            };
            if reset {
                self.expect_count = 0;
            }
            self.function_resets.push(reset);
        }
        self.ancestors.push(kind);
    }

    fn leave_node(&mut self, kind: AstKind<'ast>) {
        self.ancestors.pop();
        if matches!(
            kind,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
        ) && self.function_resets.pop().is_some_and(|reset| reset)
        {
            self.expect_count = 0;
        }
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        if is_expect_assertion(call, &self.expect_names, &self.test_names) {
            self.expect_count += 1;
            if self.expect_count > self.options.max_expects as usize {
                self.report(
                    MAX_EXPECTS_RULE,
                    "exceededMaxAssertion",
                    call.span,
                    DiagnosticData {
                        count: Some(compact_number(self.expect_count)),
                        max: Some(compact_number(self.options.max_expects)),
                        ..DiagnosticData::default()
                    },
                );
            }
        }

        let call_kind = classify_call(call, &self.test_names);
        let entered_describe = matches!(call_kind, Some(PlaywrightCall::Describe));
        if entered_describe {
            self.describe_depth += 1;
            if self.describe_depth > self.options.max_nested_describe as usize {
                self.report(
                    MAX_NESTED_DESCRIBE_RULE,
                    "exceededMaxDepth",
                    call.callee.span(),
                    DiagnosticData {
                        depth: Some(compact_number(self.describe_depth)),
                        max: Some(compact_number(self.options.max_nested_describe)),
                        ..DiagnosticData::default()
                    },
                );
            }

            if self.describe_depth == 1 {
                self.top_level_describe_count += 1;
                if self
                    .options
                    .max_top_level_describes
                    .is_some_and(|max| self.top_level_describe_count as f64 > max)
                {
                    let max = self.options.max_top_level_describes.unwrap_or_default();
                    self.report(
                        REQUIRE_TOP_LEVEL_DESCRIBE_RULE,
                        "tooManyDescribes",
                        call.callee.span(),
                        DiagnosticData {
                            amount: Some(compact_number(max)),
                            s: Some(CompactString::from(if max == 1.0 { "" } else { "s" })),
                            ..DiagnosticData::default()
                        },
                    );
                }
            }
        } else if self.describe_depth == 0 {
            match call_kind {
                Some(PlaywrightCall::Test) => self.report(
                    REQUIRE_TOP_LEVEL_DESCRIBE_RULE,
                    "unexpectedTest",
                    call.callee.span(),
                    DiagnosticData::default(),
                ),
                Some(PlaywrightCall::Hook) => self.report(
                    REQUIRE_TOP_LEVEL_DESCRIBE_RULE,
                    "unexpectedHook",
                    call.callee.span(),
                    DiagnosticData::default(),
                ),
                None | Some(PlaywrightCall::Describe) => {}
            }
        }

        walk::walk_call_expression(self, call);

        if entered_describe {
            self.describe_depth = self.describe_depth.saturating_sub(1);
        }
    }
}

impl ThresholdVisitor<'_, '_, '_, '_> {
    fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        data: DiagnosticData,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix: None,
        });
    }
}

#[derive(Clone, Copy)]
enum PlaywrightCall {
    Describe,
    Hook,
    Test,
}

fn classify_call(
    call: &CallExpression<'_>,
    test_names: &[CompactString],
) -> Option<PlaywrightCall> {
    let mut links = SmallVec::<[&str; 8]>::new();
    let root = callee_chain(&call.callee, &mut links)?;
    let has_callback = call.arguments.last().is_some_and(is_function_argument);

    if contains_name(test_names, root) {
        let Some((first, tail)) = links.split_first() else {
            return (has_callback && call.arguments.len() >= 2).then_some(PlaywrightCall::Test);
        };
        return match *first {
            "describe" if valid_describe_tail(tail) && has_callback => {
                Some(PlaywrightCall::Describe)
            }
            "afterAll" | "afterEach" | "beforeAll" | "beforeEach" if tail.is_empty() => {
                Some(PlaywrightCall::Hook)
            }
            "only" | "skip" | "fixme" | "slow"
                if tail.is_empty() && has_callback && call.arguments.len() >= 2 =>
            {
                Some(PlaywrightCall::Test)
            }
            "fail"
                if matches!(tail, [] | ["only"]) && has_callback && call.arguments.len() >= 2 =>
            {
                Some(PlaywrightCall::Test)
            }
            _ => None,
        };
    }

    if root == "describe" && valid_describe_tail(&links) && has_callback {
        return Some(PlaywrightCall::Describe);
    }
    if matches!(root, "afterAll" | "afterEach" | "beforeAll" | "beforeEach") && links.is_empty() {
        return Some(PlaywrightCall::Hook);
    }
    None
}

fn valid_describe_tail(tail: &[&str]) -> bool {
    matches!(
        tail,
        [] | ["only"]
            | ["skip"]
            | ["fixme"]
            | ["fixme", "only"]
            | ["configure"]
            | ["serial"]
            | ["serial", "only"]
            | ["serial", "skip"]
            | ["serial", "fixme"]
            | ["serial", "fixme", "only"]
            | ["parallel"]
            | ["parallel", "only"]
            | ["parallel", "skip"]
            | ["parallel", "fixme"]
            | ["parallel", "fixme", "only"]
    )
}

fn is_expect_assertion(
    call: &CallExpression<'_>,
    expect_names: &[CompactString],
    test_names: &[CompactString],
) -> bool {
    let Some(member) = member_from_expression(&call.callee) else {
        return false;
    };
    let Some(matcher) = member_name(member) else {
        return false;
    };
    if matches!(matcher, "not" | "poll" | "rejects" | "resolves" | "soft") {
        return false;
    }
    expression_contains_expect_call(member.object(), expect_names, test_names)
}

fn expression_contains_expect_call(
    expression: &Expression<'_>,
    expect_names: &[CompactString],
    test_names: &[CompactString],
) -> bool {
    match expression.get_inner_expression() {
        Expression::CallExpression(call) => is_expect_head(call, expect_names, test_names),
        member @ match_member_expression!(Expression) => expression_contains_expect_call(
            member.to_member_expression().object(),
            expect_names,
            test_names,
        ),
        _ => false,
    }
}

fn is_expect_head(
    call: &CallExpression<'_>,
    expect_names: &[CompactString],
    test_names: &[CompactString],
) -> bool {
    let mut links = SmallVec::<[&str; 8]>::new();
    let Some(root) = callee_chain(&call.callee, &mut links) else {
        return false;
    };
    if contains_name(expect_names, root) || root.starts_with("expect") || root.ends_with("Expect") {
        return matches!(links.as_slice(), [] | ["soft"] | ["poll"]);
    }
    contains_name(test_names, root)
        && matches!(
            links.as_slice(),
            ["expect"] | ["expect", "soft"] | ["expect", "poll"]
        )
}

fn callee_chain<'ast>(
    expression: &'ast Expression<'ast>,
    links: &mut SmallVec<[&'ast str; 8]>,
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

fn member_from_expression<'ast>(
    expression: &'ast Expression<'ast>,
) -> Option<&'ast MemberExpression<'ast>> {
    match expression.get_inner_expression() {
        member @ match_member_expression!(Expression) => Some(member.to_member_expression()),
        _ => None,
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

fn is_function_argument(argument: &Argument<'_>) -> bool {
    matches!(
        argument,
        Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_)
    )
}

fn module_export_name<'ast>(name: &'ast ModuleExportName<'ast>) -> Option<&'ast str> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::StringLiteral(literal) => Some(literal.value.as_str()),
    }
}

fn compact_number(value: impl std::fmt::Display) -> CompactString {
    format_compact!("{value}")
}

fn contains_name(names: &[CompactString], value: &str) -> bool {
    names.iter().any(|name| name == value)
}

fn push_unique(names: &mut SmallVec<[CompactString; 8]>, value: &str) {
    if !contains_name(names, value) {
        names.push(CompactString::from(value));
    }
}
