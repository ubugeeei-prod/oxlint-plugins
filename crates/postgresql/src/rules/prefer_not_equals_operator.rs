//! Port of `prefer-not-equals-operator`: enforce a single spelling for the
//! not-equal operator (`<>` or `!=`). Token-driven with autofix. Runs once per
//! file via the `usesParseError` entry point.

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::tokenize::tokenize;
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let target_is_angle = ctx
        .options
        .get(0)
        .and_then(|o| o.get("operator"))
        .and_then(Value::as_str)
        .unwrap_or("<>")
        == "<>";
    let (wrong, target, message_id) = if target_is_angle {
        ("!=", "<>", "preferAngle")
    } else {
        ("<>", "!=", "preferBang")
    };

    let tokens = tokenize(ctx.source).tokens;
    for tok in &tokens {
        // The lexer emits `<>` / `!=` only as Operator tokens (string contents
        // are wrapped in quotes, comments are not tokens), so matching on the
        // literal value is sufficient.
        if tok.value != wrong {
            continue;
        }
        let loc = DiagnosticLoc {
            start_line: tok.start_pos.line,
            start_column: tok.start_pos.column,
            end_line: tok.end_pos.line,
            end_column: tok.end_pos.column,
        };
        let fix = DiagnosticFix {
            start: tok.start,
            end: tok.end,
            replacement: CompactString::from(target),
        };
        ctx.report_loc(loc, message_id, SmallVec::new(), Some(fix));
    }
}
