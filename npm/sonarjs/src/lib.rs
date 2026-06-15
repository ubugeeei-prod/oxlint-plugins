//! NAPI boundary for the sonarjs oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, SonarjsScanOptions,
    implemented_sonarjs_rule_names, scan_sonarjs,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_sonarjs as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct SonarjsScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub max_lines_threshold: Option<u32>,
        pub max_lines_per_function_threshold: Option<u32>,
        pub max_switch_cases_threshold: Option<u32>,
        pub max_union_size_threshold: Option<u32>,
        pub nested_control_flow_threshold: Option<u32>,
        pub no_duplicate_string_threshold: Option<u32>,
        pub cyclomatic_complexity_threshold: Option<u32>,
        pub no_nested_functions_threshold: Option<u32>,
        pub cognitive_complexity_threshold: Option<u32>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub value: Option<String>,
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
        pub data: DiagnosticData,
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi]
    pub fn implemented_sonarjs_rule_names() -> Vec<String> {
        core::implemented_sonarjs_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_sonarjs(
        source_text: String,
        filename: String,
        options: Option<SonarjsScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let default_options = core::SonarjsOptions::default();
        let core_options = core::SonarjsOptions {
            rule_names: compact_rule_names(options.rule_names),
            max_lines_threshold: options
                .max_lines_threshold
                .unwrap_or(default_options.max_lines_threshold),
            max_lines_per_function_threshold: options
                .max_lines_per_function_threshold
                .unwrap_or(default_options.max_lines_per_function_threshold),
            max_switch_cases_threshold: options
                .max_switch_cases_threshold
                .unwrap_or(default_options.max_switch_cases_threshold),
            max_union_size_threshold: options
                .max_union_size_threshold
                .unwrap_or(default_options.max_union_size_threshold),
            nested_control_flow_threshold: options
                .nested_control_flow_threshold
                .unwrap_or(default_options.nested_control_flow_threshold),
            no_duplicate_string_threshold: options
                .no_duplicate_string_threshold
                .unwrap_or(default_options.no_duplicate_string_threshold),
            cyclomatic_complexity_threshold: options
                .cyclomatic_complexity_threshold
                .unwrap_or(default_options.cyclomatic_complexity_threshold),
            no_nested_functions_threshold: options
                .no_nested_functions_threshold
                .unwrap_or(default_options.no_nested_functions_threshold),
            cognitive_complexity_threshold: options
                .cognitive_complexity_threshold
                .unwrap_or(default_options.cognitive_complexity_threshold),
        };

        core::scan_sonarjs(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    value: diagnostic.data.value.map(|value| value.into_string()),
                },
                loc: DiagnosticLoc {
                    start_line: diagnostic.loc.start_line,
                    start_column: diagnostic.loc.start_column,
                    end_line: diagnostic.loc.end_line,
                    end_column: diagnostic.loc.end_column,
                },
                fix: diagnostic.fix.map(|fix| DiagnosticFix {
                    start: fix.start,
                    end: fix.end,
                    replacement: fix.replacement.into_string(),
                }),
            })
            .collect()
    }

    fn compact_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 32]> {
        values.map_or_else(
            || {
                core::implemented_sonarjs_rule_names()
                    .iter()
                    .map(|name| CompactString::from(*name))
                    .collect()
            },
            |values| {
                values
                    .into_iter()
                    .filter(|value| {
                        core::implemented_sonarjs_rule_names().contains(&value.as_str())
                    })
                    .map(CompactString::from)
                    .collect()
            },
        )
    }
}
