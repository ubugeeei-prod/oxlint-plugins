//! Port of `no-unnecessary-quoted-identifier`: disallow unnecessary
//! double-quoting of identifiers (e.g. `"users"` when `users` means the same
//! thing). Autofixes by stripping the quotes — but only when the inner text is
//! already exclusively lowercase, so unquoting cannot silently rename the
//! object (PostgreSQL case-folds unquoted identifiers to lowercase).
//!
//! Operates on the token stream (not the AST), mirroring upstream's
//! `Program` handler that walks `context.sourceCode.ast.tokens`. Invoked once
//! with `node = Value::Null` (the `uses_parse_error` trigger) and returns early
//! for every real AST node.

use serde_json::Value;

use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Keywords that require double-quoting when used as an identifier in any
/// context (RESERVED / COL_NAME / TYPE_FUNC_NAME from PostgreSQL 17 kwlist.h).
/// Mirrors upstream `PG_KEYWORDS_REQUIRING_QUOTES` (`src/utils/pg-keywords.ts`).
static PG_KEYWORDS_REQUIRING_QUOTES: phf::Set<&'static str> = phf::phf_set! {
    "all", "analyse", "analyze", "and", "any", "array", "as", "asc",
    "asymmetric", "authorization", "between", "bigint", "binary", "bit", "boolean", "both",
    "case", "cast", "char", "character", "check", "coalesce", "collate", "collation",
    "column", "concurrently", "constraint", "create", "cross", "current_catalog", "current_date", "current_role",
    "current_schema", "current_time", "current_timestamp", "current_user", "dec", "decimal", "default", "deferrable",
    "desc", "distinct", "do", "else", "end", "except", "exists", "extract",
    "false", "fetch", "float", "for", "foreign", "freeze", "from", "full",
    "grant", "greatest", "group", "grouping", "having", "ilike", "in", "initially",
    "inner", "inout", "int", "integer", "intersect", "interval", "into", "is",
    "isnull", "join", "json", "json_array", "json_arrayagg", "json_exists", "json_object", "json_objectagg",
    "json_query", "json_scalar", "json_serialize", "json_table", "json_value", "lateral", "leading", "least",
    "left", "like", "limit", "localtime", "localtimestamp", "merge_action", "national", "natural",
    "nchar", "none", "normalize", "not", "notnull", "null", "nullif", "numeric",
    "offset", "on", "only", "or", "order", "out", "outer", "overlaps",
    "overlay", "placing", "position", "precision", "primary", "real", "references", "returning",
    "right", "row", "select", "session_user", "setof", "similar", "smallint", "some",
    "substring", "symmetric", "system_user", "table", "tablesample", "then", "time", "timestamp",
    "to", "trailing", "treat", "trim", "true", "union", "unique", "user",
    "using", "values", "varchar", "variadic", "verbose", "when", "where", "window",
    "with", "xmlattributes", "xmlconcat", "xmlelement", "xmlexists", "xmlforest", "xmlnamespaces", "xmlparse",
    "xmlpi", "xmlroot", "xmlserialize", "xmltable",
};

/// Mirrors upstream's `/^[a-z_][a-z0-9_$]*$/`: the inner text is already an
/// unambiguously-lowercase identifier (so unquoting is lossless).
fn is_safe_unquoted(raw: &str) -> bool {
    let mut chars = raw.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '$')
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only execute on the one-time program-level trigger (Value::Null).
    if !node.is_null() {
        return;
    }

    let tokens = ctx.tokens;
    for token in tokens {
        // The parser emits `"..."` as a String-typed token; distinguish it from
        // a `'...'` literal by the opening quote.
        if !token.value.starts_with('"') {
            continue;
        }
        if token.value.len() < 2 || !token.value.ends_with('"') {
            continue;
        }
        // Strip the surrounding `"` (both ASCII single-byte).
        let raw = &token.value[1..token.value.len() - 1];
        // Doubled quotes inside a quoted identifier are an embedded `"`; such a
        // value cannot be unquoted.
        if raw.contains("\"\"") {
            continue;
        }
        if !is_safe_unquoted(raw) {
            continue;
        }
        if PG_KEYWORDS_REQUIRING_QUOTES.contains(raw) {
            continue;
        }

        let loc = DiagnosticLoc {
            start_line: token.start_pos.line,
            start_column: token.start_pos.column,
            end_line: token.end_pos.line,
            end_column: token.end_pos.column,
        };
        let fix = DiagnosticFix {
            start: token.start,
            end: token.end,
            replacement: CompactString::from(raw),
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("inner"),
            value: CompactString::from(raw),
        });
        ctx.report_loc(loc, "unnecessaryQuoting", data, Some(fix));
    }
}
