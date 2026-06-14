//! Port of `require-if-exists`: require `IF EXISTS` on every `DROP` statement
//! so re-running a migration on a database that already lost the object does
//! not error.
//!
//! Visits `DropStmt`, `DropdbStmt`, `DropRoleStmt`, and `DropSubscriptionStmt`
//! AST nodes. If the node already carries `missing_ok: true` (i.e. `IF EXISTS`
//! is present in the SQL), the rule is silent. Otherwise it locates the `DROP`
//! keyword token within the node's own range (to avoid false positives from
//! `ALTER TABLE … DROP COLUMN`) and reports the span from `DROP` to the
//! following keyword (`TABLE`, `DATABASE`, …). No autofix — inserting
//! `IF EXISTS` changes runtime semantics and must be a deliberate author
//! decision.

use serde_json::Value;

use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::SmallVec;

const DROP_NODE_TYPES: [&str; 4] = [
    "DropStmt",
    "DropdbStmt",
    "DropRoleStmt",
    "DropSubscriptionStmt",
];

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only fire on the four DROP-statement AST node types.
    let node_type = match node.get("type").and_then(Value::as_str) {
        Some(t) => t,
        None => return,
    };
    if !DROP_NODE_TYPES.contains(&node_type) {
        return;
    }

    // `IF EXISTS` is present when the parser sets `missing_ok: true`.
    if node
        .get("missing_ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return;
    }

    // Get the UTF-16 range that bounds this statement.
    let range = match node.get("range").and_then(Value::as_array) {
        Some(r) => r,
        None => return,
    };
    let node_start = match range.first().and_then(Value::as_u64) {
        Some(v) => v as u32,
        None => return,
    };
    let node_end = match range.get(1).and_then(Value::as_u64) {
        Some(v) => v as u32,
        None => return,
    };
    if node_end <= node_start {
        return;
    }

    // Find the first `DROP` keyword token within the node's range.
    // Constraining the search to the node range avoids false attribution to
    // `ALTER TABLE … DROP COLUMN` / `ALTER TABLE … DROP CONSTRAINT` when
    // those precede a standalone `DROP` statement.
    let tokenized = tokenize(ctx.source);
    let tokens = &tokenized.tokens;

    let drop_idx = match tokens.iter().position(|tok| {
        tok.kind == TokenKind::Keyword
            && tok.value.eq_ignore_ascii_case("DROP")
            && tok.start >= node_start
            && tok.start < node_end
    }) {
        Some(i) => i,
        None => return,
    };

    let next_idx = drop_idx + 1;
    if next_idx >= tokens.len() {
        return;
    }

    let drop_tok = &tokens[drop_idx];
    let kind_tok = &tokens[next_idx];

    // Guard: the next token must still be within the node's range.
    if kind_tok.end > node_end {
        return;
    }

    let loc = DiagnosticLoc {
        start_line: drop_tok.start_pos.line,
        start_column: drop_tok.start_pos.column,
        end_line: kind_tok.end_pos.line,
        end_column: kind_tok.end_pos.column,
    };
    ctx.report_loc(loc, "missingIfExists", SmallVec::new(), None);
}
