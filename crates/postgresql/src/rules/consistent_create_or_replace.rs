//! Port of `consistent-create-or-replace`: enforce a consistent stance on
//! `CREATE OR REPLACE` for `FUNCTION` / `PROCEDURE` / `VIEW`.
//!
//! Default style `"always"` requires `CREATE OR REPLACE`.
//! Style `"never"` forbids `OR REPLACE`.
//!
//! Reports at the `CREATE` keyword token's location. Upstream resolves the
//! CREATE keyword via a per-file cursor over the token stream (because the
//! PostgreSQL parser's `stmt_location` for statements after the first points
//! to the preceding newline, not the C of CREATE). We replicate that faithfully:
//! for each visited node we scan forward from `node.range[0]` in the token
//! stream to find the first `CREATE` keyword at or after that offset.
//!
//! No autofix (upstream deliberately omits one: adding/removing `OR REPLACE`
//! changes runtime semantics).

use serde_json::Value;

use crate::ast::is_type;
use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Return the UTF-16 start offset stored in `node.range[0]`, or 0 if absent.
fn node_range_start(node: &Value) -> u32 {
    node.get("range")
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(Value::as_u64)
        .map(|n| n as u32)
        .unwrap_or(0)
}

/// Locate the `DiagnosticLoc` of the `CREATE` keyword that begins the
/// statement containing `node`, by scanning the token stream from
/// `node.range[0]` forward. Returns `None` if no such token is found.
fn find_create_loc(node: &Value, ctx: &RuleContext) -> Option<DiagnosticLoc> {
    let start_off = node_range_start(node);
    let tokenized = tokenize(ctx.source);
    let tok = tokenized.tokens.iter().find(|t| {
        t.start >= start_off
            && t.kind == TokenKind::Keyword
            && t.value.eq_ignore_ascii_case("CREATE")
    })?;
    Some(DiagnosticLoc {
        start_line: tok.start_pos.line,
        start_column: tok.start_pos.column,
        end_line: tok.end_pos.line,
        end_column: tok.end_pos.column,
    })
}

fn visit(node: &Value, has_or_replace: bool, kind: &str, style: &str, ctx: &mut RuleContext) {
    let Some(loc) = find_create_loc(node, ctx) else {
        return;
    };
    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("kind"),
        value: CompactString::from(kind),
    });
    if style == "always" && !has_or_replace {
        ctx.report_loc(loc, "preferOrReplace", data, None);
    } else if style == "never" && has_or_replace {
        ctx.report_loc(loc, "unexpectedOrReplace", data, None);
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    let style = ctx
        .options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always");

    if is_type(node, "CreateFunctionStmt") {
        let has_or_replace = node
            .get("replace")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        // `is_procedure === true` means it's a PROCEDURE, not a FUNCTION.
        let kind = if node
            .get("is_procedure")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "PROCEDURE"
        } else {
            "FUNCTION"
        };
        visit(node, has_or_replace, kind, style, ctx);
    } else if is_type(node, "ViewStmt") {
        let has_or_replace = node
            .get("replace")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        visit(node, has_or_replace, "VIEW", style, ctx);
    }
}
