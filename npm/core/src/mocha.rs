//! NAPI boundary for the mocha oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticLoc, MochaScanOptions, implemented_mocha_rule_names, scan_mocha,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_mocha as core;

    #[napi(object, namespace = "mocha")]
    #[derive(Clone, Debug, Default)]
    pub struct MochaScanOptions {
        pub consistent_interface: Option<String>,
        pub max_top_level_suites_limit: Option<u32>,
        pub handle_done_ignore_pending: Option<bool>,
        pub no_hooks_allowed: Option<Vec<String>>,
        pub no_hooks_for_single_case_allowed: Option<Vec<String>>,
        pub no_synchronous_allowed: Option<Vec<String>>,
        pub no_empty_title_message: Option<String>,
        pub valid_suite_title_pattern: Option<String>,
        pub valid_suite_title_message: Option<String>,
        pub valid_test_title_pattern: Option<String>,
        pub valid_test_title_message: Option<String>,
        pub prefer_arrow_allow_named_functions: Option<bool>,
        pub prefer_arrow_allow_unbound_this: Option<bool>,
    }

    #[napi(object, namespace = "mocha")]
    #[derive(Clone, Debug)]
    pub struct DiagnosticLoc {
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
    }

    #[napi(object, namespace = "mocha")]
    #[derive(Clone, Debug)]
    pub struct Diagnostic {
        pub rule_name: String,
        pub message: String,
        pub loc: DiagnosticLoc,
    }

    #[napi(namespace = "mocha")]
    pub fn implemented_mocha_rule_names() -> Vec<String> {
        core::implemented_mocha_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi(namespace = "mocha")]
    pub fn scan_mocha(
        source_text: String,
        filename: String,
        options: Option<MochaScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let default_options = core::MochaOptions::default();
        let core_options = core::MochaOptions {
            consistent_interface: option_compact_string(options.consistent_interface)
                .unwrap_or(default_options.consistent_interface),
            max_top_level_suites_limit: options
                .max_top_level_suites_limit
                .unwrap_or(default_options.max_top_level_suites_limit),
            handle_done_ignore_pending: options
                .handle_done_ignore_pending
                .unwrap_or(default_options.handle_done_ignore_pending),
            no_hooks_allowed: compact_strings4(options.no_hooks_allowed.unwrap_or_default()),
            no_hooks_for_single_case_allowed: compact_strings4(
                options.no_hooks_for_single_case_allowed.unwrap_or_default(),
            ),
            no_synchronous_allowed: options
                .no_synchronous_allowed
                .map(compact_strings3)
                .unwrap_or(default_options.no_synchronous_allowed),
            no_empty_title_message: option_compact_string(options.no_empty_title_message),
            valid_suite_title_pattern: option_compact_string(options.valid_suite_title_pattern),
            valid_suite_title_message: option_compact_string(options.valid_suite_title_message),
            valid_test_title_pattern: option_compact_string(options.valid_test_title_pattern),
            valid_test_title_message: option_compact_string(options.valid_test_title_message),
            prefer_arrow_allow_named_functions: options
                .prefer_arrow_allow_named_functions
                .unwrap_or(default_options.prefer_arrow_allow_named_functions),
            prefer_arrow_allow_unbound_this: options
                .prefer_arrow_allow_unbound_this
                .unwrap_or(default_options.prefer_arrow_allow_unbound_this),
        };

        core::scan_mocha(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message: diagnostic.message.into_string(),
                loc: DiagnosticLoc {
                    start_line: diagnostic.loc.start_line,
                    start_column: diagnostic.loc.start_column,
                    end_line: diagnostic.loc.end_line,
                    end_column: diagnostic.loc.end_column,
                },
            })
            .collect()
    }

    fn option_compact_string(value: Option<String>) -> Option<CompactString> {
        value
            .filter(|value| !value.is_empty())
            .map(CompactString::from)
    }

    fn compact_strings3(values: Vec<String>) -> SmallVec<[CompactString; 3]> {
        values
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(CompactString::from)
            .collect()
    }

    fn compact_strings4(values: Vec<String>) -> SmallVec<[CompactString; 4]> {
        values
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(CompactString::from)
            .collect()
    }
}
