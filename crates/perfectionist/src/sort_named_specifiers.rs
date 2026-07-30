#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "serde_json option maps require String keys and configured group/specifier collections are user-sized rather than bounded rule state."
)]

//! Shared configurable named-import/export engine pinned to Perfectionist v5.9.1.

use std::cmp::Ordering;
use std::sync::{OnceLock, RwLock};

use oxc_ast::{
    Comment, CommentKind,
    ast::{
        Argument, BindingPattern, ExportAllDeclaration, ExportNamedDeclaration, ExportSpecifier,
        Expression, ImportDeclaration, ImportDeclarationSpecifier, ImportOrExportKind,
        ImportSpecifier, ModuleExportName, Statement, TSImportEqualsDeclaration, TSModuleReference,
        VariableDeclaration,
    },
};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};

use crate::sort_options::SortOptions;
use crate::types::{LineIndex, RuleDiagnostic, RuleDiagnosticData, RuleDiagnosticFix};

/// Compiled regexes keyed by their configured `(pattern, flags)` pair.
type RegexCache = RwLock<FastHashMap<(CompactString, CompactString), Option<Regex>>>;

#[derive(Clone, Copy)]
pub(crate) struct RuleContract {
    pub(crate) rule: &'static str,
    pub(crate) selector: &'static str,
    pub(crate) order_message_id: &'static str,
    pub(crate) group_order_message_id: &'static str,
    pub(crate) extra_spacing_message_id: &'static str,
    pub(crate) missed_spacing_message_id: &'static str,
    pub(crate) missed_comment_above_message_id: Option<&'static str>,
}

const IMPORT_CONTRACT: RuleContract = RuleContract {
    rule: "sort-named-imports",
    selector: "import",
    order_message_id: "unexpectedNamedImportsOrder",
    group_order_message_id: "unexpectedNamedImportsGroupOrder",
    extra_spacing_message_id: "extraSpacingBetweenNamedImports",
    missed_spacing_message_id: "missedSpacingBetweenNamedImports",
    missed_comment_above_message_id: None,
};

const EXPORT_CONTRACT: RuleContract = RuleContract {
    rule: "sort-named-exports",
    selector: "export",
    order_message_id: "unexpectedNamedExportsOrder",
    group_order_message_id: "unexpectedNamedExportsGroupOrder",
    extra_spacing_message_id: "extraSpacingBetweenNamedExports",
    missed_spacing_message_id: "missedSpacingBetweenNamedExports",
    missed_comment_above_message_id: None,
};

const SORT_EXPORTS_CONTRACT: RuleContract = RuleContract {
    rule: "sort-exports",
    selector: "export",
    order_message_id: "unexpectedExportsOrder",
    group_order_message_id: "unexpectedExportsGroupOrder",
    extra_spacing_message_id: "extraSpacingBetweenExports",
    missed_spacing_message_id: "missedSpacingBetweenExports",
    missed_comment_above_message_id: Some("missedCommentAboveExport"),
};

const SORT_IMPORTS_CONTRACT: RuleContract = RuleContract {
    rule: "sort-imports",
    selector: "import",
    order_message_id: "unexpectedImportsOrder",
    group_order_message_id: "unexpectedImportsGroupOrder",
    extra_spacing_message_id: "extraSpacingBetweenImports",
    missed_spacing_message_id: "missedSpacingBetweenImports",
    missed_comment_above_message_id: Some("missedCommentAboveImport"),
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Newlines {
    Ignore,
    Count(usize),
    Between,
}

#[derive(Debug)]
enum GroupEntry {
    Group {
        names: SmallVec<[CompactString; 2]>,
        overrides: Option<Map<String, Value>>,
        newlines_inside: Option<Newlines>,
        comment_above: Option<CompactString>,
    },
    Newlines(Newlines),
}

#[derive(Debug)]
struct CustomMatch {
    element_name_pattern: Option<Value>,
    modifiers: SmallVec<[CompactString; 2]>,
    selector: Option<CompactString>,
}

#[derive(Debug)]
struct CustomGroup {
    group_name: CompactString,
    matches: SmallVec<[CustomMatch; 2]>,
    overrides: Map<String, Value>,
    newlines_inside: Option<Newlines>,
}

#[derive(Debug)]
pub(crate) struct RuleOptions {
    raw: Map<String, Value>,
    sort: SortOptions,
    groups: Vec<GroupEntry>,
    custom_groups: Vec<CustomGroup>,
    partition_by_comment: Value,
    partition_by_new_line: bool,
    newlines_between: Newlines,
    newlines_inside: Newlines,
}

pub(crate) struct SortableNode<'a> {
    pub(crate) span: Span,
    pub(crate) name: CompactString,
    pub(crate) compare_name: CompactString,
    pub(crate) source: &'a str,
    pub(crate) source_start: u32,
    pub(crate) source_end: u32,
    pub(crate) size: usize,
    pub(crate) group: CompactString,
    pub(crate) group_index: usize,
    pub(crate) partition_id: usize,
    pub(crate) is_disabled: bool,
    pub(crate) is_ignored: bool,
    pub(crate) preserve_order_in_group: bool,
    pub(crate) is_type_import: bool,
    pub(crate) dependencies: SmallVec<[CompactString; 2]>,
    pub(crate) dependency_names: SmallVec<[CompactString; 4]>,
    pub(crate) add_safety_semicolon_when_inline: bool,
    pub(crate) use_original_groups_for_spacing: bool,
    pub(crate) requires_comma_separator: bool,
}

struct PendingDiagnostic {
    message_id: &'static str,
    right_index: usize,
    left_index: Option<usize>,
    missed_comment_above: Option<CompactString>,
    node_dependent_on_right: Option<CompactString>,
}

enum ExportDeclarationRef<'a> {
    Named(&'a ExportNamedDeclaration<'a>),
    All(&'a ExportAllDeclaration<'a>),
}

enum ImportDeclarationRef<'a> {
    Es(&'a ImportDeclaration<'a>),
    Equals(&'a TSImportEqualsDeclaration<'a>),
    Require(&'a VariableDeclaration<'a>),
}

impl ExportDeclarationRef<'_> {
    fn span(&self) -> Span {
        match self {
            Self::Named(declaration) => declaration.span,
            Self::All(declaration) => declaration.span,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Named(declaration) => declaration
                .source
                .as_ref()
                .map_or("", |source| source.value.as_str()),
            Self::All(declaration) => declaration.source.value.as_str(),
        }
    }

    fn export_kind(&self) -> ImportOrExportKind {
        match self {
            Self::Named(declaration) => declaration.export_kind,
            Self::All(declaration) => declaration.export_kind,
        }
    }

    fn export_type(&self) -> &'static str {
        match self {
            Self::Named(_) => "named",
            Self::All(_) => "wildcard",
        }
    }
}

impl ImportDeclarationRef<'_> {
    fn span(&self) -> Span {
        match self {
            Self::Es(declaration) => declaration.span,
            Self::Equals(declaration) => declaration.span,
            Self::Require(declaration) => declaration.span,
        }
    }
}

pub(crate) fn check(
    source_text: &str,
    body: &[Statement<'_>],
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let lines = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();

    for statement in body {
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };
        let Some(specifiers) = declaration.specifiers.as_ref() else {
            continue;
        };
        let named_specifiers: SmallVec<[&ImportSpecifier<'_>; 8]> = specifiers
            .iter()
            .filter_map(|specifier| {
                let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                    return None;
                };
                Some(&**specifier)
            })
            .collect();
        if named_specifiers.len() < 2 {
            continue;
        }

        let selected = select_import_options(raw_options, declaration, &named_specifiers);
        let options = RuleOptions::from_object(selected);
        let mut specifiers: SmallVec<[SortableNode<'_>; 8]> = named_specifiers
            .iter()
            .enumerate()
            .filter_map(|(index, specifier)| {
                let boundary = named_specifiers
                    .get(index + 1)
                    .map_or(declaration.span.end, |next| next.span.start);
                named_import(
                    source_text,
                    comments,
                    declaration,
                    specifier,
                    boundary,
                    &options,
                    IMPORT_CONTRACT,
                )
            })
            .collect();
        if specifiers.len() < 2 {
            continue;
        }

        check_specifiers(
            source_text,
            comments,
            &options,
            &mut specifiers,
            IMPORT_CONTRACT,
            &lines,
            &mut diagnostics,
        );
    }
    diagnostics
}

pub(crate) fn check_exports(
    source_text: &str,
    body: &[Statement<'_>],
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let lines = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();

    for statement in body {
        let Statement::ExportNamedDeclaration(declaration) = statement else {
            continue;
        };
        if declaration.specifiers.len() < 2 {
            continue;
        }
        let named_specifiers: SmallVec<[&ExportSpecifier<'_>; 8]> =
            declaration.specifiers.iter().collect();
        let selected = select_export_options(raw_options, declaration, &named_specifiers);
        let options = RuleOptions::from_object(selected);
        let mut specifiers: SmallVec<[SortableNode<'_>; 8]> = named_specifiers
            .iter()
            .enumerate()
            .filter_map(|(index, specifier)| {
                let boundary = named_specifiers
                    .get(index + 1)
                    .map_or(declaration.span.end, |next| next.span.start);
                named_export(
                    source_text,
                    comments,
                    declaration,
                    specifier,
                    boundary,
                    &options,
                    EXPORT_CONTRACT,
                )
            })
            .collect();
        if specifiers.len() < 2 {
            continue;
        }

        check_specifiers(
            source_text,
            comments,
            &options,
            &mut specifiers,
            EXPORT_CONTRACT,
            &lines,
            &mut diagnostics,
        );
    }
    diagnostics
}

pub(crate) fn check_sort_imports(
    source_text: &str,
    body: &[Statement<'_>],
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let options = RuleOptions::from_object(sort_import_options(raw_options));
    let lines = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();
    let mut block: SmallVec<[ImportDeclarationRef<'_>; 16]> = SmallVec::new();

    for statement in body {
        if let Some(declaration) = import_declaration_ref(statement) {
            block.push(declaration);
            continue;
        }
        check_import_block(
            source_text,
            comments,
            &options,
            &block,
            &lines,
            &mut diagnostics,
        );
        block.clear();
    }
    check_import_block(
        source_text,
        comments,
        &options,
        &block,
        &lines,
        &mut diagnostics,
    );
    diagnostics
}

fn check_import_block<'a>(
    source_text: &'a str,
    comments: &[Comment],
    options: &RuleOptions,
    declarations: &[ImportDeclarationRef<'a>],
    lines: &LineIndex,
    diagnostics: &mut SmallVec<[RuleDiagnostic; 8]>,
) {
    if declarations.is_empty() {
        return;
    }
    let explicit_groups: SmallVec<[&str; 32]> = options.group_names().collect();
    let sort_side_effects = options
        .raw
        .get("sortSideEffects")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sort_by_specifier = options.raw.get("sortBy").and_then(Value::as_str) == Some("specifier");
    let max_line_length = options
        .raw
        .get("maxLineLength")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(usize::MAX);

    let mut nodes: SmallVec<[SortableNode<'a>; 16]> = declarations
        .iter()
        .enumerate()
        .filter_map(|(index, declaration)| {
            let span = declaration.span();
            let boundary = declarations
                .get(index + 1)
                .map_or(source_text.len() as u32, |next| next.span().start);
            let name = import_declaration_name(source_text, declaration)?;
            let specifier_name = import_specifier_name(source_text, declaration);
            let modifiers = import_modifiers(source_text, declaration);
            let selectors = import_selectors(options, name.as_str(), declaration, &modifiers);
            let group = compute_group_with_selectors(
                options,
                name.as_str(),
                modifiers.as_slice(),
                selectors.as_slice(),
            );
            let group_index = options.group_index(group.as_str());
            let source_start = movable_leading_comment_start(source_text, comments, span, options);
            let source_end = comments
                .iter()
                .filter(|comment| {
                    comment.span.start >= span.end
                        && comment.span.end <= boundary
                        && is_same_line(source_text, span.end, comment.span.start)
                })
                .map(|comment| comment.span.end)
                .max()
                .unwrap_or(span.end);
            let source = source_text
                .get(usize::try_from(source_start).ok()?..usize::try_from(source_end).ok()?)?;
            let node_source = source_text
                .get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)?;
            let is_side_effect = modifiers.contains(&"side-effect");
            let is_style_side_effect =
                is_side_effect && selectors.contains(&"side-effect-style");
            let regroup_side_effect = explicit_groups.contains(&"side-effect");
            let regroup_style_side_effect = explicit_groups.contains(&"side-effect-style");
            let is_ignored = !sort_side_effects
                && is_side_effect
                && !regroup_side_effect
                && (!is_style_side_effect || !regroup_style_side_effect);
            let mut size = node_source
                .strip_suffix(';')
                .unwrap_or(node_source)
                .encode_utf16()
                .count();
            if matches!(declaration, ImportDeclarationRef::Es(value) if value.specifiers.as_ref().is_some_and(|specifiers| specifiers.len() > 1))
                && size > max_line_length
            {
                size = name.encode_utf16().count() + 10;
            }
            let compare_name = if sort_by_specifier {
                specifier_name.unwrap_or_else(|| CompactString::new(""))
            } else {
                name.clone()
            };
            Some(SortableNode {
                span,
                name,
                compare_name,
                source,
                source_start,
                source_end,
                size,
                group,
                group_index,
                partition_id: 0,
                is_disabled: is_rule_disabled(
                    source_text,
                    comments,
                    span,
                    SORT_IMPORTS_CONTRACT.rule,
                ),
                is_ignored,
                preserve_order_in_group: !sort_side_effects && is_side_effect,
                is_type_import: modifiers.contains(&"type"),
                dependencies: import_dependencies(source_text, declaration),
                dependency_names: import_dependency_names(source_text, declaration),
                add_safety_semicolon_when_inline: true,
                use_original_groups_for_spacing: false,
                requires_comma_separator: false,
            })
        })
        .collect();
    if nodes.is_empty() {
        return;
    }
    check_specifiers(
        source_text,
        comments,
        options,
        &mut nodes,
        SORT_IMPORTS_CONTRACT,
        lines,
        diagnostics,
    );
}

pub(crate) fn check_sort_exports(
    source_text: &str,
    body: &[Statement<'_>],
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let selected = match raw_options {
        Value::Array(values) => values
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
        Value::Object(object) => object.clone(),
        _ => Map::new(),
    };
    let options = RuleOptions::from_object(selected);
    let declarations: SmallVec<[ExportDeclarationRef<'_>; 16]> = body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ExportNamedDeclaration(declaration) if declaration.source.is_some() => {
                Some(ExportDeclarationRef::Named(declaration))
            }
            Statement::ExportAllDeclaration(declaration) => {
                Some(ExportDeclarationRef::All(declaration))
            }
            _ => None,
        })
        .collect();
    if declarations.is_empty() {
        return SmallVec::new();
    }

    let mut nodes: SmallVec<[SortableNode<'_>; 16]> = declarations
        .iter()
        .enumerate()
        .filter_map(|(index, declaration)| {
            let span = declaration.span();
            let boundary = declarations
                .get(index + 1)
                .map_or(source_text.len() as u32, |next| next.span().start);
            let source_start = movable_leading_comment_start(source_text, comments, span, &options);
            let source_end = comments
                .iter()
                .filter(|comment| {
                    comment.span.start >= span.end
                        && comment.span.end <= boundary
                        && is_same_line(source_text, span.end, comment.span.start)
                })
                .map(|comment| comment.span.end)
                .max()
                .unwrap_or(span.end);
            let source = source_text
                .get(usize::try_from(source_start).ok()?..usize::try_from(source_end).ok()?)?;
            let node_source = source_text
                .get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)?;
            let kind = if declaration.export_kind() == ImportOrExportKind::Type {
                "type"
            } else {
                "value"
            };
            let line_count = if node_source
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
            {
                "multiline"
            } else {
                "singleline"
            };
            let modifiers = [kind, declaration.export_type(), line_count];
            let name = CompactString::from(declaration.name());
            let group = compute_group(
                &options,
                name.as_str(),
                &modifiers,
                SORT_EXPORTS_CONTRACT.selector,
            );
            let group_index = options.group_index(group.as_str());
            Some(SortableNode {
                span,
                compare_name: name.clone(),
                name,
                source,
                source_start,
                source_end,
                size: node_source
                    .strip_suffix(';')
                    .unwrap_or(node_source)
                    .encode_utf16()
                    .count(),
                group,
                group_index,
                partition_id: 0,
                is_disabled: is_rule_disabled(
                    source_text,
                    comments,
                    span,
                    SORT_EXPORTS_CONTRACT.rule,
                ),
                is_ignored: false,
                preserve_order_in_group: false,
                is_type_import: false,
                dependencies: SmallVec::new(),
                dependency_names: SmallVec::new(),
                add_safety_semicolon_when_inline: true,
                use_original_groups_for_spacing: true,
                requires_comma_separator: false,
            })
        })
        .collect();
    if nodes.is_empty() {
        return SmallVec::new();
    }

    let lines = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();
    check_specifiers(
        source_text,
        comments,
        &options,
        &mut nodes,
        SORT_EXPORTS_CONTRACT,
        &lines,
        &mut diagnostics,
    );
    diagnostics
}

fn sort_import_options(raw_options: &Value) -> Map<String, Value> {
    let mut selected = match raw_options {
        Value::Array(values) => values
            .first()
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
        Value::Object(object) => object.clone(),
        _ => Map::new(),
    };
    let defaults = serde_json::json!({
        "groups": [
            "type-import",
            ["value-builtin", "value-external"],
            "type-internal",
            "value-internal",
            ["type-parent", "type-sibling", "type-index"],
            ["value-parent", "value-sibling", "value-index"],
            "ts-equals-import",
            "unknown"
        ],
        "internalPattern": ["^~/.+", "^@/.+", "^#.+"],
        "useExperimentalDependencyDetection": true,
        "fallbackSort": { "type": "unsorted" },
        "partitionByComment": false,
        "partitionByNewLine": false,
        "specialCharacters": "keep",
        "sortSideEffects": false,
        "type": "alphabetical",
        "environment": "node",
        "newlinesBetween": 1,
        "newlinesInside": 0,
        "customGroups": [],
        "ignoreCase": true,
        "locales": "en-US",
        "sortBy": "path",
        "alphabet": "",
        "order": "asc"
    });
    if let Some(defaults) = defaults.as_object() {
        for (key, value) in defaults {
            selected.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    selected
}

fn import_declaration_ref<'a>(statement: &'a Statement<'a>) -> Option<ImportDeclarationRef<'a>> {
    match statement {
        Statement::ImportDeclaration(declaration) => Some(ImportDeclarationRef::Es(declaration)),
        Statement::TSImportEqualsDeclaration(declaration) => {
            Some(ImportDeclarationRef::Equals(declaration))
        }
        Statement::VariableDeclaration(declaration) if is_require_declaration(declaration) => {
            Some(ImportDeclarationRef::Require(declaration))
        }
        _ => None,
    }
}

fn is_require_declaration(declaration: &VariableDeclaration<'_>) -> bool {
    require_literal(declaration).is_some()
}

fn require_literal<'a>(declaration: &'a VariableDeclaration<'a>) -> Option<&'a str> {
    let initializer = declaration.declarations.first()?.init.as_ref()?;
    let Expression::CallExpression(call) = initializer else {
        return None;
    };
    let Expression::Identifier(callee) = &call.callee else {
        return None;
    };
    if callee.name != "require" {
        return None;
    }
    let Argument::StringLiteral(literal) = call.arguments.first()? else {
        return None;
    };
    Some(literal.value.as_str())
}

fn import_declaration_name(
    source_text: &str,
    declaration: &ImportDeclarationRef<'_>,
) -> Option<CompactString> {
    match declaration {
        ImportDeclarationRef::Es(declaration) => {
            Some(CompactString::from(declaration.source.value.as_str()))
        }
        ImportDeclarationRef::Equals(declaration) => match &declaration.module_reference {
            TSModuleReference::ExternalModuleReference(reference) => {
                Some(CompactString::from(reference.expression.value.as_str()))
            }
            reference => source_for_span(source_text, reference.span()).map(CompactString::from),
        },
        ImportDeclarationRef::Require(declaration) => {
            require_literal(declaration).map(CompactString::from)
        }
    }
}

fn import_specifier_name(
    source_text: &str,
    declaration: &ImportDeclarationRef<'_>,
) -> Option<CompactString> {
    match declaration {
        ImportDeclarationRef::Es(declaration) => {
            let specifier = declaration.specifiers.as_ref()?.first()?;
            let name = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    specifier.local.name.as_str()
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                    specifier.local.name.as_str()
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                    specifier.local.name.as_str()
                }
            };
            Some(CompactString::from(name))
        }
        ImportDeclarationRef::Equals(declaration) => {
            Some(CompactString::from(declaration.id.name.as_str()))
        }
        ImportDeclarationRef::Require(declaration) => declaration
            .declarations
            .first()
            .and_then(|declarator| binding_specifier_name(source_text, &declarator.id)),
    }
}

fn binding_specifier_name(
    source_text: &str,
    pattern: &BindingPattern<'_>,
) -> Option<CompactString> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => {
            Some(CompactString::from(identifier.name.as_str()))
        }
        BindingPattern::ObjectPattern(pattern) => {
            if let Some(property) = pattern.properties.first() {
                source_for_span(source_text, property.value.span()).map(CompactString::from)
            } else {
                pattern.rest.as_ref().and_then(|rest| {
                    source_for_span(source_text, rest.argument.span()).map(CompactString::from)
                })
            }
        }
        BindingPattern::ArrayPattern(pattern) => pattern
            .elements
            .first()
            .and_then(Option::as_ref)
            .and_then(|element| {
                source_for_span(source_text, element.span()).map(CompactString::from)
            })
            .or_else(|| {
                pattern.rest.as_ref().and_then(|rest| {
                    source_for_span(source_text, rest.argument.span()).map(CompactString::from)
                })
            }),
        BindingPattern::AssignmentPattern(pattern) => {
            binding_specifier_name(source_text, &pattern.left)
        }
    }
}

fn import_modifiers<'a>(
    source_text: &str,
    declaration: &ImportDeclarationRef<'a>,
) -> SmallVec<[&'static str; 12]> {
    let mut modifiers = SmallVec::new();
    let is_type = match declaration {
        ImportDeclarationRef::Es(declaration) => {
            declaration.import_kind == ImportOrExportKind::Type
        }
        ImportDeclarationRef::Equals(declaration) => {
            declaration.import_kind == ImportOrExportKind::Type
        }
        ImportDeclarationRef::Require(_) => false,
    };
    modifiers.push(if is_type { "type" } else { "value" });
    match declaration {
        ImportDeclarationRef::Es(declaration) => {
            let specifiers = declaration.specifiers.as_ref();
            if specifiers.is_none_or(|specifiers| specifiers.is_empty())
                && !source_for_span(source_text, declaration.span)
                    .unwrap_or("")
                    .contains("} from")
            {
                modifiers.push("side-effect");
            }
            if specifiers
                .into_iter()
                .flat_map(|value| value.iter())
                .any(|specifier| {
                    matches!(
                        specifier,
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                    )
                })
            {
                modifiers.push("default");
            }
            if specifiers
                .into_iter()
                .flat_map(|value| value.iter())
                .any(|specifier| {
                    matches!(
                        specifier,
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(_)
                    )
                })
            {
                modifiers.push("wildcard");
            }
            if specifiers
                .into_iter()
                .flat_map(|value| value.iter())
                .any(|specifier| {
                    matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(_))
                })
            {
                modifiers.push("named");
            }
        }
        ImportDeclarationRef::Equals(_) => modifiers.push("ts-equals"),
        ImportDeclarationRef::Require(_) => modifiers.push("require"),
    }
    let span = declaration.span();
    let multiline = source_for_span(source_text, span).is_some_and(|source| {
        source
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    });
    modifiers.push(if multiline { "multiline" } else { "singleline" });
    modifiers
}

fn import_selectors<'a>(
    options: &RuleOptions,
    name: &str,
    declaration: &ImportDeclarationRef<'a>,
    modifiers: &[&'static str],
) -> SmallVec<[&'static str; 12]> {
    let mut selectors = SmallVec::new();
    if modifiers.contains(&"type") {
        selectors.push("type");
    }
    let is_side_effect = modifiers.contains(&"side-effect");
    let is_style = is_style_import(name);
    if is_side_effect && is_style {
        selectors.push("side-effect-style");
    }
    if is_side_effect {
        selectors.push("side-effect");
    }
    if is_style {
        selectors.push("style");
    }
    if !matches!(
        declaration,
        ImportDeclarationRef::Equals(value)
            if !matches!(value.module_reference, TSModuleReference::ExternalModuleReference(_))
    ) {
        if options.raw.contains_key("tsconfig") && name.starts_with('$') {
            selectors.push("tsconfig-path");
        }
        if is_index_import(name) {
            selectors.push("index");
        }
        if name.starts_with("./") {
            selectors.push("sibling");
        }
        if name.starts_with("..") {
            selectors.push("parent");
        }
        if name.starts_with('#') {
            selectors.push("subpath");
        }
        let internal = (options.raw.contains_key("tsconfig") && name == "sort-imports")
            || options
                .raw
                .get("internalPattern")
                .is_some_and(|patterns| matches_regex(name, patterns));
        if internal {
            selectors.push("internal");
        }
        if is_builtin_import(
            name,
            options
                .raw
                .get("environment")
                .and_then(Value::as_str)
                .unwrap_or("node"),
        ) {
            selectors.push("builtin");
        }
        if !internal && !name.starts_with('.') && !name.starts_with('/') && !name.starts_with('$') {
            selectors.push("external");
        }
    }
    selectors.push("import");
    selectors
}

fn is_style_import(name: &str) -> bool {
    let clean = name.split('?').next().unwrap_or(name);
    [".less", ".scss", ".sass", ".styl", ".pcss", ".css", ".sss"]
        .iter()
        .any(|extension| clean.ends_with(extension))
}

fn is_index_import(name: &str) -> bool {
    matches!(
        name,
        "./index.d.js" | "./index.d.ts" | "./index.js" | "./index.ts" | "./index" | "./" | "."
    )
}

fn is_builtin_import(name: &str, environment: &str) -> bool {
    let clean = name.trim_start_matches("node:");
    let base = clean.split('/').next().unwrap_or(clean);
    const NODE_BUILTINS: &[&str] = &[
        "assert",
        "assert/strict",
        "async_hooks",
        "buffer",
        "child_process",
        "cluster",
        "console",
        "constants",
        "crypto",
        "dgram",
        "diagnostics_channel",
        "dns",
        "domain",
        "events",
        "fs",
        "http",
        "http2",
        "https",
        "module",
        "net",
        "os",
        "path",
        "perf_hooks",
        "process",
        "punycode",
        "querystring",
        "readline",
        "repl",
        "stream",
        "string_decoder",
        "sys",
        "timers",
        "tls",
        "trace_events",
        "tty",
        "url",
        "util",
        "v8",
        "vm",
        "wasi",
        "worker_threads",
        "zlib",
    ];
    NODE_BUILTINS.contains(&clean)
        || NODE_BUILTINS.contains(&base)
        || matches!(name, "node:sqlite" | "node:test" | "node:sea")
        || (environment == "bun"
            && matches!(
                name,
                "detect-libc"
                    | "bun:sqlite"
                    | "bun:test"
                    | "bun:wrap"
                    | "bun:ffi"
                    | "bun:jsc"
                    | "undici"
                    | "bun"
                    | "ws"
            ))
}

fn import_dependencies(
    source_text: &str,
    declaration: &ImportDeclarationRef<'_>,
) -> SmallVec<[CompactString; 2]> {
    let ImportDeclarationRef::Equals(declaration) = declaration else {
        return SmallVec::new();
    };
    let TSModuleReference::QualifiedName(reference) = &declaration.module_reference else {
        return SmallVec::new();
    };
    let source = source_for_span(source_text, reference.span).unwrap_or("");
    source
        .split('.')
        .next()
        .filter(|name| !name.is_empty())
        .map_or_else(SmallVec::new, |name| {
            SmallVec::from_vec(vec![CompactString::from(name)])
        })
}

fn import_dependency_names(
    _source_text: &str,
    declaration: &ImportDeclarationRef<'_>,
) -> SmallVec<[CompactString; 4]> {
    match declaration {
        ImportDeclarationRef::Es(declaration) => declaration
            .specifiers
            .as_ref()
            .into_iter()
            .flat_map(|specifiers| specifiers.iter())
            .map(|specifier| match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    module_export_name(&specifier.imported)
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                    CompactString::from(specifier.local.name.as_str())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                    CompactString::from(specifier.local.name.as_str())
                }
            })
            .collect(),
        ImportDeclarationRef::Equals(declaration) => {
            SmallVec::from_vec(vec![CompactString::from(declaration.id.name.as_str())])
        }
        ImportDeclarationRef::Require(_) => SmallVec::new(),
    }
}

fn source_for_span(source_text: &str, span: Span) -> Option<&str> {
    source_text.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
}

pub(crate) fn check_specifiers(
    source_text: &str,
    comments: &[Comment],
    options: &RuleOptions,
    specifiers: &mut [SortableNode<'_>],
    contract: RuleContract,
    lines: &LineIndex,
    diagnostics: &mut SmallVec<[RuleDiagnostic; 8]>,
) {
    assign_partitions(source_text, comments, options, specifiers);
    let sorted_indices = sort_specifiers(options, specifiers, false);
    let fix_indices = sort_specifiers(options, specifiers, true);
    let mut sorted_positions = vec![0; specifiers.len()];
    for (position, &original_index) in sorted_indices.iter().enumerate() {
        sorted_positions[original_index] = position;
    }

    let mut pending: SmallVec<[PendingDiagnostic; 8]> = SmallVec::new();
    if let Some(message_id) = contract.missed_comment_above_message_id
        && !specifiers[0].is_disabled
        && let Some(comment) =
            missing_comment_above(source_text, comments, options, None, &specifiers[0])
    {
        pending.push(PendingDiagnostic {
            message_id,
            right_index: 0,
            left_index: None,
            missed_comment_above: Some(comment),
            node_dependent_on_right: None,
        });
    }
    for right_index in 1..specifiers.len() {
        let left_index = right_index - 1;
        let left = &specifiers[left_index];
        let right = &specifiers[right_index];

        let right_fix_position = fix_indices
            .iter()
            .position(|index| *index == right_index)
            .unwrap_or(right_index);
        if !right.is_disabled
            && (sorted_positions[left_index] > sorted_positions[right_index]
                || (left.is_disabled && sorted_positions[left_index] >= right_fix_position))
        {
            let dependent = dependency_violation(specifiers, right_index);
            let message_id = if dependent.is_some() {
                "unexpectedImportsDependencyOrder"
            } else if left.group_index == right.group_index {
                contract.order_message_id
            } else {
                contract.group_order_message_id
            };
            pending.push(PendingDiagnostic {
                message_id,
                right_index,
                left_index: Some(left_index),
                missed_comment_above: None,
                node_dependent_on_right: dependent,
            });
        }

        if !left.is_disabled
            && !right.is_disabled
            && left.partition_id == right.partition_id
            && left.group_index <= right.group_index
            && let Newlines::Count(expected) = newlines_between(options, left, right)
        {
            let actual = empty_lines_between(source_text, left.source_end, right.source_start);
            if actual < expected {
                pending.push(PendingDiagnostic {
                    message_id: contract.missed_spacing_message_id,
                    right_index,
                    left_index: Some(left_index),
                    missed_comment_above: None,
                    node_dependent_on_right: None,
                });
            } else if actual > expected {
                pending.push(PendingDiagnostic {
                    message_id: contract.extra_spacing_message_id,
                    right_index,
                    left_index: Some(left_index),
                    missed_comment_above: None,
                    node_dependent_on_right: None,
                });
            }
        }
        if let Some(message_id) = contract.missed_comment_above_message_id
            && !right.is_disabled
            && let Some(comment) = missing_comment_above(
                source_text,
                comments,
                options,
                Some(left.group_index),
                right,
            )
        {
            pending.push(PendingDiagnostic {
                message_id,
                right_index,
                left_index: Some(left_index),
                missed_comment_above: Some(comment),
                node_dependent_on_right: None,
            });
        }
    }
    if pending.is_empty() {
        return;
    }

    let fix = build_fix(source_text, options, specifiers, &fix_indices)
        .or_else(|| build_comment_fix(source_text, comments, options, specifiers, &fix_indices));
    let Some(fix) = fix else {
        return;
    };
    for pending in pending {
        let PendingDiagnostic {
            message_id,
            right_index,
            left_index,
            missed_comment_above,
            node_dependent_on_right,
        } = pending;
        let right = &specifiers[right_index];
        let left = left_index.map(|index| &specifiers[index]);
        let is_group_error = message_id == contract.group_order_message_id;
        diagnostics.push(RuleDiagnostic {
            rule_name: contract.rule,
            message_id,
            data: RuleDiagnosticData {
                left: if missed_comment_above.is_some() || node_dependent_on_right.is_some() {
                    CompactString::new("")
                } else {
                    left.map_or_else(|| CompactString::new(""), |node| node.name.clone())
                },
                right: right.name.clone(),
                left_group: is_group_error.then(|| {
                    left.map_or_else(|| CompactString::new(""), |node| node.group.clone())
                }),
                right_group: is_group_error.then(|| right.group.clone()),
                missed_comment_above,
                node_dependent_on_right,
            },
            loc: lines.loc_for_span(source_text, right.span),
            fix: fix.clone(),
        });
    }
}

impl RuleOptions {
    pub(crate) fn from_object(raw: Map<String, Value>) -> Self {
        let value = Value::Object(raw.clone());
        let groups = raw
            .get("groups")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |groups| {
                groups.iter().filter_map(parse_group).collect()
            });
        let custom_groups = raw
            .get("customGroups")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |groups| {
                groups.iter().filter_map(parse_custom_group).collect()
            });
        Self {
            sort: SortOptions::from_json(&value),
            groups,
            custom_groups,
            partition_by_comment: raw
                .get("partitionByComment")
                .cloned()
                .unwrap_or(Value::Bool(false)),
            partition_by_new_line: raw
                .get("partitionByNewLine")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            newlines_between: raw
                .get("newlinesBetween")
                .map_or(Newlines::Ignore, parse_newlines),
            newlines_inside: raw
                .get("newlinesInside")
                .map_or(Newlines::Between, parse_newlines),
            raw,
        }
    }

    fn group_names(&self) -> impl Iterator<Item = &str> {
        self.groups.iter().flat_map(|group| match group {
            GroupEntry::Group { names, .. } => names.iter().map(CompactString::as_str).collect(),
            GroupEntry::Newlines(_) => Vec::new(),
        })
    }

    pub(crate) fn group_index(&self, group_name: &str) -> usize {
        self.groups
            .iter()
            .position(|group| match group {
                GroupEntry::Group { names, .. } => {
                    names.iter().any(|name| name.as_str() == group_name)
                }
                GroupEntry::Newlines(_) => false,
            })
            .unwrap_or(self.groups.len())
    }

    fn sort_options_for_group(&self, group_index: usize) -> (SortOptions, Value) {
        let mut merged = self.raw.clone();
        if let Some(GroupEntry::Group {
            overrides: Some(overrides),
            ..
        }) = self.groups.get(group_index)
        {
            merge_sort_overrides(&mut merged, overrides);
        }
        let group_name = self.groups.get(group_index).and_then(|entry| match entry {
            GroupEntry::Group { names, .. } if names.len() == 1 => {
                names.first().map(CompactString::as_str)
            }
            _ => None,
        });
        if let Some(custom_group) = group_name.and_then(|name| {
            self.custom_groups
                .iter()
                .find(|custom| custom.group_name.as_str() == name)
        }) {
            merge_sort_overrides(&mut merged, &custom_group.overrides);
        }
        let value = Value::Object(merged);
        (SortOptions::from_json(&value), value)
    }
}

fn select_import_options(
    raw_options: &Value,
    declaration: &ImportDeclaration<'_>,
    specifiers: &[&ImportSpecifier<'_>],
) -> Map<String, Value> {
    let candidates: Vec<&Map<String, Value>> = match raw_options {
        Value::Array(values) => values.iter().filter_map(Value::as_object).collect(),
        Value::Object(object) => vec![object],
        _ => Vec::new(),
    };
    for candidate in candidates {
        let Some(condition) = candidate
            .get("useConfigurationIf")
            .and_then(Value::as_object)
        else {
            return candidate.clone();
        };
        let ignore_alias = candidate
            .get("ignoreAlias")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if let Some(pattern) = condition.get("allNamesMatchPattern")
            && !specifiers.iter().all(|specifier| {
                let name = import_name(specifier, ignore_alias);
                matches_regex(name.as_str(), pattern)
            })
        {
            continue;
        }
        if let Some(selector) = condition.get("matchesAstSelector").and_then(Value::as_str)
            && !matches_import_selector(selector, declaration)
        {
            continue;
        }
        return candidate.clone();
    }
    Map::new()
}

fn select_export_options(
    raw_options: &Value,
    declaration: &ExportNamedDeclaration<'_>,
    specifiers: &[&ExportSpecifier<'_>],
) -> Map<String, Value> {
    let candidates: Vec<&Map<String, Value>> = match raw_options {
        Value::Array(values) => values.iter().filter_map(Value::as_object).collect(),
        Value::Object(object) => vec![object],
        _ => Vec::new(),
    };
    for candidate in candidates {
        let Some(condition) = candidate
            .get("useConfigurationIf")
            .and_then(Value::as_object)
        else {
            return candidate.clone();
        };
        let ignore_alias = candidate
            .get("ignoreAlias")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if let Some(pattern) = condition.get("allNamesMatchPattern")
            && !specifiers.iter().all(|specifier| {
                let name = export_name(specifier, ignore_alias);
                matches_regex(name.as_str(), pattern)
            })
        {
            continue;
        }
        if let Some(selector) = condition.get("matchesAstSelector").and_then(Value::as_str)
            && !matches_export_selector(selector, declaration)
        {
            continue;
        }
        return candidate.clone();
    }
    Map::new()
}

fn matches_import_selector(selector: &str, declaration: &ImportDeclaration<'_>) -> bool {
    let selector = selector.trim();
    if matches!(
        selector,
        "ImportDeclaration"
            | "* > ImportDeclaration"
            | "Program > ImportDeclaration"
            | "Program ImportDeclaration"
    ) {
        return true;
    }
    if let Some(attribute) = selector
        .strip_prefix("ImportDeclaration[")
        .and_then(|value| value.strip_suffix(']'))
    {
        for operator in ["=", "=="] {
            if let Some((field, expected)) = attribute.split_once(operator) {
                let expected = expected.trim().trim_matches(['\'', '"']);
                if matches!(field.trim(), "source.value" | "source.raw") {
                    return declaration.source.value.as_str() == expected;
                }
            }
        }
    }
    false
}

fn matches_export_selector(selector: &str, declaration: &ExportNamedDeclaration<'_>) -> bool {
    let selector = selector.trim();
    if matches!(
        selector,
        "ExportNamedDeclaration"
            | "* > ExportNamedDeclaration"
            | "Program > ExportNamedDeclaration"
            | "Program ExportNamedDeclaration"
    ) {
        return true;
    }
    if let Some(attribute) = selector
        .strip_prefix("ExportNamedDeclaration[")
        .and_then(|value| value.strip_suffix(']'))
    {
        for operator in ["=", "=="] {
            if let Some((field, expected)) = attribute.split_once(operator) {
                let expected = expected.trim().trim_matches(['\'', '"']);
                if matches!(field.trim(), "source.value" | "source.raw") {
                    return declaration
                        .source
                        .as_ref()
                        .is_some_and(|source| source.value.as_str() == expected);
                }
            }
        }
    }
    false
}

fn parse_group(value: &Value) -> Option<GroupEntry> {
    match value {
        Value::String(name) => Some(GroupEntry::Group {
            names: SmallVec::from_vec(vec![CompactString::from(name.as_str())]),
            overrides: None,
            newlines_inside: None,
            comment_above: None,
        }),
        Value::Array(names) => Some(GroupEntry::Group {
            names: names
                .iter()
                .filter_map(Value::as_str)
                .map(CompactString::from)
                .collect(),
            overrides: None,
            newlines_inside: None,
            comment_above: None,
        }),
        Value::Object(object) if object.contains_key("group") => {
            let names = match object.get("group") {
                Some(Value::String(name)) => {
                    SmallVec::from_vec(vec![CompactString::from(name.as_str())])
                }
                Some(Value::Array(names)) => names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(CompactString::from)
                    .collect(),
                _ => SmallVec::new(),
            };
            Some(GroupEntry::Group {
                names,
                overrides: Some(object.clone()),
                newlines_inside: object.get("newlinesInside").map(parse_newlines),
                comment_above: object
                    .get("commentAbove")
                    .and_then(Value::as_str)
                    .map(CompactString::from),
            })
        }
        Value::Object(object) => object
            .get("newlinesBetween")
            .map(|newlines| GroupEntry::Newlines(parse_newlines(newlines))),
        _ => None,
    }
}

fn parse_custom_group(value: &Value) -> Option<CustomGroup> {
    let object = value.as_object()?;
    let group_name = CompactString::from(object.get("groupName")?.as_str()?);
    let matches: SmallVec<[CustomMatch; 2]> =
        if let Some(any_of) = object.get("anyOf").and_then(Value::as_array) {
            any_of.iter().filter_map(parse_custom_match).collect()
        } else {
            SmallVec::from_vec(vec![parse_custom_match(value)?])
        };
    Some(CustomGroup {
        group_name,
        matches,
        overrides: object.clone(),
        newlines_inside: object.get("newlinesInside").map(parse_newlines),
    })
}

fn parse_custom_match(value: &Value) -> Option<CustomMatch> {
    let object = value.as_object()?;
    Some(CustomMatch {
        element_name_pattern: object.get("elementNamePattern").cloned(),
        modifiers: object
            .get("modifiers")
            .and_then(Value::as_array)
            .map_or_else(SmallVec::new, |modifiers| {
                modifiers
                    .iter()
                    .filter_map(Value::as_str)
                    .map(CompactString::from)
                    .collect()
            }),
        selector: object
            .get("selector")
            .and_then(Value::as_str)
            .map(CompactString::from),
    })
}

fn named_import<'a>(
    source_text: &'a str,
    comments: &[Comment],
    declaration: &ImportDeclaration<'_>,
    specifier: &ImportSpecifier<'_>,
    boundary: u32,
    options: &RuleOptions,
    contract: RuleContract,
) -> Option<SortableNode<'a>> {
    let source_start =
        movable_leading_comment_start(source_text, comments, specifier.span, options);
    let source_end = comments
        .iter()
        .filter(|comment| {
            comment.span.start >= specifier.span.end
                && comment.span.end <= boundary
                && is_same_line(source_text, specifier.span.end, comment.span.start)
        })
        .map(|comment| comment.span.end)
        .max()
        .unwrap_or(specifier.span.end);
    let source =
        source_text.get(usize::try_from(source_start).ok()?..usize::try_from(source_end).ok()?)?;
    let node_source = source_text.get(
        usize::try_from(specifier.span.start).ok()?..usize::try_from(specifier.span.end).ok()?,
    )?;
    let name = import_name(specifier, options.sort.ignore_alias);
    let modifier = if declaration.import_kind == ImportOrExportKind::Type
        || specifier.import_kind == ImportOrExportKind::Type
    {
        "type"
    } else {
        "value"
    };
    let group = compute_group(options, name.as_str(), &[modifier], contract.selector);
    let group_index = options.group_index(group.as_str());
    Some(SortableNode {
        span: specifier.span,
        compare_name: name.clone(),
        name,
        source,
        source_start,
        source_end,
        size: node_source.encode_utf16().count(),
        group,
        group_index,
        partition_id: 0,
        is_disabled: false,
        is_ignored: false,
        preserve_order_in_group: false,
        is_type_import: false,
        dependencies: SmallVec::new(),
        dependency_names: SmallVec::new(),
        add_safety_semicolon_when_inline: false,
        use_original_groups_for_spacing: false,
        requires_comma_separator: false,
    })
}

fn named_export<'a>(
    source_text: &'a str,
    comments: &[Comment],
    declaration: &ExportNamedDeclaration<'_>,
    specifier: &ExportSpecifier<'_>,
    boundary: u32,
    options: &RuleOptions,
    contract: RuleContract,
) -> Option<SortableNode<'a>> {
    let source_start =
        movable_leading_comment_start(source_text, comments, specifier.span, options);
    let source_end = comments
        .iter()
        .filter(|comment| {
            comment.span.start >= specifier.span.end
                && comment.span.end <= boundary
                && is_same_line(source_text, specifier.span.end, comment.span.start)
        })
        .map(|comment| comment.span.end)
        .max()
        .unwrap_or(specifier.span.end);
    let source =
        source_text.get(usize::try_from(source_start).ok()?..usize::try_from(source_end).ok()?)?;
    let node_source = source_text.get(
        usize::try_from(specifier.span.start).ok()?..usize::try_from(specifier.span.end).ok()?,
    )?;
    let name = export_name(specifier, options.sort.ignore_alias);
    let modifier = if declaration.export_kind == ImportOrExportKind::Type
        || specifier.export_kind == ImportOrExportKind::Type
    {
        "type"
    } else {
        "value"
    };
    let group = compute_group(options, name.as_str(), &[modifier], contract.selector);
    let group_index = options.group_index(group.as_str());
    Some(SortableNode {
        span: specifier.span,
        compare_name: name.clone(),
        name,
        source,
        source_start,
        source_end,
        size: node_source.encode_utf16().count(),
        group,
        group_index,
        partition_id: 0,
        is_disabled: false,
        is_ignored: false,
        preserve_order_in_group: false,
        is_type_import: false,
        dependencies: SmallVec::new(),
        dependency_names: SmallVec::new(),
        add_safety_semicolon_when_inline: false,
        use_original_groups_for_spacing: false,
        requires_comma_separator: false,
    })
}

pub(crate) fn movable_leading_comment_start(
    source_text: &str,
    comments: &[Comment],
    specifier_span: Span,
    options: &RuleOptions,
) -> u32 {
    let mut leading: SmallVec<[&Comment; 4]> = comments
        .iter()
        .filter(|comment| {
            comment.attached_to == specifier_span.start && comment.span.end <= specifier_span.start
        })
        .collect();
    leading.sort_by_key(|comment| comment.span.start);
    let mut start = specifier_span.start;
    for comment in leading.into_iter().rev() {
        if is_partition_comment(source_text, comment, &options.partition_by_comment)
            || is_eslint_block_directive(source_text, comment)
            || empty_lines_between(source_text, comment.span.end, start) > 0
        {
            break;
        }
        start = comment.span.start;
    }
    start
}

pub(crate) fn is_rule_disabled(
    source_text: &str,
    comments: &[Comment],
    span: Span,
    rule_name: &str,
) -> bool {
    let node_line = line_number_at(source_text, span.start);
    let mut block_disabled = false;
    let mut ordered_comments: SmallVec<[&Comment; 16]> = comments.iter().collect();
    ordered_comments.sort_by_key(|comment| comment.span.start);
    for comment in ordered_comments {
        let content = comment_content(source_text, comment);
        if comment.span.end <= span.start {
            if let Some(rules) = content
                .strip_prefix("eslint-disable ")
                .or_else(|| (content == "eslint-disable").then_some(""))
            {
                if eslint_directive_applies(rules, rule_name) {
                    block_disabled = true;
                }
            } else if let Some(rules) = content
                .strip_prefix("eslint-enable ")
                .or_else(|| (content == "eslint-enable").then_some(""))
                && eslint_directive_applies(rules, rule_name)
            {
                block_disabled = false;
            }
        }
        let comment_line = line_number_at(source_text, comment.span.start);
        if let Some(rules) = content.strip_prefix("eslint-disable-next-line")
            && comment_line + 1 == node_line
            && eslint_directive_applies(rules, rule_name)
        {
            return true;
        }
        if let Some(rules) = content.strip_prefix("eslint-disable-line")
            && comment_line == node_line
            && eslint_directive_applies(rules, rule_name)
        {
            return true;
        }
    }
    block_disabled
}

fn is_eslint_block_directive(source_text: &str, comment: &Comment) -> bool {
    let content = comment_content(source_text, comment);
    content.starts_with("eslint-disable") || content.starts_with("eslint-enable")
}

fn comment_content<'a>(source_text: &'a str, comment: &Comment) -> &'a str {
    let content_span = comment.content_span();
    source_text
        .get(
            usize::try_from(content_span.start).unwrap_or(0)
                ..usize::try_from(content_span.end).unwrap_or(0),
        )
        .unwrap_or("")
        .trim()
}

fn eslint_directive_applies(rules: &str, rule_name: &str) -> bool {
    let rules = rules.trim().trim_start_matches(|character: char| {
        character == ':' || character == '-' || character.is_whitespace()
    });
    rules.is_empty()
        || rules.split(',').any(|rule| {
            let rule = rule.split_whitespace().next().unwrap_or("");
            rule == rule_name
                || rule
                    .strip_prefix("perfectionist/")
                    .or_else(|| rule.strip_prefix("rule-to-test/"))
                    .or_else(|| rule.strip_prefix("@perfectionist/"))
                    == Some(rule_name)
        })
}

fn line_number_at(source_text: &str, offset: u32) -> u32 {
    let offset = usize::try_from(offset)
        .unwrap_or(source_text.len())
        .min(source_text.len());
    source_text[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count() as u32
        + 1
}

pub(crate) fn compute_group(
    options: &RuleOptions,
    name: &str,
    modifiers: &[&str],
    selector: &str,
) -> CompactString {
    let configured: SmallVec<[&str; 8]> = options.group_names().collect();
    for custom in &options.custom_groups {
        if !configured
            .iter()
            .any(|configured_name| *configured_name == custom.group_name.as_str())
        {
            continue;
        }
        if custom
            .matches
            .iter()
            .any(|matcher| custom_match(matcher, name, modifiers, selector))
        {
            return custom.group_name.clone();
        }
    }
    let mut predefined = predefined_groups(modifiers, selector);
    predefined.push(selector.to_owned());
    for predefined in predefined {
        if configured.contains(&predefined.as_str()) {
            return CompactString::from(predefined);
        }
    }
    CompactString::new("unknown")
}

fn compute_group_with_selectors(
    options: &RuleOptions,
    name: &str,
    modifiers: &[&str],
    selectors: &[&str],
) -> CompactString {
    let configured: SmallVec<[&str; 32]> = options.group_names().collect();
    for custom in &options.custom_groups {
        if !configured
            .iter()
            .any(|configured_name| *configured_name == custom.group_name.as_str())
        {
            continue;
        }
        if custom.matches.iter().any(|matcher| {
            matcher
                .selector
                .as_ref()
                .is_none_or(|selector| selectors.contains(&selector.as_str()))
                && matcher
                    .modifiers
                    .iter()
                    .all(|modifier| modifiers.contains(&modifier.as_str()))
                && matcher
                    .element_name_pattern
                    .as_ref()
                    .is_none_or(|pattern| matches_regex(name, pattern))
        }) {
            return custom.group_name.clone();
        }
    }
    for selector in selectors {
        if *selector == "import"
            && modifiers.contains(&"side-effect")
            && configured.contains(&"side-effect-import")
        {
            return CompactString::from("side-effect-import");
        }
        for predefined in predefined_groups(modifiers, selector) {
            if configured.contains(&predefined.as_str()) {
                return CompactString::from(predefined);
            }
        }
        if configured.contains(selector) {
            return CompactString::from(*selector);
        }
    }
    CompactString::new("unknown")
}

fn custom_match(matcher: &CustomMatch, name: &str, modifiers: &[&str], selector: &str) -> bool {
    if matcher
        .selector
        .as_ref()
        .is_some_and(|candidate| candidate.as_str() != selector)
    {
        return false;
    }
    if !matcher.modifiers.is_empty()
        && !matcher
            .modifiers
            .iter()
            .all(|candidate| modifiers.contains(&candidate.as_str()))
    {
        return false;
    }
    matcher
        .element_name_pattern
        .as_ref()
        .is_none_or(|pattern| matches_regex(name, pattern))
}

fn predefined_groups(modifiers: &[&str], selector: &str) -> Vec<String> {
    let mut groups = Vec::new();
    for size in (1..=modifiers.len()).rev() {
        let mut combination = Vec::with_capacity(size);
        collect_modifier_combinations(modifiers, selector, size, 0, &mut combination, &mut groups);
    }
    groups
}

fn collect_modifier_combinations(
    modifiers: &[&str],
    selector: &str,
    size: usize,
    start: usize,
    combination: &mut Vec<usize>,
    groups: &mut Vec<String>,
) {
    if combination.len() == size {
        let mut permutation = combination.clone();
        collect_modifier_permutations(modifiers, selector, &mut permutation, 0, groups);
        return;
    }
    for index in start..modifiers.len() {
        combination.push(index);
        collect_modifier_combinations(modifiers, selector, size, index + 1, combination, groups);
        combination.pop();
    }
}

fn collect_modifier_permutations(
    modifiers: &[&str],
    selector: &str,
    permutation: &mut [usize],
    first: usize,
    groups: &mut Vec<String>,
) {
    if first == permutation.len() {
        let mut group = permutation
            .iter()
            .map(|index| modifiers[*index])
            .collect::<Vec<_>>()
            .join("-");
        group.push('-');
        group.push_str(selector);
        groups.push(group);
        return;
    }
    for index in first..permutation.len() {
        permutation.swap(first, index);
        collect_modifier_permutations(modifiers, selector, permutation, first + 1, groups);
        permutation.swap(first, index);
    }
}

fn assign_partitions(
    source_text: &str,
    comments: &[Comment],
    options: &RuleOptions,
    specifiers: &mut [SortableNode<'_>],
) {
    let mut partition_id = 1;
    for index in 0..specifiers.len() {
        if index > 0 {
            let has_partition_comment = comments.iter().any(|comment| {
                comment.attached_to == specifiers[index].span.start
                    && comment.span.end <= specifiers[index].span.start
                    && is_partition_comment(source_text, comment, &options.partition_by_comment)
            });
            let has_partition_newline = options.partition_by_new_line
                && empty_lines_between(
                    source_text,
                    specifiers[index - 1].source_end,
                    specifiers[index].source_start,
                ) > 0;
            if has_partition_comment || has_partition_newline {
                partition_id += 1;
            }
        }
        specifiers[index].partition_id = partition_id;
    }
}

/// Merges the group and custom-group sort overrides once per group present in
/// the declaration, so comparisons never rebuild option maps or collators
/// inside `sort_by`.
fn group_sort_options(
    options: &RuleOptions,
    specifiers: &[SortableNode<'_>],
) -> Vec<Option<(SortOptions, Value)>> {
    let mut cached: Vec<Option<(SortOptions, Value)>> =
        (0..=options.groups.len()).map(|_| None).collect();
    for specifier in specifiers {
        let Some(slot) = cached.get_mut(specifier.group_index) else {
            continue;
        };
        if slot.is_none() {
            *slot = Some(options.sort_options_for_group(specifier.group_index));
        }
    }
    cached
}

fn sort_specifiers(
    options: &RuleOptions,
    specifiers: &[SortableNode<'_>],
    keep_disabled_in_place: bool,
) -> Vec<usize> {
    let group_options = group_sort_options(options, specifiers);
    let mut sorted = Vec::with_capacity(specifiers.len());
    let mut start = 0;
    while start < specifiers.len() {
        let partition = specifiers[start].partition_id;
        let mut end = start + 1;
        while end < specifiers.len() && specifiers[end].partition_id == partition {
            end += 1;
        }
        let frozen_indices: Vec<usize> = (start..end)
            .filter(|index| {
                specifiers[*index].is_ignored
                    || (keep_disabled_in_place && specifiers[*index].is_disabled)
            })
            .collect();
        let mut partition_indices: Vec<usize> = (start..end)
            .filter(|index| {
                !specifiers[*index].is_ignored
                    && (!keep_disabled_in_place || !specifiers[*index].is_disabled)
            })
            .collect();
        partition_indices.sort_by(|&left, &right| {
            specifiers[left]
                .group_index
                .cmp(&specifiers[right].group_index)
                .then_with(|| {
                    compare_in_group(
                        options,
                        &group_options,
                        &specifiers[left],
                        &specifiers[right],
                    )
                })
                .then_with(|| left.cmp(&right))
        });
        for frozen_index in frozen_indices {
            partition_indices.insert(frozen_index - start, frozen_index);
        }
        sorted.extend(partition_indices);
        start = end;
    }
    apply_dependency_order(specifiers, &mut sorted, keep_disabled_in_place);
    sorted
}

fn apply_dependency_order(
    nodes: &[SortableNode<'_>],
    sorted: &mut Vec<usize>,
    keep_disabled_in_place: bool,
) {
    let mut passes = 0;
    while passes < nodes.len() {
        let mut changed = false;
        for dependent_position in 0..sorted.len() {
            let dependent_index = sorted[dependent_position];
            let dependent = &nodes[dependent_index];
            if dependent.dependencies.is_empty()
                || dependent.is_ignored
                || (keep_disabled_in_place && dependent.is_disabled)
            {
                continue;
            }
            let provider_position = ((dependent_position + 1)..sorted.len()).find(|position| {
                let provider = &nodes[sorted[*position]];
                !provider.is_ignored
                    && (!keep_disabled_in_place || !provider.is_disabled)
                    && dependent.dependencies.iter().any(|dependency| {
                        provider
                            .dependency_names
                            .iter()
                            .any(|provided| provided == dependency)
                    })
            });
            let Some(provider_position) = provider_position else {
                continue;
            };
            let provider = sorted.remove(provider_position);
            sorted.insert(dependent_position, provider);
            changed = true;
            break;
        }
        if !changed {
            break;
        }
        passes += 1;
    }
}

fn dependency_violation(
    nodes: &[SortableNode<'_>],
    provider_index: usize,
) -> Option<CompactString> {
    let provider = &nodes[provider_index];
    nodes[..provider_index].iter().rev().find_map(|dependent| {
        dependent
            .dependencies
            .iter()
            .any(|dependency| {
                provider
                    .dependency_names
                    .iter()
                    .any(|provided| provided == dependency)
            })
            .then(|| dependent.name.clone())
    })
}

fn compare_in_group(
    options: &RuleOptions,
    group_options: &[Option<(SortOptions, Value)>],
    left: &SortableNode<'_>,
    right: &SortableNode<'_>,
) -> Ordering {
    if left.preserve_order_in_group || right.preserve_order_in_group {
        return Ordering::Equal;
    }
    let Some((sort, raw)) = group_options.get(left.group_index).and_then(Option::as_ref) else {
        return Ordering::Equal;
    };
    let object = raw.as_object();
    let primary = object
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("alphabetical");
    if primary == "type-import-first" {
        return compare_type_imports(
            left,
            right,
            object
                .and_then(|value| value.get("order"))
                .and_then(Value::as_str),
        );
    }
    if primary == "unsorted" {
        return Ordering::Equal;
    }
    if primary == "subgroup-order" {
        let subgroup = subgroup_compare(options, left, right, raw.as_object());
        if subgroup != Ordering::Equal {
            return subgroup;
        }
        return fallback_compare(options, left, right, raw.as_object());
    }
    let compared = sort.compare(
        left.compare_name.as_str(),
        left.size,
        right.compare_name.as_str(),
        right.size,
    );
    if compared == Ordering::Equal {
        let fallback = object
            .and_then(|value| value.get("fallbackSort"))
            .and_then(Value::as_object);
        match fallback
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
        {
            Some("subgroup-order") => subgroup_compare(options, left, right, fallback),
            Some("type-import-first") => compare_type_imports(
                left,
                right,
                fallback
                    .and_then(|value| value.get("order"))
                    .and_then(Value::as_str),
            ),
            _ => compared,
        }
    } else {
        compared
    }
}

fn compare_type_imports(
    left: &SortableNode<'_>,
    right: &SortableNode<'_>,
    order: Option<&str>,
) -> Ordering {
    let ordering = match (left.is_type_import, right.is_type_import) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => Ordering::Equal,
    };
    if order == Some("desc") {
        ordering.reverse()
    } else {
        ordering
    }
}

fn fallback_compare(
    options: &RuleOptions,
    left: &SortableNode<'_>,
    right: &SortableNode<'_>,
    raw: Option<&Map<String, Value>>,
) -> Ordering {
    let Some(fallback) = raw
        .and_then(|value| value.get("fallbackSort"))
        .and_then(Value::as_object)
    else {
        return Ordering::Equal;
    };
    if fallback.get("type").and_then(Value::as_str) == Some("subgroup-order") {
        return subgroup_compare(options, left, right, Some(fallback));
    }
    let mut value = raw.cloned().unwrap_or_default();
    for (key, item) in fallback {
        value.insert(key.clone(), item.clone());
    }
    value.insert(
        "fallbackSort".to_owned(),
        serde_json::json!({ "type": "unsorted" }),
    );
    SortOptions::from_json(&Value::Object(value)).compare(
        left.compare_name.as_str(),
        left.size,
        right.compare_name.as_str(),
        right.size,
    )
}

fn subgroup_compare(
    options: &RuleOptions,
    left: &SortableNode<'_>,
    right: &SortableNode<'_>,
    raw: Option<&Map<String, Value>>,
) -> Ordering {
    let Some(GroupEntry::Group { names, .. }) = options.groups.get(left.group_index) else {
        return Ordering::Equal;
    };
    if names.len() < 2 {
        return Ordering::Equal;
    }
    let ordering = names
        .iter()
        .position(|name| name == left.group)
        .cmp(&names.iter().position(|name| name == right.group));
    if raw
        .and_then(|value| value.get("order"))
        .and_then(Value::as_str)
        == Some("desc")
    {
        ordering.reverse()
    } else {
        ordering
    }
}

fn newlines_between(
    options: &RuleOptions,
    left: &SortableNode<'_>,
    right: &SortableNode<'_>,
) -> Newlines {
    if left.group_index == right.group_index {
        if let Some(custom) = options
            .custom_groups
            .iter()
            .find(|custom| custom.group_name == left.group)
            && let Some(newlines) = custom.newlines_inside
        {
            return newlines;
        }
        if let Some(GroupEntry::Group {
            newlines_inside: Some(newlines),
            ..
        }) = options.groups.get(left.group_index)
        {
            return *newlines;
        }
        return match options.newlines_inside {
            Newlines::Between => match options.newlines_between {
                Newlines::Ignore => Newlines::Ignore,
                _ => Newlines::Count(0),
            },
            other => other,
        };
    }
    if right.group_index == left.group_index + 2
        && let Some(GroupEntry::Newlines(newlines)) = options.groups.get(left.group_index + 1)
    {
        return *newlines;
    }
    if right.group_index > left.group_index + 2 {
        let mut maximum = None;
        let mut ignored = false;
        // `group_index` yields `groups.len()` for the implicit `unknown` group,
        // so the inclusive end has to be clamped to the configured entries.
        let end = right
            .group_index
            .saturating_add(1)
            .min(options.groups.len());
        let relevant = options
            .groups
            .get(left.group_index..end)
            .unwrap_or_default();
        for (index, entry) in relevant.iter().enumerate() {
            let configured = match entry {
                GroupEntry::Newlines(newlines) => Some(*newlines),
                GroupEntry::Group { .. }
                    if index > 0 && matches!(relevant[index - 1], GroupEntry::Group { .. }) =>
                {
                    Some(options.newlines_between)
                }
                GroupEntry::Group { .. } => None,
            };
            if let Some(newlines) = configured {
                match newlines {
                    Newlines::Count(value) => {
                        maximum = Some(maximum.map_or(value, |current: usize| current.max(value)));
                    }
                    Newlines::Ignore => ignored = true,
                    Newlines::Between => {}
                }
            }
        }
        if maximum.is_some_and(|value| value >= 1) {
            return Newlines::Count(maximum.unwrap_or(0));
        }
        if ignored {
            return Newlines::Ignore;
        }
        return Newlines::Count(0);
    }
    options.newlines_between
}

fn missing_comment_above(
    source_text: &str,
    comments: &[Comment],
    options: &RuleOptions,
    left_group_index: Option<usize>,
    right: &SortableNode<'_>,
) -> Option<CompactString> {
    if left_group_index.is_some_and(|left| left >= right.group_index) {
        return None;
    }
    let Some(GroupEntry::Group {
        comment_above: Some(expected),
        ..
    }) = options.groups.get(right.group_index)
    else {
        return None;
    };
    let expected_lowercase = expected.trim().to_lowercase();
    let exists = comments.iter().any(|comment| {
        comment.attached_to == right.span.start
            && comment.span.end <= right.span.start
            && comment_content(source_text, comment)
                .to_lowercase()
                .contains(&expected_lowercase)
    });
    (!exists).then(|| expected.clone())
}

struct CommentEdit {
    start: u32,
    end: u32,
    replacement: CompactString,
}

fn build_comment_fix(
    source_text: &str,
    comments: &[Comment],
    options: &RuleOptions,
    nodes: &[SortableNode<'_>],
    sorted_indices: &[usize],
) -> Option<RuleDiagnosticFix> {
    let configured_comments: SmallVec<[&str; 8]> = options
        .groups
        .iter()
        .filter_map(|entry| match entry {
            GroupEntry::Group {
                comment_above: Some(comment),
                ..
            } => Some(comment.as_str()),
            _ => None,
        })
        .collect();
    let mut edits: Vec<CommentEdit> = Vec::new();

    for position in 0..sorted_indices.len() {
        let node = &nodes[*sorted_indices.get(position)?];
        let left_group_index = position
            .checked_sub(1)
            .and_then(|left| sorted_indices.get(left))
            .map(|index| nodes[*index].group_index);
        let expected = if left_group_index.is_some_and(|left| left >= node.group_index) {
            None
        } else {
            options
                .groups
                .get(node.group_index)
                .and_then(|entry| match entry {
                    GroupEntry::Group { comment_above, .. } => comment_above.as_ref(),
                    GroupEntry::Newlines(_) => None,
                })
        };
        let mut attached: SmallVec<[&Comment; 8]> = comments
            .iter()
            .filter(|comment| {
                comment.attached_to == node.span.start && comment.span.end <= node.span.start
            })
            .collect();
        attached.sort_by_key(|comment| comment.span.start);
        let mut leading: SmallVec<[&Comment; 8]> = SmallVec::new();
        let mut cursor = node.span.start;
        for comment in attached.into_iter().rev() {
            if empty_lines_between(source_text, comment.span.end, cursor) > 0 {
                break;
            }
            leading.push(comment);
            cursor = comment.span.start;
        }
        leading.reverse();
        let expected_exists = expected.is_some_and(|expected| {
            let expected = expected.trim().to_lowercase();
            leading.iter().any(|comment| {
                comment_content(source_text, comment)
                    .to_lowercase()
                    .contains(&expected)
            })
        });
        let mismatched_auto: SmallVec<[&Comment; 4]> = leading
            .iter()
            .copied()
            .filter(|comment| {
                let content = comment_content(source_text, comment);
                comment.kind == CommentKind::Line
                    && configured_comments.contains(&content.trim_start_matches('/').trim())
                    && expected.is_none_or(|expected| content.trim() != expected.as_str())
            })
            .collect();

        if let Some(expected) = expected.filter(|_| !expected_exists) {
            let insertion = leading
                .first()
                .map_or(node.span.start, |comment| comment.span.start);
            let replacement_comment = if left_group_index.is_none() {
                mismatched_auto.first().copied()
            } else {
                mismatched_auto
                    .first()
                    .copied()
                    .filter(|comment| comment.span.start == insertion)
            };
            if let Some(comment) = replacement_comment {
                edits.push(CommentEdit {
                    start: comment.span.start,
                    end: next_comment_or_node_start(&leading, comment, node.span.start),
                    replacement: CompactString::from(format!("// {expected}\n")),
                });
                for comment in mismatched_auto
                    .iter()
                    .copied()
                    .filter(|candidate| candidate.span.start != comment.span.start)
                {
                    edits.push(CommentEdit {
                        start: comment.span.start,
                        end: next_comment_or_node_start(&leading, comment, node.span.start),
                        replacement: CompactString::new(""),
                    });
                }
            } else {
                edits.push(CommentEdit {
                    start: insertion,
                    end: insertion,
                    replacement: CompactString::from(format!("// {expected}\n")),
                });
                for comment in mismatched_auto {
                    edits.push(CommentEdit {
                        start: comment.span.start,
                        end: next_comment_or_node_start(&leading, comment, node.span.start),
                        replacement: CompactString::new(""),
                    });
                }
            }
        } else {
            for comment in mismatched_auto {
                edits.push(CommentEdit {
                    start: comment.span.start,
                    end: next_comment_or_node_start(&leading, comment, node.span.start),
                    replacement: CompactString::new(""),
                });
            }
        }
    }
    if edits.is_empty() {
        return None;
    }
    edits.sort_by_key(|edit| (edit.start, edit.end));
    let start = edits.first()?.start;
    let end = edits.iter().map(|edit| edit.end).max()?;
    let mut replacement = CompactString::new("");
    let mut cursor = start;
    for edit in edits {
        if edit.start < cursor {
            continue;
        }
        replacement.push_str(
            source_text.get(usize::try_from(cursor).ok()?..usize::try_from(edit.start).ok()?)?,
        );
        replacement.push_str(edit.replacement.as_str());
        cursor = edit.end;
    }
    replacement
        .push_str(source_text.get(usize::try_from(cursor).ok()?..usize::try_from(end).ok()?)?);
    Some(RuleDiagnosticFix {
        start: LineIndex::utf16_offset(source_text, start),
        end: LineIndex::utf16_offset(source_text, end),
        replacement,
    })
}

fn next_comment_or_node_start(leading: &[&Comment], comment: &Comment, node_start: u32) -> u32 {
    leading
        .iter()
        .find(|candidate| candidate.span.start > comment.span.start)
        .map_or(node_start, |candidate| candidate.span.start)
}

fn build_fix(
    source_text: &str,
    options: &RuleOptions,
    specifiers: &[SortableNode<'_>],
    sorted_indices: &[usize],
) -> Option<RuleDiagnosticFix> {
    let mut replacement = CompactString::new("");
    let mut desired_source_starts = Vec::with_capacity(specifiers.len());
    let mut desired_source_ends = Vec::with_capacity(specifiers.len());
    let mut changed_start: Option<u32> = None;
    let mut changed_end: Option<u32> = None;
    for position in 0..specifiers.len() {
        desired_source_starts.push(replacement.len());
        let sorted = &specifiers[*sorted_indices.get(position)?];
        replacement.push_str(sorted.source);
        if position + 1 < specifiers.len()
            && sorted.add_safety_semicolon_when_inline
            && is_same_line(
                source_text,
                specifiers[position].span.end,
                specifiers[position + 1].span.start,
            )
            && !node_ends_with_safe_character(source_text, sorted)
        {
            let between = source_text.get(
                usize::try_from(specifiers[position].span.end).ok()?
                    ..usize::try_from(specifiers[position + 1].span.start).ok()?,
            )?;
            if !between.trim_start().starts_with([';', ',']) {
                let insertion = replacement
                    .len()
                    .checked_sub(usize::try_from(sorted.source_end - sorted.span.end).ok()?)?;
                replacement.insert(insertion, ';');
            }
        }
        desired_source_ends.push(replacement.len());
        if sorted_indices[position] != position {
            changed_start = Some(
                changed_start.map_or(specifiers[position].source_start, |start| {
                    start.min(specifiers[position].source_start)
                }),
            );
            changed_end = Some(changed_end.map_or(specifiers[position].source_end, |end| {
                end.max(specifiers[position].source_end)
            }));
        }
        if position + 1 == specifiers.len() {
            continue;
        }
        let separator_start = usize::try_from(specifiers[position].source_end).ok()?;
        let separator_end = usize::try_from(specifiers[position + 1].source_start).ok()?;
        let separator = source_text.get(separator_start..separator_end)?;
        let sorted_left = &specifiers[*sorted_indices.get(position)?];
        let sorted_right = &specifiers[*sorted_indices.get(position + 1)?];
        let separator = if sorted_left.requires_comma_separator {
            array_separator(sorted_left, separator)
        } else {
            CompactString::from(separator)
        };
        let checks_original_groups = specifiers
            .iter()
            .any(|specifier| specifier.use_original_groups_for_spacing);
        let groups_allow_spacing = if checks_original_groups {
            specifiers[position].partition_id == specifiers[position + 1].partition_id
                && specifiers[position].group_index <= specifiers[position + 1].group_index
        } else {
            sorted_left.partition_id == sorted_right.partition_id
                && sorted_left.group_index <= sorted_right.group_index
        };
        // A separator after a moved declaration with an inline comment belongs
        // to that declaration in upstream's first fix pass. Leave it in place
        // so a later spacing diagnostic can normalize it independently.
        let separator_follows_moved_trailing_comment = sorted_indices[position] != position
            && (specifiers[position].source_end > specifiers[position].span.end
                || sorted_left.source_end > sorted_left.span.end);
        let desired_separator = if !separator_follows_moved_trailing_comment
            && groups_allow_spacing
            && let Newlines::Count(expected) = newlines_between(options, sorted_left, sorted_right)
        {
            normalize_separator(
                separator.as_str(),
                expected,
                is_same_line(
                    source_text,
                    specifiers[position].span.end,
                    specifiers[position + 1].span.start,
                ),
            )
        } else {
            separator.clone()
        };
        if desired_separator.as_str() != separator.as_str() {
            changed_start = Some(
                changed_start.map_or(specifiers[position].span.end, |start| {
                    start.min(specifiers[position].span.end)
                }),
            );
            changed_end = Some(
                changed_end.map_or(specifiers[position + 1].source_start, |end| {
                    end.max(specifiers[position + 1].source_start)
                }),
            );
        }
        replacement.push_str(desired_separator.as_str());
    }
    let changed_start = changed_start?;
    let changed_end = changed_end?;
    let replacement_start = desired_offset_for_boundary(
        changed_start,
        specifiers,
        &desired_source_starts,
        &desired_source_ends,
    )?;
    let mut replacement_end = desired_offset_for_boundary(
        changed_end,
        specifiers,
        &desired_source_starts,
        &desired_source_ends,
    )?;
    if specifiers.iter().any(|specifier| {
        specifier.requires_comma_separator
            && specifier.source_end == changed_end
            && specifier.source_end > specifier.span.end
    }) && replacement.as_bytes().get(replacement_end) == Some(&b',')
    {
        replacement_end += 1;
    }
    Some(RuleDiagnosticFix {
        start: LineIndex::utf16_offset(source_text, changed_start),
        end: LineIndex::utf16_offset(source_text, changed_end),
        replacement: CompactString::from(
            replacement
                .as_str()
                .get(replacement_start..replacement_end)?,
        ),
    })
}

fn array_separator(node: &SortableNode<'_>, separator: &str) -> CompactString {
    let embedded_comma = node
        .source
        .get(usize::try_from(node.span.end - node.source_start).unwrap_or(0)..)
        .is_some_and(|trailing| trailing.contains(','));
    let comma = separator.find(',');
    match (embedded_comma, comma) {
        (true, Some(index)) => {
            let mut normalized = CompactString::from(separator);
            normalized.remove(index);
            normalized
        }
        (false, None) => {
            let mut normalized = CompactString::new(",");
            normalized.push_str(separator);
            normalized
        }
        _ => CompactString::from(separator),
    }
}

fn node_ends_with_safe_character(source_text: &str, node: &SortableNode<'_>) -> bool {
    let Ok(start) = usize::try_from(node.span.start) else {
        return false;
    };
    let Ok(end) = usize::try_from(node.span.end) else {
        return false;
    };
    source_text
        .get(start..end)
        .is_some_and(|source| source.trim_end().ends_with([';', ',']))
}

fn desired_offset_for_boundary(
    boundary: u32,
    specifiers: &[SortableNode<'_>],
    desired_source_starts: &[usize],
    desired_source_ends: &[usize],
) -> Option<usize> {
    for (index, specifier) in specifiers.iter().enumerate() {
        if boundary == specifier.source_start {
            return desired_source_starts.get(index).copied();
        }
        if boundary == specifier.source_end {
            return desired_source_ends.get(index).copied();
        }
        if boundary == specifier.span.end {
            let offset =
                usize::try_from(specifier.span.end.checked_sub(specifier.source_start)?).ok()?;
            return desired_source_starts.get(index)?.checked_add(offset);
        }
    }
    let first_start = specifiers.first()?.source_start;
    usize::try_from(boundary.checked_sub(first_start)?).ok()
}

fn normalize_separator(separator: &str, newlines: usize, is_same_line: bool) -> CompactString {
    static BLANK_LINES: OnceLock<regex::Regex> = OnceLock::new();
    static NEWLINES: OnceLock<regex::Regex> = OnceLock::new();
    let blank_lines = BLANK_LINES.get_or_init(|| {
        regex::Regex::new(r"\n\s*\n").expect("static blank-line regex must compile")
    });
    let repeated_newlines = NEWLINES
        .get_or_init(|| regex::Regex::new(r"\n+").expect("static newline regex must compile"));
    let mut normalized = blank_lines.replace_all(separator, "\n").into_owned();
    normalized = repeated_newlines
        .replace_all(&normalized, "\n")
        .into_owned();
    if newlines == 0 {
        return CompactString::from(normalized);
    }
    for _ in 0..newlines + usize::from(is_same_line) {
        if let Some(index) = normalized.find('\n') {
            normalized.insert(index, '\n');
        } else {
            normalized.push('\n');
        }
    }
    CompactString::from(normalized)
}

fn merge_sort_overrides(target: &mut Map<String, Value>, overrides: &Map<String, Value>) {
    for key in [
        "type",
        "order",
        "ignoreCase",
        "specialCharacters",
        "locales",
        "alphabet",
        "sortBy",
    ] {
        if let Some(value) = overrides.get(key) {
            target.insert(key.to_owned(), value.clone());
        }
    }
    if let Some(Value::Object(fallback)) = overrides.get("fallbackSort") {
        let target_fallback = target
            .entry("fallbackSort")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(target_fallback) = target_fallback {
            for (key, value) in fallback {
                target_fallback.insert(key.clone(), value.clone());
            }
        }
    }
}

fn parse_newlines(value: &Value) -> Newlines {
    match value {
        Value::String(value) if value == "newlinesBetween" => Newlines::Between,
        Value::String(value) if value == "ignore" => Newlines::Ignore,
        Value::Number(value) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map_or(Newlines::Ignore, Newlines::Count),
        _ => Newlines::Ignore,
    }
}

fn import_name(specifier: &ImportSpecifier<'_>, ignore_alias: bool) -> CompactString {
    if ignore_alias {
        module_export_name(&specifier.imported)
    } else {
        CompactString::from(specifier.local.name.as_str())
    }
}

fn export_name(specifier: &ExportSpecifier<'_>, ignore_alias: bool) -> CompactString {
    if ignore_alias {
        module_export_name(&specifier.local)
    } else {
        module_export_name(&specifier.exported)
    }
}

pub(crate) fn matches_regex(value: &str, option: &Value) -> bool {
    match option {
        Value::Array(options) => options.iter().any(|option| matches_regex(value, option)),
        Value::String(pattern) => matches_single_regex(value, pattern, ""),
        Value::Object(object) => {
            object
                .get("pattern")
                .and_then(Value::as_str)
                .is_some_and(|pattern| {
                    matches_single_regex(
                        value,
                        pattern,
                        object.get("flags").and_then(Value::as_str).unwrap_or(""),
                    )
                })
        }
        _ => false,
    }
}

fn matches_single_regex(value: &str, pattern: &str, flags: &str) -> bool {
    // Rust's linear-time regex engine deliberately omits look-around. Support
    // the common Perfectionist negative-lookahead form without weakening the
    // rest of the matcher.
    if let Some(needle) = pattern
        .strip_prefix("^(?!.*")
        .and_then(|value| value.strip_suffix(").*$"))
    {
        return if flags.contains('i') {
            !value.to_lowercase().contains(&needle.to_lowercase())
        } else {
            !value.contains(needle)
        };
    }
    with_compiled_regex(pattern, flags, |regex| {
        regex.is_some_and(|regex| regex.is_match(value))
    })
}

/// Compiles each configured `(pattern, flags)` pair once and reuses it, because
/// group matching and conditional configuration re-test the same patterns for
/// every specifier of every import declaration.
fn with_compiled_regex<T>(
    pattern: &str,
    flags: &str,
    visit: impl FnOnce(Option<&Regex>) -> T,
) -> T {
    static CACHE: OnceLock<RegexCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(FastHashMap::default()));
    let key = (CompactString::from(pattern), CompactString::from(flags));
    if let Ok(compiled) = cache.read()
        && let Some(regex) = compiled.get(&key)
    {
        return visit(regex.as_ref());
    }
    let mut builder = RegexBuilder::new(pattern);
    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'));
    let compiled = builder.build().ok();
    let result = visit(compiled.as_ref());
    if let Ok(mut cache) = cache.write() {
        cache.insert(key, compiled);
    }
    result
}

fn is_partition_comment(source_text: &str, comment: &Comment, option: &Value) -> bool {
    if option == &Value::Bool(false) {
        return false;
    }
    let content_span = comment.content_span();
    let content = source_text
        .get(
            usize::try_from(content_span.start).unwrap_or(0)
                ..usize::try_from(content_span.end).unwrap_or(0),
        )
        .unwrap_or("")
        .trim();
    if content.starts_with("eslint-") {
        return false;
    }
    match option {
        Value::Bool(value) => *value,
        Value::String(_) | Value::Array(_) => matches_regex(content, option),
        Value::Object(object) if object.contains_key("pattern") => matches_regex(content, option),
        Value::Object(object) => {
            let relevant = match comment.kind {
                CommentKind::Line => object.get("line"),
                CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => object.get("block"),
            };
            relevant.is_some_and(|value| match value {
                Value::Bool(enabled) => *enabled,
                _ => matches_regex(content, value),
            })
        }
        _ => false,
    }
}

fn empty_lines_between(source_text: &str, left: u32, right: u32) -> usize {
    let Ok(left) = usize::try_from(left) else {
        return 0;
    };
    let Ok(right) = usize::try_from(right) else {
        return 0;
    };
    source_text
        .get(left..right)
        .map_or(0, |between| between.matches('\n').count().saturating_sub(1))
}

pub(crate) fn is_same_line(source_text: &str, left: u32, right: u32) -> bool {
    let Ok(left) = usize::try_from(left) else {
        return false;
    };
    let Ok(right) = usize::try_from(right) else {
        return false;
    };
    source_text.get(left..right).is_some_and(|between| {
        !between
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

fn module_export_name(name: &ModuleExportName<'_>) -> CompactString {
    match name {
        ModuleExportName::IdentifierName(identifier) => {
            CompactString::from(identifier.name.as_str())
        }
        ModuleExportName::IdentifierReference(identifier) => {
            CompactString::from(identifier.name.as_str())
        }
        ModuleExportName::StringLiteral(literal) => CompactString::from(literal.value.as_str()),
    }
}

#[cfg(test)]
mod sort_import_group_tests {
    use super::{RuleOptions, compute_group_with_selectors, sort_import_options};
    use serde_json::json;

    #[test]
    fn selects_side_effect_import_before_external() {
        let options = RuleOptions::from_object(sort_import_options(&json!([{
            "groups": ["side-effect-import", "external", "value-import"],
            "sortSideEffects": true
        }])));
        assert_eq!(
            compute_group_with_selectors(
                &options,
                "./z",
                &["value", "side-effect", "singleline"],
                &["side-effect", "sibling", "import"]
            ),
            "side-effect-import"
        );
    }
}
