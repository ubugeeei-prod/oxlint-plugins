//! Diagnostic types and line indexing for the playwright port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Restriction {
    pub value: CompactString,
    pub message: Option<CompactString>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaywrightOptions {
    pub allowed_title_prefixes: SmallVec<[CompactString; 4]>,
    pub assert_function_names: SmallVec<[CompactString; 4]>,
    pub assert_function_patterns: SmallVec<[CompactString; 4]>,
    pub lowercase_title_ignored_methods: SmallVec<[CompactString; 4]>,
    pub ignore_top_level_describe: bool,
    pub restricted_locators: SmallVec<[Restriction; 8]>,
    pub restricted_matchers: SmallVec<[Restriction; 8]>,
    pub restricted_roles: SmallVec<[Restriction; 8]>,
    pub expect_aliases: SmallVec<[CompactString; 4]>,
    pub test_aliases: SmallVec<[CompactString; 4]>,
    pub valid_title: ValidTitleOptions,
    pub valid_test_tags: ValidTestTagsOptions,
    pub max_expects: u32,
    pub max_nested_describe: u32,
    pub max_top_level_describes: Option<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidTitleOptions {
    pub disallowed_words: SmallVec<[CompactString; 8]>,
    pub ignore_spaces: bool,
    pub ignore_type_of_describe_name: bool,
    pub ignore_type_of_step_name: bool,
    pub ignore_type_of_test_name: bool,
    pub must_match: TitlePatternOptions,
    pub must_not_match: TitlePatternOptions,
}

impl Default for ValidTitleOptions {
    fn default() -> Self {
        Self {
            disallowed_words: SmallVec::new(),
            ignore_spaces: false,
            ignore_type_of_describe_name: false,
            ignore_type_of_step_name: true,
            ignore_type_of_test_name: false,
            must_match: TitlePatternOptions::default(),
            must_not_match: TitlePatternOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TitlePatternOptions {
    pub describe: Option<TitlePattern>,
    pub step: Option<TitlePattern>,
    pub test: Option<TitlePattern>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TitlePattern {
    pub source: CompactString,
    pub message: Option<CompactString>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValidTestTagsOptions {
    pub allowed_tags: SmallVec<[TagPattern; 8]>,
    pub disallowed_tags: SmallVec<[TagPattern; 8]>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TagPattern {
    pub source: CompactString,
    pub flags: CompactString,
    pub is_regex: bool,
}

impl Default for PlaywrightOptions {
    fn default() -> Self {
        Self {
            allowed_title_prefixes: SmallVec::new(),
            assert_function_names: SmallVec::new(),
            assert_function_patterns: SmallVec::new(),
            lowercase_title_ignored_methods: SmallVec::new(),
            ignore_top_level_describe: false,
            restricted_locators: SmallVec::new(),
            restricted_matchers: SmallVec::new(),
            restricted_roles: SmallVec::new(),
            expect_aliases: SmallVec::new(),
            test_aliases: SmallVec::new(),
            valid_title: ValidTitleOptions::default(),
            valid_test_tags: ValidTestTagsOptions::default(),
            max_expects: 5,
            max_nested_describe: 5,
            max_top_level_describes: None,
        }
    }
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
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    /// UTF-8 byte offset into the source text.
    pub start: u32,
    /// UTF-8 byte offset into the source text.
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub message: CompactString,
    pub amount: Option<CompactString>,
    pub count: Option<CompactString>,
    pub depth: Option<CompactString>,
    pub max: Option<CompactString>,
    pub method: Option<CompactString>,
    pub restriction: Option<CompactString>,
    pub role: Option<CompactString>,
    pub function_name: Option<CompactString>,
    pub pattern: Option<CompactString>,
    pub tag: Option<CompactString>,
    pub word: Option<CompactString>,
    pub s: Option<CompactString>,
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
