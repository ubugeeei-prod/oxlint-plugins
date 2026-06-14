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
                    fix: None,
                });
            }
            previous = Some(member);
        }
    }
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
        natural_cmp(left, right)
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

fn natural_cmp(left: &str, right: &str) -> core::cmp::Ordering {
    let mut left_iter = left.char_indices().peekable();
    let mut right_iter = right.char_indices().peekable();

    loop {
        match (left_iter.peek().copied(), right_iter.peek().copied()) {
            (None, None) => return core::cmp::Ordering::Equal,
            (None, Some(_)) => return core::cmp::Ordering::Less,
            (Some(_), None) => return core::cmp::Ordering::Greater,
            (Some((_, left_ch)), Some((_, right_ch)))
                if left_ch.is_ascii_digit() && right_ch.is_ascii_digit() =>
            {
                let left_number = consume_digit_run(left, &mut left_iter);
                let right_number = consume_digit_run(right, &mut right_iter);
                let ordering = compare_digit_runs(left_number, right_number);
                if !ordering.is_eq() {
                    return ordering;
                }
            }
            (Some((_, left_ch)), Some((_, right_ch))) => {
                let _ = left_iter.next();
                let _ = right_iter.next();
                let ordering = left_ch.cmp(&right_ch);
                if !ordering.is_eq() {
                    return ordering;
                }
            }
        }
    }
}

fn consume_digit_run<'a>(
    source: &'a str,
    iter: &mut core::iter::Peekable<core::str::CharIndices<'a>>,
) -> &'a str {
    let Some((start, _)) = iter.peek().copied() else {
        return "";
    };
    let mut end = start;
    while let Some((index, ch)) = iter.peek().copied() {
        if !ch.is_ascii_digit() {
            break;
        }
        end = index + ch.len_utf8();
        let _ = iter.next();
    }
    &source[start..end]
}

fn compare_digit_runs(left: &str, right: &str) -> core::cmp::Ordering {
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');
    let left_cmp = if left_trimmed.is_empty() {
        "0"
    } else {
        left_trimmed
    };
    let right_cmp = if right_trimmed.is_empty() {
        "0"
    } else {
        right_trimmed
    };

    left_cmp
        .len()
        .cmp(&right_cmp.len())
        .then_with(|| left_cmp.cmp(right_cmp))
        .then_with(|| left.len().cmp(&right.len()))
}

fn is_line_separated(
    source_text: &str,
    line_index: &LineIndex,
    prev_span: ByteSpan,
    current_span: ByteSpan,
) -> bool {
    let (prev_end_line, _) = line_index.position_for_offset(source_text, prev_span.end);
    let (current_start_line, _) = line_index.position_for_offset(source_text, current_span.start);
    if current_start_line.saturating_sub(prev_end_line) < 2 {
        return false;
    }

    for line in (prev_end_line + 1)..current_start_line {
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
                Some(MemberFact {
                    // JSON5 identifier names may carry Unicode escapes (e.g.
                    // `foot`), which momoa decodes for the key's value while
                    // the raw key keeps the source text. Mirror that so duplicate
                    // and sort comparisons see the decoded name.
                    key: decode_identifier_escapes(ident),
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
                    key_has_escape: false,
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
                while let Some(ch) = self.current_char() {
                    self.advance_char();
                    if matches!(ch, '\n' | '\r') {
                        break;
                    }
                }
            } else if self.source[self.pos..].starts_with("/*") {
                self.pos += 2;
                while self.pos < self.source.len() && !self.source[self.pos..].starts_with("*/") {
                    self.advance_char();
                }
                if self.source[self.pos..].starts_with("*/") {
                    self.pos += 2;
                }
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
    use super::{NormalizationForm, RULE_NAMES, ScanOptions, SortDirection, scan_eslint_json};

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
