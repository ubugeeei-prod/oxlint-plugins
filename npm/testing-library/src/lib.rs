//! NAPI boundary for the testing-library oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticLoc, TestingLibraryScanOptions, implemented_testing_library_rule_names,
    scan_testing_library,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_testing_library as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct TestingLibraryScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub test_id_pattern: Option<String>,
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
    pub struct Diagnostic {
        pub rule_name: String,
        pub message: String,
        pub loc: DiagnosticLoc,
    }

    #[napi]
    pub fn implemented_testing_library_rule_names() -> Vec<String> {
        core::implemented_testing_library_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_testing_library(
        source_text: String,
        filename: String,
        options: Option<TestingLibraryScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let default_options = core::TestingLibraryOptions::default();
        let core_options = core::TestingLibraryOptions {
            rule_names: compact_rule_names(options.rule_names),
            test_id_pattern: options
                .test_id_pattern
                .filter(|value| !value.is_empty())
                .map(CompactString::from)
                .unwrap_or(default_options.test_id_pattern),
        };

        core::scan_testing_library(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message: diagnostic.message.into_string(),
                loc: DiagnosticLoc {
                    start_line: diagnostic.loc.start_line,
                    start_column: diagnostic.loc.start_column,
                    end_line: diagnostic.loc.end_line,
                    end_column: diagnostic.loc.end_column,
                },
            })
            .collect()
    }

    fn compact_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 29]> {
        values.map_or_else(
            || {
                core::implemented_testing_library_rule_names()
                    .iter()
                    .map(|name| CompactString::from(*name))
                    .collect()
            },
            |values| {
                values
                    .into_iter()
                    .filter(|value| {
                        core::implemented_testing_library_rule_names().contains(&value.as_str())
                    })
                    .map(CompactString::from)
                    .collect()
            },
        )
    }
}
