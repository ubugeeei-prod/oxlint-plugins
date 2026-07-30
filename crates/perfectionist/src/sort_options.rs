//! Shared scalar comparator for the configurable `sort-named-imports` engine.
//!
//! Group selection, partitioning, newline policies, and conditional
//! configuration live in the rule module and reuse this comparator after
//! applying their per-group overrides.

use std::cmp::Ordering;

use icu_collator::{CollatorBorrowed, CollatorPreferences, options::CollatorOptions};
use icu_locale_core::Locale;
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SortType {
    SubgroupOrder,
    Alphabetical,
    Natural,
    LineLength,
    Custom,
    Unsorted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SpecialCharacters {
    Keep,
    Trim,
    Remove,
}

#[derive(Clone, Debug)]
struct FallbackSort {
    kind: SortType,
    order: SortOrder,
}

#[derive(Debug)]
pub(crate) struct SortOptions {
    kind: SortType,
    order: SortOrder,
    fallback: FallbackSort,
    special_characters: SpecialCharacters,
    alphabet: CompactString,
    pub(crate) ignore_alias: bool,
    ignore_case: bool,
    collator: Option<CollatorBorrowed<'static>>,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            kind: SortType::Alphabetical,
            order: SortOrder::Ascending,
            fallback: FallbackSort {
                kind: SortType::Unsorted,
                order: SortOrder::Ascending,
            },
            special_characters: SpecialCharacters::Keep,
            alphabet: CompactString::new(""),
            ignore_alias: false,
            ignore_case: true,
            collator: collator_for_locale("en-US"),
        }
    }
}

impl SortOptions {
    pub(crate) fn from_json(options: &Value) -> Self {
        let Some(object) = options
            .as_array()
            .and_then(|values| values.first())
            .and_then(Value::as_object)
            .or_else(|| options.as_object())
        else {
            return Self::default();
        };

        let mut parsed = Self::default();
        parsed.kind = object
            .get("type")
            .and_then(Value::as_str)
            .and_then(parse_sort_type)
            .unwrap_or(parsed.kind);
        parsed.order = object
            .get("order")
            .and_then(Value::as_str)
            .and_then(parse_order)
            .unwrap_or(parsed.order);
        parsed.ignore_case = object
            .get("ignoreCase")
            .and_then(Value::as_bool)
            .unwrap_or(parsed.ignore_case);
        parsed.ignore_alias = object
            .get("ignoreAlias")
            .and_then(Value::as_bool)
            .unwrap_or(parsed.ignore_alias);
        parsed.special_characters = object
            .get("specialCharacters")
            .and_then(Value::as_str)
            .and_then(parse_special_characters)
            .unwrap_or(parsed.special_characters);
        if let Some(alphabet) = object.get("alphabet").and_then(Value::as_str) {
            parsed.alphabet = CompactString::from(alphabet);
        }
        if let Some(locale) = first_locale(object.get("locales")) {
            parsed.collator = collator_for_locale(locale);
        }
        if let Some(fallback) = object.get("fallbackSort").and_then(Value::as_object) {
            parsed.fallback.kind = fallback
                .get("type")
                .and_then(Value::as_str)
                .and_then(parse_sort_type)
                .unwrap_or(parsed.fallback.kind);
            parsed.fallback.order = fallback
                .get("order")
                .and_then(Value::as_str)
                .and_then(parse_order)
                .unwrap_or(parsed.fallback.order);
        }
        parsed
    }

    pub(crate) fn compare(
        &self,
        left_name: &str,
        left_size: usize,
        right_name: &str,
        right_size: usize,
    ) -> Ordering {
        // Upstream deliberately ignores `fallbackSort` when the primary sorter
        // is `unsorted`.
        if matches!(self.kind, SortType::Unsorted | SortType::SubgroupOrder) {
            return Ordering::Equal;
        }
        let primary = self.compare_with(
            self.kind, self.order, left_name, left_size, right_name, right_size,
        );
        if primary != Ordering::Equal {
            return primary;
        }
        self.compare_with(
            self.fallback.kind,
            self.fallback.order,
            left_name,
            left_size,
            right_name,
            right_size,
        )
    }

    fn compare_with(
        &self,
        kind: SortType,
        order: SortOrder,
        left_name: &str,
        left_size: usize,
        right_name: &str,
        right_size: usize,
    ) -> Ordering {
        let ordering = match kind {
            SortType::Alphabetical => {
                let left = self.normalize(left_name);
                let right = self.normalize(right_name);
                self.collator.as_ref().map_or_else(
                    || compare_en_us(left.as_str(), right.as_str()),
                    |collator| collator.compare(left.as_str(), right.as_str()),
                )
            }
            SortType::Natural => {
                let left = self.normalize(left_name);
                let right = self.normalize(right_name);
                if left.is_ascii() && right.is_ascii() {
                    natural_compare(left.as_str(), right.as_str())
                } else {
                    self.collator.as_ref().map_or_else(
                        || left.cmp(&right),
                        |collator| collator.compare(left.as_str(), right.as_str()),
                    )
                }
            }
            SortType::LineLength => left_size.cmp(&right_size),
            SortType::Custom => {
                let left = self.normalize(left_name);
                let right = self.normalize(right_name);
                custom_compare(left.as_str(), right.as_str(), self.alphabet.as_str())
            }
            SortType::SubgroupOrder | SortType::Unsorted => Ordering::Equal,
        };
        match order {
            SortOrder::Ascending => ordering,
            SortOrder::Descending => ordering.reverse(),
        }
    }

    fn normalize(&self, value: &str) -> CompactString {
        let normalized: CompactString = if self.ignore_case {
            value.chars().flat_map(char::to_lowercase).collect()
        } else {
            CompactString::from(value)
        };

        let mut formatted = CompactString::new("");
        let mut found_letter = false;
        for character in normalized.chars() {
            if character.is_whitespace() {
                continue;
            }
            let is_letter = is_perfectionist_letter(character);
            match self.special_characters {
                SpecialCharacters::Keep => formatted.push(character),
                SpecialCharacters::Trim if found_letter || is_letter => {
                    found_letter = true;
                    formatted.push(character);
                }
                SpecialCharacters::Trim => {}
                SpecialCharacters::Remove if is_letter => formatted.push(character),
                SpecialCharacters::Remove => {}
            }
        }
        formatted
    }
}

fn first_locale(value: Option<&Value>) -> Option<&str> {
    match value {
        Some(Value::String(locale)) => Some(locale),
        Some(Value::Array(locales)) => locales.iter().find_map(Value::as_str),
        _ => None,
    }
}

fn collator_for_locale(locale: &str) -> Option<CollatorBorrowed<'static>> {
    let preferences = locale
        .parse::<Locale>()
        .map(CollatorPreferences::from)
        .unwrap_or_default();
    CollatorBorrowed::try_new(preferences, CollatorOptions::default()).ok()
}

fn is_perfectionist_letter(character: char) -> bool {
    matches!(
        character as u32,
        0x0041..=0x005A
            | 0x0061..=0x007A
            | 0x00C0..=0x024F
            | 0x1E00..=0x1EFF
    )
}

/// Identifier-focused approximation of `localeCompare(..., "en-US")`.
///
/// The named-import rule overwhelmingly compares JavaScript identifiers.
/// Folding ASCII letters first and using lowercase-before-uppercase as the
/// variant tiebreak matches the ICU collation used by Node for those names,
/// while keeping the implementation deterministic and allocation free.
fn compare_en_us(left: &str, right: &str) -> Ordering {
    let mut left_characters = left.chars();
    let mut right_characters = right.chars();
    loop {
        match (left_characters.next(), right_characters.next()) {
            (Some(left_character), Some(right_character)) => {
                let folded = ascii_collation_rank(left_character)
                    .cmp(&ascii_collation_rank(right_character));
                if folded != Ordering::Equal {
                    return folded;
                }
                if left_character != right_character {
                    let variant =
                        ascii_case_rank(left_character).cmp(&ascii_case_rank(right_character));
                    if variant != Ordering::Equal {
                        return variant;
                    }
                    return left_character.cmp(&right_character);
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }
    }
}

fn ascii_collation_rank(character: char) -> (u8, char) {
    match character {
        '_' => (0, character),
        '$' => (1, character),
        '0'..='9' => (2, character),
        'A'..='Z' | 'a'..='z' => (3, character.to_ascii_lowercase()),
        _ => (4, character),
    }
}

fn ascii_case_rank(character: char) -> u8 {
    if character.is_ascii_lowercase() {
        0
    } else if character.is_ascii_uppercase() {
        1
    } else {
        0
    }
}

fn parse_sort_type(value: &str) -> Option<SortType> {
    match value {
        "subgroup-order" => Some(SortType::SubgroupOrder),
        "alphabetical" => Some(SortType::Alphabetical),
        "natural" => Some(SortType::Natural),
        "line-length" => Some(SortType::LineLength),
        "custom" => Some(SortType::Custom),
        "unsorted" => Some(SortType::Unsorted),
        _ => None,
    }
}

fn parse_order(value: &str) -> Option<SortOrder> {
    match value {
        "asc" => Some(SortOrder::Ascending),
        "desc" => Some(SortOrder::Descending),
        _ => None,
    }
}

fn parse_special_characters(value: &str) -> Option<SpecialCharacters> {
    match value {
        "keep" => Some(SpecialCharacters::Keep),
        "trim" => Some(SpecialCharacters::Trim),
        "remove" => Some(SpecialCharacters::Remove),
        _ => None,
    }
}

fn natural_compare(left: &str, right: &str) -> Ordering {
    // `natural-orderby` lowercases strings internally, even when
    // Perfectionist's `ignoreCase` formatter was disabled.
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = digit_run_end(left, left_index);
            let right_end = digit_run_end(right, right_index);
            let left_digits = trim_zeroes(&left[left_index..left_end]);
            let right_digits = trim_zeroes(&right[right_index..right_end]);
            let ordering = left_digits
                .len()
                .cmp(&right_digits.len())
                .then_with(|| left_digits.cmp(right_digits))
                // Equal numeric values are ordered by their original chunk
                // text, e.g. `01` before `1`.
                .then_with(|| left[left_index..left_end].cmp(&right[right_index..right_end]));
            if ordering != Ordering::Equal {
                return ordering;
            }
            left_index = left_end;
            right_index = right_end;
            continue;
        }
        let ordering = left[left_index].cmp(&right[right_index]);
        if ordering != Ordering::Equal {
            return ordering;
        }
        left_index += 1;
        right_index += 1;
    }
    left.len().cmp(&right.len())
}

fn digit_run_end(value: &[u8], mut index: usize) -> usize {
    while value.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    index
}

fn trim_zeroes(value: &[u8]) -> &[u8] {
    let first_non_zero = value
        .iter()
        .position(|&character| character != b'0')
        .unwrap_or(value.len().saturating_sub(1));
    &value[first_non_zero..]
}

fn custom_compare(left: &str, right: &str, alphabet: &str) -> Ordering {
    let alphabet: SmallVec<[u16; 128]> = alphabet.encode_utf16().collect();
    let left: SmallVec<[u16; 32]> = left.encode_utf16().collect();
    let right: SmallVec<[u16; 32]> = right.encode_utf16().collect();
    for (&left_character, &right_character) in left.iter().zip(&right) {
        let left_rank = alphabet
            .iter()
            .position(|&candidate| candidate == left_character);
        let right_rank = alphabet
            .iter()
            .position(|&candidate| candidate == right_character);
        let ordering = match (left_rank, right_rank) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            // Upstream assigns Infinity to all characters outside the custom
            // alphabet, so two unknown characters compare equal.
            (None, None) => Ordering::Equal,
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "serde_json::json! intentionally constructs public option payloads in tests."
)]
mod tests {
    use super::{SortOptions, SortType};
    use serde_json::json;
    use std::cmp::Ordering;

    fn compare(options: serde_json::Value, left: &str, right: &str) -> Ordering {
        SortOptions::from_json(&options).compare(left, left.len(), right, right.len())
    }

    #[test]
    fn defaults_to_case_insensitive_ascending_alphabetical_order() {
        assert_eq!(compare(json!([]), "A", "b"), Ordering::Less);
        assert_eq!(compare(json!([]), "a", "A"), Ordering::Equal);
    }

    #[test]
    fn reverses_the_configured_order() {
        assert_eq!(
            compare(json!([{ "order": "desc" }]), "a", "b"),
            Ordering::Greater
        );
    }

    #[test]
    fn compares_natural_numeric_chunks_and_leading_zeroes() {
        assert_eq!(
            compare(json!([{ "type": "natural" }]), "item2", "item10"),
            Ordering::Less
        );
        assert_eq!(
            compare(json!([{ "type": "natural" }]), "item01", "item1"),
            Ordering::Less
        );
    }

    #[test]
    fn compares_the_full_specifier_length() {
        let options = SortOptions::from_json(&json!([{ "type": "line-length" }]));
        assert_eq!(options.compare("A", 9, "B", 1), Ordering::Greater);
    }

    #[test]
    fn honors_a_custom_alphabet() {
        assert_eq!(
            compare(json!([{ "type": "custom", "alphabet": "cba" }]), "c", "a"),
            Ordering::Less
        );
    }

    #[test]
    fn applies_the_fallback_only_after_a_primary_tie() {
        assert_eq!(
            compare(
                json!([{
                    "type": "line-length",
                    "fallbackSort": { "type": "alphabetical", "order": "desc" }
                }]),
                "a",
                "b"
            ),
            Ordering::Greater
        );
    }

    #[test]
    fn leaves_unsorted_input_stable_even_with_a_fallback() {
        let options = SortOptions::from_json(&json!([{
            "type": "unsorted",
            "fallbackSort": { "type": "alphabetical" }
        }]));
        assert_eq!(options.kind, SortType::Unsorted);
        assert_eq!(options.compare("z", 1, "a", 1), Ordering::Equal);
    }

    #[test]
    fn trims_or_removes_special_characters_before_comparing() {
        assert_eq!(
            compare(json!([{ "specialCharacters": "trim" }]), "_a", "b"),
            Ordering::Less
        );
        assert_eq!(
            compare(json!([{ "specialCharacters": "remove" }]), "a_b", "ab"),
            Ordering::Equal
        );
    }

    #[test]
    fn uses_locale_aware_chinese_collation() {
        assert_eq!(
            compare(json!([{ "locales": "zh-CN" }]), "你好", "世界"),
            Ordering::Less
        );
        assert_eq!(
            compare(json!([{ "locales": "zh-CN" }]), "世界", "a"),
            Ordering::Less
        );
    }

    #[test]
    fn accepts_locale_preference_arrays_and_swedish_collation() {
        assert_eq!(
            compare(json!([{ "locales": ["zh-CN", "en-US"] }]), "世界", "a"),
            Ordering::Less
        );
        assert_eq!(
            compare(json!([{ "locales": "sv" }]), "z", "å"),
            Ordering::Less
        );
    }
}
