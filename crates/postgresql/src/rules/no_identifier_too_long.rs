//! Port of `no-identifier-too-long`: disallow identifiers whose UTF-8 byte
//! length exceeds PostgreSQL's `NAMEDATALEN - 1` limit (default 63). Operates
//! on the token stream (not the AST) because libpg_query silently truncates
//! over-length identifiers before the AST is constructed.

use core::fmt::Write as _;
use serde_json::Value;

use crate::tokenize::TokenKind;
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, FastHashSet, SmallVec};

const DEFAULT_MAX: usize = 63;

/// Strip surrounding `"` from a quoted identifier and replace `""` with `"`.
fn unquote_identifier(value: &str) -> CompactString {
    // Caller ensures value.len() >= 2 and it starts/ends with `"`.
    let content = &value[1..value.len() - 1];
    let mut result = CompactString::new("");
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' && chars.peek() == Some(&'"') {
            chars.next(); // skip the escaped second quote
        }
        result.push(c);
    }
    result
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only execute on the one-time program-level trigger (Value::Null).
    if !node.is_null() {
        return;
    }

    let max = ctx
        .options
        .get(0)
        .and_then(|o| o.get("max"))
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(DEFAULT_MAX);

    let tokens = ctx.tokens;

    // Deduplicate by start offset (upstream uses `seen.has(range[0])`).
    let mut seen: FastHashSet<u32> = FastHashSet::default();

    for token in tokens {
        if seen.contains(&token.start) {
            continue;
        }
        seen.insert(token.start);

        let name: CompactString;
        let byte_len: usize;

        match token.kind {
            TokenKind::Identifier => {
                byte_len = token.value.len(); // UTF-8 byte count
                if byte_len <= max {
                    continue;
                }
                name = CompactString::from(token.value.as_str());
            }
            TokenKind::String => {
                // Double-quoted identifiers: `"foo"` or `"a""b"` (escaped `"`)
                if token.value.len() < 2
                    || !token.value.starts_with('"')
                    || !token.value.ends_with('"')
                {
                    continue;
                }
                name = unquote_identifier(&token.value);
                byte_len = name.len(); // CompactString::len() = UTF-8 bytes
                if byte_len <= max {
                    continue;
                }
            }
            _ => continue,
        }

        let mut length_str = CompactString::new("");
        let _ = write!(length_str, "{byte_len}");
        let mut max_str = CompactString::new("");
        let _ = write!(max_str, "{max}");

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
            value: length_str,
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("max"),
            value: max_str,
        });
        ctx.report_loc(loc, "identifierTooLong", data, None);
    }
}
