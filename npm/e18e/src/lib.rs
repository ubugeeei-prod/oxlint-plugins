//! NAPI boundary for the e18e oxlint plugin.

pub use napi_abi::{
    BanDependency, Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, E18eScanOptions,
    implemented_e18e_rule_names, scan_e18e,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_e18e as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct BanDependency {
        pub module_name: String,
        pub message_id: Option<String>,
        pub replacement: Option<String>,
        pub url: Option<String>,
        pub description: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct E18eScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub banned_dependencies: Option<Vec<BanDependency>>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub array: Option<String>,
        pub index: Option<String>,
        pub item: Option<String>,
        pub length: Option<String>,
        pub value: Option<String>,
        pub iterable: Option<String>,
        pub mapper: Option<String>,
        pub regex: Option<String>,
        pub string: Option<String>,
        pub original: Option<String>,
        pub name: Option<String>,
        pub replacement: Option<String>,
        pub url: Option<String>,
        pub description: Option<String>,
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
    pub fn implemented_e18e_rule_names() -> Vec<String> {
        core::implemented_e18e_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_e18e(
        source_text: String,
        filename: String,
        options: Option<E18eScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::E18eOptions {
            rule_names: compact_rule_names(options.rule_names),
            banned_dependencies: compact_banned_dependencies(
                options.banned_dependencies.unwrap_or_default(),
            ),
        };

        core::scan_e18e(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    array: diagnostic.data.array.map(|value| value.into_string()),
                    index: diagnostic.data.index.map(|value| value.into_string()),
                    item: diagnostic.data.item.map(|value| value.into_string()),
                    length: diagnostic.data.length.map(|value| value.into_string()),
                    value: diagnostic.data.value.map(|value| value.into_string()),
                    iterable: diagnostic.data.iterable.map(|value| value.into_string()),
                    mapper: diagnostic.data.mapper.map(|value| value.into_string()),
                    regex: diagnostic.data.regex.map(|value| value.into_string()),
                    string: diagnostic.data.string.map(|value| value.into_string()),
                    original: diagnostic.data.original.map(|value| value.into_string()),
                    name: diagnostic.data.name.map(|value| value.into_string()),
                    replacement: diagnostic.data.replacement.map(|value| value.into_string()),
                    url: diagnostic.data.url.map(|value| value.into_string()),
                    description: diagnostic.data.description.map(|value| value.into_string()),
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

    fn compact_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 25]> {
        values.map_or_else(
            || {
                core::implemented_e18e_rule_names()
                    .iter()
                    .map(|name| CompactString::from(*name))
                    .collect()
            },
            |values| {
                values
                    .into_iter()
                    .filter(|value| core::implemented_e18e_rule_names().contains(&value.as_str()))
                    .map(CompactString::from)
                    .collect()
            },
        )
    }

    fn compact_banned_dependencies(
        values: Vec<BanDependency>,
    ) -> SmallVec<[core::BanDependency; 16]> {
        values
            .into_iter()
            .filter(|value| !value.module_name.is_empty())
            .map(|value| core::BanDependency {
                module_name: CompactString::from(value.module_name),
                message_id: CompactString::from(
                    value
                        .message_id
                        .unwrap_or_else(|| "removalReplacement".to_owned()),
                ),
                replacement: value.replacement.map(CompactString::from),
                url: value.url.map(CompactString::from),
                description: value.description.map(CompactString::from),
            })
            .collect()
    }
}
