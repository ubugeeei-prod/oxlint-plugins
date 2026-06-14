//! Port of `prefer-in-list-over-or`: rewrite a chain of `x = a OR x = b OR ...`
//! (same left-hand side, compared by source text) into `x IN (a, b, ...)`.
//! Source ranges are computed with a full-subtree range union because a
//! `TypeCast` node's own `range` is partial.
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

fn is_equality(node: &Value) -> bool {
    is_type(node, "A_Expr")
        && str_field(node, "kind") == Some("AEXPR_OP")
        && array_field(node, "name").is_some_and(|name| {
            name.len() == 1 && name[0].get("sval").and_then(Value::as_str) == Some("=")
        })
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "BoolExpr") {
        return;
    }
    if str_field(node, "boolop") != Some("OR_EXPR") {
        return;
    }
    let Some(args) = array_field(node, "args") else {
        return;
    };
    if args.len() < 2 {
        return;
    }

    let mut rhs_ranges: Vec<(u32, u32)> = Vec::new();
    let mut lhs_text: Option<String> = None;
    let mut chain_start = u32::MAX;
    let mut chain_end: i64 = -1;
    for arg in args {
        if !is_equality(arg) {
            return;
        }
        let (Some(lexpr), Some(rexpr)) = (field(arg, "lexpr"), field(arg, "rexpr")) else {
            return;
        };
        let (Some(lhs_r), Some(rhs_r)) = (full_source_range(lexpr), full_source_range(rexpr))
        else {
            return;
        };
        let lhs_src = ctx.source.slice(lhs_r.0, lhs_r.1);
        match &lhs_text {
            None => lhs_text = Some(lhs_src),
            Some(existing) if *existing != lhs_src => return,
            _ => {}
        }
        rhs_ranges.push(rhs_r);
        if lhs_r.0 < chain_start {
            chain_start = lhs_r.0;
        }
        if (rhs_r.1 as i64) > chain_end {
            chain_end = rhs_r.1 as i64;
        }
    }
    let Some(lhs_text) = lhs_text else {
        return;
    };
    if chain_end < 0 {
        return;
    }
    let chain_end = chain_end as u32;

    let rhs_texts: Vec<String> = rhs_ranges
        .iter()
        .map(|(s, e)| ctx.source.slice(*s, *e))
        .collect();
    let mut replacement = CompactString::default();
    replacement.push_str(&lhs_text);
    replacement.push_str(" IN (");
    replacement.push_str(&rhs_texts.join(", "));
    replacement.push(')');

    let sp = ctx.source.position(chain_start);
    let ep = ctx.source.position(chain_end);
    let loc = DiagnosticLoc {
        start_line: sp.line,
        start_column: sp.column,
        end_line: ep.line,
        end_column: ep.column,
    };
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("lhs"),
        value: CompactString::from(lhs_text.as_str()),
    });
    ctx.report_loc(
        loc,
        "preferIn",
        data,
        Some(DiagnosticFix {
            start: chain_start,
            end: chain_end,
            replacement,
        }),
    );
}
