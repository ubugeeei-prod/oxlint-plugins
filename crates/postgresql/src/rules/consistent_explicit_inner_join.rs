//! Port of `consistent-explicit-inner-join`: enforce a consistent stance on
//! the explicit `INNER` keyword in `INNER JOIN`.
//!
//! Default style `"always"` requires the `INNER` keyword (flag bare `JOIN`).
//! Style `"never"` forbids the `INNER` keyword (flag `INNER JOIN` sequences).
//!
//! Operates on the token stream. Reports fixes via character-range replacement.

use serde_json::Value;

use crate::tokenize::TokenKind;
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Keywords that, when appearing immediately before `JOIN`, already declare the
/// join's kind — a bare `JOIN` (none of these preceding it) is the case that
/// `always` mode rewrites to `INNER JOIN`.
fn is_join_kind_keyword(value: &str) -> bool {
    let up = value.to_ascii_uppercase();
    matches!(
        up.as_str(),
        "INNER" | "OUTER" | "LEFT" | "RIGHT" | "FULL" | "CROSS" | "NATURAL"
    )
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // This rule operates on the token stream, not on the AST.
    // It is triggered once via uses_parse_error (node == null).
    if !node.is_null() {
        return;
    }

    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    let tokens = ctx.tokens;

    for i in 0..tokens.len() {
        let token = &tokens[i];
        if token.kind != TokenKind::Keyword {
            continue;
        }
        if !token.value.eq_ignore_ascii_case("JOIN") {
            continue;
        }

        let prev = if i > 0 { tokens.get(i - 1) } else { None };
        let prev_is_kind = prev
            .map(|p| p.kind == TokenKind::Keyword && is_join_kind_keyword(&p.value))
            .unwrap_or(false);

        if style == "always" {
            if prev_is_kind {
                continue;
            }
            // Bare JOIN without a preceding kind keyword → report and fix.
            let loc = DiagnosticLoc {
                start_line: token.start_pos.line,
                start_column: token.start_pos.column,
                end_line: token.end_pos.line,
                end_column: token.end_pos.column,
            };
            // Fix: insert "INNER " before the JOIN token.
            let fix = DiagnosticFix {
                start: token.start,
                end: token.start,
                replacement: CompactString::from("INNER "),
            };
            ctx.report_loc(loc, "preferInnerJoin", SmallVec::new(), Some(fix));
        } else {
            // style == "never": flag `INNER JOIN`.
            let Some(prev) = prev else { continue };
            if prev.kind != TokenKind::Keyword || !prev.value.eq_ignore_ascii_case("INNER") {
                continue;
            }
            // Report from the start of INNER to the end of JOIN.
            let loc = DiagnosticLoc {
                start_line: prev.start_pos.line,
                start_column: prev.start_pos.column,
                end_line: token.end_pos.line,
                end_column: token.end_pos.column,
            };
            // Fix: remove from INNER.start to JOIN.start (removes "INNER ").
            let fix = DiagnosticFix {
                start: prev.start,
                end: token.start,
                replacement: CompactString::from(""),
            };
            ctx.report_loc(loc, "unexpectedInnerJoin", SmallVec::new(), Some(fix));
        }
    }
}
