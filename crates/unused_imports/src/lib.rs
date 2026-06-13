#![doc = "Rust implementation of eslint-plugin-unused-imports rule logic."]

mod fixers;
mod scanner;
mod types;

#[cfg(test)]
mod tests;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxlint_plugins_carton::{FastHashMap, SmallVec};

use crate::fixers::{is_used_in_jsdoc, unused_message};
use crate::scanner::{collect_import_bindings, report_unused_imports, should_report_unused_symbol};
use crate::types::{ImportBinding, LineIndex, SpanKey};

pub use crate::types::{
    Diagnostic, DiagnosticFix, DiagnosticLoc, UnusedImportsOptions,
};

pub const RULE_NAMES: [&str; 2] = ["no-unused-imports", "no-unused-vars"];

pub fn implemented_unused_imports_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_unused_imports(
    source_text: &str,
    filename: &str,
    options: &UnusedImportsOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::mjs())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let line_index = LineIndex::new(source_text);
    let import_bindings = collect_import_bindings(&parser_return.program);
    let import_by_local_span: FastHashMap<SpanKey, ImportBinding<'_>> = import_bindings
        .iter()
        .map(|binding| (SpanKey::from(binding.local_span), *binding))
        .collect();
    let semantic_return = SemanticBuilder::new().build(&parser_return.program);
    if !semantic_return.errors.is_empty() {
        return SmallVec::new();
    }

    let semantic = semantic_return.semantic;
    let scoping = semantic.scoping();
    let mut diagnostics = SmallVec::<[Diagnostic; 16]>::new();

    if options.has_rule("no-unused-imports") {
        let mut unused_imports = SmallVec::<[ImportBinding<'_>; 16]>::new();
        for symbol_id in scoping.symbol_ids() {
            let flags = scoping.symbol_flags(symbol_id);
            if !flags.is_import() || !scoping.get_resolved_reference_ids(symbol_id).is_empty() {
                continue;
            }
            let span = scoping.symbol_span(symbol_id);
            let Some(binding) = import_by_local_span.get(&SpanKey::from(span)) else {
                continue;
            };
            if is_used_in_jsdoc(binding.name, source_text) {
                continue;
            }
            unused_imports.push(*binding);
        }
        diagnostics.extend(report_unused_imports(
            source_text,
            &line_index,
            &unused_imports,
        ));
    }

    if options.has_rule("no-unused-vars") {
        for symbol_id in scoping.symbol_ids() {
            let flags = scoping.symbol_flags(symbol_id);
            if flags.is_import()
                || !should_report_unused_symbol(flags)
                || !scoping.get_resolved_reference_ids(symbol_id).is_empty()
            {
                continue;
            }
            let span = scoping.symbol_span(symbol_id);
            let name = scoping.symbol_name(symbol_id);
            diagnostics.push(Diagnostic {
                rule_name: "no-unused-vars",
                message: unused_message(name),
                loc: line_index.loc_for_span(source_text, span),
                fix: None,
            });
        }
    }

    diagnostics.sort_by(|a, b| {
        a.loc
            .start_line
            .cmp(&b.loc.start_line)
            .then(a.loc.start_column.cmp(&b.loc.start_column))
            .then(a.rule_name.cmp(b.rule_name))
    });
    diagnostics
}
