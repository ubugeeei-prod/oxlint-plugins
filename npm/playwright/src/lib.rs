//! NAPI boundary for the playwright oxlint plugin.

pub use napi_abi::{Diagnostic, DiagnosticLoc, implemented_playwright_rule_names, scan_playwright};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec; values are converted before returning to JavaScript."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_playwright as core;

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
        pub message_id: String,
        pub loc: DiagnosticLoc,
    }

    #[napi]
    pub fn implemented_playwright_rule_names() -> Vec<String> {
        core::implemented_playwright_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_playwright(source_text: String, filename: String) -> Vec<Diagnostic> {
        core::scan_playwright(&source_text, &filename)
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
            })
            .collect()
    }
}
