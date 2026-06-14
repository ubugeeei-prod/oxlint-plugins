//! Port of `plpgsql-keyword-case`: enforce a consistent case for reserved
//! SQL / PL/pgSQL keywords inside `plpgsql` function bodies.
//!
//! Program-level: the scan engine does not attach `EmbeddedCode` to the
//! per-statement tree it walks (that only happens on the ESLint parse path), so
//! when invoked once with `node == null` (uses_parse_error) this rule re-derives
//! the bodies by re-parsing and running `attach_embedded_code`, then scans each
//! plpgsql body's UTF-16 units exactly like upstream's regex word scan.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "Program-level keyword-case rule: reconstructs source text, re-derives embedded PL/pgSQL bodies, and scans them as UTF-16 units, mirroring upstream's JS string manipulation over serde_json's owned String/Vec."
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::{Value, json};

use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

/// RESERVED_KEYWORD (kwlist.h) + PL/pgSQL reserved kwlist, lower-cased.
const RESERVED: &[&str] = &[
    "all",
    "analyse",
    "analyze",
    "and",
    "any",
    "array",
    "as",
    "asc",
    "asymmetric",
    "begin",
    "both",
    "by",
    "case",
    "cast",
    "check",
    "collate",
    "column",
    "constraint",
    "create",
    "current_catalog",
    "current_date",
    "current_role",
    "current_time",
    "current_timestamp",
    "current_user",
    "declare",
    "default",
    "deferrable",
    "desc",
    "distinct",
    "do",
    "else",
    "end",
    "except",
    "execute",
    "false",
    "fetch",
    "for",
    "foreach",
    "foreign",
    "from",
    "grant",
    "group",
    "having",
    "if",
    "in",
    "initially",
    "intersect",
    "into",
    "lateral",
    "leading",
    "limit",
    "localtime",
    "localtimestamp",
    "loop",
    "not",
    "null",
    "offset",
    "on",
    "only",
    "or",
    "order",
    "placing",
    "primary",
    "references",
    "returning",
    "select",
    "session_user",
    "some",
    "strict",
    "symmetric",
    "system_user",
    "table",
    "then",
    "to",
    "trailing",
    "true",
    "union",
    "unique",
    "user",
    "using",
    "variadic",
    "when",
    "where",
    "while",
    "window",
    "with",
];

const QUOTE: u16 = b'\'' as u16;
const DASH: u16 = b'-' as u16;
const SLASH: u16 = b'/' as u16;
const STAR: u16 = b'*' as u16;
const NL: u16 = b'\n' as u16;
const SP: u16 = b' ' as u16;
const TAB: u16 = b'\t' as u16;
const DOT: u16 = b'.' as u16;

fn is_word_start(u: u16) -> bool {
    u < 128 && {
        let b = u as u8;
        b.is_ascii_alphabetic() || b == b'_'
    }
}

fn is_word(u: u16) -> bool {
    u < 128 && {
        let b = u as u8;
        b.is_ascii_alphanumeric() || b == b'_'
    }
}

fn in_skip(off: u32, skip: &[(u32, u32)]) -> bool {
    skip.iter().any(|&(s, e)| off >= s && off < e)
}

fn collect_skip(u: &[u16]) -> Vec<(u32, u32)> {
    let n = u.len();
    let mut skip: Vec<(u32, u32)> = Vec::new();
    let mut i = 0usize;
    while i < n {
        let c = u[i];
        if c == QUOTE {
            let start = i;
            i += 1;
            while i < n {
                if u[i] == QUOTE {
                    if i + 1 < n && u[i + 1] == QUOTE {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            skip.push((start as u32, i as u32));
        } else if c == DASH && i + 1 < n && u[i + 1] == DASH {
            let start = i;
            while i < n && u[i] != NL {
                i += 1;
            }
            skip.push((start as u32, i as u32));
        } else if c == SLASH && i + 1 < n && u[i + 1] == STAR {
            let start = i;
            i += 2;
            while i + 1 < n && !(u[i] == STAR && u[i + 1] == SLASH) {
                i += 1;
            }
            i += 2;
            let end = i.min(n);
            skip.push((start as u32, end as u32));
        } else {
            i += 1;
        }
    }
    skip
}

fn is_field_access(u: &[u16], start: usize) -> bool {
    if start == 0 {
        return false;
    }
    let mut i = start as isize - 1;
    while i >= 0 && (u[i as usize] == SP || u[i as usize] == TAB) {
        i -= 1;
    }
    if i < 0 || u[i as usize] != DOT {
        return false;
    }
    if i > 0 && u[(i - 1) as usize] == DOT {
        return false;
    }
    true
}

fn scan_body(ctx: &mut RuleContext, source: &str, base: u32, upper: bool) {
    let units: Vec<u16> = source.encode_utf16().collect();
    let n = units.len();
    let skip = collect_skip(&units);
    let mut i = 0usize;
    while i < n {
        if !is_word_start(units[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && is_word(units[i]) {
            i += 1;
        }
        let start_u = start as u32;
        if in_skip(start_u, &skip) || is_field_access(&units, start) {
            continue;
        }
        let word = String::from_utf16_lossy(&units[start..i]);
        let lower = word.to_lowercase();
        if !RESERVED.contains(&lower.as_str()) {
            continue;
        }
        let expected = if upper {
            word.to_uppercase()
        } else {
            word.to_lowercase()
        };
        if word == expected {
            continue;
        }
        let abs_start = base + start_u;
        let abs_end = abs_start + (i - start) as u32;
        let sp = ctx.source.position(abs_start);
        let ep = ctx.source.position(abs_end);
        let loc = DiagnosticLoc {
            start_line: sp.line,
            start_column: sp.column,
            end_line: ep.line,
            end_column: ep.column,
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("actual"),
            value: CompactString::from(word.as_str()),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("expected"),
            value: CompactString::from(expected.as_str()),
        });
        let message_id = if upper {
            "expectedUpper"
        } else {
            "expectedLower"
        };
        let fix = Some(DiagnosticFix {
            start: abs_start,
            end: abs_end,
            replacement: CompactString::from(expected.as_str()),
        });
        ctx.report_loc(loc, message_id, data, fix);
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let upper = !matches!(
        ctx.options
            .get(0)
            .and_then(|o| o.get("case"))
            .and_then(Value::as_str),
        Some("lower")
    );
    let src = ctx.source.slice(0, ctx.source.len());
    let crate::parse::Parsed {
        tokens,
        statements,
        source: parsed_source,
        ..
    } = crate::parse::parse(&src);
    let mut program = json!({ "type": "Program", "body": statements });
    crate::embedded_code::attach_embedded_code(&mut program, &tokens, &parsed_source);

    let Some(body) = program.get("body").and_then(Value::as_array) else {
        return;
    };
    let mut jobs: Vec<(String, u32)> = Vec::new();
    for stmt in body {
        let Some(ec) = stmt.get("embeddedCode") else {
            continue;
        };
        if ec.get("language").and_then(Value::as_str) != Some("plpgsql") {
            continue;
        }
        let Some(base) = ec
            .get("range")
            .and_then(Value::as_array)
            .and_then(|r| r.first())
            .and_then(Value::as_u64)
        else {
            continue;
        };
        let Some(source) = ec.get("source").and_then(Value::as_str) else {
            continue;
        };
        jobs.push((source.to_owned(), base as u32));
    }
    for (source, base) in jobs {
        scan_body(ctx, &source, base, upper);
    }
}
