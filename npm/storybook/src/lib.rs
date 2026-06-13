//! NAPI boundary for the Storybook oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, StorybookScanOptions,
    implemented_storybook_rule_names, scan_storybook,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_storybook as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct StorybookScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub installed_addons: Option<Vec<String>>,
        pub ignored_addons: Option<Vec<String>>,
        pub package_json_path: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub method: Option<String>,
        pub meta_title: Option<String>,
        pub property: Option<String>,
        pub renderer_package: Option<String>,
        pub suggestions: Option<String>,
        pub library: Option<String>,
        pub addon_name: Option<String>,
        pub package_json_path: Option<String>,
        pub name: Option<String>,
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
        pub fixes: Option<Vec<DiagnosticFix>>,
    }

    #[napi]
    pub fn implemented_storybook_rule_names() -> Vec<String> {
        core::implemented_storybook_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_storybook(
        source_text: String,
        filename: String,
        options: Option<StorybookScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let default_options = core::StorybookOptions::default();
        let core_options = core::StorybookOptions {
            rule_names: compact_rule_names(options.rule_names),
            installed_addons: compact_strings16(options.installed_addons.unwrap_or_default()),
            ignored_addons: compact_strings8(options.ignored_addons.unwrap_or_default()),
            package_json_path: options
                .package_json_path
                .filter(|value| !value.is_empty())
                .map(CompactString::from)
                .unwrap_or(default_options.package_json_path),
        };

        core::scan_storybook(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    method: diagnostic.data.method.map(|value| value.into_string()),
                    meta_title: diagnostic.data.meta_title.map(|value| value.into_string()),
                    property: diagnostic.data.property.map(|value| value.into_string()),
                    renderer_package: diagnostic
                        .data
                        .renderer_package
                        .map(|value| value.into_string()),
                    suggestions: diagnostic.data.suggestions.map(|value| value.into_string()),
                    library: diagnostic.data.library.map(|value| value.into_string()),
                    addon_name: diagnostic.data.addon_name.map(|value| value.into_string()),
                    package_json_path: diagnostic
                        .data
                        .package_json_path
                        .map(|value| value.into_string()),
                    name: diagnostic.data.name.map(|value| value.into_string()),
                },
                loc: DiagnosticLoc {
                    start_line: diagnostic.loc.start_line,
                    start_column: diagnostic.loc.start_column,
                    end_line: diagnostic.loc.end_line,
                    end_column: diagnostic.loc.end_column,
                },
                fixes: if diagnostic.fixes.is_empty() {
                    None
                } else {
                    Some(
                        diagnostic
                            .fixes
                            .into_iter()
                            .map(|fix| DiagnosticFix {
                                start: fix.start,
                                end: fix.end,
                                replacement: fix.replacement.into_string(),
                            })
                            .collect(),
                    )
                },
            })
            .collect()
    }

    fn compact_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 16]> {
        values.map_or_else(
            || {
                core::implemented_storybook_rule_names()
                    .iter()
                    .map(|name| CompactString::from(*name))
                    .collect()
            },
            |values| {
                values
                    .into_iter()
                    .filter(|value| {
                        core::implemented_storybook_rule_names().contains(&value.as_str())
                    })
                    .map(CompactString::from)
                    .collect()
            },
        )
    }

    fn compact_strings8(values: Vec<String>) -> SmallVec<[CompactString; 8]> {
        values
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(CompactString::from)
            .collect()
    }

    fn compact_strings16(values: Vec<String>) -> SmallVec<[CompactString; 16]> {
        values
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(CompactString::from)
            .collect()
    }
}
