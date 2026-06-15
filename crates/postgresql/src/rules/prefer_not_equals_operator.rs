//! Port of `prefer-not-equals-operator`: enforce a single style for the
//! not-equal operator (`<>` or `!=`). Configurable via `{ operator: "<>" }`
//! (default) or `{ operator: "!=" }`. Produces an autofix.
//!
//! Operates on the token stream (not the AST) because the operator is a
//! lexeme; the rule is invoked once with `node = Value::Null` (the
//! `uses_parse_error` trigger) and returns early for every real AST node.

use serde_json::Value;

use crate::tokenize::TokenKind;
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only execute on the one-time program-level trigger (Value::Null).
    if !node.is_null() {
        return;
    }

    // Determine the target operator from options (default: "<>").
    let target = ctx
        .options
        .get(0)
        .and_then(|o| o.get("operator"))
        .and_then(Value::as_str)
        .unwrap_or("<>");
    let wrong = if target == "<>" { "!=" } else { "<>" };
    let message_id: &'static str = if target == "<>" {
        "preferAngle"
    } else {
        "preferBang"
    };

    let tokens = ctx.tokens;
    for token in tokens {
        if token.kind != TokenKind::Operator {
            continue;
        }
        if token.value != wrong {
            continue;
        }
        let loc = DiagnosticLoc {
            start_line: token.start_pos.line,
            start_column: token.start_pos.column,
            end_line: token.end_pos.line,
            end_column: token.end_pos.column,
        };
        let fix = DiagnosticFix {
            start: token.start,
            end: token.end,
            replacement: CompactString::from(target),
        };
        ctx.report_loc(loc, message_id, SmallVec::new(), Some(fix));
    }
}
