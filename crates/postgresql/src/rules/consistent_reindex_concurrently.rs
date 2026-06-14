//! Port of `consistent-reindex-concurrently`: enforce a consistent stance on
//! `CONCURRENTLY` for `REINDEX` (either always require it, or always forbid it).
//! `CONCURRENTLY` is carried as a `DefElem { defname: "concurrently" }` in the
//! statement's `params` list.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type, str_field};

fn style(options: &Value) -> &str {
    options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "ReindexStmt") {
        return;
    }
    let concurrent = array_field(node, "params").is_some_and(|params| {
        params
            .iter()
            .any(|p| is_type(p, "DefElem") && str_field(p, "defname") == Some("concurrently"))
    });
    let opt = style(ctx.options);
    let always = opt == "always";
    let never = opt == "never";
    if always && !concurrent {
        ctx.report(node, "preferReindexConcurrently");
    } else if never && concurrent {
        ctx.report(node, "unexpectedReindexConcurrently");
    }
}
