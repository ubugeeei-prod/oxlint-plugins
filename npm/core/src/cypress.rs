//! NAPI boundary for the cypress oxlint plugin.

pub use napi_abi::{
    CypressScanOptions, Diagnostic, DiagnosticFix, DiagnosticLoc, implemented_cypress_rule_names,
    scan_cypress,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_cypress as core;

    #[napi(object, namespace = "cypress")]
    #[derive(Clone, Debug, Default)]
    pub struct CypressScanOptions {
        pub unsafe_to_chain_methods: Option<Vec<String>>,
    }

    #[napi(object, namespace = "cypress")]
    #[derive(Clone, Debug)]
    pub struct DiagnosticLoc {
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
    }

    #[napi(object, namespace = "cypress")]
    #[derive(Clone, Debug)]
    pub struct DiagnosticFix {
        pub start: u32,
        pub end: u32,
        pub replacement: String,
    }

    #[napi(object, namespace = "cypress")]
    #[derive(Clone, Debug)]
    pub struct Diagnostic {
        pub rule_name: String,
        pub message_id: String,
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi(namespace = "cypress")]
    pub fn implemented_cypress_rule_names() -> Vec<String> {
        core::implemented_cypress_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi(namespace = "cypress")]
    pub fn scan_cypress(
        source_text: String,
        filename: String,
        options: Option<CypressScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::CypressOptions {
            unsafe_to_chain_methods: compact_strings8(
                options.unsafe_to_chain_methods.unwrap_or_default(),
            ),
        };

        core::scan_cypress(&source_text, &filename, &core_options)
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
                    replacement: fix.replacement.to_owned(),
                }),
            })
            .collect()
    }

    fn compact_strings8(values: Vec<String>) -> SmallVec<[CompactString; 8]> {
        values.into_iter().map(CompactString::from).collect()
    }
}
