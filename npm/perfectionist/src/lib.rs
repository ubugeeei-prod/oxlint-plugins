//! NAPI boundary for the perfectionist oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, implemented_perfectionist_rule_names,
    scan_perfectionist, scan_perfectionist_rule,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi::Result;
    use napi_derive::napi;
    use oxlint_plugins_perfectionist as core;
    use serde_json::Value;

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
        pub data: Option<DiagnosticData>,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticData {
        pub left: String,
        pub right: String,
        pub left_group: Option<String>,
        pub right_group: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticFix {
        pub start: u32,
        pub end: u32,
        pub replacement: String,
    }

    #[napi]
    pub fn implemented_perfectionist_rule_names() -> Vec<String> {
        core::implemented_perfectionist_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_perfectionist(source_text: String, filename: String) -> Vec<Diagnostic> {
        core::scan_perfectionist(&source_text, &filename)
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
                data: None,
                fix: None,
            })
            .collect()
    }

    #[napi]
    pub fn scan_perfectionist_rule(
        source_text: String,
        filename: String,
        rule_name: String,
        options: Value,
    ) -> Result<Vec<Diagnostic>> {
        Ok(
            core::scan_perfectionist_rule(&source_text, &filename, &rule_name, &options)
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
                    data: Some(DiagnosticData {
                        left: diagnostic.data.left.into_string(),
                        right: diagnostic.data.right.into_string(),
                        left_group: diagnostic.data.left_group.map(|value| value.into_string()),
                        right_group: diagnostic.data.right_group.map(|value| value.into_string()),
                    }),
                    fix: Some(DiagnosticFix {
                        start: diagnostic.fix.start,
                        end: diagnostic.fix.end,
                        replacement: diagnostic.fix.replacement.into_string(),
                    }),
                })
                .collect(),
        )
    }
}
