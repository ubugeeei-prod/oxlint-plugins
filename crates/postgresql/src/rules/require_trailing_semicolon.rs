//! Port of `require-trailing-semicolon`: require a trailing `;` at the end of
//! the SQL file. Operates at file level on the token stream (invoked with the
//! null node via `uses_parse_error`): the last source token must be `;`. The
//! autofix replaces any trailing whitespace after the last token with `;`, so a
//! file ending in `\n` is fixed to end in `;` (matching upstream, whose harness
//! trims the source before linting).

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::tokenize::tokenize;
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let tokens = tokenize(ctx.source).tokens;
    let Some(last) = tokens.last() else {
        return;
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
    let fix = DiagnosticFix {
        start: last.end,
        end: ctx.source.len(),
        replacement: CompactString::from(";"),
    };
    ctx.report_loc(loc, "missingSemicolon", SmallVec::new(), Some(fix));
}
