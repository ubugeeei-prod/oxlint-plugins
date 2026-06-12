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
        pub allow_throw_to_reject_promises: Option<bool>,
        pub allow_try_catch: Option<bool>,
        pub allow_try_finally: Option<bool>,
        pub readonly_type_mode: Option<String>,
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
        };

        core::scan_functional(&source_text, &filename, &core_options)
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
