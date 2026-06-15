//! Port of `prefer-keyword-case`: enforce a consistent case (upper or lower)
//! for SQL keywords.
//!
//! This rule operates on the token stream, visiting it once per file (triggered
//! by `uses_parse_error = true` with `node = Value::Null`). It collects AST
//! positions that are identifier contexts — column names, table names, type
//! names, constraint column lists — and exempts keyword tokens that fall in
//! those positions from case-folding.

use serde_json::Value;

use oxlint_plugins_carton::{CompactString, FastHashSet, SmallVec};

use crate::tokenize::TokenKind;
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

// Node types whose `range[0]` is a general identifier (only the first token
// of the node's span is the identifier — the rest of the span may be keywords).
const GENERAL_IDENTIFIER_START_TYPES: &[&str] = &["ColumnDef", "Constraint"];

// Node types whose entire `range` is a general identifier span (every token
// inside is an identifier, not a keyword).
const GENERAL_IDENTIFIER_RANGE_TYPES: &[&str] = &["ColumnRef", "RangeVar"];

// Node types whose `range[0]` is a type-name position.  Tokens at these
// positions are controlled by the `types` option rather than the general
// `case` option.
const TYPE_NAME_NODE_TYPES: &[&str] = &["names", "TypeName"];

struct ScopedName {
    name: CompactString,
    range: (u32, u32),
}

struct Positions {
    general_identifier_starts: FastHashSet<u32>,
    general_identifier_ranges: SmallVec<[(u32, u32); 16]>,
    scoped_identifier_names: SmallVec<[ScopedName; 8]>,
    type_name_starts: FastHashSet<u32>,
}

impl Positions {
    fn new() -> Self {
        Self {
            general_identifier_starts: FastHashSet::default(),
            general_identifier_ranges: SmallVec::new(),
            scoped_identifier_names: SmallVec::new(),
            type_name_starts: FastHashSet::default(),
        }
    }
}

fn get_range(node: &Value) -> Option<(u32, u32)> {
    let arr = node.get("range")?.as_array()?;
    let start = arr.first()?.as_u64()? as u32;
    let end = arr.get(1)?.as_u64()? as u32;
    Some((start, end))
}

fn add_scoped_names_from_array(
    arr: Option<&Value>,
    stmt_range: (u32, u32),
    positions: &mut Positions,
) {
    let Some(arr) = arr.and_then(Value::as_array) else {
        return;
    };
    for item in arr {
        if item.get("type").and_then(Value::as_str) == Some("String")
            && let Some(sval) = item.get("sval").and_then(Value::as_str)
        {
            positions.scoped_identifier_names.push(ScopedName {
                name: CompactString::from(sval),
                range: stmt_range,
            });
        }
    }
}

/// Walk the JSON AST, collecting identifier-position data that the main loop
/// uses to exempt keyword tokens from case-folding.
fn collect_positions(stmts: &[Value], positions: &mut Positions) {
    for stmt in stmts {
        // Each statement is a direct child of the implicit Program node; pass
        // `parent_type = "Program"` and `stmt_range = None` so the statement's
        // own range becomes the scoping range for nested identifier lookups.
        visit_node(stmt, Some("Program"), None, positions);
    }
}

fn visit_node(
    node: &Value,
    parent_type: Option<&str>,
    stmt_range: Option<(u32, u32)>,
    positions: &mut Positions,
) {
    match node {
        Value::Array(items) => {
            for item in items {
                visit_node(item, parent_type, stmt_range, positions);
            }
        }
        Value::Object(map) => {
            let type_str = map.get("type").and_then(Value::as_str);
            let mut next_stmt_range = stmt_range;

            if let Some(t) = type_str
                && let Some((start, end)) = get_range(node)
            {
                let scoped = (start, end);

                // The first typed node that is a direct child of the
                // implicit Program becomes the scoping range for nested
                // identifier name lookups (Constraint.keys, IndexElem.name).
                if stmt_range.is_none() && parent_type == Some("Program") {
                    next_stmt_range = Some(scoped);
                }

                if GENERAL_IDENTIFIER_START_TYPES.contains(&t) {
                    positions.general_identifier_starts.insert(start);
                    // Constraint.keys / pk_attrs / fk_attrs hold column
                    // names as String nodes. Collect them as scoped names
                    // so the second pass can resolve their token positions.
                    if t == "Constraint"
                        && let Some(sr) = stmt_range
                    {
                        add_scoped_names_from_array(map.get("keys"), sr, positions);
                        add_scoped_names_from_array(map.get("pk_attrs"), sr, positions);
                        add_scoped_names_from_array(map.get("fk_attrs"), sr, positions);
                    }
                } else if GENERAL_IDENTIFIER_RANGE_TYPES.contains(&t) {
                    positions.general_identifier_ranges.push(scoped);
                } else if TYPE_NAME_NODE_TYPES.contains(&t) {
                    positions.type_name_starts.insert(start);
                } else if t == "String" && parent_type != Some("A_Const") {
                    // A bare `String` node (not inside an A_Const literal)
                    // is an identifier reference.  Its range covers exactly
                    // the identifier span — exempt every token inside it.
                    positions.general_identifier_ranges.push(scoped);
                } else if t == "ResTarget" {
                    // ResTarget without a `val` is an INSERT/UPDATE column
                    // list entry; the range covers the identifier token.
                    let has_val = map.get("val").is_some_and(|v| !v.is_null());
                    if !has_val {
                        positions.general_identifier_ranges.push(scoped);
                    }
                } else if t == "IndexElem"
                    && let Some(name) = map.get("name").and_then(Value::as_str)
                    && let Some(sr) = stmt_range
                {
                    // IndexElem.name is a plain string property, not a
                    // String node, so it has no per-name range.  Record
                    // the name scoped to the enclosing statement range so
                    // the token-scan pass can resolve the position.
                    positions.scoped_identifier_names.push(ScopedName {
                        name: CompactString::from(name),
                        range: sr,
                    });
                }
            }

            let current_type = type_str.or(parent_type);

            for (key, value) in map {
                if matches!(key.as_str(), "parent" | "range" | "loc") {
                    continue;
                }
                if value.is_object() || value.is_array() {
                    visit_node(value, current_type, next_stmt_range, positions);
                }
            }
        }
        _ => {}
    }
}

/// `position >= s && end <= e` — the token is fully contained within the range.
fn is_in_range(position: u32, end: u32, ranges: &[(u32, u32)]) -> bool {
    ranges.iter().any(|&(s, e)| position >= s && end <= e)
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    // This rule operates on the whole token stream; it is invoked once via
    // `uses_parse_error = true` (with node = Null).  Every per-node call is
    // a no-op.
    if !node.is_null() {
        return;
    }

    let option = ctx.options.get(0);
    let target = option
        .and_then(|o| o.get("case"))
        .and_then(Value::as_str)
        .unwrap_or("upper");
    let types_mode = option
        .and_then(|o| o.get("types"))
        .and_then(Value::as_str)
        .unwrap_or("skip");

    // `transformType = null` in upstream when types_mode == "skip".
    let apply_types = types_mode != "skip";

    let tokens = ctx.tokens;

    // Phase 1: collect AST position exemptions.
    let mut positions = Positions::new();
    collect_positions(ctx.statements, &mut positions);

    // Phase 2: resolve scoped identifier names into concrete token positions.
    // For each (name, statement-range) entry, scan Keyword tokens inside the
    // statement range whose lowercase value matches the name.
    let mut scoped_identifier_token_starts: FastHashSet<u32> = FastHashSet::default();
    if !positions.scoped_identifier_names.is_empty() {
        for token in tokens {
            if token.kind != TokenKind::Keyword {
                continue;
            }
            let tok_lower = token.value.to_ascii_lowercase();
            for entry in &positions.scoped_identifier_names {
                if entry.name.as_str() != tok_lower {
                    continue;
                }
                let (s, e) = entry.range;
                if token.start >= s && token.end <= e {
                    scoped_identifier_token_starts.insert(token.start);
                    break;
                }
            }
        }
    }

    // Phase 3: build the field-access guard.  A Keyword token immediately
    // after a single `.` Punctuator is a dotted field reference (`kv.key`,
    // `NEW.role`), NOT a keyword — exempt it from case-folding.
    let mut field_access_token_starts: FastHashSet<u32> = FastHashSet::default();
    for i in 1..tokens.len() {
        let token = &tokens[i];
        if token.kind != TokenKind::Keyword {
            continue;
        }
        let prev = &tokens[i - 1];
        if prev.kind == TokenKind::Punctuator && prev.value == "." {
            // Guard against `..`: only single-dot counts.
            let prev_prev_is_dot =
                i >= 2 && tokens[i - 2].kind == TokenKind::Punctuator && tokens[i - 2].value == ".";
            if !prev_prev_is_dot {
                field_access_token_starts.insert(token.start);
            }
        }
    }

    // Phase 4: main token loop — report mis-cased Keyword tokens.
    for token in tokens {
        if token.kind != TokenKind::Keyword {
            continue;
        }
        if positions.general_identifier_starts.contains(&token.start) {
            continue;
        }
        if is_in_range(token.start, token.end, &positions.general_identifier_ranges) {
            continue;
        }
        if scoped_identifier_token_starts.contains(&token.start) {
            continue;
        }
        if field_access_token_starts.contains(&token.start) {
            continue;
        }

        let (desired, message_id): (CompactString, &'static str) =
            if positions.type_name_starts.contains(&token.start) {
                // Type-name positions: only case-fold when the user opted in.
                if !apply_types {
                    continue;
                }
                let d = if types_mode == "upper" {
                    CompactString::from(token.value.to_ascii_uppercase().as_str())
                } else {
                    CompactString::from(token.value.to_ascii_lowercase().as_str())
                };
                let mid = if types_mode == "upper" {
                    "expectedUpper"
                } else {
                    "expectedLower"
                };
                (d, mid)
            } else {
                let d = if target == "upper" {
                    CompactString::from(token.value.to_ascii_uppercase().as_str())
                } else {
                    CompactString::from(token.value.to_ascii_lowercase().as_str())
                };
                let mid = if target == "upper" {
                    "expectedUpper"
                } else {
                    "expectedLower"
                };
                (d, mid)
            };

        if token.value.as_str() == desired.as_str() {
            continue;
        }

        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("actual"),
            value: CompactString::from(token.value.as_str()),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("expected"),
            value: desired.clone(),
        });

        let loc = DiagnosticLoc {
            start_line: token.start_pos.line,
            start_column: token.start_pos.column,
            end_line: token.end_pos.line,
            end_column: token.end_pos.column,
        };
        let fix = DiagnosticFix {
            start: token.start,
            end: token.end,
            replacement: desired,
        };
        ctx.report_loc(loc, message_id, data, Some(fix));
    }
}
