#![doc = "Rust implementation of eslint-plugin-react-hooks rule logic."]

mod expressions;
mod helpers;
mod scanner;
mod statements;

use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 1] = ["rules-of-hooks"];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub hook: Option<CompactString>,
    pub function_name: Option<CompactString>,
}

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
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Debug)]
pub(crate) struct FunctionFrame {
    pub(crate) name: Option<CompactString>,
    pub(crate) valid_scope: bool,
    pub(crate) inside_valid_scope: bool,
    pub(crate) async_function: bool,
    pub(crate) in_class: bool,
    pub(crate) conditional_depth: u32,
    pub(crate) loop_depth: u32,
    pub(crate) try_depth: u32,
    pub(crate) possible_early_return: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct HookCall {
    pub(crate) name: CompactString,
    pub(crate) span: Span,
    pub(crate) is_use: bool,
}

pub(crate) struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    pub(crate) fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    pub(crate) fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
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

pub fn implemented_react_hooks_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn is_react_component_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
}

pub fn is_hook_name(name: &str) -> bool {
    if name == "use" {
        return true;
    }

    let Some(rest) = name.strip_prefix("use") else {
        return false;
    };
    let Some(next) = rest.chars().next() else {
        return false;
    };
    next.is_ascii_uppercase() || next.is_ascii_digit()
}

pub fn scan_react_hooks(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = source_type_for_filename(filename);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        frames: SmallVec::new(),
        class_depth: 0,
    };
    scanner.scan_statement_list(&parser_return.program.body);
    scanner.diagnostics
}

fn source_type_for_filename(filename: &str) -> SourceType {
    if filename.ends_with(".tsx") {
        SourceType::tsx()
    } else if filename.ends_with(".jsx") {
        SourceType::jsx()
    } else {
        SourceType::from_path(Path::new(filename))
            .unwrap_or_else(|_| SourceType::mjs())
            .with_module(true)
    }
}

#[cfg(test)]
mod tests;
