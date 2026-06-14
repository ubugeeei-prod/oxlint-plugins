//! Port of `prefer-exists-over-in-subquery`: prefer `EXISTS (...)` over
//! `... IN (subquery)` (and the equivalent `= ANY (subquery)`). Both forms parse
//! to a `SubLink` of `ANY_SUBLINK` kind, reported at the SubLink's operator span.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "SubLink") {
        return;
    }
    // ANY_SUBLINK covers both `x IN (subquery)` and `x = ANY (subquery)`.
    if str_field(node, "subLinkType") != Some("ANY_SUBLINK") {
        return;
    }
    ctx.report(node, "preferExists");
}
