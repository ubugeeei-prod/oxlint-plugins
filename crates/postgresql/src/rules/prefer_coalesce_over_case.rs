//! Port of `prefer-coalesce-over-case`: prefer `COALESCE(x, y)` over
//! `CASE WHEN x IS NULL THEN y ELSE x END` (and its IS NOT NULL mirror).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

fn is_noise_key(k: &str) -> bool {
    matches!(k, "parent" | "loc" | "range")
}

fn same_node(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(xs), Value::Array(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys.iter()).all(|(x, y)| same_node(x, y))
        }
        (Value::Object(ao), Value::Object(bo)) => {
            let a_count = ao.keys().filter(|k| !is_noise_key(k)).count();
            let b_count = bo.keys().filter(|k| !is_noise_key(k)).count();
            if a_count != b_count {
                return false;
            }
            ao.iter()
                .filter(|(k, _)| !is_noise_key(k))
                .all(|(k, av)| match bo.get(k) {
                    Some(bv) => same_node(av, bv),
                    None => false,
                })
        }
        _ => false,
    }
}

fn is_null_test(node: &Value, kind: &str) -> bool {
    is_type(node, "NullTest") && str_field(node, "nulltesttype") == Some(kind)
}

fn is_coalesce_shape(node: &Value) -> bool {
    // Only the searched `CASE WHEN ... END` form (no `CASE expr WHEN ...`).
    if field(node, "arg").is_some() {
        return false;
    }
    let Some(args) = array_field(node, "args") else {
        return false;
    };
    if args.len() != 1 {
        return false;
    }
    let branch = &args[0];
    let Some(expr) = field(branch, "expr") else {
        return false;
    };
    let Some(defresult) = field(node, "defresult") else {
        return false;
    };
    if defresult.is_null() {
        return false;
    }

    // CASE WHEN x IS NULL THEN y ELSE x END  →  COALESCE(x, y)
    if is_null_test(expr, "IS_NULL")
        && let Some(expr_arg) = field(expr, "arg")
        && same_node(expr_arg, defresult)
        && field(branch, "result").is_none_or(|r| !same_node(expr_arg, r))
    {
        return true;
    }
    // CASE WHEN x IS NOT NULL THEN x ELSE y END  →  COALESCE(x, y)
    if is_null_test(expr, "IS_NOT_NULL")
        && let Some(expr_arg) = field(expr, "arg")
        && let Some(branch_result) = field(branch, "result")
        && same_node(expr_arg, branch_result)
        && !same_node(expr_arg, defresult)
    {
        return true;
    }
    false
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CaseExpr") {
        return;
    }
    if !is_coalesce_shape(node) {
        return;
    }
    ctx.report(node, "preferCoalesceOverCase");
}
