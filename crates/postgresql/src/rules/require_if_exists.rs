//! Port of `require-if-exists`: require `IF EXISTS` on every `DROP` statement.
//!
//! Upstream walks the statement's token stream to find the `DROP` keyword and
//! the object-type keyword that follows it, reporting that span. This engine
//! exposes no token stream to rules, so the span is reconstructed from the
//! node's source `range` via `ctx.source`: skip leading trivia/comments to the
//! `DROP` keyword, then take the next word (the object-type keyword) and report
//! `DROP <KEYWORD>`.

use serde_json::Value;

use oxlint_plugins_carton::SmallVec;

use crate::ast::node_type;
use crate::text::Source;
use crate::{DiagnosticLoc, RuleContext};

fn is_drop_node(node: &Value) -> bool {
    matches!(
        node_type(node),
        Some("DropStmt" | "DropdbStmt" | "DropRoleStmt" | "DropSubscriptionStmt")
    )
}

fn skip_trivia(src: &Source, mut i: u32, end: u32) -> u32 {
    while i < end {
        match src.ascii_at(i) {
            Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => i += 1,
            Some(b'-') if src.ascii_at(i + 1) == Some(b'-') => {
                i += 2;
                while i < end && src.ascii_at(i) != Some(b'\n') {
                    i += 1;
                }
            }
            Some(b'/') if src.ascii_at(i + 1) == Some(b'*') => {
                i += 2;
                while i < end {
                    if src.ascii_at(i) == Some(b'*') && src.ascii_at(i + 1) == Some(b'/') {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => break,
        }
    }
    i
}

/// The next identifier-ish word `[start, end)`, skipping leading trivia.
fn read_word(src: &Source, start: u32, end: u32) -> Option<(u32, u32)> {
    let ws = skip_trivia(src, start, end);
    let mut j = ws;
    while j < end {
        match src.ascii_at(j) {
            Some(c) if c.is_ascii_alphanumeric() || c == b'_' => j += 1,
            _ => break,
        }
    }
    if j > ws { Some((ws, j)) } else { None }
}

fn word_eq_ignore_case(src: &Source, s: u32, e: u32, target: &[u8]) -> bool {
    if (e - s) as usize != target.len() {
        return false;
    }
    for (k, t) in target.iter().enumerate() {
        match src.ascii_at(s + k as u32) {
            Some(c) if c.eq_ignore_ascii_case(t) => {}
            _ => return false,
        }
    }
    true
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_drop_node(node) {
        return;
    }
    if node.get("missing_ok").and_then(Value::as_bool) == Some(true) {
        return;
    }
    let Some(range) = node.get("range").and_then(Value::as_array) else {
        return;
    };
    let (Some(start), Some(end)) = (
        range.first().and_then(Value::as_u64),
        range.get(1).and_then(Value::as_u64),
    ) else {
        return;
    };
    let (start, end) = (start as u32, end as u32);
    if end <= start {
        return;
    }
    let src = ctx.source;
    let Some((drop_s, drop_e)) = read_word(src, start, end) else {
        return;
    };
    if !word_eq_ignore_case(src, drop_s, drop_e, b"drop") {
        return;
    }
    let Some((_kind_s, kind_e)) = read_word(src, drop_e, end) else {
        return;
    };
    let sp = src.position(drop_s);
    let ep = src.position(kind_e);
    ctx.report_loc(
        DiagnosticLoc {
            start_line: sp.line,
            start_column: sp.column,
            end_line: ep.line,
            end_column: ep.column,
        },
        "missingIfExists",
        SmallVec::new(),
        None,
    );
}
