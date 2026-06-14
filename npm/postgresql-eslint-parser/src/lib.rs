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

    // libpg_query's bison parser keeps its grammar stack on the heap
    // (`YYMAXDEPTH` ~ 10000), so a pathologically nested statement can yield an
    // AST far deeper than the native call stack tolerates. The enrichment passes
    // (`add_types`/`add_location`/…/`build_visitor_keys`) recurse over that tree,
    // and a stack overflow under the release profile's `panic = "abort"` would
    // abort the whole linter process. Run the parse on a dedicated thread with a
    // large (reserved, not committed) stack so any realistically reachable depth
    // is handled without truncating valid input.
    const PARSE_STACK_SIZE: usize = 256 * 1024 * 1024;

    /// Parse `sourceText` and return the upstream `parseForESLint` result as a
    /// JSON string (`{ "ast": …, "visitorKeys": …, "scopeManager": null }`).
    #[napi]
    pub fn parse_for_eslint_json(source_text: String) -> String {
        std::thread::Builder::new()
            .name("postgresql-parse".to_string())
            .stack_size(PARSE_STACK_SIZE)
            .spawn(move || core::parse_for_eslint_json(&source_text))
            .expect("failed to spawn parser thread")
            .join()
            .expect("parser thread panicked")
    }
}
