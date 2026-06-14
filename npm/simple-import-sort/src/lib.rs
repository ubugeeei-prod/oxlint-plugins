//! NAPI boundary for the simple-import-sort oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticFix, DiagnosticLoc, SimpleImportSortScanOptions,
    implemented_simple_import_sort_rule_names, scan_simple_import_sort,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_simple_import_sort as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct SimpleImportSortScanOptions {
        pub import_groups: Option<Vec<Vec<String>>>,
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
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi]
    pub fn implemented_simple_import_sort_rule_names() -> Vec<String> {
        core::implemented_simple_import_sort_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_simple_import_sort(
        source_text: String,
        filename: String,
        options: Option<SimpleImportSortScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::SimpleImportSortOptions {
            // None = use default 5 groups; Some([]) = explicit empty → single rest group
            import_groups: options.import_groups.map(compact_groups),
        };

        core::scan_simple_import_sort(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
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
            })
            .collect()
    }

    fn compact_groups(values: Vec<Vec<String>>) -> SmallVec<[SmallVec<[CompactString; 4]>; 8]> {
        values
            .into_iter()
            .filter_map(|group| {
                let group: SmallVec<[CompactString; 4]> = group
                    .into_iter()
                    .filter(|value| !value.is_empty())
                    .map(CompactString::from)
                    .collect();
                (!group.is_empty()).then_some(group)
            })
            .collect()
    }
}
