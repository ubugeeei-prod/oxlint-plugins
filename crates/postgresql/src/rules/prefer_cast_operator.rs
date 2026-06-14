//! Port of `prefer-cast-operator`: enforce a single cast style — `x::type`
//! operator form (default) vs `CAST(x AS type)` function form. The cast's source
//! form is identified by the token at the node's start (`CAST` keyword vs `::`
//! operator); the type expression's end is found by walking qualifier dots and a
//! single matched typmod `(...)` group.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "autofix boundary: slices source text and builds fix replacement strings"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{field, is_type};
use crate::tokenize::{Token, TokenKind, tokenize};
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

fn form(ctx: &RuleContext) -> &'static str {
    match ctx
        .options
        .get(0)
        .and_then(|o| o.get("form"))
        .and_then(Value::as_str)
    {
        Some("function") => "function",
        _ => "operator",
    }
}

// Walk forward through tokens while they look like a type expression (qualifier
// `schema.name` chains, then a single matched `(...)` typmod group). Returns the
// end offset, or None if the shape doesn't fit.
fn find_type_end(tokens: &[Token], start_index: usize) -> Option<u32> {
    let mut i = start_index;
    let first = tokens.get(i)?;
    if !matches!(first.kind, TokenKind::Identifier | TokenKind::Keyword) {
        return None;
    }
    let mut end = first.end;
    i += 1;
    while i + 1 < tokens.len() {
        let dot = &tokens[i];
        let next = &tokens[i + 1];
        if dot.kind == TokenKind::Punctuator
            && dot.value == "."
            && matches!(next.kind, TokenKind::Identifier | TokenKind::Keyword)
        {
            end = next.end;
            i += 2;
        } else {
            break;
        }
    }
    if let Some(t) = tokens.get(i)
        && t.kind == TokenKind::Punctuator
        && t.value == "("
    {
        let mut depth = 1;
        i += 1;
        while i < tokens.len() && depth > 0 {
            let t = &tokens[i];
            if t.kind == TokenKind::Punctuator && t.value == "(" {
                depth += 1;
            } else if t.kind == TokenKind::Punctuator && t.value == ")" {
                depth -= 1;
            }
            end = t.end;
            i += 1;
        }
        if depth != 0 {
            return None;
        }
    }
    Some(end)
}

fn report_replace(
    ctx: &mut RuleContext,
    start: u32,
    end: u32,
    message_id: &'static str,
    replacement: CompactString,
) {
    let sp = ctx.source.position(start);
    let ep = ctx.source.position(end);
    let loc = DiagnosticLoc {
        start_line: sp.line,
        start_column: sp.column,
        end_line: ep.line,
        end_column: ep.column,
    };
    ctx.report_loc(
        loc,
        message_id,
        SmallVec::new(),
        Some(DiagnosticFix {
            start,
            end,
            replacement,
        }),
    );
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "TypeCast") {
        return;
    }
    let Some(tc_start) = node
        .get("range")
        .and_then(Value::as_array)
        .and_then(|r| r.first())
        .and_then(Value::as_u64)
    else {
        return;
    };
    let tc_start = tc_start as u32;
    let Some(arg) = field(node, "arg") else {
        return;
    };
    let Some(arg_range) = arg.get("range").and_then(Value::as_array) else {
        return;
    };
    let (Some(arg_start), Some(arg_end)) = (
        arg_range.first().and_then(Value::as_u64),
        arg_range.get(1).and_then(Value::as_u64),
    ) else {
        return;
    };
    let arg_start = arg_start as u32;
    let arg_end = arg_end as u32;
    let target = form(ctx);
    let tokens = tokenize(ctx.source).tokens;
    let Some(head_index) = tokens.iter().position(|t| t.start == tc_start) else {
        return;
    };
    let head = &tokens[head_index];
    let is_function = head.kind == TokenKind::Keyword && head.value.eq_ignore_ascii_case("CAST");
    let is_operator = head.kind == TokenKind::Operator && head.value == "::";
    if !is_function && !is_operator {
        return;
    }
    if (is_function && target == "function") || (is_operator && target == "operator") {
        return;
    }

    if is_function {
        // CAST ( arg AS type ) — find the matching `)` and the top-level `AS`.
        let head_start = head.start;
        let Some(open) = tokens.get(head_index + 1) else {
            return;
        };
        if !(open.kind == TokenKind::Punctuator && open.value == "(") {
            return;
        }
        let mut depth = 1i32;
        let mut close_idx: Option<usize> = None;
        let mut as_idx: Option<usize> = None;
        let mut i = head_index + 2;
        while i < tokens.len() {
            let t = &tokens[i];
            if t.kind == TokenKind::Punctuator && t.value == "(" {
                depth += 1;
            } else if t.kind == TokenKind::Punctuator && t.value == ")" {
                depth -= 1;
                if depth == 0 {
                    close_idx = Some(i);
                    break;
                }
            } else if depth == 1
                && t.kind == TokenKind::Keyword
                && t.value.eq_ignore_ascii_case("AS")
            {
                as_idx = Some(i);
            }
            i += 1;
        }
        let (Some(as_idx), Some(close_idx)) = (as_idx, close_idx) else {
            return;
        };
        let close_end = tokens[close_idx].end;
        let type_start_idx = as_idx + 1;
        let Some(type_end) = find_type_end(&tokens, type_start_idx) else {
            return;
        };
        let arg_src = ctx.source.slice(arg_start, arg_end);
        let type_src = ctx.source.slice(tokens[type_start_idx].start, type_end);
        let mut replacement = CompactString::default();
        replacement.push_str(&arg_src);
        replacement.push_str("::");
        replacement.push_str(&type_src);
        report_replace(ctx, head_start, close_end, "preferOperator", replacement);
        return;
    }

    // Operator form: arg :: type
    let type_start_idx = head_index + 1;
    let Some(type_end) = find_type_end(&tokens, type_start_idx) else {
        return;
    };
    let arg_src = ctx.source.slice(arg_start, arg_end);
    let type_src = ctx.source.slice(tokens[type_start_idx].start, type_end);
    let mut replacement = CompactString::default();
    replacement.push_str("CAST(");
    replacement.push_str(&arg_src);
    replacement.push_str(" AS ");
    replacement.push_str(&type_src);
    replacement.push(')');
    report_replace(ctx, arg_start, type_end, "preferFunction", replacement);
}
