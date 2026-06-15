//! NAPI boundary for the postgresql oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticLoc, PostgresqlOptions,
    implemented_postgresql_rule_names, scan_postgresql,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI and proc macros require String/Vec/Option; values are converted before calling core helpers."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_postgresql as core;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PostgresqlOptions {
        pub identity_style: Option<String>,
        pub jsonb_style: Option<String>,
        pub text_style: Option<String>,
        pub timestamptz_style: Option<String>,
        pub not_equals_operator: Option<String>,
        pub cast_form: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub op: Option<String>,
        pub type_name: Option<String>,
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
    pub fn implemented_postgresql_rule_names() -> Vec<String> {
        core::implemented_postgresql_rule_names()
            .iter()
            .map(|rule| (*rule).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_postgresql(
        source_text: String,
        filename: String,
        options: Option<PostgresqlOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let core_options = core::ScanOptions {
            identity_style: style_or_default(options.identity_style),
            jsonb_style: style_or_default(options.jsonb_style),
            text_style: style_or_default(options.text_style),
            timestamptz_style: style_or_default(options.timestamptz_style),
            not_equals_operator: not_equals_operator_or_default(options.not_equals_operator),
            cast_form: cast_form_or_default(options.cast_form),
        };

        core::scan_postgresql(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    op: diagnostic.data.op.map(|value| value.as_str().to_owned()),
                    type_name: diagnostic
                        .data
                        .type_name
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

    fn style_or_default(value: Option<String>) -> core::Style {
        match value.as_deref() {
            Some("never") => core::Style::Never,
            _ => core::Style::Always,
        }
    }

    fn not_equals_operator_or_default(value: Option<String>) -> core::NotEqualsOperator {
        match value.as_deref() {
            Some("!=") => core::NotEqualsOperator::Bang,
            _ => core::NotEqualsOperator::Angle,
        }
    }

    fn cast_form_or_default(value: Option<String>) -> core::CastForm {
        match value.as_deref() {
            Some("function") => core::CastForm::Function,
            _ => core::CastForm::Operator,
        }
    }
}
