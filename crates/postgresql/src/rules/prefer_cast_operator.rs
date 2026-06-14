//! Port of `prefer-cast-operator`: enforce a consistent style for PostgreSQL
//! type casts — either the `::` operator form or the `CAST(... AS ...)`
//! function form.  Produces autofixes in both directions.
//!
//! The rule tokenizes the source once in the program-exit trigger (null node)
//! and then walks the full statement tree to find TypeCast nodes.

use serde_json::Value;

use crate::tokenize::{Token, TokenKind, tokenize};
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

/// A fully resolved cast fix ready to emit.
struct TypeCastDiag {
    loc: DiagnosticLoc,
    message_id: &'static str,
    fix_start: u32,
    fix_end: u32,
    replacement: CompactString,
}

/// Return the exclusive end offset of the type expression that begins at
/// `tokens[start_idx]`.  The type may be:
///   - a bare keyword / identifier (e.g. `integer`, `text`)
///   - a schema-qualified name (e.g. `public.my_type`)
///   - a name with typmod parens (e.g. `varchar(255)`, `numeric(10, 2)`)
fn find_type_end(tokens: &[Token], start_idx: usize) -> u32 {
    let first = &tokens[start_idx];
    if !first.is_identifier_like() {
        return first.end;
    }
    let mut type_end = first.end;
    let mut i = start_idx + 1;
    // Optional dot-identifier chain (e.g. `public.my_type`).
    while i + 1 < tokens.len() && tokens[i].value == "." && tokens[i + 1].is_identifier_like() {
        type_end = tokens[i + 1].end;
        i += 2;
    }
    // Optional typmod paren group (e.g. `(255)` in `varchar(255)`).
    if i < tokens.len() && tokens[i].value == "(" {
        let mut depth: i32 = 1;
        i += 1;
        while i < tokens.len() && depth > 0 {
            if tokens[i].value == "(" {
                depth += 1;
            } else if tokens[i].value == ")" {
                depth -= 1;
                if depth == 0 {
                    type_end = tokens[i].end;
                }
            }
            i += 1;
        }
    }
    type_end
}

/// Recursively collect every `TypeCast` node in the JSON tree.
fn collect_type_casts<'a>(node: &'a Value, out: &mut SmallVec<[&'a Value; 16]>) {
    if node
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|t| t == "TypeCast")
    {
        out.push(node);
    }
    match node {
        Value::Object(map) => {
            for (key, val) in map {
                if !matches!(key.as_str(), "parent" | "type" | "range" | "loc") {
                    collect_type_casts(val, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_type_casts(item, out);
            }
        }
        _ => {}
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // Only execute on the one-time program-level trigger (Value::Null).
    if !node.is_null() {
        return;
    }

    // Copy long-lived references out of ctx so we can call ctx.report_loc later.
    let stmts = ctx.statements;
    let options = ctx.options;
    let source = ctx.source;

    let form = options
        .get(0)
        .and_then(|o| o.get("form"))
        .and_then(Value::as_str)
        .unwrap_or("operator");
    let target_operator = form != "function";

    let tokenized = tokenize(source);
    let tokens = &tokenized.tokens;

    // Collect all TypeCast nodes across all statements.
    let mut type_casts: SmallVec<[&Value; 16]> = SmallVec::new();
    for stmt in stmts {
        collect_type_casts(stmt, &mut type_casts);
    }

    // Process each TypeCast and accumulate diagnostics.
    let mut diags: SmallVec<[TypeCastDiag; 8]> = SmallVec::new();

    for tc_node in &type_casts {
        let tc_range_start = match tc_node.get("range").and_then(Value::as_array) {
            Some(r) if !r.is_empty() => r[0].as_u64().unwrap_or(0) as u32,
            _ => continue,
        };
        let arg = match tc_node.get("arg") {
            Some(a) => a,
            None => continue,
        };
        let arg_range = match arg.get("range").and_then(Value::as_array) {
            Some(r) if r.len() >= 2 => (
                r[0].as_u64().unwrap_or(0) as u32,
                r[1].as_u64().unwrap_or(0) as u32,
            ),
            _ => continue,
        };
        let (arg_start, arg_end) = arg_range;

        let Some(head_idx) = tokens.iter().position(|t| t.start == tc_range_start) else {
            continue;
        };
        let head = &tokens[head_idx];

        let is_function =
            head.kind == TokenKind::Keyword && head.value.eq_ignore_ascii_case("CAST");
        let is_operator = head.kind == TokenKind::Operator && head.value == "::";

        if !is_function && !is_operator {
            continue;
        }
        // Skip if already in the target form.
        if target_operator && is_operator {
            continue;
        }
        if !target_operator && is_function {
            continue;
        }

        if is_function {
            // CAST(arg AS type) → arg::type
            // Find the opening '(' immediately after the CAST keyword.
            let Some(open_idx) = tokens[head_idx + 1..]
                .iter()
                .position(|t| t.value == "(")
                .map(|pos| head_idx + 1 + pos)
            else {
                continue;
            };

            // Scan for the `AS` keyword at depth 1 and the closing `)` at depth 0.
            let mut depth: i32 = 1;
            let mut as_idx: Option<usize> = None;
            let mut close_idx: Option<usize> = None;
            let mut i = open_idx + 1;
            while i < tokens.len() {
                if tokens[i].value == "(" {
                    depth += 1;
                } else if tokens[i].value == ")" {
                    depth -= 1;
                    if depth == 0 {
                        close_idx = Some(i);
                        break;
                    }
                } else if depth == 1
                    && tokens[i].kind == TokenKind::Keyword
                    && tokens[i].value.eq_ignore_ascii_case("AS")
                {
                    as_idx = Some(i);
                }
                i += 1;
            }
            let (Some(as_idx), Some(close_idx)) = (as_idx, close_idx) else {
                continue;
            };
            let type_start_idx = as_idx + 1;
            if type_start_idx >= tokens.len() {
                continue;
            }

            let type_end = find_type_end(tokens, type_start_idx);
            let arg_src = source.slice(arg_start, arg_end);
            let type_src = source.slice(tokens[type_start_idx].start, type_end);
            let mut replacement = CompactString::new("");
            replacement.push_str(arg_src.as_str());
            replacement.push_str("::");
            replacement.push_str(type_src.as_str());

            let fix_start = head.start;
            let fix_end = tokens[close_idx].end;
            let start_pos = source.position(fix_start);
            let end_pos = source.position(fix_end);
            diags.push(TypeCastDiag {
                loc: DiagnosticLoc {
                    start_line: start_pos.line,
                    start_column: start_pos.column,
                    end_line: end_pos.line,
                    end_column: end_pos.column,
                },
                message_id: "preferOperator",
                fix_start,
                fix_end,
                replacement,
            });
        } else {
            // arg::type → CAST(arg AS type)
            let type_start_idx = head_idx + 1;
            if type_start_idx >= tokens.len() {
                continue;
            }

            let type_end = find_type_end(tokens, type_start_idx);
            let arg_src = source.slice(arg_start, arg_end);
            let type_src = source.slice(tokens[type_start_idx].start, type_end);
            let mut replacement = CompactString::new("");
            replacement.push_str("CAST(");
            replacement.push_str(arg_src.as_str());
            replacement.push_str(" AS ");
            replacement.push_str(type_src.as_str());
            replacement.push(')');

            let fix_start = arg_start;
            let fix_end = type_end;
            let start_pos = source.position(fix_start);
            let end_pos = source.position(fix_end);
            diags.push(TypeCastDiag {
                loc: DiagnosticLoc {
                    start_line: start_pos.line,
                    start_column: start_pos.column,
                    end_line: end_pos.line,
                    end_column: end_pos.column,
                },
                message_id: "preferFunction",
                fix_start,
                fix_end,
                replacement,
            });
        }
    }

    for diag in diags {
        ctx.report_loc(
            diag.loc,
            diag.message_id,
            SmallVec::new(),
            Some(DiagnosticFix {
                start: diag.fix_start,
                end: diag.fix_end,
                replacement: diag.replacement,
            }),
        );
    }
}
