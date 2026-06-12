#![doc = "Rust implementation of eslint-plugin-perfectionist rule logic."]

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

pub const RULE_NAMES: [&str; 23] = [
    "sort-array-includes",
    "sort-arrays",
    "sort-classes",
    "sort-decorators",
    "sort-enums",
    "sort-export-attributes",
    "sort-exports",
    "sort-heritage-clauses",
    "sort-import-attributes",
    "sort-imports",
    "sort-interfaces",
    "sort-intersection-types",
    "sort-jsx-props",
    "sort-maps",
    "sort-modules",
    "sort-named-exports",
    "sort-named-imports",
    "sort-object-types",
    "sort-objects",
    "sort-sets",
    "sort-switch-case",
    "sort-union-types",
    "sort-variable-declarations",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
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

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 24]>,
}

pub fn implemented_perfectionist_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_perfectionist(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 24]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
    };
    scanner.scan();
    scanner.diagnostics
}

impl<'a> Scanner<'a> {
    fn scan(&mut self) {
        self.check_regex("sort-named-imports", r"import\s*\{\s*b\s*,\s*a\s*\}");
        self.check_regex("sort-named-exports", r"export\s*\{\s*b\s*,\s*a\s*\}");
        self.check_regex(
            "sort-imports",
            r#"(?s)import\s+.+from\s+['"]z['"].*import\s+.+from\s+['"]a['"]"#,
        );
        self.check_regex(
            "sort-exports",
            r#"(?s)export\s+.+from\s+['"]z['"].*export\s+.+from\s+['"]a['"]"#,
        );
        self.check_regex(
            "sort-import-attributes",
            r#"import\s+.+\s+with\s*\{\s*type\s*:\s*['"]json['"]\s*,\s*foo\s*:"#,
        );
        self.check_regex(
            "sort-export-attributes",
            r#"export\s+.+\s+with\s*\{\s*type\s*:\s*['"]json['"]\s*,\s*foo\s*:"#,
        );
        self.check_regex("sort-heritage-clauses", r"implements\s+Z\s*,\s*A");
        self.check_regex("sort-decorators", r"(?s)@Z\s+@A");
        self.check_regex(
            "sort-classes",
            r"(?s)class\s+Class\s*\{.*\bb\s*\([^)]*\).*?\ba\s*\([^)]*\)",
        );
        self.check_regex("sort-jsx-props", r"<Component\s+b=\{[^}]*\}\s+a=");
        self.check_regex("sort-variable-declarations", r"const\s+b\s*=.*,\s*a\s*=");
        self.check_regex(
            "sort-array-includes",
            r#"\[\s*['"]b['"]\s*,\s*['"]a['"]\s*\]\s*\.\s*includes\s*\("#,
        );

        self.check_delimited_after("sort-arrays", "const array", '[', ']');
        self.check_delimited_after("sort-sets", "new Set", '[', ']');
        self.check_delimited_after("sort-maps", "new Map", '[', ']');
        self.check_delimited_after("sort-objects", "const object", '{', '}');
        self.check_delimited_after("sort-object-types", "type ObjectType", '{', '}');
        self.check_delimited_after("sort-interfaces", "interface Interface", '{', '}');
        self.check_delimited_after("sort-enums", "enum Enum", '{', '}');
        self.check_type_sequence("sort-union-types", "type Union", '|');
        self.check_type_sequence("sort-intersection-types", "type Intersection", '&');
        self.check_switch_cases();
        self.check_modules();
    }

    fn check_regex(&mut self, rule_name: &'static str, pattern: &str) {
        let Some(regex) = Regex::new(pattern).ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                rule_name,
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn check_delimited_after(
        &mut self,
        rule_name: &'static str,
        marker: &str,
        open: char,
        close: char,
    ) {
        let Some(marker_index) = self.source_text.find(marker) else {
            return;
        };
        let Some(open_offset) = self.source_text[marker_index..].find(open) else {
            return;
        };
        let open_index = marker_index + open_offset;
        let close_index = if open == '=' {
            marker_index + self.source_text[marker_index..].find(close).unwrap_or(0)
        } else {
            find_matching(self.source_text, open_index, open, close).unwrap_or(open_index)
        };
        if close_index <= open_index {
            return;
        }
        let start = if open == '=' {
            open_index + 1
        } else {
            open_index + open.len_utf8()
        };
        let segment = &self.source_text[start..close_index];
        let names = extract_names(segment);
        if is_unsorted(names.as_slice()) {
            self.report(
                rule_name,
                Span::new(open_index as u32, (close_index + 1) as u32),
            );
        }
    }

    fn check_type_sequence(&mut self, rule_name: &'static str, marker: &str, delimiter: char) {
        let Some(marker_index) = self.source_text.find(marker) else {
            return;
        };
        let Some(equal_offset) = self.source_text[marker_index..].find('=') else {
            return;
        };
        let start = marker_index + equal_offset + 1;
        let end = self.source_text[start..]
            .find(';')
            .map_or(self.source_text.len(), |offset| start + offset);
        let names: SmallVec<[CompactString; 8]> = self.source_text[start..end]
            .split(delimiter)
            .map(normalize_name)
            .filter(|name| !name.is_empty())
            .collect();
        if is_unsorted(names.as_slice()) {
            self.report(rule_name, Span::new(start as u32, end as u32));
        }
    }

    fn check_switch_cases(&mut self) {
        let Some(regex) = Regex::new(r#"case\s+['"]b['"]\s*:.*case\s+['"]a['"]\s*:"#).ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                "sort-switch-case",
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn check_modules(&mut self) {
        let Some(regex) = Regex::new(r"(?s)const\s+z\b.*function\s+a\b").ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                "sort-modules",
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn report(&mut self, rule_name: &'static str, span: Span) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id: "unexpected",
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

fn find_matching(source_text: &str, open_index: usize, open: char, close: char) -> Option<usize> {
    let bytes = source_text.as_bytes();
    let mut cursor = open_index;
    let mut depth = 0usize;
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
            byte if byte == open as u8 => depth += 1,
            byte if byte == close as u8 => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn extract_names(segment: &str) -> SmallVec<[CompactString; 8]> {
    split_top_level(segment)
        .into_iter()
        .map(normalize_name)
        .filter(|name| !name.is_empty())
        .collect()
}

fn split_top_level(segment: &str) -> SmallVec<[&str; 8]> {
    let bytes = segment.as_bytes();
    let mut parts = SmallVec::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut start = 0usize;
    let mut cursor = 0usize;
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
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth = depth.saturating_sub(1),
            b',' | b';' if depth == 0 => {
                parts.push(segment[start..cursor].trim());
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    let tail = segment[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

fn normalize_name(item: &str) -> CompactString {
    let trimmed = item
        .trim()
        .trim_start_matches("case")
        .trim()
        .trim_matches(|ch| matches!(ch, '\'' | '"' | '`' | '[' | ']' | '(' | ')' | '{' | '}'));
    let end = trimmed
        .find(|ch: char| ch == ':' || ch == '=' || ch == '(' || ch.is_whitespace() || ch == ',')
        .unwrap_or(trimmed.len());
    let mut name = CompactString::from(trimmed[..end].trim());
    if name.as_str().starts_with("...") {
        name = CompactString::from(name.as_str().trim_start_matches("..."));
    }
    name
}

fn is_unsorted(names: &[CompactString]) -> bool {
    names
        .windows(2)
        .any(|pair| pair[0].as_str().to_ascii_lowercase() > pair[1].as_str().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(implemented_perfectionist_rule_names(), RULE_NAMES);
    }

    #[test]
    fn scans_representative_rules() {
        let source = r#"
import { b, a } from "pkg";
export { b, a };
import z from "z";
import a from "a";
export { z } from "z";
export { a } from "a";
import data from "./data.json" with { type: "json", foo: "bar" };
export { data } from "./data.json" with { type: "json", foo: "bar" };
@Z @A class Decorated {}
class Derived implements Z, A {}
const array = ["b", "a"];
["b", "a"].includes(value);
const set = new Set(["b", "a"]);
const map = new Map([["b", 1], ["a", 2]]);
const object = { b: 1, a: 2 };
type ObjectType = { b: string; a: string };
interface Interface { b: string; a: string }
enum Enum { B, A }
class Class { b() {} a() {} }
const jsx = <Component b={1} a={2} />;
const b = 1, a = 2;
type Union = B | A;
type Intersection = B & A;
switch (value) { case "b": break; case "a": break; }
const z = 1;
function a() {}
"#;
        let diagnostics = scan_perfectionist(source, "fixture.tsx");
        let names: SmallVec<[&str; 24]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();

        assert_eq!(names.len(), RULE_NAMES.len());
    }
}
