//! Port of `no-natural-join`: disallow `NATURAL JOIN` — the join columns are
//! implicit and any future column with a matching name silently changes the
//! result.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "JoinExpr") {
        return;
    }
    let is_natural = node
        .get("isNatural")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if is_natural {
        ctx.report(node, "noNaturalJoin");
    }
}
