//! Port of `consistent-explicit-outer-join`: enforce a consistent stance on the
//! explicit `OUTER` keyword in `LEFT`/`RIGHT`/`FULL OUTER JOIN` (always require
//! it, or always forbid it). Token-driven with an autofix. Runs once per file
//! via the `usesParseError` entry point.
#![allow(
    clippy::disallowed_methods,
    reason = "autofix boundary: builds fix replacement text"
)]

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

fn style(options: &Value) -> &str {
    options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always")
}

fn is_side_keyword(upper: &str) -> bool {
    matches!(upper, "LEFT" | "RIGHT" | "FULL")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let always = style(ctx.options) == "always";

    let tokens = tokenize(ctx.source).tokens;
    let len = tokens.len();
    for i in 0..len.saturating_sub(1) {
        let side = &tokens[i];
        let next = &tokens[i + 1];
        if !matches!(side.kind, TokenKind::Keyword) {
            continue;
        }
        let side_upper = side.value.to_ascii_uppercase();
        if !is_side_keyword(&side_upper) {
            continue;
        }
        if !matches!(next.kind, TokenKind::Keyword) {
            continue;
        }
        if always {
            if !next.value.eq_ignore_ascii_case("JOIN") {
                continue;
            }
            let loc = DiagnosticLoc {
                start_line: side.start_pos.line,
                start_column: side.start_pos.column,
                end_line: next.end_pos.line,
                end_column: next.end_pos.column,
            };
            let fix = DiagnosticFix {
                start: next.start,
                end: next.start,
                replacement: CompactString::from("OUTER "),
            };
            let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
            data.push(DiagnosticDatum {
                key: CompactString::from("side"),
                value: CompactString::from(side_upper),
            });
            ctx.report_loc(loc, "preferOuterJoin", data, Some(fix));
            continue;
        }
        // style === "never": flag `LEFT/RIGHT/FULL OUTER JOIN`.
        if !next.value.eq_ignore_ascii_case("OUTER") {
            continue;
        }
        let Some(after) = tokens.get(i + 2) else {
            continue;
        };
        if !matches!(after.kind, TokenKind::Keyword) || !after.value.eq_ignore_ascii_case("JOIN") {
            continue;
        }
        let loc = DiagnosticLoc {
            start_line: side.start_pos.line,
            start_column: side.start_pos.column,
            end_line: after.end_pos.line,
            end_column: after.end_pos.column,
        };
        let fix = DiagnosticFix {
            start: next.start,
            end: after.start,
            replacement: CompactString::from(""),
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("side"),
            value: CompactString::from(side_upper),
        });
        ctx.report_loc(loc, "unexpectedOuterJoin", data, Some(fix));
    }
}
