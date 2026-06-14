//! Port of `consistent-explicit-outer-join`
use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

fn is_side_keyword(value: &str) -> bool {
    let up = value.to_ascii_uppercase();
    matches!(up.as_str(), "LEFT" | "RIGHT" | "FULL")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");
    let tokenized = tokenize(ctx.source);
    let tokens = &tokenized.tokens;

    for i in 0..tokens.len() {
        let token = &tokens[i];
        if token.kind != TokenKind::Keyword {
            continue;
        }
        if !is_side_keyword(&token.value) {
            continue;
        }

        let Some(next) = tokens.get(i + 1) else {
            continue;
        };
        if next.kind != TokenKind::Keyword {
            continue;
        }

        if style == "always" {
            // Look for SIDE JOIN (next token is JOIN)
            if !next.value.eq_ignore_ascii_case("JOIN") {
                continue;
            }
            let side = token.value.to_ascii_uppercase();
            let loc = DiagnosticLoc {
                start_line: token.start_pos.line,
                start_column: token.start_pos.column,
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
                value: CompactString::from(side),
            });
            ctx.report_loc(loc, "preferOuterJoin", data, Some(fix));
        } else {
            // style == "never": look for SIDE OUTER JOIN
            if !next.value.eq_ignore_ascii_case("OUTER") {
                continue;
            }
            let Some(after) = tokens.get(i + 2) else {
                continue;
            };
            if !after.value.eq_ignore_ascii_case("JOIN") {
                continue;
            }
            let side = token.value.to_ascii_uppercase();
            let loc = DiagnosticLoc {
                start_line: token.start_pos.line,
                start_column: token.start_pos.column,
                end_line: after.end_pos.line,
                end_column: after.end_pos.column,
            };
            // Fix: remove "OUTER " from next.start to after.start
            let fix = DiagnosticFix {
                start: next.start,
                end: after.start,
                replacement: CompactString::from(""),
            };
            let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
            data.push(DiagnosticDatum {
                key: CompactString::from("side"),
                value: CompactString::from(side),
            });
            ctx.report_loc(loc, "unexpectedOuterJoin", data, Some(fix));
        }
    }
}
