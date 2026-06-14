//! Port of `no-vacuum-full`: disallow `VACUUM FULL` (takes ACCESS EXCLUSIVE and
//! rewrites the whole table). A plain `VACUUM` (no `FULL`) is fine.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "VacuumStmt") {
        return;
    }
    let Some(options) = array_field(node, "options") else {
        return;
    };
    let has_full = options
        .iter()
        .any(|option| is_type(option, "DefElem") && str_field(option, "defname") == Some("full"));
    if has_full {
        ctx.report(node, "noVacuumFull");
    }
}
