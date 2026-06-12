#![doc = "Rust implementation of eslint-plugin-unused-imports rule logic."]

use std::fmt::Write as _;

use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier, Program, Statement};
use oxc_parser::Parser;
use oxc_semantic::{SemanticBuilder, SymbolFlags};
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};
use regex::Regex;

pub const RULE_NAMES: [&str; 2] = ["no-unused-imports", "no-unused-vars"];

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
    pub message: CompactString,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnusedImportsOptions {
    pub rule_names: SmallVec<[CompactString; 2]>,
}

impl Default for UnusedImportsOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|name| CompactString::from(*name))
                .collect(),
        }
    }
}

impl UnusedImportsOptions {
    fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SpanKey {
    start: u32,
    end: u32,
}

impl From<Span> for SpanKey {
    fn from(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct DeclarationKey {
    start: u32,
    end: u32,
}

impl From<Span> for DeclarationKey {
    fn from(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImportSpecifierKind {
    Named,
    Default,
    Namespace,
}

#[derive(Clone, Copy, Debug)]
struct ImportBinding<'a> {
    name: &'a str,
    local_span: Span,
    specifier_span: Span,
    declaration_span: Span,
    specifier_index: usize,
    specifier_count: usize,
    named_specifier_count: usize,
    kind: ImportSpecifierKind,
}

#[derive(Default)]
struct DeclarationUsage {
    specifier_count: usize,
    unused_count: usize,
    first_unused_index: usize,
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

    fn line_for_offset(&self, offset: u32) -> u32 {
        let offset = offset as usize;
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        line_index.saturating_sub(1) as u32 + 1
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

fn should_report_unused_symbol(flags: SymbolFlags) -> bool {
    flags.intersects(
        SymbolFlags::Variable
            | SymbolFlags::Function
            | SymbolFlags::Class
            | SymbolFlags::TypeAlias
            | SymbolFlags::Interface
            | SymbolFlags::Enum
            | SymbolFlags::TypeParameter
            | SymbolFlags::CatchVariable,
    )
}

fn collect_import_bindings<'a>(program: &'a Program<'a>) -> SmallVec<[ImportBinding<'a>; 16]> {
    let mut bindings = SmallVec::new();
    for statement in &program.body {
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };
        bindings.extend(bindings_for_declaration(declaration));
    }
    bindings
}

fn bindings_for_declaration<'a>(
    declaration: &'a ImportDeclaration<'a>,
) -> SmallVec<[ImportBinding<'a>; 4]> {
    let Some(specifiers) = &declaration.specifiers else {
        return SmallVec::new();
    };
    let specifier_count = specifiers.len();
    let named_specifier_count = specifiers
        .iter()
        .filter(|specifier| matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(_)))
        .count();
    let mut bindings = SmallVec::new();
    for (specifier_index, specifier) in specifiers.iter().enumerate() {
        let (name, local_span, specifier_span, kind) = match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(specifier) => (
                specifier.local.name.as_str(),
                specifier.local.span,
                specifier.span,
                ImportSpecifierKind::Named,
            ),
            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => (
                specifier.local.name.as_str(),
                specifier.local.span,
                specifier.span,
                ImportSpecifierKind::Default,
            ),
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => (
                specifier.local.name.as_str(),
                specifier.local.span,
                specifier.span,
                ImportSpecifierKind::Namespace,
            ),
        };
        bindings.push(ImportBinding {
            name,
            local_span,
            specifier_span,
            declaration_span: declaration.span,
            specifier_index,
            specifier_count,
            named_specifier_count,
            kind,
        });
    }
    bindings
}

fn report_unused_imports(
    source_text: &str,
    line_index: &LineIndex,
    unused_imports: &[ImportBinding<'_>],
) -> SmallVec<[Diagnostic; 16]> {
    let mut declarations: FastHashMap<DeclarationKey, DeclarationUsage> = FastHashMap::default();
    for binding in unused_imports {
        let key = DeclarationKey::from(binding.declaration_span);
        declarations
            .entry(key)
            .and_modify(|usage| {
                usage.unused_count += 1;
                usage.first_unused_index = usage.first_unused_index.min(binding.specifier_index);
            })
            .or_insert(DeclarationUsage {
                specifier_count: binding.specifier_count,
                unused_count: 1,
                first_unused_index: binding.specifier_index,
            });
    }

    let mut diagnostics = SmallVec::new();
    for binding in unused_imports {
        let usage = declarations
            .get(&DeclarationKey::from(binding.declaration_span))
            .expect("declaration usage is inserted before reporting");
        let remove_declaration = usage.unused_count == usage.specifier_count;
        if remove_declaration && binding.specifier_index != usage.first_unused_index {
            continue;
        }
        diagnostics.push(Diagnostic {
            rule_name: "no-unused-imports",
            message: unused_message(binding.name),
            loc: line_index.loc_for_span(source_text, binding.local_span),
            fix: Some(if remove_declaration {
                fix_remove_declaration(source_text, line_index, binding.declaration_span)
            } else {
                fix_remove_specifier(source_text, *binding)
            }),
        });
    }
    diagnostics
}

fn unused_message(name: &str) -> CompactString {
    let mut message = CompactString::new("'");
    let _ = write!(&mut message, "{name}' is defined but never used.");
    message
}

fn fix_remove_declaration(
    source_text: &str,
    line_index: &LineIndex,
    declaration_span: Span,
) -> DiagnosticFix {
    let mut end = declaration_span.end;
    let mut replacement = CompactString::new("");
    let next_start = next_non_whitespace(source_text, end as usize);
    if next_start < source_text.len() {
        let import_line = line_index.line_for_offset(declaration_span.start);
        let next_line = line_index.line_for_offset(next_start as u32);
        let count = next_line.saturating_sub(import_line + 1);
        replacement = CompactString::from("\n".repeat(count as usize));
        end = next_start as u32;
    }
    DiagnosticFix {
        start: declaration_span.start,
        end,
        replacement,
    }
}

fn fix_remove_specifier(source_text: &str, binding: ImportBinding<'_>) -> DiagnosticFix {
    let start;
    let end;
    if binding.specifier_index + 1 < binding.specifier_count {
        start = token_before_end(source_text, binding.specifier_span.start as usize);
        end = comma_after_end(source_text, binding.specifier_span.end as usize);
    } else if binding.kind == ImportSpecifierKind::Named && binding.named_specifier_count == 1 {
        start = comma_before_named_group(source_text, binding.specifier_span.start as usize);
        end = brace_after_end(source_text, binding.specifier_span.end as usize);
    } else {
        start = comma_before_start(source_text, binding.specifier_span.start as usize);
        end = binding.specifier_span.end as usize;
    }
    DiagnosticFix {
        start: start as u32,
        end: end as u32,
        replacement: CompactString::new(""),
    }
}

fn next_non_whitespace(source_text: &str, from: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = from;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn token_before_end(source_text: &str, before: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = before.min(bytes.len());
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    index
}

fn comma_after_end(source_text: &str, after: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = after.min(bytes.len());
    while index < bytes.len() {
        match bytes[index] {
            b',' => return index + 1,
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => return after.min(bytes.len()),
        }
    }
    after.min(bytes.len())
}

fn comma_before_start(source_text: &str, before: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = before.min(bytes.len());
    while index > 0 {
        match bytes[index - 1] {
            b',' => return index - 1,
            byte if byte.is_ascii_whitespace() => index -= 1,
            _ => return before.min(bytes.len()),
        }
    }
    before.min(bytes.len())
}

fn comma_before_named_group(source_text: &str, before: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = before.min(bytes.len());
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    if index > 0 && bytes[index - 1] == b'{' {
        index -= 1;
    }
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    if index > 0 && bytes[index - 1] == b',' {
        return index - 1;
    }
    comma_before_start(source_text, before)
}

fn brace_after_end(source_text: &str, after: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = after.min(bytes.len());
    while index < bytes.len() {
        match bytes[index] {
            b'}' => return index + 1,
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => return after.min(bytes.len()),
        }
    }
    after.min(bytes.len())
}

fn is_used_in_jsdoc(identifier_name: &str, source_text: &str) -> bool {
    let escaped_name = regex::escape(identifier_name);
    let mut pattern_text = CompactString::new("");
    let _ = write!(
        &mut pattern_text,
        r"(?:(?:@(?:link|linkcode|linkplain|see)\s+{escaped_name}\b)|(?:\{{@(?:link|linkcode|linkplain)\s+{escaped_name}\b\}})|(?:[@\{{](?:type|typedef|param|returns?|template|augments|extends|implements)\s+[^}}]*\b{escaped_name}\b))"
    );
    let Ok(pattern) = Regex::new(pattern_text.as_str()) else {
        return false;
    };
    let bytes = source_text.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'*' {
            let comment_start = index + 2;
            let mut end = comment_start;
            while end + 1 < bytes.len() && !(bytes[end] == b'*' && bytes[end + 1] == b'/') {
                end += 1;
            }
            if pattern.is_match(&source_text[comment_start..end]) {
                return true;
            }
            index = end.saturating_add(2);
        } else {
            index += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use oxlint_plugins_carton::CompactString;

    use super::{UnusedImportsOptions, scan_unused_imports};

    fn apply_first_fix(source: &str, filename: &str) -> CompactString {
        let diagnostics = scan_unused_imports(source, filename, &UnusedImportsOptions::default());
        let fix = diagnostics
            .iter()
            .find_map(|diagnostic| diagnostic.fix.as_ref())
            .expect("expected fix");
        let mut output = CompactString::new("");
        output.push_str(&source[..fix.start as usize]);
        output.push_str(&fix.replacement);
        output.push_str(&source[fix.end as usize..]);
        output
    }

    #[test]
    fn removes_unused_named_import() {
        let source =
            "import x from \"package\";\nimport { a, b } from \"./utils\";\nconst c = b(x);\n";
        let diagnostics = scan_unused_imports(source, "file.js", &UnusedImportsOptions::default());
        assert_eq!(diagnostics[0].rule_name, "no-unused-imports");
        assert_eq!(diagnostics[0].message, "'a' is defined but never used.");
        assert_eq!(
            apply_first_fix(source, "file.js"),
            "import x from \"package\";\nimport { b } from \"./utils\";\nconst c = b(x);\n"
        );
    }

    #[test]
    fn removes_whole_import_and_preserves_comment_spacing() {
        let source = "import y from \"package\";\nimport { a } from \"./utils\";\n\n/** c is the number 4 */\nconst c = y;\n";
        assert_eq!(
            apply_first_fix(source, "file.js"),
            "import y from \"package\";\n\n/** c is the number 4 */\nconst c = y;\n"
        );
    }

    #[test]
    fn removes_last_named_import_with_default_left() {
        let source = "import fallback, { a } from \"./utils\";\nconsole.log(fallback);\n";
        assert_eq!(
            apply_first_fix(source, "file.js"),
            "import fallback from \"./utils\";\nconsole.log(fallback);\n"
        );
    }

    #[test]
    fn honors_jsdoc_identifier_references() {
        let source = "import { UsedInJSDoc } from \"./used\";\n/** Reference to {@link UsedInJSDoc} */\nconst example = \"test\";\n";
        let diagnostics = scan_unused_imports(source, "file.js", &UnusedImportsOptions::default());
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule_name != "no-unused-imports")
        );
    }

    #[test]
    fn type_usage_counts_as_used() {
        let source =
            "import type { SomeType } from \"./types\";\nconst value: SomeType = {} as SomeType;\n";
        let diagnostics = scan_unused_imports(source, "file.ts", &UnusedImportsOptions::default());
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message != "'SomeType' is defined but never used.")
        );
    }

    #[test]
    fn shadowed_inner_name_does_not_mark_import_used() {
        let source = "import { a } from \"./utils\";\nfunction fn(a) { return a; }\nfn(1);\n";
        let diagnostics = scan_unused_imports(source, "file.js", &UnusedImportsOptions::default());
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_name == "no-unused-imports"
                    && diagnostic.message == "'a' is defined but never used.")
        );
    }
}
