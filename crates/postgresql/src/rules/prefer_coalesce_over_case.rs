//! Port of `prefer-coalesce-over-case`: flag the verbose `COALESCE` written as
//! `CASE WHEN x IS NULL THEN y ELSE x END` (and its `IS NOT NULL` mirror).
//! Only the searched single-arm form with a matching `ELSE` collapses to a
//! two-argument `COALESCE`, so the structural shape is checked exactly.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

// Position/identity keys ignored when comparing two subtrees for equality
// (mirrors upstream NOISE_KEYS). `type` is intentionally NOT ignored.
fn is_noise_key(key: &str) -> bool {
    matches!(key, "parent" | "loc" | "range")
}

fn same_node(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(ao), Value::Object(bo)) => {
            let a_len = ao.keys().filter(|k| !is_noise_key(k.as_str())).count();
            let b_len = bo.keys().filter(|k| !is_noise_key(k.as_str())).count();
            if a_len != b_len {
                return false;
            }
            ao.iter()
                .filter(|(k, _)| !is_noise_key(k.as_str()))
                .all(|(k, av)| match bo.get(k.as_str()) {
                    Some(bv) => same_node(av, bv),
                    None => false,
                })
        }
        (Value::Array(aa), Value::Array(ba)) => {
            aa.len() == ba.len() && aa.iter().zip(ba.iter()).all(|(x, y)| same_node(x, y))
        }
        _ => a == b,
    }
}

fn is_null_test(node: &Value, kind: &str) -> bool {
    is_type(node, "NullTest") && node.get("nulltesttype").and_then(Value::as_str) == Some(kind)
}

fn is_coalesce_shape(node: &Value) -> bool {
    // Only the searched `CASE WHEN ... END` form (no `CASE expr WHEN ...`).
    if node.get("arg").is_some() {
        return false;
    }
    let Some(args) = node.get("args").and_then(Value::as_array) else {
        return false;
    };
    if args.len() != 1 {
        return false;
    }
    let branch = &args[0];
    let Some(expr) = branch.get("expr") else {
        return false;
    };
    let Some(defresult) = node.get("defresult") else {
        return false;
    };
    let result = branch.get("result").unwrap_or(&Value::Null);

    // CASE WHEN x IS NULL THEN y ELSE x END  ->  COALESCE(x, y)
    if is_null_test(expr, "IS_NULL")
        && let Some(arg) = expr.get("arg")
        && same_node(arg, defresult)
        && !same_node(arg, result)
    {
        return true;
    }
    // CASE WHEN x IS NOT NULL THEN x ELSE y END  ->  COALESCE(x, y)
    if is_null_test(expr, "IS_NOT_NULL")
        && let Some(arg) = expr.get("arg")
        && same_node(arg, result)
        && !same_node(arg, defresult)
    {
        return true;
    }
    false
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CaseExpr") {
        return;
    }
    if is_coalesce_shape(node) {
        ctx.report(node, "preferCoalesceOverCase");
    }
}
