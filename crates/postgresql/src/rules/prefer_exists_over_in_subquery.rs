//! Port of `prefer-exists-over-in-subquery`: prefer `EXISTS (...)` over
//! `IN (subquery)` because `IN` returns NULL when the subquery has any NULL row,
//! silently turning rows into no-matches; `EXISTS` is unambiguously boolean.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SubLink") {
        return;
    }
    if str_field(node, "subLinkType") == Some("ANY_SUBLINK") {
        ctx.report(node, "preferExists");
    }
}
