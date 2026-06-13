#![doc = "Rust implementation of @e18e/eslint-plugin rule logic."]
#![allow(
    clippy::collapsible_if,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::needless_borrow,
    clippy::question_mark,
    reason = "The e18e port builds many small autofix strings from source slices; keeping that string assembly local is clearer than adding broad formatting abstractions in the first native port."
)]

mod helpers;
mod rules;
mod scanner;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::ban_dependency_diagnostic;
use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 25] = [
    "prefer-array-at",
    "prefer-array-fill",
    "prefer-array-from-map",
    "prefer-includes",
    "prefer-array-to-reversed",
    "prefer-array-to-sorted",
    "prefer-array-to-spliced",
    "prefer-exponentiation-operator",
    "prefer-nullish-coalescing",
    "prefer-object-has-own",
    "prefer-spread-syntax",
    "prefer-url-canparse",
    "no-indexof-equality",
    "prefer-timer-args",
    "prefer-date-now",
    "prefer-regex-test",
    "prefer-array-some",
    "prefer-static-regex",
    "prefer-inline-equality",
    "prefer-string-fromcharcode",
    "prefer-includes-over-regex-test",
    "no-delete-property",
    "no-spread-in-reduce",
    "prefer-static-collator",
    "ban-dependencies",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub array: Option<CompactString>,
    pub index: Option<CompactString>,
    pub item: Option<CompactString>,
    pub length: Option<CompactString>,
    pub value: Option<CompactString>,
    pub iterable: Option<CompactString>,
    pub mapper: Option<CompactString>,
    pub regex: Option<CompactString>,
    pub string: Option<CompactString>,
    pub original: Option<CompactString>,
    pub name: Option<CompactString>,
    pub replacement: Option<CompactString>,
    pub url: Option<CompactString>,
    pub description: Option<CompactString>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BanDependency {
    pub module_name: CompactString,
    pub message_id: CompactString,
    pub replacement: Option<CompactString>,
    pub url: Option<CompactString>,
    pub description: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E18eOptions {
    pub rule_names: SmallVec<[CompactString; 25]>,
    pub banned_dependencies: SmallVec<[BanDependency; 16]>,
}

impl Default for E18eOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            banned_dependencies: SmallVec::new(),
        }
    }
}

impl E18eOptions {
    pub(crate) fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
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

pub fn implemented_e18e_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_e18e(
    source_text: &str,
    filename: &str,
    options: &E18eOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let line_index = LineIndex::new(source_text);
    if filename.ends_with("package.json") {
        return scan_package_json_dependencies(source_text, options, &line_index);
    }

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
        line_index,
        options,
        diagnostics: SmallVec::new(),
        function_depth: 0,
    };
    scanner.scan_program(&parser_return.program);
    scanner.diagnostics
}

fn scan_package_json_dependencies(
    source_text: &str,
    options: &E18eOptions,
    line_index: &LineIndex,
) -> SmallVec<[Diagnostic; 32]> {
    let mut diagnostics = SmallVec::new();
    if !options.has_rule("ban-dependencies") {
        return diagnostics;
    }

    for dependency in &options.banned_dependencies {
        let needle = format!("\"{}\"", dependency.module_name);
        let mut search_start = 0usize;
        while let Some(offset) = source_text[search_start..].find(&needle) {
            let start = search_start + offset;
            let span = Span::new(start as u32, (start + needle.len()) as u32);
            diagnostics.push(ban_dependency_diagnostic(
                dependency,
                span,
                source_text,
                line_index,
            ));
            search_start = start + needle.len();
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(rule: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
        scan_e18e(
            source,
            "sample.ts",
            &E18eOptions {
                rule_names: [CompactString::from(rule)].into_iter().collect(),
                banned_dependencies: SmallVec::new(),
            },
        )
    }

    #[test]
    fn modern_array_rules_report_and_fix() {
        let diagnostics = scan(
            "prefer-array-from-map",
            "const out = [...items].map(item => item.id);",
        );
        assert_eq!(diagnostics[0].message_id, "preferArrayFrom");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "Array.from(items, item => item.id)"
        );

        let diagnostics = scan("prefer-array-at", "const last = items[items.length - 1];");
        assert_eq!(diagnostics[0].message_id, "preferAt");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "items.at(-1)"
        );
    }

    #[test]
    fn performance_rules_report_and_fix() {
        let diagnostics = scan(
            "prefer-exponentiation-operator",
            "const x = Math.pow(a, 2);",
        );
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "(a) ** (2)"
        );

        let diagnostics = scan(
            "prefer-string-fromcharcode",
            "String.fromCodePoint(65, 66);",
        );
        assert_eq!(diagnostics[0].loc.start_column, 7);
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "fromCharCode"
        );
    }

    #[test]
    fn boolean_rules_report_and_fix() {
        let diagnostics = scan("prefer-includes", "if (items.indexOf(id) !== -1) ok();");
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "items.includes(id)"
        );

        let diagnostics = scan(
            "prefer-array-some",
            "if (items.filter(fn).length > 0) ok();",
        );
        assert_eq!(
            diagnostics[0]
                .fix
                .as_ref()
                .expect("diagnostic should include a fix")
                .replacement,
            "items.some(fn)"
        );
    }

    #[test]
    fn ban_dependencies_uses_options() {
        let diagnostics = scan_e18e(
            "import merge from 'lodash.merge';",
            "sample.js",
            &E18eOptions {
                rule_names: [CompactString::from("ban-dependencies")]
                    .into_iter()
                    .collect(),
                banned_dependencies: [BanDependency {
                    module_name: CompactString::from("lodash.merge"),
                    message_id: CompactString::from("documentedReplacement"),
                    replacement: Some(CompactString::from("deepmerge-ts")),
                    url: Some(CompactString::from("https://example.com")),
                    description: None,
                }]
                .into_iter()
                .collect(),
            },
        );
        assert_eq!(diagnostics[0].message_id, "documentedReplacement");
        assert_eq!(diagnostics[0].data.name.as_deref(), Some("lodash.merge"));
    }
}
