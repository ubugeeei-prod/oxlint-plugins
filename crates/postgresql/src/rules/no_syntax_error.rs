//! Port of `no-syntax-error`: disallow PostgreSQL syntax errors.
//!
//! Upstream's parser turns an unparseable file into a single `SQLParseError`
//! node spanning the whole program and reports it with the parser's error
//! message. This port mirrors that: it runs once (the `uses_parse_error`
//! trigger) and, when [`crate::parse::parse`] captured an error, reports it at
//! the program span with the libpg_query message interpolated.

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only act on the one-time program-level trigger.
    if !node.is_null() {
        return;
    }
    let Some(error) = ctx.error else {
        return;
    };

    // The SQLParseError node covers the whole program: `[0, source.len()]`.
    let start = ctx.source.position(0);
    let end = ctx.source.position(ctx.source.len());
    let loc = DiagnosticLoc {
        start_line: start.line,
        start_column: start.column,
        end_line: end.line,
        end_column: end.column,
    };

    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("message"),
        value: CompactString::from(error.message.as_str()),
    });
    ctx.report_loc(loc, "syntaxError", data, None);
}
