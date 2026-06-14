//! Port of `consistent-between-over-and`: `always` rewrites `x >= a AND x <= b`
//! into `x BETWEEN a AND b`; `never` rewrites `BETWEEN` into the two explicit
//! comparisons. Source spans are computed with a full-subtree range union
//! (`getFullSourceRange`) because TypeCast/ColumnRef node ranges are partial.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "autofix boundary: slices source text and builds fix replacement strings"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
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

// A single-operator `A_Expr` (kind AEXPR_OP, one name segment): its operator.
fn op_name(node: &Value) -> Option<&str> {
    if !is_type(node, "A_Expr") {
        return None;
    }
    if str_field(node, "kind") != Some("AEXPR_OP") {
        return None;
    }
    let name = array_field(node, "name")?;
    if name.len() != 1 {
        return None;
    }
    name[0].get("sval").and_then(Value::as_str)
}

fn visit(node: &Value, min: &mut i64, max: &mut i64) {
    match node {
        Value::Object(map) => {
            if let Some(r) = map.get("range").and_then(Value::as_array)
                && let (Some(a), Some(b)) = (
                    r.first().and_then(Value::as_i64),
                    r.get(1).and_then(Value::as_i64),
                )
                && a != 0
            {
                if a < *min {
                    *min = a;
                }
                if b > *max {
                    *max = b;
                }
            }
            for (k, v) in map {
                if matches!(k.as_str(), "parent" | "range" | "loc") {
                    continue;
                }
                visit(v, min, max);
            }
        }
        Value::Array(items) => {
            for it in items {
                visit(it, min, max);
            }
        }
        _ => {}
    }
}

// Union of all descendant `range`s (skipping the [0,0] no-location placeholder).
fn full_source_range(node: &Value) -> Option<(u32, u32)> {
    let mut min: i64 = i64::MAX;
    let mut max: i64 = -1;
    visit(node, &mut min, &mut max);
    if max < 0 || min == i64::MAX {
        None
    } else {
        Some((min as u32, max as u32))
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "shared report helper for both message variants"
)]
fn report_range(
    ctx: &mut RuleContext,
    start: u32,
    end: u32,
    message_id: &'static str,
    lhs: &str,
    lower: &str,
    upper: &str,
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
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("lhs"),
        value: CompactString::from(lhs),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("lower"),
        value: CompactString::from(lower),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("upper"),
        value: CompactString::from(upper),
    });
    ctx.report_loc(
        loc,
        message_id,
        data,
        Some(DiagnosticFix {
            start,
            end,
            replacement,
        }),
    );
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    let s = style(ctx);
    if s == "always" && is_type(node, "BoolExpr") {
        if str_field(node, "boolop") != Some("AND_EXPR") {
            return;
        }
        let Some(args) = array_field(node, "args") else {
            return;
        };
        if args.len() != 2 {
            return;
        }
        let a = &args[0];
        let b = &args[1];
        if op_name(a) != Some(">=") || op_name(b) != Some("<=") {
            return;
        }
        let (Some(a_lex), Some(a_rex), Some(b_lex), Some(b_rex)) = (
            field(a, "lexpr"),
            field(a, "rexpr"),
            field(b, "lexpr"),
            field(b, "rexpr"),
        ) else {
            return;
        };
        let (Some(al), Some(ar), Some(bl), Some(br)) = (
            full_source_range(a_lex),
            full_source_range(a_rex),
            full_source_range(b_lex),
            full_source_range(b_rex),
        ) else {
            return;
        };
        let a_lex_src = ctx.source.slice(al.0, al.1);
        let b_lex_src = ctx.source.slice(bl.0, bl.1);
        if a_lex_src != b_lex_src {
            return;
        }
        let lower = ctx.source.slice(ar.0, ar.1);
        let upper = ctx.source.slice(br.0, br.1);
        let mut replacement = CompactString::default();
        replacement.push_str(&a_lex_src);
        replacement.push_str(" BETWEEN ");
        replacement.push_str(&lower);
        replacement.push_str(" AND ");
        replacement.push_str(&upper);
        report_range(
            ctx,
            al.0,
            br.1,
            "preferBetween",
            &a_lex_src,
            &lower,
            &upper,
            replacement,
        );
        return;
    }
    if s == "never" && is_type(node, "A_Expr") {
        let kind = str_field(node, "kind");
        if kind != Some("AEXPR_BETWEEN") && kind != Some("AEXPR_BETWEEN_SYM") {
            return;
        }
        let Some(lexpr) = field(node, "lexpr") else {
            return;
        };
        let Some(rexpr) = field(node, "rexpr") else {
            return;
        };
        let Some(items) = array_field(rexpr, "items") else {
            return;
        };
        if items.len() != 2 {
            return;
        }
        let (Some(lhs), Some(lo), Some(hi)) = (
            full_source_range(lexpr),
            full_source_range(&items[0]),
            full_source_range(&items[1]),
        ) else {
            return;
        };
        let lhs_src = ctx.source.slice(lhs.0, lhs.1);
        let lower = ctx.source.slice(lo.0, lo.1);
        let upper = ctx.source.slice(hi.0, hi.1);
        let mut replacement = CompactString::default();
        replacement.push_str(&lhs_src);
        replacement.push_str(" >= ");
        replacement.push_str(&lower);
        replacement.push_str(" AND ");
        replacement.push_str(&lhs_src);
        replacement.push_str(" <= ");
        replacement.push_str(&upper);
        report_range(
            ctx,
            lhs.0,
            hi.1,
            "unexpectedBetween",
            &lhs_src,
            &lower,
            &upper,
            replacement,
        );
    }
}
