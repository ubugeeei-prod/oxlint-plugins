//! Port of `require-on-delete-action`: require an explicit `ON DELETE` clause
//! on every foreign-key constraint, and (when `allowed` is set) restrict which
//! action is used.
//!
//! libpg_query reports `fk_del_action: "a"` for both an omitted `ON DELETE`
//! and an explicit `ON DELETE NO ACTION`, so the AST alone cannot tell them
//! apart. Upstream walks the token stream; this port reproduces that by
//! scanning the source text forward from the FK Constraint's range start —
//! tracking paren depth to find the end of the FK clause (a `,`/`;` at depth 0
//! or a closing `)` that drops below depth 0), then locating the `ON DELETE`
//! keyword pair and the action that follows it. Spans are emitted in UTF-16
//! unit space via `ctx.source.position`.

#![allow(
    clippy::disallowed_types,
    reason = "rule helpers operate on arbitrary-length identifier/column lists and reconstructed source text at the rule boundary, where owned String/Vec and per-rule formatting are appropriate"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{is_type, str_field};
use crate::text::Source;
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};

struct ActionSpan {
    name: &'static str,
    start: u32,
    end: u32,
}

struct FkScan {
    /// End offset (UTF-16 units) of the last token in the FK clause.
    clause_end: u32,
    has_on_delete: bool,
    action: Option<ActionSpan>,
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn scan_fk_clause(src: &Source, start: u32) -> FkScan {
    let len = src.len();
    let mut i = start;
    let mut depth: i32 = 0;
    let mut last_end = start;
    let mut words: Vec<(u32, u32, String)> = Vec::new();
    while i < len {
        match src.ascii_at(i) {
            Some(b) if b.is_ascii_whitespace() => i += 1,
            // line comment `-- ...`
            Some(b'-') if src.ascii_at(i + 1) == Some(b'-') => {
                i += 2;
                while i < len && src.ascii_at(i) != Some(b'\n') {
                    i += 1;
                }
            }
            // block comment `/* ... */`
            Some(b'/') if src.ascii_at(i + 1) == Some(b'*') => {
                i += 2;
                while i < len
                    && !(src.ascii_at(i) == Some(b'*') && src.ascii_at(i + 1) == Some(b'/'))
                {
                    i += 1;
                }
                i = (i + 2).min(len);
            }
            // single-quoted string literal ('' escapes an inner quote)
            Some(b'\'') => {
                i += 1;
                while i < len {
                    if src.ascii_at(i) == Some(b'\'') {
                        if src.ascii_at(i + 1) == Some(b'\'') {
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                last_end = i;
            }
            // double-quoted identifier ("" escapes an inner quote)
            Some(b'"') => {
                i += 1;
                while i < len {
                    if src.ascii_at(i) == Some(b'"') {
                        if src.ascii_at(i + 1) == Some(b'"') {
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                last_end = i;
            }
            Some(b'(') => {
                depth += 1;
                last_end = i + 1;
                i += 1;
            }
            Some(b')') => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                last_end = i + 1;
                i += 1;
            }
            Some(b',') | Some(b';') if depth == 0 => break,
            Some(b) if is_word_byte(b) => {
                let s = i;
                while i < len {
                    match src.ascii_at(i) {
                        Some(x) if is_word_byte(x) => i += 1,
                        _ => break,
                    }
                }
                words.push((s, i, src.slice(s, i).to_ascii_uppercase()));
                last_end = i;
            }
            // any other punctuation/operator, or a non-ASCII unit
            _ => {
                i += 1;
                last_end = i;
            }
        }
    }

    let mut idx = None;
    if words.len() >= 2 {
        for k in 0..words.len() - 1 {
            if words[k].2 == "ON" && words[k + 1].2 == "DELETE" {
                idx = Some(k);
                break;
            }
        }
    }

    let action = idx.and_then(|k| {
        let (s, e, a) = words.get(k + 2)?;
        let second = words.get(k + 3).map(|w| w.2.as_str());
        match (a.as_str(), second) {
            ("CASCADE", _) => Some(ActionSpan {
                name: "CASCADE",
                start: *s,
                end: *e,
            }),
            ("RESTRICT", _) => Some(ActionSpan {
                name: "RESTRICT",
                start: *s,
                end: *e,
            }),
            ("NO", Some("ACTION")) => Some(ActionSpan {
                name: "NO ACTION",
                start: *s,
                end: words[k + 3].1,
            }),
            ("SET", Some("NULL")) => Some(ActionSpan {
                name: "SET NULL",
                start: *s,
                end: words[k + 3].1,
            }),
            ("SET", Some("DEFAULT")) => Some(ActionSpan {
                name: "SET DEFAULT",
                start: *s,
                end: words[k + 3].1,
            }),
            _ => None,
        }
    });

    FkScan {
        clause_end: last_end,
        has_on_delete: idx.is_some(),
        action,
    }
}

fn loc(src: &Source, start: u32, end: u32) -> DiagnosticLoc {
    let sp = src.position(start);
    let ep = src.position(end);
    DiagnosticLoc {
        start_line: sp.line,
        start_column: sp.column,
        end_line: ep.line,
        end_column: ep.column,
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "Constraint") {
        return;
    }
    if str_field(node, "contype") != Some("CONSTR_FOREIGN") {
        return;
    }
    let Some(start) = node
        .get("range")
        .and_then(Value::as_array)
        .and_then(|r| r.first())
        .and_then(Value::as_u64)
    else {
        return;
    };
    let start = start as u32;

    let scan = scan_fk_clause(ctx.source, start);

    if !scan.has_on_delete {
        let l = loc(ctx.source, start, scan.clause_end);
        ctx.report_loc(l, "missingOnDelete", SmallVec::new(), None);
        return;
    }

    let allowed: Option<Vec<String>> = ctx
        .options
        .get(0)
        .and_then(|o| o.get("allowed"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
    let Some(allowed) = allowed else {
        return;
    };
    let Some(act) = scan.action else {
        return;
    };
    if allowed.iter().any(|a| a == act.name) {
        return;
    }

    let l = loc(ctx.source, act.start, act.end);
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("action"),
        value: CompactString::from(act.name),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("allowedList"),
        value: CompactString::from(allowed.join(", ").as_str()),
    });
    ctx.report_loc(l, "disallowedAction", data, None);
}
