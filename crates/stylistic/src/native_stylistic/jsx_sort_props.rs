//! Native implementation of stable `@stylistic/jsx-sort-props`.
//!
//! The rule intentionally preserves upstream's comparison precedence and its
//! comment-block fixer. Spread attributes divide independent sortable groups;
//! comments that belong to a following attribute move as one source slice.

use std::{cmp::Ordering, collections::BTreeMap};

use oxc_allocator::Allocator;
use oxc_ast::{
    Comment,
    ast::{JSXAttribute, JSXAttributeItem, JSXElementName, JSXOpeningElement},
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE_NAME: &str = "jsx-sort-props";
const RESERVED_PROPS: &[&str] = &["children", "dangerouslySetInnerHTML", "key", "ref"];

const LIST_IS_EMPTY: (&str, &str) = (
    "listIsEmpty",
    "A customized reserved first list must not be empty",
);
const RESERVED_FIRST: (&str, &str) = (
    "listReservedPropsFirst",
    "Reserved props must be listed before all other props",
);
const RESERVED_LAST: (&str, &str) = (
    "listReservedPropsLast",
    "Reserved props must be listed after all other props",
);
const CALLBACKS_LAST: (&str, &str) = (
    "listCallbacksLast",
    "Callbacks must be listed after all other props",
);
const SHORTHAND_FIRST: (&str, &str) = (
    "listShorthandFirst",
    "Shorthand props must be listed before all other props",
);
const SHORTHAND_LAST: (&str, &str) = (
    "listShorthandLast",
    "Shorthand props must be listed after all other props",
);
const MULTILINE_FIRST: (&str, &str) = (
    "listMultilineFirst",
    "Multiline props must be listed before all other props",
);
const MULTILINE_LAST: (&str, &str) = (
    "listMultilineLast",
    "Multiline props must be listed after all other props",
);
const SORT_ALPHA: (&str, &str) = ("sortPropsByAlpha", "Props should be sorted alphabetically");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Multiline {
    Ignore,
    First,
    Last,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReservedFirst {
    Disabled,
    Default,
    Custom(Vec<String>),
}

#[derive(Clone, Debug)]
struct Options {
    ignore_case: bool,
    callbacks_last: bool,
    shorthand_first: bool,
    shorthand_last: bool,
    multiline: Multiline,
    no_sort_alphabetically: bool,
    reserved_first: ReservedFirst,
    reserved_last: Vec<String>,
    locale: String,
}

impl Options {
    fn from_value(options: &Value) -> Self {
        let object = match options {
            Value::Array(values) => values.first().and_then(Value::as_object),
            Value::Object(object) => Some(object),
            _ => None,
        };
        let bool_option = |name: &str| {
            object
                .and_then(|value| value.get(name))
                .and_then(Value::as_bool)
        };
        let string_list = |name: &str| {
            object
                .and_then(|value| value.get(name))
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };

        let reserved_first = match object.and_then(|value| value.get("reservedFirst")) {
            Some(Value::Bool(true)) => ReservedFirst::Default,
            Some(Value::Array(values)) if values.iter().all(Value::is_string) => {
                ReservedFirst::Custom(
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                )
            }
            _ => ReservedFirst::Disabled,
        };
        let multiline = match object
            .and_then(|value| value.get("multiline"))
            .and_then(Value::as_str)
        {
            Some("first") => Multiline::First,
            Some("last") => Multiline::Last,
            _ => Multiline::Ignore,
        };

        Self {
            ignore_case: bool_option("ignoreCase").unwrap_or(false),
            callbacks_last: bool_option("callbacksLast").unwrap_or(false),
            shorthand_first: bool_option("shorthandFirst").unwrap_or(false),
            shorthand_last: bool_option("shorthandLast").unwrap_or(false),
            multiline,
            no_sort_alphabetically: bool_option("noSortAlphabetically").unwrap_or(false),
            reserved_first,
            reserved_last: string_list("reservedLast"),
            locale: object
                .and_then(|value| value.get("locale"))
                .and_then(Value::as_str)
                .unwrap_or("auto")
                .to_owned(),
        }
    }

    fn reserved_first_enabled(&self) -> bool {
        self.reserved_first != ReservedFirst::Disabled
    }

    fn reserved_list(&self, dom_component: bool) -> Vec<String> {
        let list = match &self.reserved_first {
            ReservedFirst::Custom(values) => values.clone(),
            ReservedFirst::Default | ReservedFirst::Disabled => {
                RESERVED_PROPS.iter().copied().map(str::to_owned).collect()
            }
        };
        list.into_iter()
            .filter(|name| dom_component || name != "dangerouslySetInnerHTML")
            .collect()
    }
}

pub(crate) fn check_jsx_sort_props(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, options, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, options, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = JsxSortProps {
        source,
        comments: &parsed.program.comments,
        options: Options::from_value(options),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct JsxSortProps<'source, 'comments, 'diagnostics> {
    source: &'source str,
    comments: &'comments [Comment],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for JsxSortProps<'_, '_, '_> {
    fn visit_jsx_opening_element(&mut self, opening: &JSXOpeningElement<'ast>) {
        self.check_opening(opening);
        walk::walk_jsx_opening_element(self, opening);
    }
}

#[derive(Clone, Copy)]
struct PendingReport<'ast> {
    attribute: &'ast JSXAttribute<'ast>,
    contract: (&'static str, &'static str),
}

impl JsxSortProps<'_, '_, '_> {
    fn check_opening<'ast>(&mut self, opening: &'ast JSXOpeningElement<'ast>) {
        let attributes = opening.attributes.as_slice();
        let Some(first) = attributes.first() else {
            return;
        };
        let reserved_list = self.options.reserved_list(is_dom_component(&opening.name));
        let mut reports = Vec::<PendingReport<'ast>>::new();
        let mut memo = Some(first);

        for (index, item) in attributes.iter().enumerate() {
            if matches!(item, JSXAttributeItem::SpreadAttribute(_)) {
                memo = attributes.get(index + 1);
                continue;
            }
            let Some(current) = as_attribute(item) else {
                continue;
            };
            let Some(previous) = memo.and_then(as_attribute) else {
                memo = Some(item);
                continue;
            };

            let mut previous_name = attribute_name(self.source, previous);
            let mut current_name = attribute_name(self.source, current);
            let previous_namespace = namespace(&previous_name).to_owned();
            let current_namespace = namespace(&current_name).to_owned();
            let previous_callback = is_callback(&previous_name);
            let current_callback = is_callback(&current_name);

            if self.options.ignore_case {
                previous_name = previous_name.to_lowercase();
                current_name = current_name.to_lowercase();
            }

            if self.options.reserved_first_enabled() {
                if matches!(&self.options.reserved_first, ReservedFirst::Custom(list) if list.is_empty())
                {
                    push_report(&mut reports, current, LIST_IS_EMPTY);
                    continue;
                }

                let previous_index = reserved_index(&previous_name, &reserved_list);
                let current_index = reserved_index(&current_name, &reserved_list);
                if previous_index.is_some() && current_index.is_none() {
                    memo = Some(item);
                    continue;
                }
                let custom_out_of_order =
                    !matches!(self.options.reserved_first, ReservedFirst::Default)
                        && previous_index.zip(current_index).is_some_and(
                            |(previous_index, current_index)| previous_index > current_index,
                        );
                if custom_out_of_order || (previous_index.is_none() && current_index.is_some()) {
                    push_report(&mut reports, current, RESERVED_FIRST);
                    continue;
                }
                if previous_index.is_some()
                    && current_index.is_some()
                    && current_index > previous_index
                    && previous_namespace != current_namespace
                {
                    memo = Some(item);
                    continue;
                }
            }

            if !self.options.reserved_last.is_empty() {
                let previous_index = reserved_index(&previous_name, &self.options.reserved_last);
                let current_index = reserved_index(&current_name, &self.options.reserved_last);
                if previous_index.is_none() && current_index.is_some() {
                    memo = Some(item);
                    continue;
                }
                if previous_index
                    .zip(current_index)
                    .is_some_and(|(previous_index, current_index)| previous_index < current_index)
                    || (previous_index.is_some() && current_index.is_none())
                {
                    push_report(&mut reports, current, RESERVED_LAST);
                    continue;
                }
                if previous_index.is_some()
                    && current_index.is_some()
                    && current_index > previous_index
                    && previous_namespace != current_namespace
                {
                    memo = Some(item);
                    continue;
                }
            }

            if self.options.callbacks_last {
                if !previous_callback && current_callback {
                    memo = Some(item);
                    continue;
                }
                if previous_callback && !current_callback {
                    push_report(&mut reports, previous, CALLBACKS_LAST);
                    continue;
                }
            }

            if self.options.shorthand_first {
                if current.value.is_some() && previous.value.is_none() {
                    memo = Some(item);
                    continue;
                }
                if current.value.is_none() && previous.value.is_some() {
                    push_report(&mut reports, current, SHORTHAND_FIRST);
                    continue;
                }
            }

            if self.options.shorthand_last {
                if current.value.is_none() && previous.value.is_some() {
                    memo = Some(item);
                    continue;
                }
                if current.value.is_some() && previous.value.is_none() {
                    push_report(&mut reports, previous, SHORTHAND_LAST);
                    continue;
                }
            }

            let previous_multiline = !is_single_line(self.source, previous.span);
            let current_multiline = !is_single_line(self.source, current.span);
            if self.options.multiline == Multiline::First {
                if previous_multiline && !current_multiline {
                    memo = Some(item);
                    continue;
                }
                if !previous_multiline && current_multiline {
                    push_report(&mut reports, current, MULTILINE_FIRST);
                    continue;
                }
            } else if self.options.multiline == Multiline::Last {
                if !previous_multiline && current_multiline {
                    memo = Some(item);
                    continue;
                }
                if previous_multiline && !current_multiline {
                    push_report(&mut reports, previous, MULTILINE_LAST);
                    continue;
                }
            }

            if !self.options.no_sort_alphabetically
                && compare_names(
                    &previous_name,
                    &current_name,
                    self.options.ignore_case,
                    &self.options.locale,
                ) == Ordering::Greater
            {
                push_report(&mut reports, current, SORT_ALPHA);
                continue;
            }

            memo = Some(item);
        }

        if reports.is_empty() {
            return;
        }
        let fix = generate_fix(
            self.source,
            self.comments,
            opening,
            &self.options,
            &reserved_list,
        );
        for report in reports {
            let span = if report.contract.0 == LIST_IS_EMPTY.0 {
                report.attribute.span
            } else {
                report.attribute.name.span()
            };
            let range = TextRange::new(span.start, span.end);
            let suggestions = fix.as_ref().map_or_else(Vec::new, |fix| {
                std::iter::once(LintSuggestion {
                    message_id: report.contract.0.to_owned(),
                    message: report.contract.1.to_owned(),
                    fixes: std::iter::once(fix.clone()).collect(),
                })
                .collect()
            });
            self.diagnostics.push(LintDiagnostic {
                rule_name: RULE_NAME.to_owned(),
                message_id: report.contract.0.to_owned(),
                message: report.contract.1.to_owned(),
                data: BTreeMap::new(),
                range,
                suggestions,
            });
        }
    }
}

fn push_report<'ast>(
    reports: &mut Vec<PendingReport<'ast>>,
    attribute: &'ast JSXAttribute<'ast>,
    contract: (&'static str, &'static str),
) {
    if reports
        .iter()
        .any(|report| report.attribute.span == attribute.span && report.contract.0 == contract.0)
    {
        return;
    }
    reports.push(PendingReport {
        attribute,
        contract,
    });
}

fn as_attribute<'ast>(item: &'ast JSXAttributeItem<'ast>) -> Option<&'ast JSXAttribute<'ast>> {
    match item {
        JSXAttributeItem::Attribute(attribute) => Some(attribute),
        JSXAttributeItem::SpreadAttribute(_) => None,
    }
}

fn attribute_name(source: &str, attribute: &JSXAttribute<'_>) -> String {
    source_span(source, attribute.name.span())
        .unwrap_or_default()
        .to_owned()
}

fn namespace(name: &str) -> &str {
    name.split_once(':')
        .map_or(name, |(namespace, _)| namespace)
}

fn reserved_index(name: &str, list: &[String]) -> Option<usize> {
    let namespace = namespace(name);
    list.iter().position(|reserved| reserved == namespace)
}

fn is_callback(name: &str) -> bool {
    let mut characters = name.chars();
    characters.next() == Some('o')
        && characters.next() == Some('n')
        && characters
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
}

fn is_dom_component(name: &JSXElementName<'_>) -> bool {
    name.get_identifier_name()
        .and_then(|name| name.chars().next())
        .is_some_and(char::is_lowercase)
}

fn is_single_line(source: &str, span: Span) -> bool {
    source_span(source, span).is_some_and(|text| {
        !text
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

fn compare_names(left: &str, right: &str, ignore_case: bool, locale: &str) -> Ordering {
    if !ignore_case && locale == "auto" {
        return utf16_cmp(left, right);
    }
    let lowered_left = ignore_case.then(|| left.to_lowercase());
    let lowered_right = ignore_case.then(|| right.to_lowercase());
    let left = lowered_left.as_deref().unwrap_or(left);
    let right = lowered_right.as_deref().unwrap_or(right);

    if locale.eq_ignore_ascii_case("sk")
        || locale
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("sk-"))
    {
        slovak_cmp(left, right)
    } else {
        utf16_cmp(left, right)
    }
}

fn utf16_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn slovak_cmp(left: &str, right: &str) -> Ordering {
    let left = slovak_weights(left);
    let right = slovak_weights(right);
    left.cmp(&right)
}

fn slovak_weights(value: &str) -> Vec<u32> {
    let lowercase = value.to_lowercase();
    let characters = lowercase.chars().collect::<Vec<_>>();
    let mut weights = Vec::with_capacity(characters.len());
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == 'c' && characters.get(index + 1) == Some(&'h') {
            weights.push(15);
            index += 2;
            continue;
        }
        let weight = match characters[index] {
            'a' => 1,
            'á' => 2,
            'ä' => 3,
            'b' => 4,
            'c' => 5,
            'č' => 6,
            'd' => 7,
            'ď' => 8,
            'e' => 9,
            'é' => 10,
            'f' => 12,
            'g' => 13,
            'h' => 14,
            'i' => 16,
            'í' => 17,
            'j' => 18,
            'k' => 19,
            'l' => 20,
            'ĺ' => 21,
            'ľ' => 22,
            'm' => 23,
            'n' => 24,
            'ň' => 25,
            'o' => 26,
            'ó' => 27,
            'ô' => 28,
            'p' => 29,
            'q' => 30,
            'r' => 31,
            'ŕ' => 32,
            's' => 33,
            'š' => 34,
            't' => 35,
            'ť' => 36,
            'u' => 37,
            'ú' => 38,
            'v' => 39,
            'w' => 40,
            'x' => 41,
            'y' => 42,
            'ý' => 43,
            'z' => 44,
            'ž' => 45,
            character => 0x1000 + u32::from(character),
        };
        weights.push(weight);
        index += 1;
    }
    weights
}

#[derive(Clone, Copy, Debug)]
struct SortableAttribute {
    index: usize,
    end: u32,
    has_comment: bool,
}

fn generate_fix(
    source: &str,
    comments: &[Comment],
    opening: &JSXOpeningElement<'_>,
    options: &Options,
    reserved_list: &[String],
) -> Option<LintFix> {
    let groups = sortable_groups(source, comments, opening);
    let mut edits = Vec::<(Span, String)>::new();

    for group in groups {
        let mut sorted = group.clone();
        sorted.sort_by(|left, right| {
            compare_attributes(source, opening, *left, *right, options, reserved_list)
        });
        for (original, sorted) in group.iter().zip(sorted) {
            let original_attribute = as_attribute(&opening.attributes[original.index])?;
            let sorted_attribute = as_attribute(&opening.attributes[sorted.index])?;
            let text = source_span(source, Span::new(sorted_attribute.span.start, sorted.end))?;
            edits.push((
                Span::new(original_attribute.span.start, original.end),
                text.to_owned(),
            ));
        }
    }
    if edits.is_empty() {
        return Some(LintFix::replace_range(TextRange::new(0, 0), ""));
    }

    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let range_start = edits.last()?.0.start;
    let range_end = edits.first()?.0.end;
    let mut fixed = source.to_owned();
    for (range, replacement) in &edits {
        fixed.replace_range(
            usize::try_from(range.start).ok()?..usize::try_from(range.end).ok()?,
            replacement,
        );
    }
    let replacement =
        fixed.get(usize::try_from(range_start).ok()?..usize::try_from(range_end).ok()?)?;
    let original =
        source.get(usize::try_from(range_start).ok()?..usize::try_from(range_end).ok()?)?;
    if replacement == original {
        return None;
    }
    Some(LintFix::replace_range(
        TextRange::new(range_start, range_end),
        replacement,
    ))
}

fn sortable_groups(
    source: &str,
    comments: &[Comment],
    opening: &JSXOpeningElement<'_>,
) -> Vec<Vec<SortableAttribute>> {
    let attributes = opening.attributes.as_slice();
    let mut groups = Vec::<Vec<SortableAttribute>>::new();
    let mut index = 0;
    while index < attributes.len() {
        let spread = matches!(attributes[index], JSXAttributeItem::SpreadAttribute(_));
        let previous_spread =
            index > 0 && matches!(attributes[index - 1], JSXAttributeItem::SpreadAttribute(_));
        if index == 0 || (previous_spread && !spread) {
            groups.push(Vec::new());
        }
        let Some(attribute) = as_attribute(&attributes[index]) else {
            index += 1;
            continue;
        };

        let boundary = attributes
            .get(index + 1)
            .map_or(opening.span.end, |item| item.span().start);
        let comments_after = comments
            .iter()
            .filter(|comment| {
                comment.span.start >= attribute.span.end && comment.span.end <= boundary
            })
            .collect::<Vec<_>>();
        let mut entry = None;
        if comments_after.is_empty() {
            entry = Some(SortableAttribute {
                index,
                end: attribute.span.end,
                has_comment: false,
            });
        } else {
            let attribute_line = line_of(source, attribute.span.start);
            let first = comments_after[0];
            let first_line = line_of(source, first.span.start);
            if comments_after.len() == 1 {
                if attribute_line + 1 == first_line && index + 1 < attributes.len() {
                    entry = Some(SortableAttribute {
                        index,
                        end: attributes[index + 1].span().end,
                        has_comment: true,
                    });
                    index += 1;
                } else if attribute_line == first_line {
                    if first.is_block() && index + 1 < attributes.len() {
                        entry = Some(SortableAttribute {
                            index,
                            end: attributes[index + 1].span().end,
                            has_comment: true,
                        });
                        index += 1;
                    } else {
                        entry = Some(SortableAttribute {
                            index,
                            end: first.span.end,
                            has_comment: first.is_block(),
                        });
                    }
                }
            } else if attribute_line + 1 == line_of(source, comments_after[1].span.start)
                && index + 1 < attributes.len()
            {
                let next = &attributes[index + 1];
                let next_boundary = attributes
                    .get(index + 2)
                    .map_or(opening.span.end, |item| item.span().start);
                let next_comments = comments
                    .iter()
                    .filter(|comment| {
                        comment.span.start >= next.span().end && comment.span.end <= next_boundary
                    })
                    .collect::<Vec<_>>();
                let mut end = next.span().end;
                if next_comments.len() == 1
                    && line_of(source, next.span().start)
                        == line_of(source, next_comments[0].span.start)
                {
                    end = next_comments[0].span.end;
                }
                entry = Some(SortableAttribute {
                    index,
                    end,
                    has_comment: true,
                });
                index += 1;
            }
        }
        if let Some(entry) = entry
            && let Some(group) = groups.last_mut()
        {
            group.push(entry);
        }
        index += 1;
    }
    groups
}

fn compare_attributes(
    source: &str,
    opening: &JSXOpeningElement<'_>,
    left: SortableAttribute,
    right: SortableAttribute,
    options: &Options,
    reserved_list: &[String],
) -> Ordering {
    if left.has_comment != right.has_comment {
        return left.has_comment.cmp(&right.has_comment);
    }
    let Some(left_attribute) = as_attribute(&opening.attributes[left.index]) else {
        return Ordering::Equal;
    };
    let Some(right_attribute) = as_attribute(&opening.attributes[right.index]) else {
        return Ordering::Equal;
    };
    let left_name = attribute_name(source, left_attribute);
    let right_name = attribute_name(source, right_attribute);
    let left_namespace = namespace(&left_name);
    let right_namespace = namespace(&right_name);

    if options.reserved_first_enabled() {
        let left_index = reserved_index(&left_name, reserved_list);
        let right_index = reserved_index(&right_name, reserved_list);
        match (left_index, right_index) {
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (Some(left_index), Some(right_index))
                if left_namespace != right_namespace && left_index != right_index =>
            {
                return left_index.cmp(&right_index);
            }
            _ => {}
        }
    }

    if !options.reserved_last.is_empty() {
        let left_index = reserved_index(&left_name, &options.reserved_last);
        let right_index = reserved_index(&right_name, &options.reserved_last);
        match (left_index, right_index) {
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (Some(left_index), Some(right_index))
                if left_namespace != right_namespace && left_index != right_index =>
            {
                return right_index.cmp(&left_index);
            }
            _ => {}
        }
    }

    if options.callbacks_last {
        match (is_callback(&left_name), is_callback(&right_name)) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            _ => {}
        }
    }

    if options.shorthand_first || options.shorthand_last {
        let left_shorthand = left_attribute.value.is_none();
        let right_shorthand = right_attribute.value.is_none();
        if left_shorthand != right_shorthand {
            if options.shorthand_first {
                return if left_shorthand {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            return if left_shorthand {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
    }

    if options.multiline != Multiline::Ignore {
        let left_multiline = !is_single_line(source, left_attribute.span);
        let right_multiline = !is_single_line(source, right_attribute.span);
        if left_multiline != right_multiline {
            return match options.multiline {
                Multiline::First if left_multiline => Ordering::Less,
                Multiline::First => Ordering::Greater,
                Multiline::Last if left_multiline => Ordering::Greater,
                Multiline::Last => Ordering::Less,
                Multiline::Ignore => Ordering::Equal,
            };
        }
    }

    if options.no_sort_alphabetically {
        Ordering::Equal
    } else {
        compare_names(
            &left_name,
            &right_name,
            options.ignore_case,
            &options.locale,
        )
    }
}

fn line_of(source: &str, offset: u32) -> usize {
    let Ok(offset) = usize::try_from(offset) else {
        return 1;
    };
    let Some(prefix) = source.get(..offset) else {
        return 1;
    };
    let mut lines = 1;
    let mut characters = prefix.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                lines += 1;
            }
            '\n' | '\u{2028}' | '\u{2029}' => lines += 1,
            _ => {}
        }
    }
    lines
}

fn source_span(source: &str, span: Span) -> Option<&str> {
    source.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "fixture contract assertions and exhaustive option matrices are clearest with macros"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    struct Generated {
        version: String,
        commit: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        authored_valid: usize,
        authored_invalid: usize,
        authored_diagnostics: usize,
        fixable_invalid: usize,
        unfixable_invalid: usize,
        authored_total: usize,
        parser_expanded_valid: usize,
        parser_expanded_invalid: usize,
        parser_expanded_diagnostics: usize,
        parser_expanded_total: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCase {
        code: String,
        #[serde(default)]
        options: Value,
        parsers: Vec<String>,
        #[serde(default)]
        first_pass_output: Option<String>,
        #[serde(default)]
        recursive_output: Option<String>,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        range: [u32; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-sort-props-v5.10.0.json"
        ))
        .expect("generated jsx-sort-props fixture is valid JSON")
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_sort_props(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn filename(parser: &str) -> &'static str {
        if parser == "tsx" {
            "fixture.tsx"
        } else {
            "fixture.jsx"
        }
    }

    fn first_fix(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let fix = diagnostics
            .iter()
            .find_map(|diagnostic| diagnostic.suggestions.first())
            .and_then(|suggestion| suggestion.fixes.first())?;
        let mut output = source.to_owned();
        output.replace_range(
            usize::try_from(fix.range.start).expect("fix start fits")
                ..usize::try_from(fix.range.end).expect("fix end fits"),
            &fix.replacement_text,
        );
        Some(output)
    }

    fn recursive_fix(source: &str, filename: &str, options: &Value) -> Option<String> {
        let mut output = source.to_owned();
        let mut fixed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, Some(filename), options);
            let Some(next) = first_fix(&output, &diagnostics) else {
                return fixed.then_some(output);
            };
            if next == output {
                return Some(output);
            }
            output = next;
            fixed = true;
        }
        panic!("fix did not converge after ten passes");
    }

    #[test]
    fn pinned_inventory_is_complete_and_parser_expanded() {
        let fixture = fixture();
        assert_eq!(fixture.generated.version, "v5.10.0");
        assert_eq!(
            fixture.generated.commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        let inventory = fixture.generated.inventory;
        assert_eq!(inventory.authored_valid, 43);
        assert_eq!(inventory.authored_invalid, 54);
        assert_eq!(inventory.authored_diagnostics, 120);
        assert_eq!(inventory.fixable_invalid, 53);
        assert_eq!(inventory.unfixable_invalid, 1);
        assert_eq!(inventory.authored_total, 97);
        assert_eq!(inventory.parser_expanded_valid, 86);
        assert_eq!(inventory.parser_expanded_invalid, 107);
        assert_eq!(inventory.parser_expanded_diagnostics, 239);
        assert_eq!(inventory.parser_expanded_total, 193);
    }

    #[test]
    fn accepts_every_upstream_valid_case_in_the_declared_parser_matrix() {
        for (index, test) in fixture().valid.iter().enumerate() {
            for parser in &test.parsers {
                let diagnostics = run(&test.code, Some(filename(parser)), &test.options);
                assert!(
                    diagnostics.is_empty(),
                    "valid case {index} reported for {parser}:\n{}\n{diagnostics:#?}",
                    test.code
                );
            }
        }
    }

    #[test]
    fn replays_every_upstream_invalid_case_with_exact_contract_and_recursive_output() {
        for (index, test) in fixture().invalid.iter().enumerate() {
            for parser in &test.parsers {
                let filename = filename(parser);
                let diagnostics = run(&test.code, Some(filename), &test.options);
                assert_eq!(
                    diagnostics.len(),
                    test.expected_diagnostics.len(),
                    "diagnostic count differs for case {index}, {parser}:\n{}",
                    test.code
                );
                for (actual, expected) in diagnostics.iter().zip(&test.expected_diagnostics) {
                    assert_eq!(actual.rule_name, RULE_NAME, "case {index}, {parser}");
                    assert_eq!(
                        actual.message_id, expected.message_id,
                        "case {index}, {parser}"
                    );
                    assert_eq!(actual.message, expected.message, "case {index}, {parser}");
                    assert!(actual.data.is_empty(), "case {index}, {parser}");
                    assert_eq!(
                        actual.range,
                        TextRange::new(expected.range[0], expected.range[1]),
                        "diagnostic range differs for case {index}, {parser}"
                    );
                    match (&expected.fix, actual.suggestions.first()) {
                        (Some(expected_fix), Some(suggestion)) => {
                            assert_eq!(suggestion.message_id, expected.message_id);
                            assert_eq!(suggestion.message, expected.message);
                            assert_eq!(suggestion.fixes.len(), 1);
                            assert_eq!(
                                suggestion.fixes[0].range,
                                TextRange::new(expected_fix.range[0], expected_fix.range[1]),
                                "fix range differs for case {index}, {parser}"
                            );
                            assert_eq!(
                                suggestion.fixes[0].replacement_text, expected_fix.text,
                                "fix text differs for case {index}, {parser}"
                            );
                        }
                        (None, None) => {}
                        _ => panic!("fix presence differs for case {index}, {parser}"),
                    }
                }
                assert_eq!(
                    first_fix(&test.code, &diagnostics),
                    test.first_pass_output,
                    "first-pass output differs for case {index}, {parser}"
                );
                assert_eq!(
                    recursive_fix(&test.code, filename, &test.options),
                    test.recursive_output,
                    "recursive output differs for case {index}, {parser}"
                );
            }
        }
    }

    #[test]
    fn preserves_unicode_byte_ranges_tsx_and_namespaced_names() {
        let source = concat!(
            "const prefix = \"😀\";\r\n",
            "declare const 値: unknown;\r\n",
            "<外側<T> ζeta={値} v-slot:b={{ 値 }} alpha v-slot:a={{ 値 }} />;"
        );
        let options = json!([{ "reservedLast": ["v-slot"] }]);
        let diagnostics = run(source, Some("fixture.tsx"), &options);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["listReservedPropsLast", "sortPropsByAlpha"]
        );
        for diagnostic in &diagnostics {
            let text = source_span(
                source,
                Span::new(diagnostic.range.start, diagnostic.range.end),
            )
            .expect("diagnostic is on a UTF-8 boundary");
            assert!(["alpha", "v-slot:a"].contains(&text));
        }
        assert_eq!(
            first_fix(source, &diagnostics).as_deref(),
            Some(concat!(
                "const prefix = \"😀\";\r\n",
                "declare const 値: unknown;\r\n",
                "<外側<T> alpha ζeta={値} v-slot:a={{ 値 }} v-slot:b={{ 値 }} />;"
            ))
        );
    }

    #[test]
    fn recognizes_every_ecmascript_line_terminator_for_multiline_sorting() {
        for terminator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("<App a b={{() => ({terminator}1)}} />");
            let diagnostics = run(
                &source,
                Some("fixture.jsx"),
                &json!([{ "multiline": "first" }]),
            );
            assert_eq!(diagnostics.len(), 1, "terminator {terminator:?}");
            assert_eq!(diagnostics[0].message_id, "listMultilineFirst");
            assert_eq!(
                first_fix(&source, &diagnostics).as_deref(),
                Some(format!("<App b={{() => ({terminator}1)}} a />").as_str())
            );
        }
    }

    #[test]
    fn covers_member_tags_comments_spreads_and_option_precedence() {
        let source = concat!(
            "<UI.Form z onClick a ",
            "/* keep with beta */ beta ",
            "{...rest} ref key shorthand value={1} />"
        );
        let options = json!([{
            "callbacksLast": true,
            "shorthandLast": true,
            "reservedFirst": true
        }]);
        let diagnostics = run(source, Some("fixture.tsx"), &options);
        assert!(!diagnostics.is_empty());
        let fixed = first_fix(source, &diagnostics).expect("case is fixable");
        assert!(fixed.contains("{...rest} key ref value={1} shorthand"));
        assert!(fixed.contains("/* keep with beta */ beta"));
        assert_ne!(fixed, source);
    }

    #[test]
    fn handles_malformed_options_and_parse_failures_without_panicking() {
        for options in [
            Value::Null,
            json!([]),
            json!([false]),
            json!(["invalid"]),
            json!([{ "reservedFirst": 1, "reservedLast": [false] }]),
        ] {
            assert_eq!(
                run("<App b a />", Some("fixture.jsx"), &options)
                    .iter()
                    .map(|diagnostic| diagnostic.message_id.as_str())
                    .collect::<Vec<_>>(),
                ["sortPropsByAlpha"]
            );
        }
        assert!(run("<App", Some("fixture.tsx"), &json!([])).is_empty());
    }
}
