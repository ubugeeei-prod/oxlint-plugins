//! Port of `consistent-as-for-column-alias`: enforce a consistent stance on
//! the `AS` keyword before column aliases in `SELECT` (either always require
//! it, or always forbid it).
//!
//! Operates on `SelectStmt.targetList` only — `InsertStmt` and `UpdateStmt`
//! use `ResTarget` nodes where inserting `AS` would be a syntax error.
//!
//! Default style `"always"` requires AS (flags bare aliases).
//! Style `"never"` forbids AS (flags explicit AS keywords).
//!
//! The `always` mode includes a defense against parser ranges that stop in
//! the middle of a complex value expression (TypeCast, dotted ColumnRef,
//! CASE, function calls). Only reports when the token immediately after
//! `target.val.range[1]` is the alias identifier itself.

use serde_json::Value;

use crate::ast::{array_field, field, is_type};
use crate::tokenize::{Token, TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Mirrors upstream `tokenIdentifierText`: strips surrounding double quotes
/// from the token value so it can be compared with `target.name` (which holds
/// the unquoted identifier as reported by libpg_query).
fn token_identifier_text(token: &Token) -> &str {
    let v = &token.value;
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        &v[1..v.len() - 1]
    } else {
        v.as_str()
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }

    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    let Some(target_list) = array_field(node, "targetList") else {
        return;
    };

    let tokenized = tokenize(ctx.source);
    let tokens = &tokenized.tokens;

    for target in target_list {
        visit_target(target, style, tokens, ctx);
    }
}

fn visit_target(target: &Value, style: &str, tokens: &[Token], ctx: &mut RuleContext) {
    if !is_type(target, "ResTarget") {
        return;
    }
    let Some(alias_name) = target.get("name").and_then(Value::as_str) else {
        return;
    };

    // `target.val.range[1]` — the direct (not full-source) range end of the
    // value expression. Using the direct range is intentional: for complex
    // expressions (FuncCall, TypeCast, dotted ColumnRef) the parser reports
    // only the first token's position, so `val.range[1]` lands inside the
    // expression.  The following token is then part of the expression, not
    // the alias, and the defense check below catches it.
    let Some(val) = field(target, "val") else {
        return;
    };
    let Some(val_range) = val.get("range").and_then(Value::as_array) else {
        return;
    };
    let Some(val_end) = val_range.get(1).and_then(Value::as_u64).map(|v| v as u32) else {
        return;
    };

    // Find the first token at or after val_end.
    let Some((next_index, next)) = tokens.iter().enumerate().find(|(_, t)| t.start >= val_end)
    else {
        return;
    };

    let has_as = next.kind == TokenKind::Keyword && next.value.eq_ignore_ascii_case("AS");

    if style == "always" && !has_as {
        // Defense: confirm the next token is actually the alias identifier
        // (not a token in the middle of a complex expression).
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
