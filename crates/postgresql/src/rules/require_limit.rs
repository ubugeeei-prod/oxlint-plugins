//! Port of `require-limit`: require a `LIMIT` clause in every `SELECT`
//! statement to prevent accidentally fetching unbounded result sets.
//!
//! libpg_query rewrites `INSERT INTO t VALUES (...)` as an `InsertStmt` whose
//! inner `selectStmt` is a `SelectStmt` with a non-empty `valuesLists` and no
//! `targetList`. `LIMIT` has no meaning there, so those synthetic nodes are
//! skipped (upstream regression #159).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SelectStmt") {
        return;
    }
    // Skip synthetic SelectStmt nodes produced by INSERT ... VALUES (...).
    if array_field(node, "valuesLists").is_some_and(|v| !v.is_empty()) {
        return;
    }
    // A SELECT has a LIMIT when `limitCount` is present and `limitOption` is
    // not the parser's default sentinel "LIMIT_OPTION_DEFAULT".
    let has_limit = field(node, "limitCount").is_some()
        && str_field(node, "limitOption") != Some("LIMIT_OPTION_DEFAULT");
    if !has_limit {
        ctx.report(node, "missingLimit");
    }
}
