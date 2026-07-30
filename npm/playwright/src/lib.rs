//! NAPI boundary for the playwright oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticLoc, PlaywrightRestriction, PlaywrightScanOptions,
    implemented_playwright_rule_names, scan_playwright,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec; values are converted before returning to JavaScript."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_playwright as core;

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct PlaywrightRestriction {
        pub value: String,
        pub message: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PlaywrightScanOptions {
        pub restricted_locators: Option<Vec<PlaywrightRestriction>>,
        pub restricted_matchers: Option<Vec<PlaywrightRestriction>>,
        pub restricted_roles: Option<Vec<PlaywrightRestriction>>,
        pub expect_aliases: Option<Vec<String>>,
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

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticData {
        pub message: String,
        pub method: Option<String>,
        pub restriction: Option<String>,
        pub role: Option<String>,
    }

    #[napi]
    pub fn implemented_playwright_rule_names() -> Vec<String> {
        core::implemented_playwright_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_playwright(
        source_text: String,
        filename: String,
        options: Option<PlaywrightScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::PlaywrightOptions {
            restricted_locators: compact_restrictions(options.restricted_locators),
            restricted_matchers: compact_restrictions(options.restricted_matchers),
            restricted_roles: compact_restrictions(options.restricted_roles),
            expect_aliases: options
                .expect_aliases
                .unwrap_or_default()
                .into_iter()
                .map(CompactString::from)
                .collect(),
        };
        core::scan_playwright_with_options(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    message: diagnostic.data.message.into_string(),
                    method: diagnostic.data.method.map(CompactString::into_string),
                    restriction: diagnostic.data.restriction.map(CompactString::into_string),
                    role: diagnostic.data.role.map(CompactString::into_string),
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

    fn compact_restrictions(
        restrictions: Option<Vec<PlaywrightRestriction>>,
    ) -> SmallVec<[core::Restriction; 8]> {
        restrictions
            .unwrap_or_default()
            .into_iter()
            .map(|restriction| core::Restriction {
                value: CompactString::from(restriction.value),
                message: restriction.message.map(CompactString::from),
            })
            .collect()
    }
}
