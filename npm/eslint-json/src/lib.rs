//! NAPI boundary for the @eslint/json oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, EslintJsonScanOptions,
    implemented_eslint_json_rule_names, scan_eslint_json,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_eslint_json as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct EslintJsonScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub normalization_form: Option<String>,
        pub sort_direction: Option<String>,
        pub sort_case_sensitive: Option<bool>,
        pub sort_natural: Option<bool>,
        pub sort_min_keys: Option<u32>,
        pub sort_allow_line_separated_groups: Option<bool>,
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
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub key: Option<String>,
        pub value: Option<String>,
        pub surrogate: Option<String>,
        pub r#type: Option<String>,
        pub this_name: Option<String>,
        pub prev_name: Option<String>,
        pub direction: Option<String>,
        pub sensitivity: Option<String>,
        pub sort_name: Option<String>,
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
    pub fn implemented_eslint_json_rule_names() -> Vec<String> {
        core::implemented_eslint_json_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_eslint_json(
        source_text: String,
        options: Option<EslintJsonScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = normalize_options(options.unwrap_or_default());
        core::scan_eslint_json(&source_text, &options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    key: diagnostic.data.key.map(CompactString::into_string),
                    value: diagnostic.data.value.map(CompactString::into_string),
                    surrogate: diagnostic.data.surrogate.map(CompactString::into_string),
                    r#type: diagnostic.data.type_name.map(CompactString::into_string),
                    this_name: diagnostic.data.this_name.map(CompactString::into_string),
                    prev_name: diagnostic.data.prev_name.map(CompactString::into_string),
                    direction: diagnostic.data.direction.map(CompactString::into_string),
                    sensitivity: diagnostic.data.sensitivity.map(CompactString::into_string),
                    sort_name: diagnostic.data.sort_name.map(CompactString::into_string),
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

    fn normalize_options(raw: EslintJsonScanOptions) -> core::ScanOptions {
        let mut options = core::ScanOptions {
            rule_names: normalize_rule_names(raw.rule_names),
            normalization_form: normalize_form(raw.normalization_form),
            ..core::ScanOptions::default()
        };

        options.sort.direction = normalize_sort_direction(raw.sort_direction);
        options.sort.case_sensitive = raw.sort_case_sensitive.unwrap_or(true);
        options.sort.natural = raw.sort_natural.unwrap_or(false);
        options.sort.min_keys = raw
            .sort_min_keys
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value >= 2)
            .unwrap_or(2);
        options.sort.allow_line_separated_groups =
            raw.sort_allow_line_separated_groups.unwrap_or(false);

        options
    }

    fn normalize_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 6]> {
        values
            .unwrap_or_default()
            .into_iter()
            .filter(|value| core::RULE_NAMES.contains(&value.as_str()))
            .map(CompactString::from)
            .collect()
    }

    fn normalize_form(value: Option<String>) -> core::NormalizationForm {
        match value.as_deref() {
            Some("NFD") => core::NormalizationForm::Nfd,
            Some("NFKC") => core::NormalizationForm::Nfkc,
            Some("NFKD") => core::NormalizationForm::Nfkd,
            _ => core::NormalizationForm::Nfc,
        }
    }

    fn normalize_sort_direction(value: Option<String>) -> core::SortDirection {
        match value.as_deref() {
            Some("desc") | Some("descending") => core::SortDirection::Descending,
            _ => core::SortDirection::Ascending,
        }
    }
}
