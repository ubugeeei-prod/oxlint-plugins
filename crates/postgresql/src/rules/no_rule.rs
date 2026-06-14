//! Port of `no-rule`: disallow `CREATE RULE`; PostgreSQL's rule system is a
//! known foot-gun and is effectively deprecated in favor of triggers and views.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // The runtime AST tags this node as "RuleStmt" even though the
    // upstream type alias is `CreateRuleStmt` — visit by the runtime name.
    if is_type(node, "RuleStmt") {
        ctx.report(node, "noRule");
    }
}
