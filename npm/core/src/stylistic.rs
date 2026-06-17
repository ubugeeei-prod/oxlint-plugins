//! NAPI boundary for the stylistic oxlint plugin.
//!
//! Rule logic lives in `oxlint_plugins_stylistic`; this module only adapts the
//! JSON-shaped config and diagnostics across the JavaScript boundary.

use napi::Result;
use napi_derive::napi;
use serde_json::Value;

#[allow(
    clippy::disallowed_types,
    reason = "NAPI public ABI receives JavaScript strings as owned Rust String values."
)]
#[napi(namespace = "stylistic")]
pub fn run_native_stylistic_lint(source_text: String, config: Value) -> Result<Value> {
    let config = serde_json::from_value(config).map_err(into_napi_error)?;
    let diagnostics = oxlint_plugins_stylistic::run_stylistic_lint(&source_text, &config)
        .map_err(into_napi_error)?;
    serde_json::to_value(diagnostics).map_err(into_napi_error)
}

#[napi(namespace = "stylistic")]
pub fn native_stylistic_rule_metas() -> Result<Value> {
    serde_json::to_value(oxlint_plugins_stylistic::stylistic_rule_metas()).map_err(into_napi_error)
}

#[allow(
    clippy::disallowed_methods,
    reason = "NAPI errors require an owned String reason at the JavaScript boundary."
)]
fn into_napi_error(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
