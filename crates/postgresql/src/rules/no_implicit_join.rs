//! Port of `no-implicit-join`: disallow comma-separated FROM clauses (implicit
//! cross joins). Whenever the `FROM` list contains more than one item, the
//! tables are implicitly cross-joined, which is almost always a mistake.
//! Explicit `JOIN … ON …` should be used instead.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    if array_field(node, "fromClause").is_some_and(|f| f.len() > 1) {
        ctx.report(node, "noImplicitJoin");
    }
}
