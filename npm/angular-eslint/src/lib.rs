//! NAPI boundary for the angular-eslint oxlint plugin.

pub use napi_abi::{
    AngularEslintScanOptions, Diagnostic, DiagnosticDatum, DiagnosticLoc,
    implemented_angular_eslint_rule_names, scan_angular_eslint,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec; values are converted before returning to JavaScript."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_angular_eslint as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct AngularEslintScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub options: Option<serde_json::Value>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticDatum {
        pub key: String,
        pub value: String,
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
        pub data: Vec<DiagnosticDatum>,
        pub loc: DiagnosticLoc,
    }

    #[napi]
    pub fn implemented_angular_eslint_rule_names() -> Vec<String> {
        core::implemented_angular_eslint_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_angular_eslint(
        source_text: String,
        filename: String,
        options: AngularEslintScanOptions,
    ) -> Vec<Diagnostic> {
        let core_options = core::ScanOptions {
            rule_names: options
                .rule_names
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            options: options.options.unwrap_or(serde_json::Value::Null),
        };
        core::scan_angular_eslint_with_options(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: diagnostic
                    .data
                    .into_iter()
                    .map(|datum| DiagnosticDatum {
                        key: datum.key.into_string(),
                        value: datum.value.into_string(),
                    })
                    .collect(),
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
