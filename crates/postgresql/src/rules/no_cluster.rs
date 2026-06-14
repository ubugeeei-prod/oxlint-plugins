//! Port of `no-cluster`: disallow the `CLUSTER` statement (takes ACCESS
//! EXCLUSIVE, rewrites the table, and is not maintained afterwards).

use serde_json::Value;

use crate::RuleContext;
use crate::ast::is_type;

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if is_type(node, "ClusterStmt") {
        ctx.report(node, "noCluster");
    }
}
