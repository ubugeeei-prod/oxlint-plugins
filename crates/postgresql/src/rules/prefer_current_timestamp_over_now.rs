//! Port of `prefer-current-timestamp-over-now`: prefer the SQL-standard
//! `CURRENT_TIMESTAMP` / `CURRENT_TIME` over PostgreSQL's `now()` and the
//! timezone-naive `LOCALTIMESTAMP` / `LOCALTIME`. Token-driven with autofix.
//! Runs once per file via the `usesParseError` entry point.

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let tokens = tokenize(ctx.source).tokens;
    let n = tokens.len();
    for i in 0..n {
        let tok = &tokens[i];
        // `now()` — three-token sequence Identifier "(" ")".
        if matches!(tok.kind, TokenKind::Identifier)
            && tok.value.eq_ignore_ascii_case("now")
            && i + 2 < n
        {
            let open = &tokens[i + 1];
            let close = &tokens[i + 2];
            if open.value == "(" && close.value == ")" {
                let loc = DiagnosticLoc {
                    start_line: tok.start_pos.line,
                    start_column: tok.start_pos.column,
                    end_line: close.end_pos.line,
                    end_column: close.end_pos.column,
                };
                let fix = DiagnosticFix {
                    start: tok.start,
                    end: close.end,
                    replacement: CompactString::from("CURRENT_TIMESTAMP"),
                };
                ctx.report_loc(loc, "preferCurrentTimestamp", SmallVec::new(), Some(fix));
            }
            continue;
        }
        // `LOCALTIMESTAMP` / `LOCALTIME` — bareword keyword tokens.
        if !matches!(tok.kind, TokenKind::Keyword) {
            continue;
        }
        let upper = tok.value.to_ascii_uppercase();
        let loc = DiagnosticLoc {
            start_line: tok.start_pos.line,
            start_column: tok.start_pos.column,
            end_line: tok.end_pos.line,
            end_column: tok.end_pos.column,
        };
        if upper == "LOCALTIMESTAMP" {
            let fix = DiagnosticFix {
                start: tok.start,
                end: tok.end,
                replacement: CompactString::from("CURRENT_TIMESTAMP"),
            };
            ctx.report_loc(
                loc,
                "preferCurrentTimestampOverLocal",
                SmallVec::new(),
                Some(fix),
            );
        } else if upper == "LOCALTIME" {
            let fix = DiagnosticFix {
                start: tok.start,
                end: tok.end,
                replacement: CompactString::from("CURRENT_TIME"),
            };
            ctx.report_loc(
                loc,
                "preferCurrentTimeOverLocal",
                SmallVec::new(),
                Some(fix),
            );
        }
    }
}
