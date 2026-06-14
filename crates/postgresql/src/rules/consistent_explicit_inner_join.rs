//! Port of `consistent-explicit-inner-join`: enforce a consistent stance on the
//! explicit `INNER` keyword in `INNER JOIN` (always require it, or always forbid
//! it). Token-driven, with an autofix that inserts/removes the `INNER` keyword.
//! Runs once per file via the `usesParseError` entry point.
#![allow(
    clippy::disallowed_methods,
    reason = "autofix boundary: builds fix replacement text"
)]

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::tokenize::{Token, TokenKind, tokenize};
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

fn style(options: &Value) -> &str {
    options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always")
}

// Keywords that already declare a join's kind when they immediately precede
// `JOIN`. A bare `JOIN` (none of these before it) is what `always` rewrites.
fn is_join_kind_keyword(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Keyword)
        && matches!(
            token.value.to_ascii_uppercase().as_str(),
            "INNER" | "OUTER" | "LEFT" | "RIGHT" | "FULL" | "CROSS" | "NATURAL"
        )
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let always = style(ctx.options) == "always";

    let tokens = tokenize(ctx.source).tokens;
    for i in 0..tokens.len() {
        let token = &tokens[i];
        if !matches!(token.kind, TokenKind::Keyword) || !token.value.eq_ignore_ascii_case("JOIN") {
            continue;
        }
        let prev = if i > 0 { Some(&tokens[i - 1]) } else { None };
        if always {
            if prev.is_some_and(is_join_kind_keyword) {
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
                end: token.start,
                replacement: CompactString::from("INNER "),
            };
            ctx.report_loc(loc, "preferInnerJoin", SmallVec::new(), Some(fix));
            continue;
        }
        // style === "never": flag an explicit `INNER JOIN`.
        let Some(prev) = prev else {
            continue;
        };
        if !matches!(prev.kind, TokenKind::Keyword) || !prev.value.eq_ignore_ascii_case("INNER") {
            continue;
        }
        let loc = DiagnosticLoc {
            start_line: prev.start_pos.line,
            start_column: prev.start_pos.column,
            end_line: token.end_pos.line,
            end_column: token.end_pos.column,
        };
        let fix = DiagnosticFix {
            start: prev.start,
            end: token.start,
            replacement: CompactString::from(""),
        };
        ctx.report_loc(loc, "unexpectedInnerJoin", SmallVec::new(), Some(fix));
    }
}
