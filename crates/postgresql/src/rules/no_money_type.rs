//! Port of `no-money-type`: disallow the `money` column type — its output
//! format and precision depend on `lc_monetary`, so the same row renders
//! differently across servers. Reports the `ColumnDef` whose type is `money`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{field, is_type, type_name};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "ColumnDef") && type_name(field(node, "typeName")) == Some("money") {
        ctx.report(node, "noMoney");
    }
}
