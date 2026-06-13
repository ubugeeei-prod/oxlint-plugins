#![doc = "Rust implementation of @unocss/eslint-plugin rule logic."]

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

pub const RULE_NAMES: [&str; 4] = [
    "blocklist",
    "enforce-class-compile",
    "order",
    "order-attributify",
];

const DEFAULT_UNO_FUNCTIONS: [&str; 2] = ["clsx", "classnames"];
const DEFAULT_UNO_VARIABLES: [&str; 2] = ["^cls", "classNames?$"];
const IGNORED_ATTRIBUTIFY_ATTRIBUTES: [&str; 4] = ["style", "class", "classname", "value"];

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
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
    pub name: Option<CompactString>,
    pub reason: Option<CompactString>,
    pub prefix: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlocklistEntry {
    pub name: CompactString,
    pub reason: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnocssOptions {
    pub uno_functions: SmallVec<[CompactString; 4]>,
    pub uno_variables: SmallVec<[CompactString; 4]>,
    pub blocklist: SmallVec<[BlocklistEntry; 4]>,
    pub class_compile_prefix: CompactString,
    pub class_compile_enable_fix: bool,
}

impl Default for UnocssOptions {
    fn default() -> Self {
        Self {
            uno_functions: DEFAULT_UNO_FUNCTIONS
                .into_iter()
                .map(CompactString::from)
                .collect(),
            uno_variables: DEFAULT_UNO_VARIABLES
                .into_iter()
                .map(CompactString::from)
                .collect(),
            blocklist: SmallVec::new(),
            class_compile_prefix: CompactString::from(":uno:"),
            class_compile_enable_fix: true,
        }
    }
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

    fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }
}

#[derive(Clone, Copy)]
struct LiteralSpan<'a> {
    full_start: usize,
    content_start: usize,
    content_end: usize,
    content: &'a str,
}

#[derive(Clone)]
struct TokenPart<'a> {
    text: &'a str,
    index: usize,
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    options: UnocssOptions,
    variable_regexes: SmallVec<[Regex; 4]>,
    diagnostics: SmallVec<[Diagnostic; 16]>,
}

#[derive(Default)]
struct ReportData {
    fix: Option<DiagnosticFix>,
    name: Option<CompactString>,
    reason: Option<CompactString>,
    prefix: Option<CompactString>,
}

pub fn implemented_unocss_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_unocss(
    source_text: &str,
    filename: &str,
    options: &UnocssOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let variable_regexes = options
        .uno_variables
        .iter()
        .filter_map(|pattern| Regex::new(pattern.as_str()).ok())
        .collect();
    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        options: options.clone(),
        variable_regexes,
        diagnostics: SmallVec::new(),
    };
    scanner.scan_literals();
    scanner.scan_attributify();
    scanner.diagnostics
}

impl<'a> Scanner<'a> {
    fn scan_literals(&mut self) {
        for literal in collect_literals(self.source_text) {
            if literal.content.trim().is_empty() || literal.content.contains('\\') {
                continue;
            }

            let class_context = is_jsx_class_literal(self.source_text, literal);
            if class_context {
                self.check_blocklist(literal);
                self.check_class_compile(literal);
            }

            if class_context
                || is_uno_call_literal(
                    self.source_text,
                    literal.full_start,
                    &self.options.uno_functions,
                )
                || self.is_uno_variable_literal(literal.full_start)
            {
                self.check_order(literal);
            }
        }
    }

    fn scan_attributify(&mut self) {
        let bytes = self.source_text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] != b'<' || index + 1 >= bytes.len() || bytes[index + 1] == b'/' {
                index += 1;
                continue;
            }
            if !is_identifier_start(bytes[index + 1]) {
                index += 1;
                continue;
            }

            let Some(tag_end) = find_tag_end(self.source_text, index + 1) else {
                break;
            };
            self.check_tag_attributify(index, tag_end);
            index = tag_end + 1;
        }
    }

    fn check_tag_attributify(&mut self, tag_start: usize, tag_end: usize) {
        let mut cursor = tag_start + 1;
        let bytes = self.source_text.as_bytes();
        while cursor < tag_end && is_identifier_part(bytes[cursor]) {
            cursor += 1;
        }

        let mut attrs: SmallVec<[(CompactString, usize, usize); 8]> = SmallVec::new();
        while cursor < tag_end {
            while cursor < tag_end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= tag_end || bytes[cursor] == b'/' {
                break;
            }
            if !is_identifier_start(bytes[cursor]) && bytes[cursor] != b':' {
                cursor += 1;
                continue;
            }

            let name_start = cursor;
            cursor += 1;
            while cursor < tag_end && is_attr_name_part(bytes[cursor]) {
                cursor += 1;
            }
            let name_end = cursor;
            let name = &self.source_text[name_start..name_end];
            while cursor < tag_end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }

            if cursor < tag_end && bytes[cursor] == b'=' {
                cursor = skip_attribute_value(self.source_text, cursor + 1, tag_end);
                continue;
            }

            let lower = name.to_ascii_lowercase();
            if IGNORED_ATTRIBUTIFY_ATTRIBUTES.contains(&lower.as_str()) {
                continue;
            }

            self.check_blocked_token(name, Span::new(name_start as u32, name_end as u32));
            if is_unocss_token(name) {
                attrs.push((CompactString::from(name), name_start, name_end));
            }
        }

        if attrs.len() < 2 {
            return;
        }

        let names: SmallVec<[&str; 8]> = attrs.iter().map(|(name, _, _)| name.as_str()).collect();
        let sorted = sort_class_tokens(names.as_slice());
        let sorted_text = join_tokens(sorted.as_slice());
        let input_text = join_tokens(names.as_slice());
        if sorted_text == input_text {
            return;
        }

        let contiguous = attrs
            .windows(2)
            .all(|pair| self.source_text[pair[0].2..pair[1].1].trim().is_empty());
        let fix = if contiguous {
            let start = attrs[0].1;
            let end = attrs[attrs.len() - 1].2;
            Some(DiagnosticFix {
                start: start as u32,
                end: end as u32,
                replacement: sorted_text,
            })
        } else {
            None
        };
        self.report(
            "order-attributify",
            "invalid-order",
            Span::new(tag_start as u32, tag_end as u32),
            ReportData {
                fix,
                ..ReportData::default()
            },
        );
    }

    fn check_blocklist(&mut self, literal: LiteralSpan<'_>) {
        for token in literal.content.split_whitespace() {
            self.check_blocked_token(
                token,
                Span::new(literal.content_start as u32, literal.content_end as u32),
            );
        }
    }

    fn check_blocked_token(&mut self, token: &str, span: Span) {
        let matches: SmallVec<[(CompactString, CompactString); 2]> = self
            .options
            .blocklist
            .iter()
            .filter(|entry| entry.name.as_str() == token)
            .map(|entry| (entry.name.clone(), entry.reason.clone()))
            .collect();

        for (name, reason) in matches {
            self.report(
                "blocklist",
                "in-blocklist",
                span,
                ReportData {
                    name: Some(name),
                    reason: Some(reason),
                    ..ReportData::default()
                },
            );
        }
    }

    fn check_class_compile(&mut self, literal: LiteralSpan<'_>) {
        let trimmed = literal.content.trim_start();
        if trimmed.is_empty() {
            return;
        }
        let expected_prefix = prefix_with_space(self.options.class_compile_prefix.trim());
        if trimmed.starts_with(expected_prefix.as_str()) {
            return;
        }

        let mut replacement = expected_prefix.clone();
        replacement.push_str(literal.content);
        let fix = if self.options.class_compile_enable_fix {
            Some(DiagnosticFix {
                start: literal.content_start as u32,
                end: literal.content_end as u32,
                replacement,
            })
        } else {
            None
        };
        self.report(
            "enforce-class-compile",
            "missing",
            Span::new(literal.content_start as u32, literal.content_end as u32),
            ReportData {
                fix,
                prefix: Some(self.options.class_compile_prefix.clone()),
                ..ReportData::default()
            },
        );
    }

    fn check_order(&mut self, literal: LiteralSpan<'_>) {
        let Some(replacement) = sorted_class_string(literal.content) else {
            return;
        };
        self.report(
            "order",
            "invalid-order",
            Span::new(literal.content_start as u32, literal.content_end as u32),
            ReportData {
                fix: Some(DiagnosticFix {
                    start: literal.content_start as u32,
                    end: literal.content_end as u32,
                    replacement,
                }),
                ..ReportData::default()
            },
        );
    }

    fn is_uno_variable_literal(&self, start: usize) -> bool {
        let statement_start = self.source_text[..start]
            .rfind(';')
            .map_or(0, |index| index + 1);
        let statement = &self.source_text[statement_start..start];
        let Some(name) = variable_name_in_statement(statement) else {
            return false;
        };
        self.variable_regexes
            .iter()
            .any(|regex| regex.is_match(name))
    }

    fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        data: ReportData,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix: data.fix,
            name: data.name,
            reason: data.reason,
            prefix: data.prefix,
        });
    }
}

fn collect_literals(source_text: &str) -> SmallVec<[LiteralSpan<'_>; 16]> {
    let mut literals = SmallVec::new();
    let bytes = source_text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[index];
                let content_start = index + 1;
                let mut cursor = content_start;
                let mut has_template_expr = false;
                while cursor < bytes.len() {
                    if bytes[cursor] == b'\\' {
                        cursor = (cursor + 2).min(bytes.len());
                        continue;
                    }
                    if quote == b'`'
                        && cursor + 1 < bytes.len()
                        && bytes[cursor] == b'$'
                        && bytes[cursor + 1] == b'{'
                    {
                        has_template_expr = true;
                    }
                    if bytes[cursor] == quote {
                        if !has_template_expr {
                            literals.push(LiteralSpan {
                                full_start: index,
                                content_start,
                                content_end: cursor,
                                content: &source_text[content_start..cursor],
                            });
                        }
                        index = cursor + 1;
                        break;
                    }
                    cursor += 1;
                }
                if cursor >= bytes.len() {
                    break;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            _ => index += 1,
        }
    }
    literals
}

fn is_jsx_class_literal(source_text: &str, literal: LiteralSpan<'_>) -> bool {
    let prefix_start = literal.full_start.saturating_sub(80);
    let prefix = source_text[prefix_start..literal.full_start].trim_end();
    let candidates = [
        "class=",
        "className=",
        "class={",
        "className={",
        "classname=",
        "classname={",
    ];
    candidates
        .iter()
        .any(|candidate| prefix.ends_with(candidate))
}

fn is_uno_call_literal(source_text: &str, start: usize, uno_functions: &[CompactString]) -> bool {
    let prefix_start = start.saturating_sub(800);
    let prefix = &source_text[prefix_start..start];
    let last_statement = prefix
        .rfind(';')
        .map_or(prefix, |index| &prefix[index + 1..]);
    uno_functions.iter().any(|function| {
        let Some(call_index) = rfind_function_call(last_statement, function.as_str()) else {
            return false;
        };
        let after_call = &last_statement[call_index + function.len() + 1..];
        after_call.bytes().filter(|byte| *byte == b'(').count()
            <= after_call.bytes().filter(|byte| *byte == b')').count()
    }) || uno_functions.iter().any(|function| {
        let Some(call_index) = rfind_function_call(last_statement, function.as_str()) else {
            return false;
        };
        let after_call = &last_statement[call_index..];
        let opens = after_call.bytes().filter(|byte| *byte == b'(').count();
        let closes = after_call.bytes().filter(|byte| *byte == b')').count();
        opens > closes
    })
}

fn rfind_function_call(statement: &str, function_name: &str) -> Option<usize> {
    let mut end = statement.len();
    while let Some(index) = statement[..end].rfind(function_name) {
        let before = index
            .checked_sub(1)
            .and_then(|previous| statement.as_bytes().get(previous));
        if before.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'$')
        {
            end = index;
            continue;
        }
        let Some(after) = statement.as_bytes().get(index + function_name.len()) else {
            end = index;
            continue;
        };
        if *after == b'(' {
            return Some(index);
        }
        end = index;
    }
    None
}

fn variable_name_in_statement(statement: &str) -> Option<&str> {
    for keyword in ["const", "let", "var"] {
        let Some(index) = statement.rfind(keyword) else {
            continue;
        };
        let after_keyword = statement[index + keyword.len()..].trim_start();
        let name_end = after_keyword
            .find(|ch: char| !(ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()))
            .unwrap_or(after_keyword.len());
        if name_end == 0 {
            continue;
        }
        let name = &after_keyword[..name_end];
        let after_name = after_keyword[name_end..].trim_start();
        if after_name.starts_with('=') {
            return Some(name);
        }
    }
    None
}

fn sorted_class_string(input: &str) -> Option<CompactString> {
    let leading_len = input.len() - input.trim_start().len();
    let trailing_len = input.len() - input.trim_end().len();
    let leading = &input[..leading_len];
    let trailing = &input[input.len() - trailing_len..];
    let body = input.trim();
    if body.is_empty() || body.contains("${") {
        return None;
    }

    let tokens: SmallVec<[&str; 16]> = body.split_whitespace().collect();
    if tokens.len() < 2 || tokens.iter().any(|token| !is_unocss_token(token)) {
        return None;
    }
    let sorted = sort_class_tokens(tokens.as_slice());
    let sorted_body = join_tokens(sorted.as_slice());
    if sorted_body == body {
        return None;
    }
    let mut output = CompactString::new("");
    output.push_str(leading);
    output.push_str(&sorted_body);
    output.push_str(trailing);
    Some(output)
}

fn sort_class_tokens<'a>(tokens: &[&'a str]) -> SmallVec<[&'a str; 16]> {
    let mut parts: SmallVec<[TokenPart<'a>; 16]> = tokens
        .iter()
        .enumerate()
        .map(|(index, text)| TokenPart { text, index })
        .collect();
    parts.sort_by(|left, right| {
        class_rank(left.text, left.index).cmp(&class_rank(right.text, right.index))
    });
    parts.into_iter().map(|part| part.text).collect()
}

fn join_tokens(tokens: &[&str]) -> CompactString {
    let mut output = CompactString::new("");
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        output.push_str(token);
    }
    output
}

fn prefix_with_space(prefix: &str) -> CompactString {
    let mut output = CompactString::from(prefix);
    output.push(' ');
    output
}

fn class_rank(token: &str, index: usize) -> (u16, u16, usize) {
    let base = token.rsplit(':').next().unwrap_or(token);
    if base == "flex" {
        return (10, 0, index);
    }
    if base.starts_with("flex-") {
        return (11, 0, index);
    }
    if base == "grid" {
        return (12, 0, index);
    }
    if base == "block" || base == "inline" || base == "inline-block" || base == "hidden" {
        return (13, 0, index);
    }
    if base.starts_with("text-") {
        return (20, 0, index);
    }
    if let Some(axis) = spacing_axis_rank(base, 'm') {
        return (30, axis, index);
    }
    if let Some(axis) = spacing_axis_rank(base, 'p') {
        return (40, axis, index);
    }
    if let Some(axis) = position_axis_rank(base) {
        return (50, axis, index);
    }
    if base.starts_with("gap") {
        return (60, 0, index);
    }
    if base.starts_with("rounded") {
        return (70, 0, index);
    }
    if base.starts_with("border") {
        return (80, 0, index);
    }
    if base.starts_with("bg-") {
        return (90, 0, index);
    }
    if base.starts_with("h-") || base.starts_with("h") {
        return (100, 0, index);
    }
    if base.starts_with("w-") || base.starts_with("w") {
        return (101, 0, index);
    }
    (10_000, 0, index)
}

fn spacing_axis_rank(base: &str, family: char) -> Option<u16> {
    let mut chars = base.chars();
    if chars.next()? != family {
        return None;
    }
    let next = chars.next();
    match next {
        None => Some(0),
        Some('-' | '0'..='9') => Some(0),
        Some('x') => Some(1),
        Some('y') => Some(2),
        Some('l') => Some(3),
        Some('r') => Some(4),
        Some('b') => Some(5),
        Some('t') => Some(6),
        _ => None,
    }
}

fn position_axis_rank(base: &str) -> Option<u16> {
    if base.starts_with("left-") {
        return Some(3);
    }
    if base.starts_with("right-") {
        return Some(4);
    }
    if base.starts_with("bottom-") {
        return Some(5);
    }
    if base.starts_with("top-") {
        return Some(6);
    }
    None
}

fn is_unocss_token(token: &str) -> bool {
    let base = token.rsplit(':').next().unwrap_or(token);
    base == "flex"
        || base == "grid"
        || base == "block"
        || base == "inline"
        || base == "inline-block"
        || base == "hidden"
        || base.starts_with("flex-")
        || base.starts_with("text-")
        || spacing_axis_rank(base, 'm').is_some()
        || spacing_axis_rank(base, 'p').is_some()
        || position_axis_rank(base).is_some()
        || base.starts_with("gap")
        || base.starts_with("rounded")
        || base.starts_with("border")
        || base.starts_with("bg-")
        || base.starts_with("h-")
        || base.starts_with('h')
        || base.starts_with("w-")
        || base.starts_with('w')
}

fn find_tag_end(source_text: &str, mut cursor: usize) -> Option<usize> {
    let bytes = source_text.as_bytes();
    let mut quote = None;
    while cursor < bytes.len() {
        if let Some(active_quote) = quote {
            if bytes[cursor] == b'\\' {
                cursor = (cursor + 2).min(bytes.len());
                continue;
            }
            if bytes[cursor] == active_quote {
                quote = None;
            }
            cursor += 1;
            continue;
        }
        match bytes[cursor] {
            b'\'' | b'"' | b'`' => quote = Some(bytes[cursor]),
            b'>' => return Some(cursor),
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn skip_attribute_value(source_text: &str, mut cursor: usize, tag_end: usize) -> usize {
    let bytes = source_text.as_bytes();
    while cursor < tag_end && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if cursor >= tag_end {
        return cursor;
    }
    if matches!(bytes[cursor], b'\'' | b'"' | b'`') {
        let quote = bytes[cursor];
        cursor += 1;
        while cursor < tag_end {
            if bytes[cursor] == b'\\' {
                cursor = (cursor + 2).min(tag_end);
                continue;
            }
            if bytes[cursor] == quote {
                return cursor + 1;
            }
            cursor += 1;
        }
        return cursor;
    }
    if bytes[cursor] == b'{' {
        let mut depth = 1usize;
        cursor += 1;
        while cursor < tag_end && depth > 0 {
            match bytes[cursor] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            cursor += 1;
        }
        return cursor;
    }
    while cursor < tag_end && !bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    cursor
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$'
}

fn is_identifier_part(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit() || byte == b'-'
}

fn is_attr_name_part(byte: u8) -> bool {
    is_identifier_part(byte) || byte == b':' || byte == b'.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(
            implemented_unocss_rule_names(),
            [
                "blocklist",
                "enforce-class-compile",
                "order",
                "order-attributify"
            ]
        );
    }

    #[test]
    fn scans_jsx_class_rules() {
        let mut blocklist = SmallVec::new();
        blocklist.push(BlocklistEntry {
            name: CompactString::from("border"),
            reason: CompactString::new(""),
        });
        let options = UnocssOptions {
            blocklist,
            ..UnocssOptions::default()
        };
        let diagnostics = scan_unocss(
            r#"<div className="mx1 m1 border"></div>"#,
            "fixture.tsx",
            &options,
        );
        let names: SmallVec<[&str; 4]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        assert_eq!(
            names.as_slice(),
            ["blocklist", "enforce-class-compile", "order"]
        );
    }

    #[test]
    fn scans_uno_call_and_attributify_order() {
        let diagnostics = scan_unocss(
            r#"const value = clsx("mr-1 ml-1"); const node = <div p4 flex />;"#,
            "fixture.tsx",
            &UnocssOptions::default(),
        );
        let names: SmallVec<[&str; 4]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        assert_eq!(names.as_slice(), ["order", "order-attributify"]);
    }
}
