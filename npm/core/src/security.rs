//! NAPI boundary for the security oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticLoc, implemented_security_rule_names, scan_security,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec; core rule logic uses compact Rust data structures."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_security as core;

    #[napi(object, namespace = "security")]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub text: Option<String>,
        pub method: Option<String>,
        pub package_name: Option<String>,
        pub fn_name: Option<String>,
        pub indices: Option<String>,
        pub side: Option<String>,
        pub value: Option<String>,
        pub argument_type: Option<String>,
    }

    #[napi(object, namespace = "security")]
    #[derive(Clone, Debug)]
    pub struct DiagnosticLoc {
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
    }

    #[napi(object, namespace = "security")]
    #[derive(Clone, Debug)]
    pub struct Diagnostic {
        pub rule_name: String,
        pub message_id: String,
        pub data: DiagnosticData,
        pub loc: DiagnosticLoc,
    }

    #[napi(namespace = "security")]
    pub fn implemented_security_rule_names() -> Vec<String> {
        core::implemented_security_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi(namespace = "security")]
    pub fn scan_security(source_text: String, filename: String) -> Vec<Diagnostic> {
        core::scan_security(&source_text, &filename)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    text: diagnostic.data.text.map(|value| value.as_str().to_owned()),
                    method: diagnostic
                        .data
                        .method
                        .map(|value| value.as_str().to_owned()),
                    package_name: diagnostic
                        .data
                        .package_name
                        .map(|value| value.as_str().to_owned()),
                    fn_name: diagnostic
                        .data
                        .fn_name
                        .map(|value| value.as_str().to_owned()),
                    indices: diagnostic
                        .data
                        .indices
                        .map(|value| value.as_str().to_owned()),
                    side: diagnostic.data.side.map(|value| value.as_str().to_owned()),
                    value: diagnostic.data.value.map(|value| value.as_str().to_owned()),
                    argument_type: diagnostic
                        .data
                        .argument_type
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
