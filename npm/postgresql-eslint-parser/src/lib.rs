//! NAPI boundary for the postgresql-eslint-parser oxlint port.
//!
//! The whole parser (libpg_query parse, AST enrichment, visitor keys) runs in
//! the `oxlint-plugins-postgresql` core crate. This layer only marshals the
//! result across the NAPI boundary as a JSON string, which the JavaScript
//! adapter in `api.js` parses back into the `{ ast, visitorKeys, scopeManager }`
//! object an ESLint custom parser must return.

pub use napi_abi::parse_for_eslint_json;

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires owned String; the value is converted before calling core parser logic."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_postgresql as core;

    /// Parse `sourceText` and return the upstream `parseForESLint` result as a
    /// JSON string (`{ "ast": …, "visitorKeys": …, "scopeManager": null }`).
    #[napi]
    pub fn parse_for_eslint_json(source_text: String) -> String {
        core::parse_for_eslint_json(&source_text)
    }
}
