//! Port of `prefer-in-list-over-or`: prefer `x IN (a, b, c)` over a chain
//! of `x = a OR x = b OR x = c`. Produces an autofix.

use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

fn get_full_source_range(node: &Value) -> Option<(u32, u32)> {
    let mut min = u32::MAX;
    let mut max = 0u32;
    let mut found = false;
    walk_range(node, &mut min, &mut max, &mut found);
    if found { Some((min, max)) } else { None }
}

fn walk_range(node: &Value, min: &mut u32, max: &mut u32, found: &mut bool) {
    match node {
        Value::Object(map) => {
            if let Some(range) = map.get("range").and_then(Value::as_array)
                && range.len() == 2
            {
                let s = range[0].as_u64().unwrap_or(0) as u32;
                let e = range[1].as_u64().unwrap_or(0) as u32;
                if s != 0 {
                    *found = true;
                    if s < *min {
                        *min = s;
                    }
                    if e > *max {
                        *max = e;
                    }
                }
            }
            for (k, v) in map {
                if matches!(k.as_str(), "parent" | "range" | "loc") {
                    continue;
                }
                walk_range(v, min, max, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_range(item, min, max, found);
            }
        }
        _ => {}
    }
}

fn is_equality(node: &Value) -> bool {
    if !is_type(node, "A_Expr") {
        return false;
    }
    if str_field(node, "kind") != Some("AEXPR_OP") {
        return false;
    }
    let Some(name) = array_field(node, "name") else {
        return false;
    };
    if name.len() != 1 {
        return false;
    }
    name[0].get("sval").and_then(Value::as_str) == Some("=")
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

    let mut lhs_text: Option<CompactString> = None;
    let mut chain_start = u32::MAX;
    let mut chain_end = 0u32;
    let mut rhs_ranges: SmallVec<[(u32, u32); 8]> = SmallVec::new();

    for arg in args {
        if !is_equality(arg) {
            return;
        }
        let Some(lexpr) = field(arg, "lexpr") else {
            return;
        };
        let Some(rexpr) = field(arg, "rexpr") else {
            return;
        };
        let Some(lhs_r) = get_full_source_range(lexpr) else {
            return;
        };
        let Some(rhs_r) = get_full_source_range(rexpr) else {
            return;
        };
        let lhs_src = ctx.source.slice(lhs_r.0, lhs_r.1);
        if let Some(ref existing) = lhs_text {
            if existing.as_str() != lhs_src.as_str() {
                return;
            }
        } else {
            lhs_text = Some(CompactString::from(lhs_src.as_str()));
        }
        rhs_ranges.push(rhs_r);
        if lhs_r.0 < chain_start {
            chain_start = lhs_r.0;
        }
        if rhs_r.1 > chain_end {
            chain_end = rhs_r.1;
        }
    }

    let Some(lhs_text) = lhs_text else {
        return;
    };
    if chain_end == 0 {
        return;
    }

    // Build replacement: `{lhs} IN ({rhs1}, {rhs2}, ...)`
    let mut replacement = CompactString::from(lhs_text.as_str());
    replacement.push_str(" IN (");
    for (i, &(s, e)) in rhs_ranges.iter().enumerate() {
        if i > 0 {
            replacement.push_str(", ");
        }
        let rhs_src = ctx.source.slice(s, e);
        replacement.push_str(rhs_src.as_str());
    }
    replacement.push(')');

    let start_pos = ctx.source.position(chain_start);
    let end_pos = ctx.source.position(chain_end);
    let loc = DiagnosticLoc {
        start_line: start_pos.line,
        start_column: start_pos.column,
        end_line: end_pos.line,
        end_column: end_pos.column,
    };

    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("lhs"),
        value: lhs_text,
    });

    let fix = DiagnosticFix {
        start: chain_start,
        end: chain_end,
        replacement,
    };

    ctx.report_loc(loc, "preferIn", data, Some(fix));
}
