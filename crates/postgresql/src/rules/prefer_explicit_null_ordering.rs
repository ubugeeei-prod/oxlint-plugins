//! Port of `prefer-explicit-null-ordering`: when `ORDER BY` specifies an
//! explicit direction (`ASC`/`DESC`/`USING`), require an explicit
//! `NULLS FIRST` / `NULLS LAST`. A `SortBy` with a non-default direction and a
//! default nulls ordering is reported.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SortBy") {
        return;
    }
    match str_field(node, "sortby_dir") {
        Some("SORTBY_ASC" | "SORTBY_DESC" | "SORTBY_USING") => {}
        _ => return,
    }
    if str_field(node, "sortby_nulls") != Some("SORTBY_NULLS_DEFAULT") {
        return;
    }
    ctx.report(node, "preferExplicitNullOrdering");
}
