//! Port of `consistent-create-or-replace`: enforce a consistent stance on
//! `CREATE OR REPLACE` for `FUNCTION` / `PROCEDURE` / `VIEW` (always require it,
//! or always forbid it). No autofix: toggling `OR REPLACE` changes runtime
//! semantics, so the linter must not do it.
//!
//! Upstream walks `CreateFunctionStmt` / `ViewStmt` nodes and matches each to a
//! `CREATE` token via a per-file cursor (the parser can emit `[0, 0]` ranges).
//! This port scans the token stream directly: for every `CREATE` keyword it
//! reads the (optional) `OR REPLACE` and the object kind keyword, which yields
//! the same reports and the same `CREATE`-keyword report location. It runs once
//! per file via the `usesParseError` entry point.

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};

fn style(options: &Value) -> &str {
    options
        .get(0)
        .and_then(|o| o.get("style"))
        .and_then(Value::as_str)
        .unwrap_or("always")
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Program-level token scan: act only on the single parse-error entry call.
    if !node.is_null() {
        return;
    }
    let opt = style(ctx.options);
    let always = opt == "always";
    let never = opt == "never";

    let tokens = tokenize(ctx.source).tokens;
    let n = tokens.len();
    for i in 0..n {
        let create = &tokens[i];
        if !matches!(create.kind, TokenKind::Keyword)
            || !create.value.eq_ignore_ascii_case("CREATE")
        {
            continue;
        }
        let mut j = i + 1;
        let mut has_or_replace = false;
        if j + 1 < n
            && matches!(tokens[j].kind, TokenKind::Keyword)
            && tokens[j].value.eq_ignore_ascii_case("OR")
            && tokens[j + 1].value.eq_ignore_ascii_case("REPLACE")
        {
            has_or_replace = true;
            j += 2;
        }
        let Some(kind_tok) = tokens.get(j) else {
            continue;
        };
        let kind = if kind_tok.value.eq_ignore_ascii_case("FUNCTION") {
            "FUNCTION"
        } else if kind_tok.value.eq_ignore_ascii_case("PROCEDURE") {
            "PROCEDURE"
        } else if kind_tok.value.eq_ignore_ascii_case("VIEW") {
            "VIEW"
        } else {
            continue;
        };

        let message_id = if always && !has_or_replace {
            "preferOrReplace"
        } else if never && has_or_replace {
            "unexpectedOrReplace"
        } else {
            continue;
        };
        let loc = DiagnosticLoc {
            start_line: create.start_pos.line,
            start_column: create.start_pos.column,
            end_line: create.end_pos.line,
            end_column: create.end_pos.column,
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("kind"),
            value: CompactString::from(kind),
        });
        ctx.report_loc(loc, message_id, data, None);
    }
}
