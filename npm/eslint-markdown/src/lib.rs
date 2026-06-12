//! NAPI boundary for the @eslint/markdown oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, EslintMarkdownScanOptions,
    implemented_eslint_markdown_rule_names, scan_eslint_markdown,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_eslint_markdown as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct EslintMarkdownScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub required_code_languages: Option<Vec<String>>,
        pub fenced_code_meta_mode: Option<String>,
        pub frontmatter_title: Option<String>,
        pub check_closed_headings: Option<bool>,
        pub check_strikethrough: Option<bool>,
        pub allowed_html: Option<Vec<String>>,
        pub allowed_html_ignore_case: Option<bool>,
        pub allow_labels: Option<Vec<String>>,
        pub allow_definitions: Option<Vec<String>>,
        pub allow_footnote_definitions: Option<Vec<String>>,
        pub check_footnote_definitions: Option<bool>,
        pub check_duplicate_headings_siblings_only: Option<bool>,
        pub ignore_fragment_case: Option<bool>,
        pub allow_fragment_pattern: Option<String>,
        pub check_missing_table_cells: Option<bool>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub lang: Option<String>,
        pub name: Option<String>,
        pub identifier: Option<String>,
        pub label: Option<String>,
        pub first_line: Option<u32>,
        pub first_label: Option<String>,
        pub from_level: Option<u32>,
        pub to_level: Option<u32>,
        pub position: Option<String>,
        pub text: Option<String>,
        pub link_type: Option<String>,
        pub prefix: Option<String>,
        pub fragment: Option<String>,
        pub expected_cells: Option<u32>,
        pub actual_cells: Option<u32>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticLoc {
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticFix {
        pub start: u32,
        pub end: u32,
        pub replacement: String,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct Diagnostic {
        pub rule_name: String,
        pub message_id: String,
        pub data: DiagnosticData,
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi]
    pub fn implemented_eslint_markdown_rule_names() -> Vec<String> {
        core::implemented_eslint_markdown_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_eslint_markdown(
        source_text: String,
        options: Option<EslintMarkdownScanOptions>,
    ) -> Vec<Diagnostic> {
        let core_options = to_core_options(options.unwrap_or_default());
        core::scan_eslint_markdown(&source_text, &core_options)
            .into_iter()
            .map(to_diagnostic)
            .collect()
    }

    fn to_core_options(options: EslintMarkdownScanOptions) -> core::ScanOptions {
        let default_rule_names = options.rule_names.is_none();
        let mut core_options = core::ScanOptions {
            rule_names: compact_known_rule_names(options.rule_names),
            required_code_languages: compact_list(options.required_code_languages),
            fenced_code_meta_mode: match options.fenced_code_meta_mode.as_deref() {
                Some("never") => core::MetaMode::Never,
                _ => core::MetaMode::Always,
            },
            frontmatter_title: options.frontmatter_title.map(CompactString::from),
            check_closed_headings: options.check_closed_headings.unwrap_or(false),
            check_strikethrough: options.check_strikethrough.unwrap_or(false),
            allowed_html: compact_list(options.allowed_html),
            allowed_html_ignore_case: options.allowed_html_ignore_case.unwrap_or(false),
            allow_labels: compact_list(options.allow_labels),
            allow_definitions: defaulted_list(options.allow_definitions, &["//"]),
            allow_footnote_definitions: compact_list(options.allow_footnote_definitions),
            check_footnote_definitions: options.check_footnote_definitions.unwrap_or(true),
            check_duplicate_headings_siblings_only: options
                .check_duplicate_headings_siblings_only
                .unwrap_or(false),
            ignore_fragment_case: options.ignore_fragment_case.unwrap_or(true),
            allow_fragment_pattern: options.allow_fragment_pattern.map(CompactString::from),
            check_missing_table_cells: options.check_missing_table_cells.unwrap_or(false),
        };

        if core_options.rule_names.is_empty() && default_rule_names {
            core_options.rule_names = core::implemented_eslint_markdown_rule_names()
                .iter()
                .map(|name| CompactString::from(*name))
                .collect();
        }

        core_options
    }

    fn to_diagnostic(diagnostic: core::Diagnostic) -> Diagnostic {
        Diagnostic {
            rule_name: diagnostic.rule_name.to_owned(),
            message_id: diagnostic.message_id.to_owned(),
            data: DiagnosticData {
                lang: diagnostic.data.lang.map(CompactString::into_string),
                name: diagnostic.data.name.map(CompactString::into_string),
                identifier: diagnostic.data.identifier.map(CompactString::into_string),
                label: diagnostic.data.label.map(CompactString::into_string),
                first_line: diagnostic.data.first_line,
                first_label: diagnostic.data.first_label.map(CompactString::into_string),
                from_level: diagnostic.data.from_level,
                to_level: diagnostic.data.to_level,
                position: diagnostic.data.position.map(CompactString::into_string),
                text: diagnostic.data.text.map(CompactString::into_string),
                link_type: diagnostic.data.link_type.map(CompactString::into_string),
                prefix: diagnostic.data.prefix.map(CompactString::into_string),
                fragment: diagnostic.data.fragment.map(CompactString::into_string),
                expected_cells: diagnostic.data.expected_cells,
                actual_cells: diagnostic.data.actual_cells,
            },
            loc: DiagnosticLoc {
                start_line: diagnostic.loc.start_line,
                start_column: diagnostic.loc.start_column,
                end_line: diagnostic.loc.end_line,
                end_column: diagnostic.loc.end_column,
            },
            fix: diagnostic.fix.map(|fix| DiagnosticFix {
                start: fix.start,
                end: fix.end,
                replacement: fix.replacement.into_string(),
            }),
        }
    }

    fn compact_known_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 21]> {
        values.map_or_else(SmallVec::new, |values| {
            values
                .into_iter()
                .filter(|value| {
                    core::implemented_eslint_markdown_rule_names().contains(&value.as_str())
                })
                .map(CompactString::from)
                .collect()
        })
    }

    fn compact_list(values: Option<Vec<String>>) -> SmallVec<[CompactString; 8]> {
        values.map_or_else(SmallVec::new, |values| {
            values
                .into_iter()
                .filter(|value| !value.is_empty())
                .map(CompactString::from)
                .collect()
        })
    }

    fn defaulted_list(
        values: Option<Vec<String>>,
        default_values: &[&str],
    ) -> SmallVec<[CompactString; 8]> {
        values.map_or_else(
            || {
                default_values
                    .iter()
                    .map(|value| CompactString::from(*value))
                    .collect()
            },
            |values| {
                values
                    .into_iter()
                    .filter(|value| !value.is_empty())
                    .map(CompactString::from)
                    .collect()
            },
        )
    }
}
