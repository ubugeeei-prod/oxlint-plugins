//! Port of `require-trailing-semicolon`: the last token in the file must be a
//! semicolon. Produces an autofix that inserts `;` immediately after the last
//! token. Operates on the token stream (not the AST).

use serde_json::Value;

use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only execute on the one-time program-level trigger (Value::Null).
    if !node.is_null() {
        return;
    }

    let tokens = ctx.tokens;

    let last = match tokens.last() {
        Some(t) => t,
        None => return,
    };

    if last.value == ";" {
        return;
    }

    let loc = DiagnosticLoc {
        start_line: last.start_pos.line,
        start_column: last.start_pos.column,
        end_line: last.end_pos.line,
        end_column: last.end_pos.column,
    };
    // Insert `;` right after the last token (zero-length replacement = insert).
    let fix = DiagnosticFix {
        start: last.end,
        end: last.end,
        replacement: CompactString::from(";"),
    };
    ctx.report_loc(loc, "missingSemicolon", SmallVec::new(), Some(fix));
}
