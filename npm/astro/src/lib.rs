//! NAPI boundary for the Astro oxlint plugin.

pub use napi_abi::{
    AstroScanOptions, Diagnostic, DiagnosticFix, DiagnosticLoc, implemented_astro_rule_names,
    scan_astro,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_astro as core;
    use oxlint_plugins_carton::{CompactString, SmallVec};

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct AstroScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub frontmatter_only: Option<bool>,
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
        pub start: u32,
        pub end: u32,
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi]
    pub fn implemented_astro_rule_names() -> Vec<String> {
        core::implemented_astro_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_astro(
        source_text: String,
        filename: String,
        options: Option<AstroScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::AstroOptions {
            rule_names: compact_strings4(options.rule_names.unwrap_or_default()),
            frontmatter_only: options.frontmatter_only.unwrap_or(false),
        };
        core::scan_astro(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                start: diagnostic.start,
                end: diagnostic.end,
                loc: DiagnosticLoc {
                    start_line: diagnostic.loc.start_line,
                    start_column: diagnostic.loc.start_column,
                    end_line: diagnostic.loc.end_line,
                    end_column: diagnostic.loc.end_column,
                },
                fix: diagnostic.fix.map(|fix| DiagnosticFix {
                    start: fix.start,
                    end: fix.end,
                    replacement: fix.replacement.as_str().to_owned(),
                }),
            })
            .collect()
    }

    fn compact_strings4(values: Vec<String>) -> SmallVec<[CompactString; 4]> {
        values.into_iter().map(CompactString::from).collect()
    }
}
