//! Port of `consistent-as-for-column-alias`: require (or forbid) the `AS`
//! keyword before column aliases in `SELECT`. Token-driven, like upstream: the
//! parser exposes the alias only as `ResTarget.name`, so the alias token is
//! located by walking forward from the value expression's reported range end.
//! The defensive name-match guards against parser ranges that stop mid-expression
//! (TypeCast `::`, dotted ColumnRef, CASE), so `AS` is never inserted inside an
//! expression.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "autofix boundary: builds fix replacement text and consumes the owned tokenizer output"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
use crate::tokenize::{Token, TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

fn style(ctx: &RuleContext) -> &'static str {
    match ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
    {
        Some("never") => "never",
        _ => "always",
    }
}

// Strip surrounding double quotes from a quoted identifier so it can be compared
// against `ResTarget.name` (which holds the unquoted identifier).
fn token_identifier_text(token: &Token) -> &str {
    let v = token.value.as_str();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        &v[1..v.len() - 1]
    } else {
        v
    }
}

fn report(
    ctx: &mut RuleContext,
    token: &Token,
    message_id: &'static str,
    alias: &str,
    fix: Option<DiagnosticFix>,
) {
    let loc = DiagnosticLoc {
        start_line: token.start_pos.line,
        start_column: token.start_pos.column,
        end_line: token.end_pos.line,
        end_column: token.end_pos.column,
    };
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("alias"),
        value: CompactString::from(alias),
    });
    ctx.report_loc(loc, message_id, data, fix);
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only SELECT target lists; the same ResTarget type is used for INSERT column
    // lists and UPDATE SET clauses where `AS` would be a syntax error.
    if !is_type(node, "SelectStmt") {
        return;
    }
    let Some(target_list) = array_field(node, "targetList") else {
        return;
    };
    let s = style(ctx);
    let tokens = tokenize(ctx.source).tokens;
    for target in target_list {
        if !is_type(target, "ResTarget") {
            continue;
        }
        let Some(alias) = str_field(target, "name") else {
            continue;
        };
        let Some(val) = field(target, "val") else {
            continue;
        };
        let Some(val_end) = val
            .get("range")
            .and_then(Value::as_array)
            .and_then(|r| r.get(1))
            .and_then(Value::as_u64)
        else {
            continue;
        };
        let val_end = val_end as u32;
        let Some(next_index) = tokens.iter().position(|t| t.start >= val_end) else {
            continue;
        };
        let next = &tokens[next_index];
        let has_as = next.kind == TokenKind::Keyword && next.value.eq_ignore_ascii_case("AS");
        if s == "always" {
            if has_as {
                continue;
            }
            if token_identifier_text(next) != alias {
                continue;
            }
            let fix = DiagnosticFix {
                start: next.start,
                end: next.start,
                replacement: CompactString::from("AS "),
            };
            report(ctx, next, "preferAs", alias, Some(fix));
            continue;
        }
        // style === "never"
        if !has_as {
            continue;
        }
        let Some(after) = tokens.get(next_index + 1) else {
            continue;
        };
        let fix = DiagnosticFix {
            start: next.start,
            end: after.start,
            replacement: CompactString::default(),
        };
        report(ctx, next, "unexpectedAs", alias, Some(fix));
    }
}
