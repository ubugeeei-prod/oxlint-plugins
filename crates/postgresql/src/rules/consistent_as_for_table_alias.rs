//! Port of `consistent-as-for-table-alias`: require (or forbid) the `AS`
//! keyword before table aliases. The parser exposes the alias as
//! `RangeVar.alias.aliasname`; the alias token is located by walking forward
//! from the table reference's reported range end. The name-match guard ensures
//! the next token really is the alias (not `WHERE`/`JOIN`/a column list).
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "autofix boundary: builds fix replacement text and consumes the owned tokenizer output"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{field, is_type, str_field};
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
    if !is_type(node, "RangeVar") {
        return;
    }
    let Some(alias) = field(node, "alias").and_then(|a| str_field(a, "aliasname")) else {
        return;
    };
    let Some(table_end) = node
        .get("range")
        .and_then(Value::as_array)
        .and_then(|r| r.get(1))
        .and_then(Value::as_u64)
    else {
        return;
    };
    let table_end = table_end as u32;
    let tokens = tokenize(ctx.source).tokens;
    let Some(next_index) = tokens.iter().position(|t| t.start >= table_end) else {
        return;
    };
    let next = &tokens[next_index];
    let has_as = next.kind == TokenKind::Keyword && next.value.eq_ignore_ascii_case("AS");
    if style(ctx) == "always" {
        if has_as {
            return;
        }
        if next.kind != TokenKind::Identifier {
            return;
        }
        if next.value.as_str() != alias {
            return;
        }
        let fix = DiagnosticFix {
            start: next.start,
            end: next.start,
            replacement: CompactString::from("AS "),
        };
        report(ctx, next, "preferAs", alias, Some(fix));
        return;
    }
    // style === "never"
    if !has_as {
        return;
    }
    let Some(after) = tokens.get(next_index + 1) else {
        return;
    };
    if after.kind != TokenKind::Identifier {
        return;
    }
    if after.value.as_str() != alias {
        return;
    }
    let fix = DiagnosticFix {
        start: next.start,
        end: after.start,
        replacement: CompactString::default(),
    };
    report(ctx, next, "unexpectedAs", alias, Some(fix));
}
