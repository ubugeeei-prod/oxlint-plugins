//! Port of `no-unnecessary-quoted-identifier`: flag double-quoted identifiers
//! that would mean the same thing unquoted (e.g. `"users"` -> `users`). The
//! parser folds the quotes away, so the rule walks the token stream once at
//! program level (invoked with the null node via `uses_parse_error`).
//!
//! PostgreSQL case-folds unquoted identifiers to lowercase, so only an already
//! all-lowercase inner text can be unquoted without renaming the object;
//! doubled quotes (an embedded `"`) and keywords that require quoting are left
//! alone.

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

/// Mirror upstream `/^[a-z_][a-z0-9_$]*$/`.
fn is_safe_unquoted(raw: &str) -> bool {
    let mut chars = raw.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '$')
}

/// Keywords that require double-quoting as an identifier in any context
/// (RESERVED / COL_NAME / TYPE_FUNC_NAME from PostgreSQL REL_17 kwlist.h),
/// ported verbatim from upstream `src/utils/pg-keywords.ts`.
static PG_KEYWORDS_REQUIRING_QUOTES: phf::Set<&'static str> = phf::phf_set! {
    "all", "analyse", "analyze", "and", "any", "array", "as", "asc",
    "asymmetric", "authorization", "between", "bigint", "binary", "bit",
    "boolean", "both", "case", "cast", "char", "character", "check",
    "coalesce", "collate", "collation", "column", "concurrently", "constraint",
    "create", "cross", "current_catalog", "current_date", "current_role",
    "current_schema", "current_time", "current_timestamp", "current_user",
    "dec", "decimal", "default", "deferrable", "desc", "distinct", "do",
    "else", "end", "except", "exists", "extract", "false", "fetch", "float",
    "for", "foreign", "freeze", "from", "full", "grant", "greatest", "group",
    "grouping", "having", "ilike", "in", "initially", "inner", "inout", "int",
    "integer", "intersect", "interval", "into", "is", "isnull", "join", "json",
    "json_array", "json_arrayagg", "json_exists", "json_object",
    "json_objectagg", "json_query", "json_scalar", "json_serialize",
    "json_table", "json_value", "lateral", "leading", "least", "left", "like",
    "limit", "localtime", "localtimestamp", "merge_action", "national",
    "natural", "nchar", "none", "normalize", "not", "notnull", "null",
    "nullif", "numeric", "offset", "on", "only", "or", "order", "out",
    "outer", "overlaps", "overlay", "placing", "position", "precision",
    "primary", "real", "references", "returning", "right", "row", "select",
    "session_user", "setof", "similar", "smallint", "some", "substring",
    "symmetric", "system_user", "table", "tablesample", "then", "time",
    "timestamp", "to", "trailing", "treat", "trim", "true", "union", "unique",
    "user", "using", "values", "varchar", "variadic", "verbose", "when",
    "where", "window", "with", "xmlattributes", "xmlconcat", "xmlelement",
    "xmlexists", "xmlforest", "xmlnamespaces", "xmlparse", "xmlpi", "xmlroot",
    "xmlserialize", "xmltable",
};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Program-level rule: only act on the single null-node invocation.
    if !node.is_null() {
        return;
    }
    for token in tokenize(ctx.source).tokens {
        // The lexer emits both `'...'` and `"..."` as `String`; a quoted
        // identifier is the double-quoted form.
        if token.kind != TokenKind::String {
            continue;
        }
        let value = token.value.as_str();
        if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
            continue;
        }
        let raw = &value[1..value.len() - 1];
        // An embedded `"` (written `""`) cannot be unquoted.
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
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("inner"),
            value: CompactString::from(raw),
        });
        let fix = DiagnosticFix {
            start: token.start,
            end: token.end,
            replacement: CompactString::from(raw),
        };
        ctx.report_loc(loc, "unnecessaryQuoting", data, Some(fix));
    }
}
