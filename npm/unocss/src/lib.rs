//! NAPI boundary for the unocss oxlint plugin.

pub use napi_abi::{
    BlocklistEntry, Diagnostic, DiagnosticFix, DiagnosticLoc, UnocssScanOptions,
    implemented_unocss_rule_names, scan_unocss,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_unocss as core;

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct BlocklistEntry {
        pub name: String,
        pub reason: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct UnocssScanOptions {
        pub uno_functions: Option<Vec<String>>,
        pub uno_variables: Option<Vec<String>>,
        pub blocklist: Option<Vec<BlocklistEntry>>,
        pub class_compile_prefix: Option<String>,
        pub class_compile_enable_fix: Option<bool>,
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
        pub name: Option<String>,
        pub reason: Option<String>,
        pub prefix: Option<String>,
    }

    #[napi]
    pub fn implemented_unocss_rule_names() -> Vec<String> {
        core::implemented_unocss_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_unocss(
        source_text: String,
        filename: String,
        options: Option<UnocssScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let defaults = core::UnocssOptions::default();
        let core_options = core::UnocssOptions {
            uno_functions: options
                .uno_functions
                .filter(|values| !values.is_empty())
                .map_or(defaults.uno_functions.clone(), compact_strings4),
            uno_variables: options
                .uno_variables
                .filter(|values| !values.is_empty())
                .map_or(defaults.uno_variables.clone(), compact_strings4),
            blocklist: options
                .blocklist
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| !entry.name.is_empty())
                .map(|entry| core::BlocklistEntry {
                    name: CompactString::from(entry.name),
                    reason: CompactString::from(entry.reason.unwrap_or_default()),
                })
                .collect(),
            class_compile_prefix: CompactString::from(
                options
                    .class_compile_prefix
                    .filter(|prefix| !prefix.is_empty())
                    .unwrap_or_else(|| defaults.class_compile_prefix.as_str().to_owned()),
            ),
            class_compile_enable_fix: options.class_compile_enable_fix.unwrap_or(true),
        };

        core::scan_unocss(&source_text, &filename, &core_options)
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
                    replacement: fix.replacement.as_str().to_owned(),
                }),
                name: diagnostic.name.map(|name| name.as_str().to_owned()),
                reason: diagnostic.reason.map(|reason| reason.as_str().to_owned()),
                prefix: diagnostic.prefix.map(|prefix| prefix.as_str().to_owned()),
            })
            .collect()
    }

    fn compact_strings4(values: Vec<String>) -> SmallVec<[CompactString; 4]> {
        values.into_iter().map(CompactString::from).collect()
    }
}
