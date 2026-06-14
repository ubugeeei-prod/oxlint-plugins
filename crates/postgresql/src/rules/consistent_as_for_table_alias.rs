//! Port of `consistent-as-for-table-alias`: enforce a consistent stance on
//! the `AS` keyword before table aliases in `FROM` clauses (either always
//! require it, or always forbid it).
//!
//! Operates on `RangeVar` AST nodes. The alias sub-node carries only
//! `aliasname` (no range), so we locate the alias token by walking forward
//! from `node.range[1]` (the table reference's end position).
//!
//! Default style `"always"` requires AS (flags bare aliases).
//! Style `"never"` forbids AS (flags explicit AS keywords).

use serde_json::Value;

use crate::ast::is_type;
use crate::tokenize::{Token, TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Strips surrounding double quotes from a token value so it can be compared
/// with `alias.aliasname` (which holds the unquoted identifier from the parser).
fn token_identifier_text(token: &Token) -> &str {
    let v = &token.value;
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        &v[1..v.len() - 1]
    } else {
        v.as_str()
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "RangeVar") {
        return;
    }

    let Some(alias_name) = node
        .get("alias")
        .and_then(|a| a.get("aliasname"))
        .and_then(Value::as_str)
    else {
        return;
    };

    // `node.range[1]` is the byte offset past the last character of the table
    // reference (relation name + optional schema). Walking forward from here
    // finds either `AS` or the alias identifier directly.
    let Some(range_end) = node
        .get("range")
        .and_then(Value::as_array)
        .and_then(|r| r.get(1))
        .and_then(Value::as_u64)
        .map(|v| v as u32)
    else {
        return;
    };

    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    let tokenized = tokenize(ctx.source);
    let tokens = &tokenized.tokens;

    let Some((next_index, next)) = tokens
        .iter()
        .enumerate()
        .find(|(_, t)| t.start >= range_end)
    else {
        return;
    };

    let has_as = next.kind == TokenKind::Keyword && next.value.eq_ignore_ascii_case("AS");

    if style == "always" && !has_as {
        // Confirm the token is actually the alias identifier (not a keyword
        // like WHERE/JOIN that follows an un-aliased table reference).
        if token_identifier_text(next) != alias_name {
            return;
        }
        let loc = DiagnosticLoc {
            start_line: next.start_pos.line,
            start_column: next.start_pos.column,
            end_line: next.end_pos.line,
            end_column: next.end_pos.column,
        };
        // Fix: insert "AS " before the alias token.
        let fix = DiagnosticFix {
            start: next.start,
            end: next.start,
            replacement: CompactString::from("AS "),
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("alias"),
            value: CompactString::from(alias_name),
        });
        ctx.report_loc(loc, "preferAs", data, Some(fix));
    } else if style == "never" && has_as {
        let Some(after) = tokens.get(next_index + 1) else {
            return;
        };
        // Confirm the token after AS is actually the alias identifier.
        if token_identifier_text(after) != alias_name {
            return;
        }
        let loc = DiagnosticLoc {
            start_line: next.start_pos.line,
            start_column: next.start_pos.column,
            end_line: next.end_pos.line,
            end_column: next.end_pos.column,
        };
        // Fix: remove from AS.start to after.start (removes "AS " including
        // the whitespace between AS and the alias identifier).
        let fix = DiagnosticFix {
            start: next.start,
            end: after.start,
            replacement: CompactString::from(""),
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("alias"),
            value: CompactString::from(alias_name),
        });
        ctx.report_loc(loc, "unexpectedAs", data, Some(fix));
    }
}
