//! NAPI boundary for the react-refresh oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticLoc, OnlyExportComponentsOptions, default_hocs,
    is_constant_export_expression_kind, is_react_component_name, scan_only_export_components,
    should_scan_filename,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI and proc macros require String/Vec/Option; values are converted before calling core helpers."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_react_refresh as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct OnlyExportComponentsOptions {
        pub extra_hocs: Option<Vec<String>>,
        pub allow_export_names: Option<Vec<String>>,
        pub allow_constant_export: Option<bool>,
        pub check_js: Option<bool>,
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
        pub message_id: String,
        pub loc: DiagnosticLoc,
    }

    #[napi]
    pub fn is_react_component_name(name: String) -> bool {
        core::is_react_component_name(&name)
    }

    #[napi]
    pub fn should_scan_filename(filename: String, check_js: bool) -> bool {
        core::should_scan_filename(&filename, check_js)
    }

    #[napi]
    pub fn is_constant_export_expression_kind(kind: String) -> bool {
        core::is_constant_export_expression_kind(&kind)
    }

    #[napi]
    pub fn default_hocs() -> Vec<String> {
        core::DEFAULT_HOCS.into_iter().map(str::to_owned).collect()
    }

    #[napi]
    pub fn scan_only_export_components(
        source_text: String,
        filename: String,
        options: Option<OnlyExportComponentsOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::OnlyExportComponentsOptions {
            extra_hocs: compact_strings4(options.extra_hocs.unwrap_or_default()),
            allow_export_names: compact_strings8(options.allow_export_names.unwrap_or_default()),
            allow_constant_export: options.allow_constant_export.unwrap_or(false),
            check_js: options.check_js.unwrap_or(false),
        };

        core::scan_only_export_components(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
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

    fn compact_strings4(values: Vec<String>) -> SmallVec<[CompactString; 4]> {
        values.into_iter().map(CompactString::from).collect()
    }

    fn compact_strings8(values: Vec<String>) -> SmallVec<[CompactString; 8]> {
        values.into_iter().map(CompactString::from).collect()
    }
}
