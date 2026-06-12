//! NAPI boundary for the react-hooks oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticLoc, implemented_react_hooks_rule_names, is_hook_name,
    is_react_component_name, scan_react_hooks,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI and proc macros require String/Vec/Option; values are converted before leaving the boundary."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_react_hooks as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub hook: Option<String>,
        pub function_name: Option<String>,
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

    #[napi]
    pub fn implemented_react_hooks_rule_names() -> Vec<String> {
        core::implemented_react_hooks_rule_names()
            .iter()
            .map(|rule| (*rule).to_owned())
            .collect()
    }

    #[napi]
    pub fn is_react_component_name(name: String) -> bool {
        core::is_react_component_name(&name)
    }

    #[napi]
    pub fn is_hook_name(name: String) -> bool {
        core::is_hook_name(&name)
    }

    #[napi]
    pub fn scan_react_hooks(source_text: String, filename: String) -> Vec<Diagnostic> {
        core::scan_react_hooks(&source_text, &filename)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    hook: diagnostic.data.hook.map(|value| value.as_str().to_owned()),
                    function_name: diagnostic
                        .data
                        .function_name
                        .map(|value| value.as_str().to_owned()),
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
}
