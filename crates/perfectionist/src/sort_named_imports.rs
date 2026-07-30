#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "serde_json option maps require String keys and configured group/import collections are user-sized rather than bounded rule state."
)]

//! Configurable `sort-named-imports` slice pinned to Perfectionist v5.9.1.

use std::cmp::Ordering;
use std::sync::OnceLock;

use oxc_ast::{
    Comment, CommentKind,
    ast::{
        ImportDeclaration, ImportDeclarationSpecifier, ImportOrExportKind, ImportSpecifier,
        ModuleExportName, Statement,
    },
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::RegexBuilder;
use serde_json::{Map, Value};

use crate::sort_options::SortOptions;
use crate::types::{LineIndex, RuleDiagnostic, RuleDiagnosticData, RuleDiagnosticFix};

const RULE: &str = "sort-named-imports";
const ORDER_MESSAGE_ID: &str = "unexpectedNamedImportsOrder";
const GROUP_ORDER_MESSAGE_ID: &str = "unexpectedNamedImportsGroupOrder";
const EXTRA_SPACING_MESSAGE_ID: &str = "extraSpacingBetweenNamedImports";
const MISSED_SPACING_MESSAGE_ID: &str = "missedSpacingBetweenNamedImports";

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
struct RuleOptions {
    raw: Map<String, Value>,
    sort: SortOptions,
    groups: Vec<GroupEntry>,
    custom_groups: Vec<CustomGroup>,
    partition_by_comment: Value,
    partition_by_new_line: bool,
    newlines_between: Newlines,
    newlines_inside: Newlines,
}

struct NamedImport<'a> {
    span: Span,
    name: CompactString,
    source: &'a str,
    source_start: u32,
    source_end: u32,
    size: usize,
    group: CompactString,
    group_index: usize,
    partition_id: usize,
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

        let selected = select_options(raw_options, declaration, &named_specifiers);
        let options = RuleOptions::from_object(selected);
        let mut imports: SmallVec<[NamedImport<'_>; 8]> = named_specifiers
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
                )
            })
            .collect();
        if imports.len() < 2 {
            continue;
        }

        assign_partitions(source_text, comments, &options, &mut imports);
        let sorted_indices = sort_imports(&options, &imports);
        let mut sorted_positions = vec![0; imports.len()];
        for (position, &original_index) in sorted_indices.iter().enumerate() {
            sorted_positions[original_index] = position;
        }

        let mut pending: SmallVec<[(&'static str, usize, Option<usize>); 8]> = SmallVec::new();
        for right_index in 1..imports.len() {
            let left_index = right_index - 1;
            let left = &imports[left_index];
            let right = &imports[right_index];

            if sorted_positions[left_index] > sorted_positions[right_index] {
                let message_id = if left.group_index == right.group_index {
                    ORDER_MESSAGE_ID
                } else {
                    GROUP_ORDER_MESSAGE_ID
                };
                pending.push((message_id, right_index, Some(left_index)));
            }

            if left.partition_id == right.partition_id
                && left.group_index <= right.group_index
                && let Newlines::Count(expected) = newlines_between(&options, left, right)
            {
                let actual = empty_lines_between(source_text, left.source_end, right.source_start);
                if actual < expected {
                    pending.push((MISSED_SPACING_MESSAGE_ID, right_index, Some(left_index)));
                } else if actual > expected {
                    pending.push((EXTRA_SPACING_MESSAGE_ID, right_index, Some(left_index)));
                }
            }
        }
        if pending.is_empty() {
            continue;
        }

        let Some(fix) = build_fix(source_text, &options, &imports, &sorted_indices) else {
            continue;
        };
        for (message_id, right_index, left_index) in pending {
            let right = &imports[right_index];
            let left = left_index.map(|index| &imports[index]);
            let is_group_error = message_id == GROUP_ORDER_MESSAGE_ID;
            diagnostics.push(RuleDiagnostic {
                rule_name: RULE,
                message_id,
                data: RuleDiagnosticData {
                    left: left.map_or_else(|| CompactString::new(""), |node| node.name.clone()),
                    right: right.name.clone(),
                    left_group: is_group_error.then(|| {
                        left.map_or_else(|| CompactString::new(""), |node| node.group.clone())
                    }),
                    right_group: is_group_error.then(|| right.group.clone()),
                },
                loc: lines.loc_for_span(source_text, right.span),
                fix: fix.clone(),
            });
        }
    }
    diagnostics
}

impl RuleOptions {
    fn from_object(raw: Map<String, Value>) -> Self {
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

    fn group_index(&self, group_name: &str) -> usize {
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

fn select_options(
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

fn parse_group(value: &Value) -> Option<GroupEntry> {
    match value {
        Value::String(name) => Some(GroupEntry::Group {
            names: SmallVec::from_vec(vec![CompactString::from(name.as_str())]),
            overrides: None,
            newlines_inside: None,
        }),
        Value::Array(names) => Some(GroupEntry::Group {
            names: names
                .iter()
                .filter_map(Value::as_str)
                .map(CompactString::from)
                .collect(),
            overrides: None,
            newlines_inside: None,
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
) -> Option<NamedImport<'a>> {
    let source_start = movable_leading_comment_start(source_text, comments, specifier, options);
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
    let group = compute_group(options, name.as_str(), modifier);
    let group_index = options.group_index(group.as_str());
    Some(NamedImport {
        span: specifier.span,
        name,
        source,
        source_start,
        source_end,
        size: node_source.encode_utf16().count(),
        group,
        group_index,
        partition_id: 0,
    })
}

fn movable_leading_comment_start(
    source_text: &str,
    comments: &[Comment],
    specifier: &ImportSpecifier<'_>,
    options: &RuleOptions,
) -> u32 {
    let mut leading: SmallVec<[&Comment; 4]> = comments
        .iter()
        .filter(|comment| {
            comment.attached_to == specifier.span.start && comment.span.end <= specifier.span.start
        })
        .collect();
    leading.sort_by_key(|comment| comment.span.start);
    let mut start = specifier.span.start;
    for comment in leading.into_iter().rev() {
        if is_partition_comment(source_text, comment, &options.partition_by_comment)
            || empty_lines_between(source_text, comment.span.end, start) > 0
        {
            break;
        }
        start = comment.span.start;
    }
    start
}

fn compute_group(options: &RuleOptions, name: &str, modifier: &str) -> CompactString {
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
            .any(|matcher| custom_match(matcher, name, modifier))
        {
            return custom.group_name.clone();
        }
    }
    for predefined in [format!("{modifier}-import"), "import".to_owned()] {
        if configured
            .iter()
            .any(|configured_name| *configured_name == predefined)
        {
            return CompactString::from(predefined);
        }
    }
    CompactString::new("unknown")
}

fn custom_match(matcher: &CustomMatch, name: &str, modifier: &str) -> bool {
    if matcher
        .selector
        .as_ref()
        .is_some_and(|selector| selector.as_str() != "import")
    {
        return false;
    }
    if !matcher.modifiers.is_empty()
        && !matcher
            .modifiers
            .iter()
            .all(|candidate| candidate.as_str() == modifier)
    {
        return false;
    }
    matcher
        .element_name_pattern
        .as_ref()
        .is_none_or(|pattern| matches_regex(name, pattern))
}

fn assign_partitions(
    source_text: &str,
    comments: &[Comment],
    options: &RuleOptions,
    imports: &mut [NamedImport<'_>],
) {
    let mut partition_id = 1;
    for index in 0..imports.len() {
        if index > 0 {
            let has_partition_comment = comments.iter().any(|comment| {
                comment.attached_to == imports[index].span.start
                    && comment.span.end <= imports[index].span.start
                    && is_partition_comment(source_text, comment, &options.partition_by_comment)
            });
            let has_partition_newline = options.partition_by_new_line
                && empty_lines_between(
                    source_text,
                    imports[index - 1].source_end,
                    imports[index].source_start,
                ) > 0;
            if has_partition_comment || has_partition_newline {
                partition_id += 1;
            }
        }
        imports[index].partition_id = partition_id;
    }
}

fn sort_imports(options: &RuleOptions, imports: &[NamedImport<'_>]) -> Vec<usize> {
    let mut sorted = Vec::with_capacity(imports.len());
    let mut start = 0;
    while start < imports.len() {
        let partition = imports[start].partition_id;
        let mut end = start + 1;
        while end < imports.len() && imports[end].partition_id == partition {
            end += 1;
        }
        let mut partition_indices: Vec<usize> = (start..end).collect();
        partition_indices.sort_by(|&left, &right| {
            imports[left]
                .group_index
                .cmp(&imports[right].group_index)
                .then_with(|| compare_in_group(options, &imports[left], &imports[right]))
                .then_with(|| left.cmp(&right))
        });
        sorted.extend(partition_indices);
        start = end;
    }
    sorted
}

fn compare_in_group(
    options: &RuleOptions,
    left: &NamedImport<'_>,
    right: &NamedImport<'_>,
) -> Ordering {
    let (sort, raw) = options.sort_options_for_group(left.group_index);
    let object = raw.as_object();
    let primary = object
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("alphabetical");
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
        left.name.as_str(),
        left.size,
        right.name.as_str(),
        right.size,
    );
    if compared == Ordering::Equal
        && object
            .and_then(|value| value.get("fallbackSort"))
            .and_then(Value::as_object)
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            == Some("subgroup-order")
    {
        subgroup_compare(options, left, right, object)
    } else {
        compared
    }
}

fn fallback_compare(
    options: &RuleOptions,
    left: &NamedImport<'_>,
    right: &NamedImport<'_>,
    raw: Option<&Map<String, Value>>,
) -> Ordering {
    let Some(fallback) = raw
        .and_then(|value| value.get("fallbackSort"))
        .and_then(Value::as_object)
    else {
        return Ordering::Equal;
    };
    if fallback.get("type").and_then(Value::as_str) == Some("subgroup-order") {
        return subgroup_compare(options, left, right, raw);
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
        left.name.as_str(),
        left.size,
        right.name.as_str(),
        right.size,
    )
}

fn subgroup_compare(
    options: &RuleOptions,
    left: &NamedImport<'_>,
    right: &NamedImport<'_>,
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
    left: &NamedImport<'_>,
    right: &NamedImport<'_>,
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
        let relevant = &options.groups[left.group_index..=right.group_index];
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

fn build_fix(
    source_text: &str,
    options: &RuleOptions,
    imports: &[NamedImport<'_>],
    sorted_indices: &[usize],
) -> Option<RuleDiagnosticFix> {
    let mut replacement = CompactString::new("");
    let mut desired_source_starts = Vec::with_capacity(imports.len());
    let mut desired_source_ends = Vec::with_capacity(imports.len());
    let mut changed_start: Option<u32> = None;
    let mut changed_end: Option<u32> = None;
    for position in 0..imports.len() {
        desired_source_starts.push(replacement.len());
        replacement.push_str(imports[*sorted_indices.get(position)?].source);
        desired_source_ends.push(replacement.len());
        if sorted_indices[position] != position {
            changed_start = Some(
                changed_start.map_or(imports[position].source_start, |start| {
                    start.min(imports[position].source_start)
                }),
            );
            changed_end = Some(changed_end.map_or(imports[position].source_end, |end| {
                end.max(imports[position].source_end)
            }));
        }
        if position + 1 == imports.len() {
            continue;
        }
        let separator_start = usize::try_from(imports[position].source_end).ok()?;
        let separator_end = usize::try_from(imports[position + 1].source_start).ok()?;
        let separator = source_text.get(separator_start..separator_end)?;
        let sorted_left = &imports[*sorted_indices.get(position)?];
        let sorted_right = &imports[*sorted_indices.get(position + 1)?];
        let desired_separator = if sorted_left.partition_id == sorted_right.partition_id
            && sorted_left.group_index <= sorted_right.group_index
            && let Newlines::Count(expected) = newlines_between(options, sorted_left, sorted_right)
        {
            normalize_separator(
                separator,
                expected,
                is_same_line(
                    source_text,
                    imports[position].span.end,
                    imports[position + 1].span.start,
                ),
            )
        } else {
            CompactString::from(separator)
        };
        if desired_separator.as_str() != separator {
            changed_start = Some(changed_start.map_or(imports[position].span.end, |start| {
                start.min(imports[position].span.end)
            }));
            changed_end = Some(
                changed_end.map_or(imports[position + 1].source_start, |end| {
                    end.max(imports[position + 1].source_start)
                }),
            );
        }
        replacement.push_str(desired_separator.as_str());
    }
    let changed_start = changed_start?;
    let changed_end = changed_end?;
    let replacement_start = desired_offset_for_boundary(
        changed_start,
        imports,
        &desired_source_starts,
        &desired_source_ends,
    )?;
    let replacement_end = desired_offset_for_boundary(
        changed_end,
        imports,
        &desired_source_starts,
        &desired_source_ends,
    )?;
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

fn desired_offset_for_boundary(
    boundary: u32,
    imports: &[NamedImport<'_>],
    desired_source_starts: &[usize],
    desired_source_ends: &[usize],
) -> Option<usize> {
    for (index, import) in imports.iter().enumerate() {
        if boundary == import.source_start {
            return desired_source_starts.get(index).copied();
        }
        if boundary == import.source_end {
            return desired_source_ends.get(index).copied();
        }
        if boundary == import.span.end {
            let offset = usize::try_from(import.span.end.checked_sub(import.source_start)?).ok()?;
            return desired_source_starts.get(index)?.checked_add(offset);
        }
    }
    let first_start = imports.first()?.source_start;
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

fn matches_regex(value: &str, option: &Value) -> bool {
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
    let mut builder = RegexBuilder::new(pattern);
    builder
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'));
    builder.build().is_ok_and(|regex| regex.is_match(value))
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

fn is_same_line(source_text: &str, left: u32, right: u32) -> bool {
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
