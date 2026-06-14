//! Port of `no-identifier-too-long`: flag identifiers longer than PostgreSQL's
//! `NAMEDATALEN - 1` byte limit (default 63). libpg_query silently truncates
//! over-length names at parse time, so the rule walks the token stream once at
//! program level (invoked with the null node via `uses_parse_error`), where the
//! raw user text survives. Byte length is measured in UTF-8 to match the
//! server's fixed-width `name` column.
#![allow(
    clippy::disallowed_methods,
    reason = "the byte count and configured limit are formatted into diagnostic interpolation data — a presentation boundary, not rule hot-state"
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};

const DEFAULT_MAX: u64 = 63;

/// `"a""b"` is the SQL escape for the identifier `a"b`; PostgreSQL truncates the
/// post-unescape form, so that is what we measure.
fn unquote_identifier(value: &str) -> CompactString {
    CompactString::from(value[1..value.len() - 1].replace("\"\"", "\"").as_str())
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let max = ctx
        .options
        .as_array()
        .and_then(|opts| opts.first())
        .and_then(|opt| opt.get("max"))
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_MAX);

    for token in tokenize(ctx.source).tokens {
        let name: CompactString = match token.kind {
            TokenKind::Identifier => CompactString::from(token.value.as_str()),
            TokenKind::String => {
                let value = token.value.as_str();
                if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                    unquote_identifier(value)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        let length = name.len();
        if length as u64 <= max {
            continue;
        }
        let loc = DiagnosticLoc {
            start_line: token.start_pos.line,
            start_column: token.start_pos.column,
            end_line: token.end_pos.line,
            end_column: token.end_pos.column,
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("name"),
            value: name,
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("length"),
            value: CompactString::from(length.to_string().as_str()),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("max"),
            value: CompactString::from(max.to_string().as_str()),
        });
        ctx.report_loc(loc, "identifierTooLong", data, None);
    }
}
