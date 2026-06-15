//! NAPI boundary for the eslint-plugin-postgresql oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticDatum, DiagnosticFix, DiagnosticLoc, PostgresqlScanOptions,
    implemented_postgresql_rule_names, scan_postgresql,
};

#[allow(
    clippy::disallowed_types,
    clippy::disallowed_macros,
    reason = "NAPI public ABI requires String/Vec/serde_json::Value; values are converted before and after the core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::CompactString;
    use oxlint_plugins_postgresql as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PostgresqlScanOptions {
        /// The rule names enabled for this scan (the JS adapter passes one).
        pub rule_names: Option<Vec<String>>,
        /// The enabled rule's raw ESLint options array, verbatim.
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
        pub data: Vec<DiagnosticDatum>,
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi]
    pub fn implemented_postgresql_rule_names() -> Vec<String> {
        core::implemented_postgresql_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_postgresql(source_text: String, options: PostgresqlScanOptions) -> Vec<Diagnostic> {
        let scan_options = core::ScanOptions {
            rule_names: options
                .rule_names
                .unwrap_or_default()
                .into_iter()
                .map(CompactString::from)
                .collect(),
            options: options.options.unwrap_or(serde_json::Value::Null),
        };

        core::scan_postgresql(&source_text, &scan_options)
            .into_iter()
            .map(|d| Diagnostic {
                rule_name: d.rule_name.to_owned(),
                message_id: d.message_id.to_owned(),
                data: d
                    .data
                    .into_iter()
                    .map(|datum| DiagnosticDatum {
                        key: datum.key.into_string(),
                        value: datum.value.into_string(),
                    })
                    .collect(),
                loc: DiagnosticLoc {
                    start_line: d.loc.start_line,
                    start_column: d.loc.start_column,
                    end_line: d.loc.end_line,
                    end_column: d.loc.end_column,
                },
                fix: d.fix.map(|fix| DiagnosticFix {
                    start: fix.start,
                    end: fix.end,
                    replacement: fix.replacement.into_string(),
                }),
            })
            .collect()
    }
}
