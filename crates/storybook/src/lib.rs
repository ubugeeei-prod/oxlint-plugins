#![doc = "Rust implementation of eslint-plugin-storybook rule logic."]

mod helpers;
mod scanner;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectExpression};
use oxc_parser::Parser;
use oxc_semantic::{SemanticBuilder, SymbolId};
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

use crate::helpers::component_name_from_filename;
use crate::scanner::Scanner;

pub const RULE_NAMES: [&str; 16] = [
    "await-interactions",
    "context-in-play-function",
    "csf-component",
    "default-exports",
    "hierarchy-separator",
    "meta-inline-properties",
    "meta-satisfies-type",
    "no-redundant-story-name",
    "no-renderer-packages",
    "no-stories-of",
    "no-title-property-in-meta",
    "no-uninstalled-addons",
    "prefer-pascal-case",
    "story-exports",
    "use-storybook-expect",
    "use-storybook-testing-library",
];

pub(crate) const FUNCTIONS_TO_AWAIT: [&str; 7] = [
    "waitFor",
    "waitForElementToBeRemoved",
    "wait",
    "waitForElement",
    "waitForDomChange",
    "userEvent",
    "play",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub method: Option<CompactString>,
    pub meta_title: Option<CompactString>,
    pub property: Option<CompactString>,
    pub renderer_package: Option<CompactString>,
    pub suggestions: Option<CompactString>,
    pub library: Option<CompactString>,
    pub addon_name: Option<CompactString>,
    pub package_json_path: Option<CompactString>,
    pub name: Option<CompactString>,
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
    pub fixes: SmallVec<[DiagnosticFix; 2]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorybookOptions {
    pub rule_names: SmallVec<[CompactString; 16]>,
    pub installed_addons: SmallVec<[CompactString; 16]>,
    pub ignored_addons: SmallVec<[CompactString; 8]>,
    pub package_json_path: CompactString,
}

impl Default for StorybookOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|name| CompactString::from(*name))
                .collect(),
            installed_addons: SmallVec::new(),
            ignored_addons: SmallVec::new(),
            package_json_path: CompactString::from("package.json"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParentKind {
    AwaitArgument,
    ReturnArgument,
    ArrowBody,
    CallCallee,
    MemberObject,
    Other,
}

#[derive(Clone)]
pub(crate) struct VariableInfo<'a> {
    pub(crate) init: &'a Expression<'a>,
    pub(crate) type_annotation_span: Option<Span>,
}

#[derive(Clone)]
pub(crate) struct FunctionFrame {
    pub(crate) span: Span,
    pub(crate) is_async: bool,
    pub(crate) context_param: Option<CompactString>,
}

pub(crate) struct MetaResolution<'a> {
    pub(crate) object: &'a ObjectExpression<'a>,
    pub(crate) satisfies: bool,
    pub(crate) type_annotation_span: Option<Span>,
    pub(crate) as_expression_span: Option<Span>,
}

pub(crate) struct NamedExport {
    pub(crate) name: CompactString,
    pub(crate) span: Span,
    /// Symbol of the exported binding, when resolvable. Used by `prefer-pascal-case`
    /// to rename every reference to the export (not just its declaration).
    pub(crate) symbol_id: Option<SymbolId>,
}

#[derive(Default)]
pub(crate) struct StoryFilters {
    pub(crate) include: SmallVec<[Descriptor; 2]>,
    pub(crate) exclude: SmallVec<[Descriptor; 2]>,
    pub(crate) has_filter: bool,
}

pub(crate) enum Descriptor {
    Names(SmallVec<[CompactString; 4]>),
    Regex(CompactString),
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

pub fn implemented_storybook_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_storybook(
    source_text: &str,
    filename: &str,
    options: &StorybookOptions,
) -> SmallVec<[Diagnostic; 16]> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx().with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    // Semantic analysis resolves identifier references, which `prefer-pascal-case`
    // uses to rename every use of a renamed story export. The other rules do not
    // read it. Benign semantic errors (e.g. redeclarations) do not block scanning.
    let semantic = SemanticBuilder::new()
        .build(&parser_return.program)
        .semantic;
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();

    let mut scanner = Scanner {
        source_text,
        options,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        scoping,
        nodes,
        variables: FastHashMap::default(),
        function_stack: SmallVec::new(),
        first_non_import_span: None,
        component_name: component_name_from_filename(filename),
        imported_component_name: None,
        has_default_export: false,
        has_csf4_meta: false,
        has_stories_of_import: false,
        has_storybook_expect_import: false,
        user_event_is_non_storybook: false,
        named_exports: SmallVec::new(),
        story_filters: StoryFilters::default(),
        has_meta: false,
        expect_invocations: SmallVec::new(),
    };

    scanner.prepass_program(&parser_return.program.body);
    scanner.scan_statement_list(&parser_return.program.body);
    scanner.finish_program();
    scanner.diagnostics
}

#[cfg(test)]
mod tests;
