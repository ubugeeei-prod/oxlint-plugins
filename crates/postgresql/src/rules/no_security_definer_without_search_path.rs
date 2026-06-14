//! Port of `no-security-definer-without-search-path`: require `SECURITY DEFINER`
//! functions to also `SET search_path = ...` so an attacker-controlled schema
//! in the caller's `search_path` cannot shadow built-in objects called from
//! inside the function body.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

/// Return true when the DefElem arg represents the Boolean `true` value
/// (i.e. SECURITY DEFINER, not SECURITY INVOKER).
fn is_bool_true(arg: &Value) -> bool {
    // PostgreSQL 17 libpg_query JSON: {"type":"Boolean","boolval":true}
    if is_type(arg, "Boolean") {
        return arg.get("boolval") == Some(&Value::Bool(true));
    }
    false
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateFunctionStmt") {
        return;
    }

    let Some(options) = array_field(node, "options") else {
        return;
    };

    // Look for SECURITY DEFINER in the options list.
    let has_security_definer = options.iter().any(|opt| {
        str_field(opt, "defname") == Some("security") && field(opt, "arg").is_some_and(is_bool_true)
    });

    if !has_security_definer {
        return;
    }

    // Match upstream's heuristic exactly: a SET clause attaches as a `set`
    // defElem regardless of which GUC it targets, and the rule accepts ANY
    // `SET` (not `search_path` specifically) — see the upstream rule comment
    // ("users who bother to set `role` typically also fix `search_path`").
    let has_set = options
        .iter()
        .any(|opt| str_field(opt, "defname") == Some("set"));

    if has_set {
        return;
    }

    ctx.report(node, "missingSearchPath");
}
