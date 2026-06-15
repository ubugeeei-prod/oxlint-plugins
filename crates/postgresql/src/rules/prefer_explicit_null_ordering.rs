//! Port of `prefer-explicit-null-ordering`: when `ORDER BY` specifies an
//! explicit direction, require an explicit `NULLS FIRST` / `NULLS LAST` so
//! null ordering is not implicit.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SortBy") {
        return;
    }
    let dir = str_field(node, "sortby_dir");
    if !matches!(
        dir,
        Some("SORTBY_ASC") | Some("SORTBY_DESC") | Some("SORTBY_USING")
    ) {
        return;
    }
    if str_field(node, "sortby_nulls") != Some("SORTBY_NULLS_DEFAULT") {
        return;
    }
    ctx.report(node, "preferExplicitNullOrdering");
}
