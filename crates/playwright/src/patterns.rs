//! AST-backed ports of Playwright's `valid-title` and `valid-test-tags` rules.

use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, BinaryExpression, BindingPattern,
    CallExpression, Expression, ImportDeclaration, ImportDeclarationSpecifier, MemberExpression,
    ModuleExportName, ObjectExpression, ObjectPropertyKind, Program, PropertyKey,
    VariableDeclarator, match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::types::{
    Diagnostic, DiagnosticData, DiagnosticFix, LineIndex, PlaywrightOptions, TagPattern,
    TitlePattern, TitlePatternOptions,
};

const VALID_TITLE_RULE: &str = "valid-title";
const VALID_TEST_TAGS_RULE: &str = "valid-test-tags";

pub(crate) fn scan_pattern_rules<'ast>(
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

    let mut declarations = SmallVec::<[Declaration; 32]>::new();
    PatternCollector {
        declarations: &mut declarations,
        test_names: &mut test_names,
    }
    .visit_program(program);

    // `const custom = test.extend(...)` is a Playwright test function too. Resolve
    // aliases after collecting all declarations so declaration order is irrelevant.
    loop {
        let mut changed = false;
        for declaration in &declarations {
            if declaration
                .extend_root
                .as_ref()
                .is_some_and(|root| contains_name(&test_names, root.as_str()))
                && !contains_name(&test_names, declaration.name.as_str())
            {
                test_names.push(declaration.name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    PatternVisitor {
        declarations,
        diagnostics,
        line_index,
        options,
        source_text,
        test_names,
    }
    .visit_program(program);
}

struct PatternCollector<'storage> {
    declarations: &'storage mut SmallVec<[Declaration; 32]>,
    test_names: &'storage mut SmallVec<[CompactString; 8]>,
}

#[derive(Clone)]
struct Declaration {
    name: CompactString,
    value: DeclarationValue,
    extend_root: Option<CompactString>,
}

#[derive(Clone)]
enum DeclarationValue {
    Static { value: CompactString, span: Span },
    Alias(CompactString),
    AcceptedDynamic,
    Invalid(Span),
}

impl<'ast> Visit<'ast> for PatternCollector<'_> {
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
            if module_export_name(&specifier.imported) == Some("test") {
                push_unique(self.test_names, specifier.local.name.as_str());
            }
        }
    }

    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'ast>) {
        if let (BindingPattern::BindingIdentifier(identifier), Some(initializer)) =
            (&declaration.id, &declaration.init)
        {
            self.declarations.push(Declaration {
                name: CompactString::from(identifier.name.as_str()),
                value: declaration_value(initializer),
                extend_root: test_extend_root(initializer).map(CompactString::from),
            });
        }
        walk::walk_variable_declarator(self, declaration);
    }
}

struct PatternVisitor<'source, 'options, 'diagnostics> {
    declarations: SmallVec<[Declaration; 32]>,
    diagnostics: &'diagnostics mut SmallVec<[Diagnostic; 64]>,
    line_index: &'source LineIndex,
    options: &'options PlaywrightOptions,
    source_text: &'source str,
    test_names: SmallVec<[CompactString; 8]>,
}

impl<'ast> Visit<'ast> for PatternVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        if let Some(call_kind) = classify_call(call, &self.test_names) {
            self.check_valid_title(call, call_kind);
            self.check_valid_test_tags(call);
        }
        walk::walk_call_expression(self, call);
    }
}

#[derive(Clone, Copy)]
enum CallKind {
    Describe,
    Step,
    Test,
}

impl CallKind {
    fn name(self) -> &'static str {
        match self {
            Self::Describe => "describe",
            Self::Step => "step",
            Self::Test => "test",
        }
    }
}

impl PatternVisitor<'_, '_, '_> {
    fn check_valid_title(&mut self, call: &CallExpression<'_>, call_kind: CallKind) {
        let Some(title_argument) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        if matches!(call_kind, CallKind::Describe) && is_function(title_argument) {
            return;
        }

        let owned_title =
            if let Expression::Identifier(identifier) = title_argument.get_inner_expression() {
                self.resolve_declaration(identifier.name.as_str())
            } else {
                None
            };
        let (title_value, title_span, raw) = match owned_title {
            Some(DeclarationValue::Static { value, span }) => {
                let raw = self
                    .source_text
                    .get(span.start as usize..span.end as usize)
                    .unwrap_or_default();
                (value, span, raw)
            }
            Some(DeclarationValue::AcceptedDynamic) | Some(DeclarationValue::Alias(_)) => return,
            Some(DeclarationValue::Invalid(span)) => {
                if !self.should_ignore_title_type(call_kind) {
                    self.report(
                        VALID_TITLE_RULE,
                        "titleMustBeString",
                        span,
                        DiagnosticData::default(),
                        None,
                    );
                }
                return;
            }
            None => {
                let Some((value, span, raw)) = static_string(title_argument, self.source_text)
                else {
                    if is_accepted_dynamic_title(title_argument)
                        || self.should_ignore_title_type(call_kind)
                        || matches!(title_argument, Expression::TemplateLiteral(_))
                    {
                        return;
                    }
                    self.report(
                        VALID_TITLE_RULE,
                        "titleMustBeString",
                        title_argument.span(),
                        DiagnosticData::default(),
                        None,
                    );
                    return;
                };
                (CompactString::from(value), span, raw)
            }
        };
        let title_value = title_value.as_str();
        let function_name = call_kind.name();

        if title_value.is_empty() {
            let data = DiagnosticData {
                function_name: Some(CompactString::from(function_name)),
                ..DiagnosticData::default()
            };
            self.report(VALID_TITLE_RULE, "emptyTitle", call.span, data, None);
            return;
        }

        if let Some(word) =
            first_disallowed_word(title_value, &self.options.valid_title.disallowed_words)
        {
            let data = DiagnosticData {
                word: Some(CompactString::from(word)),
                ..DiagnosticData::default()
            };
            self.report(VALID_TITLE_RULE, "disallowedWord", title_span, data, None);
            return;
        }

        if !self.options.valid_title.ignore_spaces && title_value.trim().len() != title_value.len()
        {
            self.report(
                VALID_TITLE_RULE,
                "accidentalSpace",
                title_span,
                DiagnosticData::default(),
                Some(DiagnosticFix {
                    start: title_span.start,
                    end: title_span.end,
                    replacement: trim_literal_ascii_spaces(raw),
                }),
            );
        }

        let first_word = title_value.split(' ').next().unwrap_or_default();
        if first_word.to_lowercase() == function_name {
            let fix = title_value.find(' ').map(|space| DiagnosticFix {
                start: title_span.start,
                end: title_span.end,
                replacement: remove_duplicate_prefix(raw, space),
            });
            self.report(
                VALID_TITLE_RULE,
                "duplicatePrefix",
                title_span,
                DiagnosticData::default(),
                fix,
            );
        }

        if let Some(pattern) = selected_pattern(&self.options.valid_title.must_not_match, call_kind)
            && pattern_matches(pattern, title_value)
        {
            self.report_title_pattern(
                "mustNotMatch",
                "mustNotMatchCustom",
                title_span,
                function_name,
                pattern,
            );
            return;
        }
        if let Some(pattern) = selected_pattern(&self.options.valid_title.must_match, call_kind)
            && !pattern_matches(pattern, title_value)
        {
            self.report_title_pattern(
                "mustMatch",
                "mustMatchCustom",
                title_span,
                function_name,
                pattern,
            );
        }
    }

    fn check_valid_test_tags(&mut self, call: &CallExpression<'_>) {
        let Some(title) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        if let Some((title_value, _, _)) = static_string(title, self.source_text)
            && let Ok(tag_regex) = Regex::new(r"@\S+")
        {
            for found in tag_regex.find_iter(title_value) {
                self.validate_tag(&title_value[found.start()..found.end()], call.span);
            }
        }

        let Some(Expression::ObjectExpression(options)) =
            call.arguments.get(1).and_then(Argument::as_expression)
        else {
            return;
        };
        let Some(tag_value) = tag_property(options) else {
            return;
        };
        self.validate_tag_expression(tag_value, call.span);
    }

    fn validate_tag_expression(&mut self, expression: &Expression<'_>, report_span: Span) {
        if let Some((tag, _, _)) = static_string(expression, self.source_text) {
            self.validate_tag(tag, report_span);
            return;
        }
        if let Expression::ArrayExpression(array) = expression {
            self.validate_tag_array(array, report_span);
            return;
        }
        self.report(
            VALID_TEST_TAGS_RULE,
            "invalidTagValue",
            report_span,
            DiagnosticData::default(),
            None,
        );
    }

    fn validate_tag_array(&mut self, array: &ArrayExpression<'_>, report_span: Span) {
        for element in &array.elements {
            if matches!(
                element,
                ArrayExpressionElement::Elision(_) | ArrayExpressionElement::SpreadElement(_)
            ) {
                return;
            }
            let Some(expression) = element.as_expression() else {
                return;
            };
            let Some((tag, _, _)) = static_string(expression, self.source_text) else {
                // Upstream deliberately stops at a dynamic or non-string array
                // element because its eventual value cannot be determined.
                return;
            };
            self.validate_tag(tag, report_span);
        }
    }

    fn validate_tag(&mut self, tag: &str, span: Span) {
        if !tag.starts_with('@') {
            self.report(
                VALID_TEST_TAGS_RULE,
                "invalidTagFormat",
                span,
                DiagnosticData::default(),
                None,
            );
            return;
        }
        let allowed = &self.options.valid_test_tags.allowed_tags;
        if !allowed.is_empty()
            && !allowed
                .iter()
                .any(|pattern| tag_pattern_matches(pattern, tag))
        {
            let data = DiagnosticData {
                tag: Some(CompactString::from(tag)),
                ..DiagnosticData::default()
            };
            self.report(VALID_TEST_TAGS_RULE, "unknownTag", span, data, None);
            return;
        }
        if self
            .options
            .valid_test_tags
            .disallowed_tags
            .iter()
            .any(|pattern| tag_pattern_matches(pattern, tag))
        {
            let data = DiagnosticData {
                tag: Some(CompactString::from(tag)),
                ..DiagnosticData::default()
            };
            self.report(VALID_TEST_TAGS_RULE, "disallowedTag", span, data, None);
        }
    }

    fn report_title_pattern(
        &mut self,
        default_message_id: &'static str,
        custom_message_id: &'static str,
        span: Span,
        function_name: &str,
        pattern: &TitlePattern,
    ) {
        let mut rendered_pattern = CompactString::with_capacity(pattern.source.len() + 3);
        rendered_pattern.push('/');
        rendered_pattern.push_str(pattern.source.as_str());
        rendered_pattern.push_str("/u");
        let data = DiagnosticData {
            function_name: Some(CompactString::from(function_name)),
            message: pattern.message.clone().unwrap_or_default(),
            pattern: Some(rendered_pattern),
            ..DiagnosticData::default()
        };
        self.report(
            VALID_TITLE_RULE,
            if pattern.message.is_some() {
                custom_message_id
            } else {
                default_message_id
            },
            span,
            data,
            None,
        );
    }

    fn should_ignore_title_type(&self, call_kind: CallKind) -> bool {
        match call_kind {
            CallKind::Describe => self.options.valid_title.ignore_type_of_describe_name,
            CallKind::Step => self.options.valid_title.ignore_type_of_step_name,
            CallKind::Test => self.options.valid_title.ignore_type_of_test_name,
        }
    }

    fn resolve_declaration(&self, name: &str) -> Option<DeclarationValue> {
        let mut name = CompactString::from(name);
        for _ in 0..16 {
            let declaration = self
                .declarations
                .iter()
                .rev()
                .find(|declaration| declaration.name == name)?;
            match &declaration.value {
                DeclarationValue::Alias(alias) if alias != name => name = alias.clone(),
                value => return Some(value.clone()),
            }
        }
        Some(DeclarationValue::AcceptedDynamic)
    }

    fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        data: DiagnosticData,
        fix: Option<DiagnosticFix>,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix,
        });
    }
}

fn classify_call(call: &CallExpression<'_>, test_names: &[CompactString]) -> Option<CallKind> {
    if !call.arguments.last().is_some_and(is_function_argument) {
        return None;
    }
    let mut links = SmallVec::<[&str; 4]>::new();
    let root = callee_chain(&call.callee, &mut links)?;
    if !contains_name(test_names, root) {
        return None;
    }
    let kind = match links.as_slice() {
        ["describe"]
        | ["describe", "only"]
        | ["describe", "skip"]
        | ["describe", "fixme"]
        | ["describe", "fail"]
        | ["only", "describe"]
        | ["skip", "describe"] => Some(CallKind::Describe),
        ["step"] | ["step", "skip"] => Some(CallKind::Step),
        [] | ["only"] | ["skip"] | ["fixme"] | ["fail"] | ["slow"] => Some(CallKind::Test),
        _ => None,
    }?;
    if !matches!(kind, CallKind::Describe) && call.arguments.len() < 2 {
        return None;
    }
    Some(kind)
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
            links.push(member.static_property_name()?);
            Some(root)
        }
        _ => None,
    }
}

fn is_function_argument(argument: &Argument<'_>) -> bool {
    matches!(
        argument,
        Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_)
    )
}

fn is_function(expression: &Expression<'_>) -> bool {
    matches!(
        expression,
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    )
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
        _ => None,
    }
}

fn declaration_value(expression: &Expression<'_>) -> DeclarationValue {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => DeclarationValue::Static {
            value: CompactString::from(literal.value.as_str()),
            span: literal.span,
        },
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            DeclarationValue::Static {
                value: CompactString::from(
                    template
                        .quasis
                        .first()
                        .and_then(|quasi| quasi.value.cooked.as_ref())
                        .map_or("", |value| value.as_str()),
                ),
                span: template.span,
            }
        }
        Expression::Identifier(identifier) => {
            DeclarationValue::Alias(CompactString::from(identifier.name.as_str()))
        }
        expression
            if is_accepted_dynamic_title(expression)
                || matches!(expression, Expression::TemplateLiteral(_)) =>
        {
            DeclarationValue::AcceptedDynamic
        }
        expression => DeclarationValue::Invalid(expression.span()),
    }
}

fn static_string<'ast, 'source>(
    expression: &'ast Expression<'ast>,
    source_text: &'source str,
) -> Option<(&'ast str, Span, &'source str)> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Some((
            literal.value.as_str(),
            literal.span,
            source_text.get(literal.span.start as usize..literal.span.end as usize)?,
        )),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => Some((
            template
                .quasis
                .first()?
                .value
                .cooked
                .as_ref()
                .map_or("", |value| value.as_str()),
            template.span,
            source_text.get(template.span.start as usize..template.span.end as usize)?,
        )),
        _ => None,
    }
}

fn is_accepted_dynamic_title(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
    ) || matches!(
        expression.get_inner_expression(),
        Expression::BinaryExpression(binary) if binary_contains_string(binary)
    )
}

fn binary_contains_string(binary: &BinaryExpression<'_>) -> bool {
    static_string_node(&binary.right)
        || match binary.left.get_inner_expression() {
            Expression::BinaryExpression(left) => binary_contains_string(left),
            left => static_string_node(left),
        }
}

fn static_string_node(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::StringLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        _ => false,
    }
}

fn first_disallowed_word<'a>(title: &'a str, words: &[CompactString]) -> Option<&'a str> {
    if words.is_empty() {
        return None;
    }
    let mut source = CompactString::with_capacity(
        words.iter().map(CompactString::len).sum::<usize>() + words.len() + 6,
    );
    source.push_str(r"\b(");
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            source.push('|');
        }
        source.push_str(word.as_str());
    }
    source.push_str(r")\b");
    let regex = regex::RegexBuilder::new(&source)
        .case_insensitive(true)
        .unicode(true)
        .build()
        .ok()?;
    regex
        .captures(title)
        .and_then(|captures| captures.get(1))
        .map(|found| found.as_str())
}

fn selected_pattern(patterns: &TitlePatternOptions, call_kind: CallKind) -> Option<&TitlePattern> {
    match call_kind {
        CallKind::Describe => patterns.describe.as_ref(),
        CallKind::Step => patterns.step.as_ref(),
        CallKind::Test => patterns.test.as_ref(),
    }
}

fn pattern_matches(pattern: &TitlePattern, value: &str) -> bool {
    regress::Regex::with_flags(pattern.source.as_str(), "u")
        .is_ok_and(|regex| regex.find(value).is_some())
}

fn tag_pattern_matches(pattern: &TagPattern, value: &str) -> bool {
    if !pattern.is_regex {
        return pattern.source == value;
    }
    regress::Regex::with_flags(pattern.source.as_str(), pattern.flags.as_str())
        .is_ok_and(|regex| regex.find(value).is_some())
}

fn tag_property<'ast>(options: &'ast ObjectExpression<'ast>) -> Option<&'ast Expression<'ast>> {
    options.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if matches!(&property.key, PropertyKey::StaticIdentifier(identifier) if identifier.name == "tag")
        {
            Some(&property.value)
        } else {
            None
        }
    })
}

fn trim_literal_ascii_spaces(raw: &str) -> CompactString {
    if raw.len() < 2 {
        return CompactString::from(raw);
    }
    let body = &raw[1..raw.len() - 1];
    let mut output = CompactString::with_capacity(raw.len());
    output.push_str(&raw[..1]);
    output.push_str(body.trim_matches(' '));
    output.push_str(&raw[raw.len() - 1..]);
    output
}

fn remove_duplicate_prefix(raw: &str, content_space: usize) -> CompactString {
    if raw.len() < 2 {
        return CompactString::from(raw);
    }
    let mut output = CompactString::with_capacity(raw.len() - content_space);
    output.push_str(&raw[..1]);
    output.push_str(&raw[content_space + 2..]);
    output
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
