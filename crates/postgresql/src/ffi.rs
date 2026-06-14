//! Direct FFI to libpg_query's JSON parse entry point.
//!
//! The `pg_query` crate (a build dependency here) statically links libpg_query
//! — the actual PostgreSQL 17 parser — into this cdylib, but its safe Rust API
//! only exposes the protobuf parse tree, whose field names are snake_cased
//! (`target_list`). Upstream `eslint-plugin-postgresql` rules are written
//! against libpg_query's *JSON* output, which preserves the original
//! PostgreSQL node field names (`SelectStmt`, `targetList`, `ResTarget`,
//! `A_Star`, …) exactly as `@libpg-query/parser` (PG17, the parser upstream
//! depends on) emits them. To keep the port faithful we call the C JSON entry
//! point `pg_query_parse` ourselves and walk the resulting JSON, mirroring the
//! upstream TypeScript pipeline node-for-node.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "libpg_query C ABI boundary: error/parse-tree strings cross as owned String."
)]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[repr(C)]
struct PgQueryError {
    message: *mut c_char,
    funcname: *mut c_char,
    filename: *mut c_char,
    lineno: i32,
    cursorpos: i32,
    context: *mut c_char,
}

#[repr(C)]
struct PgQueryParseResult {
    parse_tree: *mut c_char,
    stderr_buffer: *mut c_char,
    error: *mut PgQueryError,
}

unsafe extern "C" {
    fn pg_query_parse(input: *const c_char) -> PgQueryParseResult;
    fn pg_query_free_parse_result(result: PgQueryParseResult);
}

/// Parse `sql` with libpg_query and return its parse tree as a JSON string.
///
/// Returns `Err` with libpg_query's diagnostic message on a syntax error (or
/// when the input contains an interior NUL, which SQL never legitimately does).
pub fn parse_to_json(sql: &str) -> Result<String, String> {
    let input = CString::new(sql).map_err(|_| "SQL contains an interior NUL byte".to_string())?;

    // SAFETY: `input` is a valid NUL-terminated C string that outlives the
    // call. `pg_query_parse` returns owned C strings that we copy into Rust
    // `String`s before handing the result back to `pg_query_free_parse_result`,
    // which frees them. We never retain any raw pointer past this block.
    #[allow(unsafe_code)]
    unsafe {
        let result = pg_query_parse(input.as_ptr());

        let outcome = if result.error.is_null() {
            if result.parse_tree.is_null() {
                Err("libpg_query returned no parse tree".to_string())
            } else {
                Ok(CStr::from_ptr(result.parse_tree)
                    .to_string_lossy()
                    .into_owned())
            }
        } else {
            // `error` is non-null here, but its `message` field is itself a
            // `*mut c_char` the C ABI allows to be null (e.g. on an allocation
            // failure inside libpg_query); guard the deref rather than trust it.
            let message_ptr = (*result.error).message;
            let message = if message_ptr.is_null() {
                "libpg_query reported an error with no message".to_string()
            } else {
                CStr::from_ptr(message_ptr).to_string_lossy().into_owned()
            };
            Err(message)
        };

        pg_query_free_parse_result(result);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::parse_to_json;

    #[test]
    fn parses_select_with_original_field_names() {
        let json = parse_to_json("SELECT *, id FROM users WHERE id = 1").expect("should parse");
        // Original PostgreSQL field names, not the protobuf snake_case forms.
        assert!(json.contains("SelectStmt"));
        assert!(json.contains("targetList"));
        assert!(json.contains("A_Star"));
        assert!(json.contains("\"location\""));
    }

    #[test]
    fn reports_syntax_error() {
        assert!(parse_to_json("SELECT FROM WHERE )(").is_err());
    }
}
