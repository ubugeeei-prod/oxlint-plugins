//! NAPI boundary for the functional oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticLoc, FunctionalScanOptions, implemented_functional_rule_names,
    scan_functional,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec/Option; values are converted before calling core rule logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_functional as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct FunctionalScanOptions {
        pub rule_names: Option<Vec<String>>,
        pub allow_rest_parameter: Option<bool>,
        pub allow_arguments_keyword: Option<bool>,
        pub allow_let_in_for_loop_init: Option<bool>,
        pub allow_in_functions: Option<bool>,
        pub allow_throw_to_reject_promises: Option<bool>,
        pub allow_try_catch: Option<bool>,
        pub allow_try_finally: Option<bool>,
        pub readonly_type_mode: Option<String>,
        pub ignore_if_readonly_wrapped: Option<bool>,
        pub ignore_identifier_pattern: Option<Vec<String>>,
        pub ignore_code_pattern: Option<Vec<String>>,
        /// "off" | "atLeastOne" | "exactlyOne" (default: "atLeastOne")
        pub enforce_parameter_count: Option<String>,
        /// default: true
        pub enforce_count_ignore_iife: Option<bool>,
        /// default: true
        pub enforce_count_ignore_getters_setters: Option<bool>,
        /// default: false
        pub enforce_count_ignore_lambda: Option<bool>,
        /// Extracted method-name strings from ignorePrefixSelector patterns.
        pub ignore_prefix_selector_names: Option<Vec<String>>,
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
        pub message: String,
        pub loc: DiagnosticLoc,
    }

    #[napi]
    pub fn implemented_functional_rule_names() -> Vec<String> {
        core::implemented_functional_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_functional(
        source_text: String,
        filename: String,
        options: Option<FunctionalScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let default_options = core::FunctionalOptions::default();
        let enforce_parameter_count =
            match options.enforce_parameter_count.as_deref() {
                Some("off") => core::EnforceParameterCount::Off,
                Some("exactlyOne") => core::EnforceParameterCount::ExactlyOne,
                Some("atLeastOne") | None => core::EnforceParameterCount::AtLeastOne,
                Some(_) => default_options.enforce_parameter_count,
            };
        let core_options = core::FunctionalOptions {
            rule_names: compact_rule_names(options.rule_names),
            allow_rest_parameter: options
                .allow_rest_parameter
                .unwrap_or(default_options.allow_rest_parameter),
            allow_arguments_keyword: options
                .allow_arguments_keyword
                .unwrap_or(default_options.allow_arguments_keyword),
            allow_let_in_for_loop_init: options
                .allow_let_in_for_loop_init
                .unwrap_or(default_options.allow_let_in_for_loop_init),
            allow_in_functions: options
                .allow_in_functions
                .unwrap_or(default_options.allow_in_functions),
            allow_throw_to_reject_promises: options
                .allow_throw_to_reject_promises
                .unwrap_or(default_options.allow_throw_to_reject_promises),
            allow_try_catch: options
                .allow_try_catch
                .unwrap_or(default_options.allow_try_catch),
            allow_try_finally: options
                .allow_try_finally
                .unwrap_or(default_options.allow_try_finally),
            readonly_type_mode: options
                .readonly_type_mode
                .filter(|value| value == "generic" || value == "keyword")
                .map(CompactString::from)
                .unwrap_or(default_options.readonly_type_mode),
            ignore_if_readonly_wrapped: options
                .ignore_if_readonly_wrapped
                .unwrap_or(default_options.ignore_if_readonly_wrapped),
            ignore_identifier_pattern: compact_pattern_list(options.ignore_identifier_pattern),
            ignore_code_pattern: compact_pattern_list(options.ignore_code_pattern),
            enforce_parameter_count,
            enforce_count_ignore_iife: options
                .enforce_count_ignore_iife
                .unwrap_or(default_options.enforce_count_ignore_iife),
            enforce_count_ignore_getters_setters: options
                .enforce_count_ignore_getters_setters
                .unwrap_or(default_options.enforce_count_ignore_getters_setters),
            enforce_count_ignore_lambda: options
                .enforce_count_ignore_lambda
                .unwrap_or(default_options.enforce_count_ignore_lambda),
            ignore_prefix_selector_names: compact_pattern_list(
                options.ignore_prefix_selector_names,
            ),
        };

        core::scan_functional(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
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

    fn compact_pattern_list(values: Option<Vec<String>>) -> SmallVec<[CompactString; 4]> {
        values
            .unwrap_or_default()
            .into_iter()
            .map(CompactString::from)
            .collect()
    }

    fn compact_rule_names(values: Option<Vec<String>>) -> SmallVec<[CompactString; 20]> {
        values.map_or_else(
            || {
                core::implemented_functional_rule_names()
                    .iter()
                    .map(|name| CompactString::from(*name))
                    .collect()
            },
            |values| {
                values
                    .into_iter()
                    .filter(|value| {
                        core::implemented_functional_rule_names().contains(&value.as_str())
                    })
                    .map(CompactString::from)
                    .collect()
            },
        )
    }
}
