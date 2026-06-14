//! Port of `consistent-between-over-and`: enforce a consistent stance on
//! `x BETWEEN a AND b` vs `x >= a AND x <= b` for closed-interval range checks.
//!
//! Default style `"always"` prefers BETWEEN (flags `>= a AND <= b` patterns).
//! Style `"never"` forbids BETWEEN (flags `BETWEEN a AND b` patterns).

use serde_json::Value;

use crate::ast::{array_field, field, is_type, str_field};
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Walk every descendant of `node` collecting `range` values, return
/// `[min_start, max_end]`. Ranges whose start == 0 are skipped (libpg_query's
/// "no location" placeholder). Mirrors upstream `getFullSourceRange`.
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

/// Mirrors upstream `opName`: returns the operator string for an `A_Expr` with
/// `kind === "AEXPR_OP"` and a single-element `name` array, else `None`.
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

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    if is_type(node, "BoolExpr") && style == "always" {
        run_bool_expr(node, ctx);
    } else if is_type(node, "A_Expr") && style == "never" {
        run_a_expr(node, ctx);
    }
}

/// `always` mode: visit `BoolExpr` with `AND_EXPR` + two args where arg[0]
/// has `>=` and arg[1] has `<=` with identical LHS source text. Report and
/// fix: replace the whole span with `{lhs} BETWEEN {lower} AND {upper}`.
fn run_bool_expr(node: &Value, ctx: &mut RuleContext) {
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
    let Some(a_lex) = field(a, "lexpr") else {
        return;
    };
    let Some(a_rex) = field(a, "rexpr") else {
        return;
    };
    let Some(b_lex) = field(b, "lexpr") else {
        return;
    };
    let Some(b_rex) = field(b, "rexpr") else {
        return;
    };
    let Some(a_lex_r) = get_full_source_range(a_lex) else {
        return;
    };
    let Some(a_rex_r) = get_full_source_range(a_rex) else {
        return;
    };
    let Some(b_lex_r) = get_full_source_range(b_lex) else {
        return;
    };
    let Some(b_rex_r) = get_full_source_range(b_rex) else {
        return;
    };

    let a_lex_src = ctx.source.slice(a_lex_r.0, a_lex_r.1);
    let b_lex_src = ctx.source.slice(b_lex_r.0, b_lex_r.1);
    if a_lex_src != b_lex_src {
        return;
    }

    let lower = ctx.source.slice(a_rex_r.0, a_rex_r.1);
    let upper = ctx.source.slice(b_rex_r.0, b_rex_r.1);
    let start = a_lex_r.0;
    let end = b_rex_r.1;

    let start_pos = ctx.source.position(start);
    let end_pos = ctx.source.position(end);
    let loc = DiagnosticLoc {
        start_line: start_pos.line,
        start_column: start_pos.column,
        end_line: end_pos.line,
        end_column: end_pos.column,
    };

    let mut replacement = CompactString::from(a_lex_src.as_str());
    replacement.push_str(" BETWEEN ");
    replacement.push_str(lower.as_str());
    replacement.push_str(" AND ");
    replacement.push_str(upper.as_str());

    let fix = DiagnosticFix {
        start,
        end,
        replacement,
    };

    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("lhs"),
        value: CompactString::from(a_lex_src.as_str()),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("lower"),
        value: CompactString::from(lower.as_str()),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("upper"),
        value: CompactString::from(upper.as_str()),
    });
    ctx.report_loc(loc, "preferBetween", data, Some(fix));
}

/// `never` mode: visit `A_Expr` with `AEXPR_BETWEEN` / `AEXPR_BETWEEN_SYM`.
/// Report and fix: replace the whole span with
/// `{lhs} >= {lower} AND {lhs} <= {upper}`.
fn run_a_expr(node: &Value, ctx: &mut RuleContext) {
    let Some(kind) = str_field(node, "kind") else {
        return;
    };
    if kind != "AEXPR_BETWEEN" && kind != "AEXPR_BETWEEN_SYM" {
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
    let Some(lhs_r) = get_full_source_range(lexpr) else {
        return;
    };
    let Some(lower_r) = get_full_source_range(&items[0]) else {
        return;
    };
    let Some(upper_r) = get_full_source_range(&items[1]) else {
        return;
    };

    let lhs_src = ctx.source.slice(lhs_r.0, lhs_r.1);
    let lower_src = ctx.source.slice(lower_r.0, lower_r.1);
    let upper_src = ctx.source.slice(upper_r.0, upper_r.1);
    let start = lhs_r.0;
    let end = upper_r.1;

    let start_pos = ctx.source.position(start);
    let end_pos = ctx.source.position(end);
    let loc = DiagnosticLoc {
        start_line: start_pos.line,
        start_column: start_pos.column,
        end_line: end_pos.line,
        end_column: end_pos.column,
    };

    let mut replacement = CompactString::from(lhs_src.as_str());
    replacement.push_str(" >= ");
    replacement.push_str(lower_src.as_str());
    replacement.push_str(" AND ");
    replacement.push_str(lhs_src.as_str());
    replacement.push_str(" <= ");
    replacement.push_str(upper_src.as_str());

    let fix = DiagnosticFix {
        start,
        end,
        replacement,
    };

    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("lhs"),
        value: CompactString::from(lhs_src.as_str()),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("lower"),
        value: CompactString::from(lower_src.as_str()),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("upper"),
        value: CompactString::from(upper_src.as_str()),
    });
    ctx.report_loc(loc, "unexpectedBetween", data, Some(fix));
}
