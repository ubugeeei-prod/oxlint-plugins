#![doc = "Rust implementation of @angular-eslint/eslint-plugin rule logic."]

mod class_suffix;
mod inline_declarations;
mod input_rename;
mod prefix;
mod scanner;
mod selector;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::LineIndex;

pub use crate::types::{Diagnostic, DiagnosticDatum, DiagnosticLoc};

#[derive(Clone, Debug, Default)]
pub struct ScanOptions {
    pub rule_names: SmallVec<[CompactString; 4]>,
    pub options: Value,
}

impl ScanOptions {
    pub(crate) fn is_enabled(&self, rule_name: &str) -> bool {
        self.rule_names.is_empty() || self.rule_names.iter().any(|name| name == rule_name)
    }
}

pub const RULE_NAMES: [&str; 48] = [
    "component-class-suffix",
    "component-max-inline-declarations",
    "component-selector",
    "computed-must-return",
    "consistent-component-styles",
    "contextual-decorator",
    "contextual-lifecycle",
    "directive-class-suffix",
    "directive-selector",
    "no-async-lifecycle-method",
    "no-attribute-decorator",
    "no-developer-preview",
    "no-duplicates-in-metadata-arrays",
    "no-empty-lifecycle-method",
    "no-experimental",
    "no-forward-ref",
    "no-implicit-take-until-destroyed",
    "no-input-prefix",
    "no-input-rename",
    "no-inputs-metadata-property",
    "no-lifecycle-call",
    "no-output-native",
    "no-output-on-prefix",
    "no-output-rename",
    "no-outputs-metadata-property",
    "no-pipe-impure",
    "no-queries-metadata-property",
    "no-uncalled-signals",
    "pipe-prefix",
    "prefer-host-metadata-property",
    "prefer-inject",
    "prefer-on-push-component-change-detection",
    "prefer-output-emitter-ref",
    "prefer-output-readonly",
    "prefer-signal-model",
    "prefer-signals",
    "prefer-standalone",
    "relative-url-prefix",
    "require-lifecycle-on-prototype",
    "require-localize-metadata",
    "runtime-localize",
    "sort-keys-in-type-decorator",
    "sort-lifecycle-methods",
    "use-component-selector",
    "use-component-view-encapsulation",
    "use-injectable-provided-in",
    "use-lifecycle-interface",
    "use-pipe-transform-interface",
];

pub fn implemented_angular_eslint_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_angular_eslint(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 64]> {
    scan_angular_eslint_with_options(source_text, filename, &ScanOptions::default())
}

pub fn scan_angular_eslint_with_options(
    source_text: &str,
    filename: &str,
    options: &ScanOptions,
) -> SmallVec<[Diagnostic; 64]> {
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
        options,
        diagnostics: SmallVec::new(),
    };
    scanner.scan(&parser_return.program);
    scanner.diagnostics
}
