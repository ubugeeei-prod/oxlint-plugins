#![doc = "Rust implementation of @eslint/json rule logic."]

use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};
use unicode_normalization::UnicodeNormalization;

pub const RULE_NAMES: [&str; 6] = [
    "no-duplicate-keys",
    "no-empty-keys",
    "no-unnormalized-keys",
    "no-unsafe-values",
    "sort-keys",
    "top-level-interop",
];

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanOptions {
    pub rule_names: SmallVec<[CompactString; 6]>,
    pub normalization_form: NormalizationForm,
    pub sort: SortOptions,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            rule_names: SmallVec::new(),
            normalization_form: NormalizationForm::Nfc,
            sort: SortOptions::default(),
        }
    }
}

impl ScanOptions {
    fn is_enabled(&self, rule_name: &str) -> bool {
        self.rule_names.is_empty() || self.rule_names.iter().any(|name| name == rule_name)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NormalizationForm {
    #[default]
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortOptions {
    pub direction: SortDirection,
    pub case_sensitive: bool,
    pub natural: bool,
    pub min_keys: usize,
    pub allow_line_separated_groups: bool,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            direction: SortDirection::Ascending,
            case_sensitive: true,
            natural: false,
            min_keys: 2,
            allow_line_separated_groups: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub key: Option<CompactString>,
    pub value: Option<CompactString>,
    pub surrogate: Option<CompactString>,
    pub type_name: Option<CompactString>,
    pub this_name: Option<CompactString>,
    pub prev_name: Option<CompactString>,
    pub direction: Option<CompactString>,
    pub sensitivity: Option<CompactString>,
    pub sort_name: Option<CompactString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ByteSpan {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MemberFact {
    key: CompactString,
    raw_key: CompactString,
    name_span: ByteSpan,
    key_content_span: ByteSpan,
    member_span: ByteSpan,
    key_has_escape: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ObjectFact {
    members: SmallVec<[MemberFact; 8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NumberFact {
    raw: CompactString,
    span: ByteSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StringFact {
    code_units: SmallVec<[u16; 16]>,
    span: ByteSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TopLevelFact {
    value_type: JsonValueType,
    span: ByteSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JsonValueType {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
    Identifier,
}

impl JsonValueType {
    fn upstream_name(self) -> &'static str {
        match self {
            Self::Object => "Object",
            Self::Array => "Array",
            Self::String => "String",
            Self::Number => "Number",
            Self::Boolean => "Boolean",
            Self::Null => "Null",
            Self::Identifier => "Identifier",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ParsedFacts {
    top_level: Option<TopLevelFact>,
    objects: SmallVec<[ObjectFact; 8]>,
    numbers: SmallVec<[NumberFact; 16]>,
    strings: SmallVec<[StringFact; 16]>,
    comments: SmallVec<[ByteSpan; 8]>,
}

pub fn implemented_eslint_json_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_eslint_json(source_text: &str, options: &ScanOptions) -> SmallVec<[Diagnostic; 16]> {
    let mut parser = Parser::new(source_text);
    let Some(facts) = parser.parse() else {
        return SmallVec::new();
    };

    let line_index = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();

    if options.is_enabled("no-duplicate-keys") {
        scan_no_duplicate_keys(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-empty-keys") {
        scan_no_empty_keys(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-unnormalized-keys") {
        scan_no_unnormalized_keys(
            source_text,
            &line_index,
            &facts,
            options.normalization_form,
            &mut diagnostics,
        );
    }
    if options.is_enabled("no-unsafe-values") {
        scan_no_unsafe_values(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("sort-keys") {
        scan_sort_keys(
            source_text,
            &line_index,
            &facts,
            &options.sort,
            &mut diagnostics,
        );
    }
    if options.is_enabled("top-level-interop") {
        scan_top_level_interop(source_text, &line_index, &facts, &mut diagnostics);
    }

    // ESLint sorts reported problems by location before returning them; objects
    // are discovered in post-order (innermost first), so a stable positional
    // sort restores document order for nested objects.
    diagnostics.sort_by(|a, b| {
        (a.loc.start_line, a.loc.start_column).cmp(&(b.loc.start_line, b.loc.start_column))
    });

    diagnostics
}

fn scan_no_duplicate_keys(
    source_text: &str,
    line_index: &LineIndex,
    facts: &ParsedFacts,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    for object in &facts.objects {
        let mut seen = FastHashMap::<CompactString, ()>::default();
        for member in &object.members {
            if seen.insert(member.key.clone(), ()).is_some() {
                diagnostics.push(Diagnostic {
                    rule_name: "no-duplicate-keys",
                    message_id: "duplicateKey",
                    data: DiagnosticData {
                        key: Some(member.raw_key.clone()),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(source_text, member.name_span),
                    fix: None,
                });
            }
        }
    }
}

fn scan_no_empty_keys(
    source_text: &str,
    line_index: &LineIndex,
    facts: &ParsedFacts,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    for object in &facts.objects {
        for member in &object.members {
            if member.key.trim().is_empty() {
                diagnostics.push(Diagnostic {
                    rule_name: "no-empty-keys",
                    message_id: "emptyKey",
                    data: DiagnosticData::default(),
                    loc: line_index.loc_for_span(source_text, member.name_span),
                    fix: None,
                });
            }
        }
    }
}

fn scan_no_unnormalized_keys(
    source_text: &str,
    line_index: &LineIndex,
    facts: &ParsedFacts,
    form: NormalizationForm,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    for object in &facts.objects {
        for member in &object.members {
            let normalized = normalize_to(member.key.as_str(), form);
            if normalized == member.key {
                continue;
            }

            let fix = (!member.key_has_escape).then(|| DiagnosticFix {
                start: utf16_offset(source_text, member.key_content_span.start),
                end: utf16_offset(source_text, member.key_content_span.end),
                replacement: normalized,
            });

            diagnostics.push(Diagnostic {
                rule_name: "no-unnormalized-keys",
                message_id: "unnormalizedKey",
                data: DiagnosticData {
                    key: Some(member.raw_key.clone()),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, member.name_span),
                fix,
            });
        }
    }
}

fn scan_no_unsafe_values(
    source_text: &str,
    line_index: &LineIndex,
    facts: &ParsedFacts,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    for number in &facts.numbers {
        scan_number(source_text, line_index, number, diagnostics);
    }
    for string in &facts.strings {
        scan_string_surrogates(source_text, line_index, string, diagnostics);
    }
}

fn scan_sort_keys(
    source_text: &str,
    line_index: &LineIndex,
    facts: &ParsedFacts,
    options: &SortOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    let comment_lines = comment_line_ranges(source_text, line_index, &facts.comments);
    for object in &facts.objects {
        if object.members.len() < options.min_keys {
            continue;
        }

        let mut previous: Option<&MemberFact> = None;
        for member in &object.members {
            let Some(prev) = previous else {
                previous = Some(member);
                continue;
            };

            if options.allow_line_separated_groups
                && is_line_separated(
                    source_text,
                    line_index,
                    &comment_lines,
                    prev.member_span,
                    member.member_span,
                )
            {
                previous = Some(member);
                continue;
            }

            if !is_valid_sort_order(prev.key.as_str(), member.key.as_str(), options) {
                diagnostics.push(Diagnostic {
                    rule_name: "sort-keys",
                    message_id: "sortKeys",
                    data: DiagnosticData {
                        this_name: Some(member.raw_key.clone()),
                        prev_name: Some(prev.raw_key.clone()),
                        direction: Some(CompactString::from(match options.direction {
                            SortDirection::Ascending => "ascending",
                            SortDirection::Descending => "descending",
                        })),
                        sensitivity: Some(CompactString::from(if options.case_sensitive {
                            "sensitive"
                        } else {
                            "insensitive"
                        })),
                        sort_name: Some(CompactString::from(if options.natural {
                            "natural"
                        } else {
                            "alphanumeric"
                        })),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(source_text, member.name_span),
                    fix: sort_keys_fix(source_text, &facts.comments, prev, member),
                });
            }
            previous = Some(member);
        }
    }
}

// Build the autofix that swaps the two out-of-order members, mirroring
// upstream's `[replaceText(member, prevText), replaceText(prevMember, memberText)]`
// (which ESLint merges into one range edit). Returns None when either member has
// an adjacent comment, exactly as upstream's fixer bails out.
fn sort_keys_fix(
    source_text: &str,
    comments: &[ByteSpan],
    prev: &MemberFact,
    member: &MemberFact,
) -> Option<DiagnosticFix> {
    if has_adjacent_comment(source_text, comments, prev.member_span)
        || has_adjacent_comment(source_text, comments, member.member_span)
    {
        return None;
    }

    let prev_text = &source_text[prev.member_span.start..prev.member_span.end];
    let between = &source_text[prev.member_span.end..member.member_span.start];
    let member_text = &source_text[member.member_span.start..member.member_span.end];

    let mut replacement = CompactString::from(member_text);
    replacement.push_str(between);
    replacement.push_str(prev_text);

    Some(DiagnosticFix {
        start: utf16_offset(source_text, prev.member_span.start),
        end: utf16_offset(source_text, member.member_span.end),
        replacement,
    })
}

// Whether a comment sits immediately before or after a member's span (skipping a
// single trailing comma after it), matching upstream's `hasAdjacentComment`
// which uses `getTokenBefore`/`getTokenAfter` with `includeComments`.
fn has_adjacent_comment(source_text: &str, comments: &[ByteSpan], span: ByteSpan) -> bool {
    let before = comments.iter().any(|comment| {
        comment.end <= span.start
            && source_text
                .get(comment.end..span.start)
                .is_some_and(|gap| gap.chars().all(char::is_whitespace))
    });
    if before {
        return true;
    }
    comments.iter().any(|comment| {
        comment.start >= span.end
            && source_text
                .get(span.end..comment.start)
                .is_some_and(is_whitespace_with_optional_comma)
    })
}

fn is_whitespace_with_optional_comma(gap: &str) -> bool {
    let mut comma_seen = false;
    for ch in gap.chars() {
        if ch == ',' && !comma_seen {
            comma_seen = true;
        } else if !ch.is_whitespace() {
            return false;
        }
    }
    true
}

// Inclusive (start_line, end_line) ranges of every comment, used to ignore blank
// lines that fall inside a comment when computing line-separated groups.
fn comment_line_ranges(
    source_text: &str,
    line_index: &LineIndex,
    comments: &[ByteSpan],
) -> SmallVec<[(u32, u32); 8]> {
    comments
        .iter()
        .map(|comment| {
            let (start_line, _) = line_index.position_for_offset(source_text, comment.start);
            let (end_line, _) = line_index.position_for_offset(source_text, comment.end);
            (start_line, end_line)
        })
        .collect()
}

fn scan_top_level_interop(
    source_text: &str,
    line_index: &LineIndex,
    facts: &ParsedFacts,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    let Some(top_level) = facts.top_level else {
        return;
    };
    if matches!(
        top_level.value_type,
        JsonValueType::Object | JsonValueType::Array
    ) {
        return;
    }

    diagnostics.push(Diagnostic {
        rule_name: "top-level-interop",
        message_id: "topLevel",
        data: DiagnosticData {
            type_name: Some(CompactString::from(top_level.value_type.upstream_name())),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, top_level.span),
        fix: None,
    });
}

fn scan_number(
    source_text: &str,
    line_index: &LineIndex,
    number: &NumberFact,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    let raw = number.raw.as_str();
    if is_hex_number(raw) {
        return;
    }

    if is_json5_non_finite(raw) {
        diagnostics.push(number_diagnostic(
            "unsafeNumber",
            raw,
            source_text,
            line_index,
            number.span,
        ));
        return;
    }

    let parse_text = normalize_number_for_parse(raw);
    let Ok(value) = parse_text.as_str().parse::<f64>() else {
        return;
    };

    if !value.is_finite() {
        diagnostics.push(number_diagnostic(
            "unsafeNumber",
            raw,
            source_text,
            line_index,
            number.span,
        ));
    } else if value == 0.0 && numeric_mantissa_has_non_zero(raw) {
        diagnostics.push(number_diagnostic(
            "unsafeZero",
            raw,
            source_text,
            line_index,
            number.span,
        ));
    } else if is_decimal_integer(raw) && value.abs() > MAX_SAFE_INTEGER {
        diagnostics.push(number_diagnostic(
            "unsafeInteger",
            raw,
            source_text,
            line_index,
            number.span,
        ));
    } else if !is_decimal_integer(raw) && value != 0.0 && value.abs() < f64::MIN_POSITIVE {
        diagnostics.push(number_diagnostic(
            "subnormal",
            raw,
            source_text,
            line_index,
            number.span,
        ));
    }
}

fn number_diagnostic(
    message_id: &'static str,
    raw: &str,
    source_text: &str,
    line_index: &LineIndex,
    span: ByteSpan,
) -> Diagnostic {
    Diagnostic {
        rule_name: "no-unsafe-values",
        message_id,
        data: DiagnosticData {
            value: Some(CompactString::from(raw)),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, span),
        fix: None,
    }
}

fn scan_string_surrogates(
    source_text: &str,
    line_index: &LineIndex,
    string: &StringFact,
    diagnostics: &mut SmallVec<[Diagnostic; 16]>,
) {
    let mut index = 0;
    while index < string.code_units.len() {
        let unit = string.code_units[index];
        if is_high_surrogate(unit) {
            if string
                .code_units
                .get(index + 1)
                .copied()
                .is_some_and(is_low_surrogate)
            {
                index += 2;
                continue;
            }
            diagnostics.push(lone_surrogate_diagnostic(
                source_text,
                line_index,
                string.span,
                unit,
            ));
        } else if is_low_surrogate(unit) {
            diagnostics.push(lone_surrogate_diagnostic(
                source_text,
                line_index,
                string.span,
                unit,
            ));
        }
        index += 1;
    }
}

fn lone_surrogate_diagnostic(
    source_text: &str,
    line_index: &LineIndex,
    span: ByteSpan,
    unit: u16,
) -> Diagnostic {
    Diagnostic {
        rule_name: "no-unsafe-values",
        message_id: "loneSurrogate",
        data: DiagnosticData {
            surrogate: Some(escaped_surrogate(unit)),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, span),
        fix: None,
    }
}

fn escaped_surrogate(unit: u16) -> CompactString {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = CompactString::from("\\u");
    out.push(HEX[((unit >> 12) & 0xf) as usize] as char);
    out.push(HEX[((unit >> 8) & 0xf) as usize] as char);
    out.push(HEX[((unit >> 4) & 0xf) as usize] as char);
    out.push(HEX[(unit & 0xf) as usize] as char);
    out
}

fn is_high_surrogate(unit: u16) -> bool {
    (0xd800..=0xdbff).contains(&unit)
}

fn is_low_surrogate(unit: u16) -> bool {
    (0xdc00..=0xdfff).contains(&unit)
}

fn normalize_to(value: &str, form: NormalizationForm) -> CompactString {
    let mut out = CompactString::new("");
    match form {
        NormalizationForm::Nfc => {
            for ch in value.nfc() {
                out.push(ch);
            }
        }
        NormalizationForm::Nfd => {
            for ch in value.nfd() {
                out.push(ch);
            }
        }
        NormalizationForm::Nfkc => {
            for ch in value.nfkc() {
                out.push(ch);
            }
        }
        NormalizationForm::Nfkd => {
            for ch in value.nfkd() {
                out.push(ch);
            }
        }
    }
    out
}

fn normalize_number_for_parse(raw: &str) -> CompactString {
    let mut out = CompactString::new("");
    if let Some(rest) = raw.strip_prefix('.') {
        out.push('0');
        out.push('.');
        out.push_str(rest);
    } else if let Some(rest) = raw.strip_prefix("+.") {
        out.push_str("+0.");
        out.push_str(rest);
    } else if let Some(rest) = raw.strip_prefix("-.") {
        out.push_str("-0.");
        out.push_str(rest);
    } else {
        out.push_str(raw);
    }
    out
}

fn is_json5_non_finite(raw: &str) -> bool {
    matches!(
        raw.trim_start_matches(['+', '-']),
        "Infinity" | "NaN" | "inf" | "Inf" | "INF"
    )
}

fn is_hex_number(raw: &str) -> bool {
    let text = raw.trim_start_matches(['+', '-']);
    text.starts_with("0x") || text.starts_with("0X")
}

fn is_decimal_integer(raw: &str) -> bool {
    let text = raw.trim_start_matches(['+', '-']);
    !text.contains(['.', 'e', 'E']) && text.chars().all(|ch| ch.is_ascii_digit())
}

fn numeric_mantissa_has_non_zero(raw: &str) -> bool {
    let text = raw.trim_start_matches(['+', '-']);
    let mantissa = text.split(['e', 'E']).next().unwrap_or(text);
    mantissa.chars().any(|ch| matches!(ch, '1'..='9'))
}

fn is_valid_sort_order(previous: &str, current: &str, options: &SortOptions) -> bool {
    let previous_key;
    let current_key;
    let (left, right) = if options.case_sensitive {
        (previous, current)
    } else {
        previous_key = lower(previous);
        current_key = lower(current);
        (previous_key.as_str(), current_key.as_str())
    };

    let order = if options.natural {
        natural_compare(left, right)
    } else {
        left.cmp(right)
    };

    match options.direction {
        SortDirection::Ascending => !order.is_gt(),
        SortDirection::Descending => !order.is_lt(),
    }
}

fn lower(value: &str) -> CompactString {
    let mut out = CompactString::new("");
    for ch in value.chars() {
        for lowered in ch.to_lowercase() {
            out.push(lowered);
        }
    }
    out
}

// Faithful port of the `natural-compare` npm package (v1.4.0, MIT, by Lauri
// Rooden) that upstream's sort-keys rule uses. It remaps each UTF-16 code unit
// through `nc_single` (so e.g. `_` sorts before letters) and compares runs of
// digits numerically. Operating on UTF-16 code units mirrors `String.charCodeAt`;
// digit runs are compared by (length, then lexicographically) — exact for any
// length and free of the float arithmetic the original uses.
fn natural_compare(left: &str, right: &str) -> core::cmp::Ordering {
    let a: SmallVec<[u16; 16]> = left.encode_utf16().collect();
    let b: SmallVec<[u16; 16]> = right.encode_utf16().collect();
    if a == b {
        return core::cmp::Ordering::Equal;
    }

    let mut pos_a = 0usize;
    let mut pos_b = 0usize;
    loop {
        let code_a = nc_single(&a, pos_a);
        let code_b = nc_single(&b, pos_b);

        // Both code units are digits `1`..`9` (mapped to 67..=75): compare the
        // whole numeric runs instead of code-unit by code-unit.
        if (67..76).contains(&code_a) && (67..76).contains(&code_b) {
            let end_a = nc_digit_run_end(&a, pos_a);
            let end_b = nc_digit_run_end(&b, pos_b);
            match compare_digit_runs(&a[pos_a..end_a], &b[pos_b..end_b]) {
                core::cmp::Ordering::Equal => {
                    pos_a = end_a;
                    pos_b = end_b;
                    continue;
                }
                ordering => return ordering,
            }
        }

        if code_a != code_b {
            return code_a.cmp(&code_b);
        }
        if code_a == 0 {
            // Both strings are exhausted at the same point.
            return core::cmp::Ordering::Equal;
        }
        pos_a += 1;
        pos_b += 1;
    }
}

// `getCode` for a single UTF-16 code unit: out-of-range yields 0, and printable
// ASCII is remapped so punctuation/digits/letters interleave the way
// natural-compare expects.
fn nc_single(units: &[u16], pos: usize) -> i32 {
    let code = i32::from(units.get(pos).copied().unwrap_or(0));
    if !(45..=127).contains(&code) {
        code
    } else if code < 46 {
        65
    } else if code < 48 {
        code - 1
    } else if code < 58 {
        code + 18
    } else if code < 65 {
        code - 11
    } else if code < 91 {
        code + 11
    } else if code < 97 {
        code - 37
    } else if code < 123 {
        code + 5
    } else {
        code - 63
    }
}

// Index just past the run of digit code units starting at `pos`.
fn nc_digit_run_end(units: &[u16], pos: usize) -> usize {
    let mut end = pos;
    while end < units.len() && (66..76).contains(&nc_single(units, end)) {
        end += 1;
    }
    end
}

// Compare two digit runs (each starting with `1`..`9`, so no leading zeros) by
// magnitude: the longer run is larger, ties broken lexicographically (which for
// ASCII digits equals numeric order).
fn compare_digit_runs(left: &[u16], right: &[u16]) -> core::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn is_line_separated(
    source_text: &str,
    line_index: &LineIndex,
    comment_lines: &[(u32, u32)],
    prev_span: ByteSpan,
    current_span: ByteSpan,
) -> bool {
    let (prev_end_line, _) = line_index.position_for_offset(source_text, prev_span.end);
    let (current_start_line, _) = line_index.position_for_offset(source_text, current_span.start);
    if current_start_line.saturating_sub(prev_end_line) < 2 {
        return false;
    }

    for line in (prev_end_line + 1)..current_start_line {
        // A blank line that falls inside a comment does not start a new group
        // (upstream excludes `commentLineNums`).
        if comment_lines
            .iter()
            .any(|(start, end)| (*start..=*end).contains(&line))
        {
            continue;
        }
        if let Some(text) = line_index.line_text(source_text, line)
            && text.trim().is_empty()
        {
            return true;
        }
    }
    false
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
    facts: ParsedFacts,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            facts: ParsedFacts::default(),
        }
    }

    fn parse(&mut self) -> Option<ParsedFacts> {
        self.skip_ws_and_comments();
        let top = self.parse_value()?;
        self.facts.top_level = Some(top);
        self.skip_ws_and_comments();
        Some(core::mem::take(&mut self.facts))
    }

    fn parse_value(&mut self) -> Option<TopLevelFact> {
        self.skip_ws_and_comments();
        let start = self.pos;
        let byte = self.current_byte()?;
        match byte {
            b'{' => self.parse_object(start),
            b'[' => self.parse_array(start),
            b'"' | b'\'' => {
                let token = self.parse_string(byte)?;
                self.facts.strings.push(StringFact {
                    code_units: token.code_units,
                    span: token.span,
                });
                Some(TopLevelFact {
                    value_type: JsonValueType::String,
                    span: token.span,
                })
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => Some(self.parse_number_or_special(start)),
            _ => Some(self.parse_identifier_value(start)),
        }
    }

    fn parse_object(&mut self, start: usize) -> Option<TopLevelFact> {
        self.pos += 1;
        let mut members = SmallVec::<[MemberFact; 8]>::new();
        loop {
            self.skip_ws_and_comments();
            if self.consume_byte(b'}') {
                break;
            }

            let member_start = self.pos;
            let mut key = self.parse_key()?;
            self.skip_ws_and_comments();
            if !self.consume_byte(b':') {
                return None;
            }
            let value = self.parse_value()?;
            key.member_span = ByteSpan {
                start: member_start,
                end: value.span.end,
            };
            members.push(key);

            self.skip_ws_and_comments();
            if self.consume_byte(b',') {
                continue;
            }
            if self.consume_byte(b'}') {
                break;
            }
            return None;
        }
        let span = ByteSpan {
            start,
            end: self.pos,
        };
        self.facts.objects.push(ObjectFact { members });
        Some(TopLevelFact {
            value_type: JsonValueType::Object,
            span,
        })
    }

    fn parse_array(&mut self, start: usize) -> Option<TopLevelFact> {
        self.pos += 1;
        loop {
            self.skip_ws_and_comments();
            if self.consume_byte(b']') {
                break;
            }
            let _ = self.parse_value()?;
            self.skip_ws_and_comments();
            if self.consume_byte(b',') {
                continue;
            }
            if self.consume_byte(b']') {
                break;
            }
            return None;
        }
        Some(TopLevelFact {
            value_type: JsonValueType::Array,
            span: ByteSpan {
                start,
                end: self.pos,
            },
        })
    }

    fn parse_key(&mut self) -> Option<MemberFact> {
        let start = self.pos;
        match self.current_byte()? {
            b'"' | b'\'' => {
                let token = self.parse_string(self.current_byte()?)?;
                let fact = StringFact {
                    code_units: token.code_units,
                    span: token.span,
                };
                self.facts.strings.push(fact);
                Some(MemberFact {
                    key: token.value,
                    raw_key: token.raw,
                    name_span: token.span,
                    key_content_span: token.content_span,
                    member_span: ByteSpan { start, end: start },
                    key_has_escape: token.has_escape,
                })
            }
            _ => {
                let ident = self.consume_identifier();
                if ident.is_empty() {
                    return None;
                }
                // JSON5 identifier names may carry Unicode escapes (e.g.
                // `foot`), which momoa decodes for the key's value while
                // the raw key keeps the source text. Mirror that so duplicate
                // and sort comparisons see the decoded name; when the decoded
                // name differs from the source, the key has an escape and
                // no-unnormalized-keys must not autofix it (upstream returns
                // null when `key !== rawKey`).
                let key = decode_identifier_escapes(ident);
                let key_has_escape = key.as_str() != ident;
                Some(MemberFact {
                    key,
                    raw_key: CompactString::from(ident),
                    name_span: ByteSpan {
                        start,
                        end: self.pos,
                    },
                    key_content_span: ByteSpan {
                        start,
                        end: self.pos,
                    },
                    member_span: ByteSpan { start, end: start },
                    key_has_escape,
                })
            }
        }
    }

    fn parse_number_or_special(&mut self, start: usize) -> TopLevelFact {
        if self.consume_signed_word(start, "Infinity") || self.consume_signed_word(start, "NaN") {
            let span = ByteSpan {
                start,
                end: self.pos,
            };
            self.facts.numbers.push(NumberFact {
                raw: CompactString::from(&self.source[start..self.pos]),
                span,
            });
            return TopLevelFact {
                value_type: JsonValueType::Number,
                span,
            };
        }

        let mut seen_dot = false;
        let mut seen_exp = false;
        if matches!(self.current_byte(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        if self.source[self.pos..].starts_with("0x") || self.source[self.pos..].starts_with("0X") {
            self.pos += 2;
            while self.current_char().is_some_and(|ch| ch.is_ascii_hexdigit()) {
                self.advance_char();
            }
        } else {
            while let Some(ch) = self.current_char() {
                if ch.is_ascii_digit() {
                    self.advance_char();
                } else if ch == '.' && !seen_dot && !seen_exp {
                    seen_dot = true;
                    self.advance_char();
                } else if matches!(ch, 'e' | 'E') && !seen_exp {
                    seen_exp = true;
                    self.advance_char();
                    if matches!(self.current_byte(), Some(b'+' | b'-')) {
                        self.pos += 1;
                    }
                } else {
                    break;
                }
            }
        }

        let span = ByteSpan {
            start,
            end: self.pos,
        };
        self.facts.numbers.push(NumberFact {
            raw: CompactString::from(&self.source[start..self.pos]),
            span,
        });
        TopLevelFact {
            value_type: JsonValueType::Number,
            span,
        }
    }

    fn parse_identifier_value(&mut self, start: usize) -> TopLevelFact {
        let ident = self.consume_identifier();
        let value_type = match ident {
            "true" | "false" => JsonValueType::Boolean,
            "null" => JsonValueType::Null,
            _ => JsonValueType::Identifier,
        };
        TopLevelFact {
            value_type,
            span: ByteSpan {
                start,
                end: self.pos,
            },
        }
    }

    fn parse_string(&mut self, quote: u8) -> Option<StringToken> {
        let start = self.pos;
        self.pos += 1;
        let content_start = self.pos;
        let mut value = CompactString::new("");
        let mut raw = CompactString::new("");
        let mut code_units = SmallVec::<[u16; 16]>::new();
        let mut has_escape = false;

        while self.pos < self.source.len() {
            let ch_start = self.pos;
            let ch = self.current_char()?;
            if ch as u32 == u32::from(quote) {
                let content_end = self.pos;
                self.advance_char();
                return Some(StringToken {
                    value,
                    raw,
                    code_units,
                    span: ByteSpan {
                        start,
                        end: self.pos,
                    },
                    content_span: ByteSpan {
                        start: content_start,
                        end: content_end,
                    },
                    has_escape,
                });
            }

            if ch == '\\' {
                has_escape = true;
                self.advance_char();
                raw.push_str(&self.source[ch_start..self.pos]);
                self.parse_escape(&mut value, &mut raw, &mut code_units);
            } else {
                self.advance_char();
                raw.push_str(&self.source[ch_start..self.pos]);
                value.push(ch);
                push_utf16_units(ch, &mut code_units);
            }
        }
        None
    }

    fn parse_escape(
        &mut self,
        value: &mut CompactString,
        raw: &mut CompactString,
        code_units: &mut SmallVec<[u16; 16]>,
    ) {
        let Some(ch) = self.current_char() else {
            return;
        };
        let escape_start = self.pos;
        self.advance_char();
        raw.push_str(&self.source[escape_start..self.pos]);
        match ch {
            '"' | '\'' | '\\' | '/' => {
                value.push(ch);
                push_utf16_units(ch, code_units);
            }
            'b' => {
                value.push('\u{0008}');
                code_units.push(0x0008);
            }
            'f' => {
                value.push('\u{000c}');
                code_units.push(0x000c);
            }
            'n' => {
                value.push('\n');
                code_units.push(0x000a);
            }
            'r' => {
                value.push('\r');
                code_units.push(0x000d);
            }
            't' => {
                value.push('\t');
                code_units.push(0x0009);
            }
            'v' => {
                value.push('\u{000b}');
                code_units.push(0x000b);
            }
            'x' => {
                if let Some(unit) = self.consume_hex_escape(2, raw) {
                    push_unit_as_value(unit, value, code_units);
                }
            }
            'u' => {
                if let Some(unit) = self.consume_hex_escape(4, raw) {
                    push_unit_as_value(unit, value, code_units);
                }
            }
            '\n' | '\r' => {}
            other => {
                value.push(other);
                push_utf16_units(other, code_units);
            }
        }
    }

    fn consume_hex_escape(&mut self, count: usize, raw: &mut CompactString) -> Option<u16> {
        let start = self.pos;
        let mut value = 0u16;
        for _ in 0..count {
            let ch = self.current_char()?;
            let digit = ch.to_digit(16)?;
            value = value.saturating_mul(16).saturating_add(digit as u16);
            self.advance_char();
        }
        raw.push_str(&self.source[start..self.pos]);
        Some(value)
    }

    fn consume_identifier(&mut self) -> &'a str {
        let start = self.pos;
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() || matches!(ch, ':' | ',' | '}' | ']' | '{' | '[') {
                break;
            }
            self.advance_char();
        }
        self.source[start..self.pos].trim()
    }

    fn consume_signed_word(&mut self, start: usize, word: &str) -> bool {
        let mut word_start = start;
        if matches!(self.source.as_bytes().get(word_start), Some(b'+' | b'-')) {
            word_start += 1;
        }
        if !self.source[word_start..].starts_with(word) {
            return false;
        }
        let end = word_start + word.len();
        if self
            .source
            .get(end..)
            .and_then(|rest| rest.chars().next())
            .is_some_and(is_identifier_continue)
        {
            return false;
        }
        self.pos = end;
        true
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            let before = self.pos;
            while self.current_char().is_some_and(char::is_whitespace) {
                self.advance_char();
            }
            if self.source[self.pos..].starts_with("//") {
                let comment_start = self.pos;
                while let Some(ch) = self.current_char() {
                    if matches!(ch, '\n' | '\r') {
                        break;
                    }
                    self.advance_char();
                }
                self.facts.comments.push(ByteSpan {
                    start: comment_start,
                    end: self.pos,
                });
            } else if self.source[self.pos..].starts_with("/*") {
                let comment_start = self.pos;
                self.pos += 2;
                while self.pos < self.source.len() && !self.source[self.pos..].starts_with("*/") {
                    self.advance_char();
                }
                if self.source[self.pos..].starts_with("*/") {
                    self.pos += 2;
                }
                self.facts.comments.push(ByteSpan {
                    start: comment_start,
                    end: self.pos,
                });
            }
            if self.pos == before {
                break;
            }
        }
    }

    fn consume_byte(&mut self, byte: u8) -> bool {
        if self.current_byte() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn current_byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.pos).copied()
    }

    fn current_char(&self) -> Option<char> {
        self.source.get(self.pos..)?.chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StringToken {
    value: CompactString,
    raw: CompactString,
    code_units: SmallVec<[u16; 16]>,
    span: ByteSpan,
    content_span: ByteSpan,
    has_escape: bool,
}

fn push_unit_as_value(unit: u16, value: &mut CompactString, code_units: &mut SmallVec<[u16; 16]>) {
    code_units.push(unit);
    if let Some(ch) = char::from_u32(u32::from(unit)) {
        value.push(ch);
    }
}

fn push_utf16_units(ch: char, units: &mut SmallVec<[u16; 16]>) {
    let mut buf = [0u16; 2];
    for unit in ch.encode_utf16(&mut buf) {
        units.push(*unit);
    }
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '$')
}

// Decode the Unicode escapes (`\uXXXX` or `\u{...}`) that a JSON5 identifier
// name may contain, returning the identifier's value. Non-escaped identifiers
// pass through unchanged so the common case allocates the raw text verbatim.
fn decode_identifier_escapes(ident: &str) -> CompactString {
    if !ident.contains('\\') {
        return CompactString::from(ident);
    }

    let mut out = CompactString::new("");
    let mut chars = ident.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' || chars.peek() != Some(&'u') {
            out.push(ch);
            continue;
        }
        chars.next();

        let code = if chars.peek() == Some(&'{') {
            chars.next();
            let mut value = 0u32;
            let mut any = false;
            while let Some(&candidate) = chars.peek() {
                if candidate == '}' {
                    chars.next();
                    break;
                }
                let Some(digit) = candidate.to_digit(16) else {
                    break;
                };
                value = value.saturating_mul(16).saturating_add(digit);
                any = true;
                chars.next();
            }
            any.then_some(value)
        } else {
            let mut value = 0u32;
            let mut count = 0;
            while count < 4 {
                let Some(digit) = chars.peek().and_then(|candidate| candidate.to_digit(16)) else {
                    break;
                };
                value = value * 16 + digit;
                count += 1;
                chars.next();
            }
            (count == 4).then_some(value)
        };

        if let Some(ch) = code.and_then(char::from_u32) {
            out.push(ch);
        }
    }
    out
}

fn utf16_offset(source_text: &str, byte_offset: usize) -> u32 {
    source_text[..byte_offset.min(source_text.len())]
        .chars()
        .map(char::len_utf16)
        .sum::<usize>() as u32
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source_text: &str, span: ByteSpan) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }

    fn line_text<'a>(&self, source_text: &'a str, one_based_line: u32) -> Option<&'a str> {
        let index = usize::try_from(one_based_line).ok()?.checked_sub(1)?;
        let start = *self.line_starts.get(index)?;
        let end = self
            .line_starts
            .get(index + 1)
            .copied()
            .unwrap_or(source_text.len());
        source_text.get(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NormalizationForm, RULE_NAMES, ScanOptions, SortDirection, natural_compare,
        scan_eslint_json,
    };
    use core::cmp::Ordering;

    // Locks in the `natural-compare` semantics, including digit runs that begin
    // with `0` (which the package does NOT treat as a number, since its chunk
    // trigger requires a unit > 66, i.e. `1`..`9`). Values verified against the
    // upstream `natural-compare` v1.4.0 package.
    #[test]
    fn natural_compare_matches_upstream() {
        assert_eq!(natural_compare("00", "9"), Ordering::Less);
        assert_eq!(natural_compare("a0b", "a00b"), Ordering::Greater);
        assert_eq!(natural_compare("0x", "00x"), Ordering::Greater);
        assert_eq!(natural_compare("10", "9"), Ordering::Greater);
        assert_eq!(natural_compare("1", "10"), Ordering::Less);
        assert_eq!(natural_compare("11", "2"), Ordering::Greater);
        // The remapping puts `_`/`$` before letters and keeps case order.
        assert_eq!(natural_compare("_", "A"), Ordering::Less);
        assert_eq!(natural_compare("$", "_"), Ordering::Less);
        assert_eq!(natural_compare("A", "a"), Ordering::Less);
        assert_eq!(natural_compare("abc", "abc"), Ordering::Equal);
    }

    // A JSON5 identifier key that is both escaped and unnormalized must be
    // reported but NOT autofixed, mirroring upstream's `key !== rawKey` bail-out.
    #[test]
    fn unnormalized_escaped_identifier_key_has_no_fix() {
        // A JSON5 identifier key written with Unicode escapes that decode
        // to `e` + combining acute (the decomposed form, which NFC
        // recomposes). Built from a backslash char so the escape token is
        // not rewritten by tooling.
        let source = "{\\u0065\\u0301: 1}";
        let diagnostics = scan_eslint_json(source, &ScanOptions::default());
        let unnormalized: oxlint_plugins_carton::SmallVec<[_; 4]> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_name == "no-unnormalized-keys")
            .collect();
        assert_eq!(unnormalized.len(), 1);
        assert!(unnormalized[0].fix.is_none());
    }

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(
            RULE_NAMES,
            [
                "no-duplicate-keys",
                "no-empty-keys",
                "no-unnormalized-keys",
                "no-unsafe-values",
                "sort-keys",
                "top-level-interop",
            ],
        );
    }

    #[test]
    fn scans_representative_json_rules() {
        let source = r#"{"": 1, "b": 2, "a": 3, "b": 4, "é": "x", "unsafe": 2e308}"#;
        let diagnostics = scan_eslint_json(source, &ScanOptions::default());
        let rule_names: oxlint_plugins_carton::SmallVec<[&str; 8]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();

        assert!(rule_names.contains(&"no-empty-keys"));
        assert!(rule_names.contains(&"no-duplicate-keys"));
        assert!(rule_names.contains(&"no-unnormalized-keys"));
        assert!(rule_names.contains(&"no-unsafe-values"));
        assert!(rule_names.contains(&"sort-keys"));
    }

    #[test]
    fn honors_sort_and_normalization_options() {
        let normalization_options = ScanOptions {
            normalization_form: NormalizationForm::Nfkd,
            ..ScanOptions::default()
        };
        let normalization_diagnostics = scan_eslint_json(r#"{"é": 3}"#, &normalization_options);

        assert!(
            normalization_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_name == "no-unnormalized-keys")
        );

        let mut sort_options = ScanOptions::default();
        sort_options.sort.direction = SortDirection::Descending;
        let sort_diagnostics = scan_eslint_json(r#"{"b": 1, "a": 2}"#, &sort_options);
        assert!(
            sort_diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule_name != "sort-keys")
        );
    }
}
