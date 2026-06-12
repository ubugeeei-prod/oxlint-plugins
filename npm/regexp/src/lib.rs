//! NAPI boundary for the regexp oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticLoc, implemented_regexp_rule_names, scan_regexp,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec; core rule logic uses compact Rust data structures."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_regexp as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub message: Option<String>,
        pub flag: Option<String>,
        pub flags: Option<String>,
        pub sorted_flags: Option<String>,
        pub expr: Option<String>,
        pub char_text: Option<String>,
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
        pub message_id: String,
        pub data: DiagnosticData,
        pub loc: DiagnosticLoc,
    }

    #[napi]
    pub fn implemented_regexp_rule_names() -> Vec<String> {
        core::implemented_regexp_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_regexp(source_text: String, filename: String) -> Vec<Diagnostic> {
        core::scan_regexp(&source_text, &filename)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    message: diagnostic
                        .data
                        .message
                        .map(|value| value.as_str().to_owned()),
                    flag: diagnostic.data.flag.map(|value| value.as_str().to_owned()),
                    flags: diagnostic.data.flags.map(|value| value.as_str().to_owned()),
                    sorted_flags: diagnostic
                        .data
                        .sorted_flags
                        .map(|value| value.as_str().to_owned()),
                    expr: diagnostic.data.expr.map(|value| value.as_str().to_owned()),
                    char_text: diagnostic
                        .data
                        .char_text
                        .map(|value| value.as_str().to_owned()),
                },
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
