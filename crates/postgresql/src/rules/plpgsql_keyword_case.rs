//! Port of `plpgsql-keyword-case`: enforce a consistent case (upper or lower)
//! for SQL and PL/pgSQL reserved keywords inside PL/pgSQL function bodies.
//!
//! Visits `EmbeddedCode` nodes where `language == "plpgsql"`, collects skip
//! ranges (string literals, comments), locates `GET [STACKED] DIAGNOSTICS`
//! spans, then walks every identifier-shaped word and reports those that are
//! in `PLPGSQL_RESERVED_KEYWORDS` with the wrong case. Each wrong-case keyword
//! gets its own separate `DiagnosticFix`.
//!
//! Default style: `"upper"`.
//!
//! Faithfully ported from upstream `src/rules/plpgsql-keyword-case.ts`.

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::is_type;
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

/// Reserved keywords in PL/pgSQL: the union of PostgreSQL's `RESERVED_KEYWORD`
/// set and the PL/pgSQL reserved kwlist. Case-folding rules must only touch
/// these because unreserved / COL_NAME / TYPE_FUNC_NAME keywords (`role`,
/// `date`, `value`, …) can legitimately be column/variable names.
///
/// Mirrors upstream `PLPGSQL_RESERVED_KEYWORDS` from `src/utils/pg-keywords.ts`.
static PLPGSQL_RESERVED_KEYWORDS: phf::Set<&'static str> = phf::phf_set! {
    "all", "analyse", "analyze", "and", "any", "array", "as", "asc",
    "asymmetric", "begin", "both", "by", "case", "cast", "check", "collate",
    "column", "constraint", "create", "current_catalog", "current_date",
    "current_role", "current_time", "current_timestamp", "current_user",
    "declare", "default", "deferrable", "desc", "distinct", "do", "else",
    "end", "except", "execute", "false", "fetch", "for", "foreach", "foreign",
    "from", "grant", "group", "having", "if", "in", "initially", "intersect",
    "into", "lateral", "leading", "limit", "localtime", "localtimestamp",
    "loop", "not", "null", "offset", "on", "only", "or", "order", "placing",
    "primary", "references", "returning", "select", "session_user", "some",
    "strict", "symmetric", "system_user", "table", "then", "to", "trailing",
    "true", "union", "unique", "user", "using", "variadic", "when", "where",
    "while", "window", "with",
};

/// Identifiers that PostgreSQL only treats as keywords inside
/// `GET [STACKED] DIAGNOSTICS … = <name>`. Everywhere else they are ordinary
/// identifiers (variable names, `information_schema` column names, etc.), so
/// they must not be case-folded unless they appear inside a GET DIAGNOSTICS
/// statement.
///
/// Mirrors upstream `DIAGNOSTIC_ITEM_NAMES`.
static DIAGNOSTIC_ITEM_NAMES: phf::Set<&'static str> = phf::phf_set! {
    "row_count", "pg_context", "returned_sqlstate", "column_name",
    "constraint_name", "pg_datatype_name", "message_text", "table_name",
    "schema_name", "pg_exception_detail", "pg_exception_hint",
    "pg_exception_context",
};

// ----- UTF-16 unit helpers --------------------------------------------------

/// True when the UTF-16 unit is an identifier-start character (`[a-zA-Z_]`).
#[inline]
fn is_ident_start(u: u16) -> bool {
    // A-Z: 0x41-0x5A, a-z: 0x61-0x7A, _: 0x5F
    matches!(u, 0x41..=0x5A | 0x61..=0x7A | 0x5F)
}

/// True when the UTF-16 unit can continue an identifier (`[a-zA-Z0-9_]`).
#[inline]
fn is_ident_cont(u: u16) -> bool {
    is_ident_start(u) || matches!(u, 0x30..=0x39) // 0-9: 0x30-0x39
}

/// True when `offset` falls inside any of the `[start, end)` ranges.
fn is_in_range(offset: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|&(s, e)| offset >= s && offset < e)
}

// ----- Skip-range collection ------------------------------------------------

/// Collect spans of string literals (`'…'`), line comments (`--…`), and block
/// comments (`/*…*/`) inside `units`. Positions are UTF-16 unit indices into
/// `units`. Ported 1:1 from upstream `collectSkipRanges`.
fn collect_skip_ranges(units: &[u16]) -> SmallVec<[(usize, usize); 16]> {
    let mut skip: SmallVec<[(usize, usize); 16]> = SmallVec::new();
    let n = units.len();
    let mut i = 0usize;
    while i < n {
        let c = units[i];
        if c == 0x27 {
            // Single-quote: scan to closing quote, treating '' as an escape.
            let start = i;
            i += 1;
            while i < n {
                if units[i] == 0x27 {
                    if i + 1 < n && units[i + 1] == 0x27 {
                        // Escaped quote ''
                        i += 2;
                        continue;
                    }
                    i += 1; // consume closing quote
                    break;
                }
                i += 1;
            }
            skip.push((start, i));
        } else if c == 0x2D && i + 1 < n && units[i + 1] == 0x2D {
            // Line comment: scan to newline (exclusive).
            let start = i;
            while i < n && units[i] != 0x0A {
                i += 1;
            }
            skip.push((start, i));
        } else if c == 0x2F && i + 1 < n && units[i + 1] == 0x2A {
            // Block comment: scan to closing */.
            let start = i;
            i += 2;
            while i + 1 < n && !(units[i] == 0x2A && units[i + 1] == 0x2F) {
                i += 1;
            }
            i += 2; // skip past */
            skip.push((start, n.min(i)));
        } else {
            i += 1;
        }
    }
    skip
}

// ----- GET DIAGNOSTICS range detection -------------------------------------

/// Find every `GET [STACKED] DIAGNOSTICS … ;` span in `units` (not in skip),
/// returning `[start_of_GET, position_of_semicolon)` ranges. Item names inside
/// these spans may be case-folded. Ported 1:1 from upstream
/// `findGetDiagnosticsRanges`.
fn find_get_diagnostics_ranges(
    units: &[u16],
    skip: &[(usize, usize)],
) -> SmallVec<[(usize, usize); 4]> {
    let mut ranges: SmallVec<[(usize, usize); 4]> = SmallVec::new();
    let n = units.len();
    let mut i = 0usize;

    while i < n {
        // Skip non-identifier-start characters.
        if !is_ident_start(units[i]) {
            i += 1;
            continue;
        }

        // Scan the complete word.
        let word_start = i;
        while i < n && is_ident_cont(units[i]) {
            i += 1;
        }
        let word_end = i;

        // Must be exactly "get" (case-insensitive).
        if word_end - word_start != 3 {
            continue;
        }
        if !word_eq_ci(&units[word_start..word_end], b"get") {
            continue;
        }
        if is_in_range(word_start, skip) {
            continue;
        }

        // Require whitespace after "get".
        let mut j = word_end;
        if j >= n || !is_ws(units[j]) {
            continue;
        }
        while j < n && is_ws(units[j]) {
            j += 1;
        }

        // Scan the next word: "stacked" or "diagnostics".
        let next_start = j;
        while j < n && is_ident_cont(units[j]) {
            j += 1;
        }
        let next_end = j;
        let next_len = next_end - next_start;

        if next_len == 7 && word_eq_ci(&units[next_start..next_end], b"stacked") {
            // Optional "stacked": require whitespace then "diagnostics".
            if j >= n || !is_ws(units[j]) {
                continue;
            }
            while j < n && is_ws(units[j]) {
                j += 1;
            }
            let diag_start = j;
            while j < n && is_ident_cont(units[j]) {
                j += 1;
            }
            if j - diag_start != 11 || !word_eq_ci(&units[diag_start..j], b"diagnostics") {
                continue;
            }
        } else if next_len == 11 && word_eq_ci(&units[next_start..next_end], b"diagnostics") {
            // Direct "get diagnostics".
        } else {
            continue;
        }

        // Scan forward to find the terminating ';' (not in skip).
        while j < n {
            if units[j] == 0x3B && !is_in_range(j, skip) {
                break;
            }
            j += 1;
        }
        ranges.push((word_start, j));
    }

    ranges
}

/// Case-insensitive ASCII comparison: `units` (all < 0x80) vs `word` (ASCII).
fn word_eq_ci(units: &[u16], word: &[u8]) -> bool {
    if units.len() != word.len() {
        return false;
    }
    units
        .iter()
        .zip(word.iter())
        .all(|(&u, &b)| (u as u8).eq_ignore_ascii_case(&b))
}

/// True when `u` is ASCII whitespace (space, tab, LF, CR).
#[inline]
fn is_ws(u: u16) -> bool {
    matches!(u, 0x20 | 0x09 | 0x0A | 0x0D)
}

// ----- Field-access detection ----------------------------------------------

/// True when the word at `start` is the right-hand side of a dotted access
/// (`NEW.role`, `t . column`). Walks backward over inline whitespace and checks
/// for a single `.` (not `..`). Ported 1:1 from upstream `isFieldAccessTarget`.
fn is_field_access_target(units: &[u16], start: usize) -> bool {
    if start == 0 {
        return false;
    }
    let mut i = start - 1;
    // Skip spaces and tabs going backward.
    loop {
        if units[i] != 0x20 && units[i] != 0x09 {
            break;
        }
        if i == 0 {
            return false;
        }
        i -= 1;
    }
    // Require a single dot (not `..`).
    if units[i] != 0x2E {
        return false;
    }
    if i > 0 && units[i - 1] == 0x2E {
        return false;
    }
    true
}

// ----- Rule entry point ----------------------------------------------------

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "EmbeddedCode") {
        return;
    }

    // Only PL/pgSQL bodies (language is already lower-cased by the parser).
    let language = node.get("language").and_then(Value::as_str).unwrap_or("");
    if language != "plpgsql" {
        return;
    }

    let source_str = node.get("source").and_then(Value::as_str).unwrap_or("");
    let Some(range_arr) = node.get("range").and_then(Value::as_array) else {
        return;
    };
    let Some(abs_body_start) = range_arr.first().and_then(Value::as_u64).map(|v| v as u32) else {
        return;
    };

    let target = ctx
        .options
        .get(0)
        .and_then(|o| o.get("case"))
        .and_then(Value::as_str)
        .unwrap_or("upper");
    let is_upper = target == "upper";
    let message_id: &'static str = if is_upper {
        "expectedUpper"
    } else {
        "expectedLower"
    };

    // Encode the body as UTF-16 units so all offsets match ESLint's convention.
    // SmallVec<[u16; 128]> avoids heap allocation for small bodies (≤128 chars).
    let body_units: SmallVec<[u16; 128]> = source_str.encode_utf16().collect();
    let body_len = body_units.len();

    let skip = collect_skip_ranges(&body_units);
    let diag_ranges = find_get_diagnostics_ranges(&body_units, &skip);

    let mut i = 0usize;
    while i < body_len {
        if !is_ident_start(body_units[i]) {
            i += 1;
            continue;
        }

        // Scan the complete word.
        let word_start = i;
        while i < body_len && is_ident_cont(body_units[i]) {
            i += 1;
        }
        let word_end = i;

        // Skip words inside string literals or comments.
        if is_in_range(word_start, &skip) {
            continue;
        }

        // Skip right-hand sides of dotted field accesses (`NEW.role`, etc.).
        if is_field_access_target(&body_units, word_start) {
            continue;
        }

        // Build the lowercase form for keyword lookup (all units are ASCII).
        let lower_cs: CompactString = body_units[word_start..word_end]
            .iter()
            .map(|&u| char::from((u as u8).to_ascii_lowercase()))
            .collect();

        // Only reserved keywords are eligible for case-folding.
        if !PLPGSQL_RESERVED_KEYWORDS.contains(lower_cs.as_str()) {
            continue;
        }

        // Diagnostic item names must only be folded when they appear inside a
        // GET DIAGNOSTICS statement; otherwise they are regular identifiers.
        if DIAGNOSTIC_ITEM_NAMES.contains(lower_cs.as_str())
            && !is_in_range(word_start, &diag_ranges)
        {
            continue;
        }

        // Build the expected (transformed) form.
        let expected_cs: CompactString = if is_upper {
            body_units[word_start..word_end]
                .iter()
                .map(|&u| char::from((u as u8).to_ascii_uppercase()))
                .collect()
        } else {
            lower_cs.clone()
        };

        // Build the original word for the message data and comparison.
        let word_cs: CompactString = body_units[word_start..word_end]
            .iter()
            .map(|&u| char::from(u as u8))
            .collect();

        // Already the correct case — nothing to report.
        if word_cs == expected_cs {
            continue;
        }

        let abs_start = abs_body_start + word_start as u32;
        let abs_end = abs_body_start + word_end as u32;

        let start_pos = ctx.source.position(abs_start);
        let end_pos = ctx.source.position(abs_end);

        let loc = DiagnosticLoc {
            start_line: start_pos.line,
            start_column: start_pos.column,
            end_line: end_pos.line,
            end_column: end_pos.column,
        };

        let fix_replacement = expected_cs.clone();

        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("actual"),
            value: word_cs,
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("expected"),
            value: expected_cs,
        });

        let fix = DiagnosticFix {
            start: abs_start,
            end: abs_end,
            replacement: fix_replacement,
        };

        ctx.report_loc(loc, message_id, data, Some(fix));
    }
}
