//! Free helper functions used across the storybook scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the storybook AST import surface; not every helper uses every type."
)]

use std::path::Path;

use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, AssignmentTarget, BindingPattern, CallExpression,
    Expression, FormalParameters, ImportDeclaration, ImportDeclarationSpecifier, ModuleExportName,
    ObjectExpression, ObjectProperty, ObjectPropertyKind, PropertyKey, StaticMemberExpression,
};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Descriptor, FUNCTIONS_TO_AWAIT, StoryFilters};

pub(crate) fn component_name_from_filename(filename: &str) -> Option<CompactString> {
    let basename = Path::new(filename).file_name()?.to_str()?;
    let name = basename.split('.').next()?;
    if name.is_empty() {
        None
    } else {
        Some(CompactString::from(name))
    }
}

pub(crate) fn source_slice(source_text: &str, span: Span) -> &str {
    let start = (span.start as usize).min(source_text.len());
    let end = (span.end as usize).min(source_text.len());
    if start <= end {
        &source_text[start..end]
    } else {
        ""
    }
}

pub(crate) fn module_export_name<'a>(name: &'a ModuleExportName<'a>) -> Option<&'a str> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::StringLiteral(literal) => Some(literal.value.as_str()),
    }
}

pub(crate) fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        PropertyKey::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn binding_identifier_name<'a>(pattern: &'a BindingPattern<'a>) -> Option<&'a str> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn context_param_name(params: &FormalParameters<'_>) -> Option<CompactString> {
    let first = params.items.first()?;
    match &first.pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            Some(CompactString::from(identifier.name.as_str()))
        }
        BindingPattern::ObjectPattern(pattern) => {
            for property in &pattern.properties {
                if property_key_name(&property.key) == Some("context") {
                    return Some(CompactString::from("context"));
                }
            }
            let rest = pattern.rest.as_ref()?;
            binding_identifier_name(&rest.argument).map(CompactString::from)
        }
        _ => None,
    }
}

pub(crate) fn find_object_property<'a>(
    object: &'a ObjectExpression<'a>,
    property_name: &str,
) -> Option<&'a ObjectProperty<'a>> {
    object
        .properties
        .iter()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if !property.computed
                    && property_key_name(&property.key) == Some(property_name) =>
            {
                Some(&**property)
            }
            _ => None,
        })
}

pub(crate) fn string_literal_value<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

pub(crate) fn raw_string_literal<'a>(
    source_text: &'a str,
    expression: &Expression<'_>,
) -> Option<&'a str> {
    if string_literal_value(expression).is_some() {
        Some(source_slice(source_text, expression.span()))
    } else {
        None
    }
}

pub(crate) fn is_inline_property_value(expression: &Expression<'_>) -> bool {
    matches!(
        expression.get_inner_expression(),
        Expression::ObjectExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::ArrayExpression(_)
    )
}

pub(crate) fn import_has_local_name(declaration: &ImportDeclaration<'_>, name: &str) -> bool {
    declaration.specifiers.as_ref().is_some_and(|specifiers| {
        specifiers.iter().any(|specifier| match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(specifier) => specifier.local.name == name,
            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                specifier.local.name == name
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                specifier.local.name == name
            }
        })
    })
}

pub(crate) fn import_has_default_specifier(declaration: &ImportDeclaration<'_>) -> bool {
    declaration.specifiers.as_ref().is_some_and(|specifiers| {
        specifiers.iter().any(|specifier| {
            matches!(
                specifier,
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
            )
        })
    })
}

pub(crate) fn renderer_framework_suggestions(
    package_name: &str,
) -> Option<(&'static str, &'static str)> {
    match package_name {
        "@storybook/html" => Some((
            "@storybook/html",
            "@storybook/html-vite, @storybook/html-webpack5",
        )),
        "@storybook/preact" => Some((
            "@storybook/preact",
            "@storybook/preact-vite, @storybook/preact-webpack5",
        )),
        "@storybook/react" => Some((
            "@storybook/react",
            "@storybook/nextjs, @storybook/react-vite, @storybook/nextjs-vite, @storybook/react-webpack5, @storybook/react-native-web-vite",
        )),
        "@storybook/server" => Some(("@storybook/server", "@storybook/server-webpack5")),
        "@storybook/svelte" => Some((
            "@storybook/svelte",
            "@storybook/svelte-vite, @storybook/svelte-webpack5, @storybook/sveltekit",
        )),
        "@storybook/vue3" => Some((
            "@storybook/vue3",
            "@storybook/vue3-vite, @storybook/vue3-webpack5",
        )),
        "@storybook/web-components" => Some((
            "@storybook/web-components",
            "@storybook/web-components-vite, @storybook/web-components-webpack5",
        )),
        _ => None,
    }
}

pub(crate) fn call_property_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    let Expression::CallExpression(call) = expression.get_inner_expression() else {
        return None;
    };
    static_member_property_name(&call.callee)
}

pub(crate) fn static_member_property_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.get_inner_expression() {
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        Expression::TSNonNullExpression(expression) => {
            static_member_property_name(&expression.expression)
        }
        _ => None,
    }
}

pub(crate) fn assignment_static_member<'a>(
    target: &'a AssignmentTarget<'a>,
) -> Option<(&'a str, &'a str)> {
    let AssignmentTarget::StaticMemberExpression(member) = target else {
        return None;
    };
    let Expression::Identifier(object) = member.object.get_inner_expression() else {
        return None;
    };
    Some((object.name.as_str(), member.property.name.as_str()))
}

pub(crate) fn method_that_should_be_awaited<'a>(
    call: &'a CallExpression<'a>,
    user_event_is_non_storybook: bool,
) -> Option<&'a str> {
    match call.callee.get_inner_expression() {
        Expression::Identifier(identifier) if should_await(identifier.name.as_str()) => {
            Some(identifier.name.as_str())
        }
        Expression::StaticMemberExpression(member) => {
            method_from_static_member(member, user_event_is_non_storybook)
        }
        Expression::TSNonNullExpression(expression) => {
            if let Expression::StaticMemberExpression(member) =
                expression.expression.get_inner_expression()
            {
                method_from_static_member(member, user_event_is_non_storybook)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn method_from_static_member<'a>(
    member: &'a StaticMemberExpression<'a>,
    user_event_is_non_storybook: bool,
) -> Option<&'a str> {
    if let Expression::Identifier(object) = member.object.get_inner_expression()
        && should_await(object.name.as_str())
        && !(object.name == "userEvent" && user_event_is_non_storybook)
    {
        return Some(object.name.as_str());
    }

    let property_name = member.property.name.as_str();
    if should_await(property_name) {
        return Some(property_name);
    }

    if let Expression::CallExpression(call) = member.object.get_inner_expression()
        && call.callee.is_specific_id("expect")
    {
        return Some(property_name);
    }

    None
}

pub(crate) fn should_await(name: &str) -> bool {
    FUNCTIONS_TO_AWAIT.contains(&name) || name.starts_with("findBy")
}

pub(crate) fn is_play_call(call: &CallExpression<'_>) -> bool {
    static_member_property_name(&call.callee) == Some("play")
}

pub(crate) fn story_filters_from_meta(meta: &ObjectExpression<'_>) -> StoryFilters {
    // Upstream builds `{ excludeStories: getDescriptor(meta, ...), includeStories:
    // getDescriptor(meta, ...) }` inside a try/catch and discards the WHOLE config
    // if either call throws (`getDescriptor` throws on a non-literal array element
    // such as `includeStories: [MyComponent.name]`), so the file is treated as
    // having no filter. Mirror that: if either descriptor fails to resolve, return
    // empty (unfiltered) filters.
    let include = match find_object_property(meta, "includeStories") {
        Some(property) => descriptor_from_expression(&property.value),
        None => Ok(None),
    };
    let exclude = match find_object_property(meta, "excludeStories") {
        Some(property) => descriptor_from_expression(&property.value),
        None => Ok(None),
    };
    let (Ok(include), Ok(exclude)) = (include, exclude) else {
        return StoryFilters::default();
    };

    let mut filters = StoryFilters::default();
    if let Some(descriptor) = include {
        filters.include.push(descriptor);
        filters.has_filter = true;
    }
    if let Some(descriptor) = exclude {
        filters.exclude.push(descriptor);
        filters.has_filter = true;
    }
    filters
}

/// Returned by [`descriptor_from_expression`] when upstream `getDescriptor` would
/// throw, signalling the whole filter config must be discarded.
pub(crate) struct InvalidDescriptor;

// Mirrors upstream `getDescriptor`. `Ok(Some(_))` is a resolved filter, `Ok(None)`
// means the property is not a descriptor shape upstream recognises (no filter), and
// `Err(_)` means upstream `getDescriptor` would throw — an array containing a
// non-string-literal element (or a hole/spread), which discards the whole config.
pub(crate) fn descriptor_from_expression(
    expression: &Expression<'_>,
) -> Result<Option<Descriptor>, InvalidDescriptor> {
    match expression.get_inner_expression() {
        Expression::ArrayExpression(array) => {
            let mut names = SmallVec::new();
            for element in &array.elements {
                let Some(Expression::StringLiteral(literal)) = element
                    .as_expression()
                    .map(Expression::get_inner_expression)
                else {
                    return Err(InvalidDescriptor);
                };
                names.push(CompactString::from(literal.value.as_str()));
            }
            Ok(Some(Descriptor::Names(names)))
        }
        Expression::StringLiteral(literal) => {
            let mut names = SmallVec::new();
            names.push(CompactString::from(literal.value.as_str()));
            Ok(Some(Descriptor::Names(names)))
        }
        Expression::RegExpLiteral(literal) => Ok(Some(Descriptor::Regex(CompactString::from(
            literal.regex.pattern.text.as_str(),
        )))),
        _ => Ok(None),
    }
}

pub(crate) fn is_story_export(name: &str, filters: &StoryFilters) -> bool {
    if name == "__namedExportsOrder" || name.starts_with('_') {
        return false;
    }
    if !filters.include.is_empty() {
        return filters
            .include
            .iter()
            .any(|descriptor| descriptor_matches(descriptor, name));
    }
    !filters
        .exclude
        .iter()
        .any(|descriptor| descriptor_matches(descriptor, name))
}

pub(crate) fn descriptor_matches(descriptor: &Descriptor, name: &str) -> bool {
    match descriptor {
        Descriptor::Names(names) => names.iter().any(|candidate| candidate == name),
        Descriptor::Regex(pattern) => simple_regex_match(pattern.as_str(), name),
    }
}

pub(crate) fn simple_regex_match(pattern: &str, name: &str) -> bool {
    if let Some(prefix) = pattern
        .strip_suffix('$')
        .and_then(|pattern| pattern.strip_prefix(".*"))
    {
        return name.ends_with(prefix);
    }
    if let Some(suffix) = pattern
        .strip_prefix('^')
        .and_then(|pattern| pattern.strip_suffix(".*"))
    {
        return name.starts_with(suffix);
    }
    pattern == name
}

pub(crate) fn is_pascal_case(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_uppercase())
}

pub(crate) fn to_pascal_case(name: &str) -> CompactString {
    let mut out = CompactString::new("");
    let mut upper_next = true;
    for ch in name.chars() {
        if ch == '-' || ch == '_' || ch.is_whitespace() {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

pub(crate) fn story_name_from_export(name: &str) -> CompactString {
    let mut out = CompactString::new("");
    let mut previous: Option<char> = None;
    let mut chars = name.chars().peekable();
    while let Some(ch) = chars.next() {
        if matches!(ch, '_' | '-') {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
            previous = Some(' ');
            continue;
        }
        if let Some(prev) = previous {
            let next_is_lower = chars.peek().is_some_and(|next| next.is_ascii_lowercase());
            if !out.ends_with(' ')
                && ((prev.is_ascii_lowercase() && ch.is_ascii_uppercase())
                    || (prev.is_ascii_digit() && ch.is_ascii_alphabetic())
                    || (prev.is_ascii_alphabetic() && ch.is_ascii_digit())
                    || (prev.is_ascii_uppercase() && ch.is_ascii_uppercase() && next_is_lower))
            {
                out.push(' ');
            }
        }
        out.push(ch);
        previous = Some(ch);
    }
    out
}

pub(crate) fn addon_name_from_array_element<'a>(
    element: &'a ArrayExpressionElement<'a>,
) -> Option<(&'a str, Span)> {
    match element.as_expression()?.get_inner_expression() {
        Expression::StringLiteral(literal) => Some((literal.value.as_str(), literal.span)),
        Expression::ObjectExpression(object) => {
            let property = find_object_property(object, "name")?;
            let value = string_literal_value(&property.value)?;
            Some((value, property.value.span()))
        }
        _ => None,
    }
}

pub(crate) fn cleaned_addon_name(addon: &str) -> CompactString {
    let mut name = addon;
    for suffix in [".mjs", ".cjs", ".js"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            name = stripped;
            break;
        }
    }
    if let Some(stripped) = name.strip_suffix("/register") {
        name = stripped;
    }
    if let Some(stripped) = name.strip_suffix("/preset") {
        name = stripped;
    }
    CompactString::from(name)
}

pub(crate) fn is_local_addon(addon: &str) -> bool {
    addon.starts_with('.')
        || addon.starts_with('/')
        || addon.starts_with('\\')
        || addon.as_bytes().get(1).is_some_and(|byte| *byte == b':')
}
