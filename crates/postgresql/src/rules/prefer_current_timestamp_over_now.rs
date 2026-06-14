//! Port of `prefer-current-timestamp-over-now`: prefer SQL-standard
//! `CURRENT_TIMESTAMP` / `CURRENT_TIME` over `now()` and the timezone-naive
//! `LOCALTIMESTAMP` / `LOCALTIME`. Produces autofixes.
//!
//! Operates on the token stream (not the AST) because the patterns are
//! lexical; the rule is invoked once with `node = Value::Null` (the
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

    let tokens = ctx.tokens;

    for (i, token) in tokens.iter().enumerate() {
        // `now()` — three-token sequence: Identifier "now", "(", ")".
        if token.kind == TokenKind::Identifier && token.value.to_lowercase() == "now" {
            if i + 2 < tokens.len() {
                let open = &tokens[i + 1];
                let close = &tokens[i + 2];
                if open.value == "(" && close.value == ")" {
                    let loc = DiagnosticLoc {
                        start_line: token.start_pos.line,
                        start_column: token.start_pos.column,
                        end_line: close.end_pos.line,
                        end_column: close.end_pos.column,
                    };
                    let fix = DiagnosticFix {
                        start: token.start,
                        end: close.end,
                        replacement: CompactString::from("CURRENT_TIMESTAMP"),
                    };
                    ctx.report_loc(loc, "preferCurrentTimestamp", SmallVec::new(), Some(fix));
                }
            }
            // Skip keyword check for this token (matches upstream `continue`).
            continue;
        }

        if token.kind != TokenKind::Keyword {
            continue;
        }

        let upper = token.value.to_uppercase();
        if upper == "LOCALTIMESTAMP" {
            let loc = DiagnosticLoc {
                start_line: token.start_pos.line,
                start_column: token.start_pos.column,
                end_line: token.end_pos.line,
                end_column: token.end_pos.column,
            };
            let fix = DiagnosticFix {
                start: token.start,
                end: token.end,
                replacement: CompactString::from("CURRENT_TIMESTAMP"),
            };
            ctx.report_loc(
                loc,
                "preferCurrentTimestampOverLocal",
                SmallVec::new(),
                Some(fix),
            );
        } else if upper == "LOCALTIME" {
            let loc = DiagnosticLoc {
                start_line: token.start_pos.line,
                start_column: token.start_pos.column,
                end_line: token.end_pos.line,
                end_column: token.end_pos.column,
            };
            let fix = DiagnosticFix {
                start: token.start,
                end: token.end,
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
