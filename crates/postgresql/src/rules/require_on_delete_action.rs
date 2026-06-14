//! Port of `require-on-delete-action`: require an explicit `ON DELETE` clause
//! on every foreign-key constraint.

use serde_json::Value;

use crate::ast::{is_type, str_field};
use crate::tokenize::{TokenKind, tokenize};
use crate::{DiagnosticDatum, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// Walk forward from `start_index` tracking paren depth.
/// Returns the index of the first `)` at depth 0, or `,`/`;` at depth 0.
/// Returns `tokens.len()` if none found.
fn find_fk_clause_end(tokens: &[crate::tokenize::Token], start_index: usize) -> usize {
    let mut depth: i32 = 0;
    for (offset, t) in tokens[start_index..].iter().enumerate() {
        if t.value == ")" && depth == 0 {
            return start_index + offset;
        } else if t.value == "(" {
            depth += 1;
        } else if t.value == ")" {
            depth -= 1;
        } else if depth == 0 && (t.value == "," || t.value == ";") {
            return start_index + offset;
        }
    }
    tokens.len()
}

/// Find the index of the "ON" keyword followed immediately by "DELETE".
fn find_on_delete(tokens: &[crate::tokenize::Token], start: usize, end: usize) -> Option<usize> {
    if end < 2 {
        return None;
    }
    for i in start..end.saturating_sub(1) {
        if i + 1 >= tokens.len() {
            break;
        }
        let a = &tokens[i];
        let b = &tokens[i + 1];
        if a.kind == TokenKind::Keyword
            && a.value.eq_ignore_ascii_case("ON")
            && b.kind == TokenKind::Keyword
            && b.value.eq_ignore_ascii_case("DELETE")
        {
            return Some(i);
        }
    }
    None
}

struct ActionResult {
    action: &'static str,
    from: usize,
    to: usize,
}

fn read_action(
    tokens: &[crate::tokenize::Token],
    on_idx: usize,
    end: usize,
) -> Option<ActionResult> {
    let first_idx = on_idx + 2;
    if first_idx >= end || first_idx >= tokens.len() {
        return None;
    }
    let a = &tokens[first_idx].value;
    let b = tokens
        .get(on_idx + 3)
        .map(|t| t.value.as_str())
        .unwrap_or("");
    if a.eq_ignore_ascii_case("CASCADE") {
        Some(ActionResult {
            action: "CASCADE",
            from: first_idx,
            to: first_idx,
        })
    } else if a.eq_ignore_ascii_case("RESTRICT") {
        Some(ActionResult {
            action: "RESTRICT",
            from: first_idx,
            to: first_idx,
        })
    } else if a.eq_ignore_ascii_case("NO") && b.eq_ignore_ascii_case("ACTION") {
        Some(ActionResult {
            action: "NO ACTION",
            from: first_idx,
            to: first_idx + 1,
        })
    } else if a.eq_ignore_ascii_case("SET") && b.eq_ignore_ascii_case("NULL") {
        Some(ActionResult {
            action: "SET NULL",
            from: first_idx,
            to: first_idx + 1,
        })
    } else if a.eq_ignore_ascii_case("SET") && b.eq_ignore_ascii_case("DEFAULT") {
        Some(ActionResult {
            action: "SET DEFAULT",
            from: first_idx,
            to: first_idx + 1,
        })
    } else {
        None
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "Constraint") {
        return;
    }
    if str_field(node, "contype") != Some("CONSTR_FOREIGN") {
        return;
    }

    // Get the constraint's UTF-16 start offset from node.range[0].
    let constraint_start = match node.get("range").and_then(Value::as_array) {
        Some(r) if !r.is_empty() => r[0].as_u64().unwrap_or(0) as u32,
        _ => return,
    };

    let tokenized = tokenize(ctx.source);
    let tokens = &tokenized.tokens;

    // Find the first token at or after the constraint start.
    let Some(start_idx) = tokens.iter().position(|t| t.start >= constraint_start) else {
        return;
    };

    let end_idx = find_fk_clause_end(tokens, start_idx);

    // Get option: allowed actions list (if configured).
    let allowed: Option<SmallVec<[&str; 8]>> = ctx
        .options
        .get(0)
        .and_then(|o| o.get("allowed"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect());

    let on_idx_opt = find_on_delete(tokens, start_idx, end_idx);

    if on_idx_opt.is_none() {
        // Report missingOnDelete across the whole FK clause.
        let end_tok_idx = if end_idx > start_idx {
            end_idx - 1
        } else {
            start_idx
        };
        let start_tok = &tokens[start_idx];
        let end_tok = tokens.get(end_tok_idx).unwrap_or(start_tok);
        let loc = DiagnosticLoc {
            start_line: start_tok.start_pos.line,
            start_column: start_tok.start_pos.column,
            end_line: end_tok.end_pos.line,
            end_column: end_tok.end_pos.column,
        };
        ctx.report_loc(loc, "missingOnDelete", SmallVec::new(), None);
        return;
    }

    let Some(on_idx) = on_idx_opt else { return };

    // If no allowed list is configured, any explicit action is accepted.
    let Some(allowed_list) = allowed else {
        return;
    };

    let Some(act) = read_action(tokens, on_idx, end_idx) else {
        return;
    };

    if allowed_list.contains(&act.action) {
        return;
    }

    let from_tok = &tokens[act.from];
    let to_tok = &tokens[act.to];
    let loc = DiagnosticLoc {
        start_line: from_tok.start_pos.line,
        start_column: from_tok.start_pos.column,
        end_line: to_tok.end_pos.line,
        end_column: to_tok.end_pos.column,
    };

    // Build allowed list string without Vec/String.
    let mut allowed_str = CompactString::new("");
    for (i, s) in allowed_list.iter().enumerate() {
        if i > 0 {
            allowed_str.push_str(", ");
        }
        allowed_str.push_str(s);
    }

    let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
    data.push(DiagnosticDatum {
        key: CompactString::from("action"),
        value: CompactString::from(act.action),
    });
    data.push(DiagnosticDatum {
        key: CompactString::from("allowedList"),
        value: allowed_str,
    });
    ctx.report_loc(loc, "disallowedAction", data, None);
}
