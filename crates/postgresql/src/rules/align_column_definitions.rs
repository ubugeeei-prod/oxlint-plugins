//! Port of `align-column-definitions`: align name/type/constraints vertically
//! inside `CREATE TABLE`. Produces an autofix.
//!
//! The rule visits `CreateStmt` nodes, extracts one `Slot` per `ColumnDef` row,
//! then recomputes the aligned layout and rewrites every misaligned row with a
//! single combined fix.
//!
//! Key implementation notes:
//! - libpg_query constraint ranges cover only the **first keyword** of multi-word
//!   constraints ("NOT" for `NOT NULL`, "PRIMARY" for `PRIMARY KEY`). The fix
//!   replacement ends there; the remaining keywords are preserved from the
//!   original text untouched.
//! - `constraintsText` = `source[typeEnd..lastConstraint.range[1]]` with
//!   whitespace collapsed — it includes everything from the end of the type to
//!   right after the first keyword of the last constraint.
//! - When a column has **no** constraints, the type is emitted without padding;
//!   padding is applied only to slots that carry at least one constraint.

use serde_json::Value;

use crate::ast::is_type;
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, FastHashSet, SmallVec};

const DEFAULT_GAP: usize = 2;

/// Extract the `range: [start, end]` pair from a node.
fn get_range(node: &Value) -> Option<(u32, u32)> {
    let arr = node.get("range").and_then(Value::as_array)?;
    if arr.len() < 2 {
        return None;
    }
    Some((arr[0].as_u64()? as u32, arr[1].as_u64()? as u32))
}

/// Collapse consecutive whitespace characters in `s` to a single space and
/// trim leading/trailing whitespace. Mirrors JS `.replace(/\s+/g, " ").trim()`.
fn normalize_whitespace(s: &str) -> CompactString {
    let mut result = CompactString::default();
    let mut first = true;
    for part in s.split_whitespace() {
        if !first {
            result.push(' ');
        }
        result.push_str(part);
        first = false;
    }
    result
}

/// One column definition ready for alignment.
struct Slot {
    /// Column name as a string (`elt.colname`).
    name: CompactString,
    /// Name length in characters (ASCII identifiers: char count == byte count).
    name_len: usize,
    /// Source text of the type (possibly extended for `TYPE(n)` / `TYPE[]`).
    type_text: CompactString,
    /// Type text length in UTF-16 units (= `type_end - base_type_start`).
    type_len: usize,
    /// Normalised text from type-end to last-constraint-end.
    /// Empty when the column carries no constraints.
    constraints_text: CompactString,
    /// Whether this slot carries at least one constraint with a valid range.
    has_constraints: bool,
    /// UTF-16 offset of the column-name start (= `colRange[0]`).
    rewrite_start: u32,
    /// UTF-16 offset right after the first keyword of the last constraint
    /// (= `lastConstraint.range[1]`), or right after the type when there are
    /// no constraints (= `typeEnd`).
    rewrite_end: u32,
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }

    let gap = ctx
        .options
        .get(0)
        .and_then(|o| o.get("gap"))
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_GAP as u64) as usize;

    let table_elts = match node.get("tableElts").and_then(Value::as_array) {
        Some(e) if !e.is_empty() => e,
        _ => return,
    };

    let mut slots: SmallVec<[Slot; 8]> = SmallVec::new();
    let mut seen_lines: FastHashSet<u32> = FastHashSet::default();

    for elt in table_elts {
        // Table-level constraints (PRIMARY KEY (a,b), CHECK (…), etc.) are
        // kept as-is — only ColumnDef rows are realigned.
        if !is_type(elt, "ColumnDef") {
            continue;
        }

        let colname = match elt.get("colname").and_then(Value::as_str) {
            Some(n) => n,
            None => return,
        };
        let col_range = match get_range(elt) {
            Some(r) => r,
            None => return,
        };

        let type_name = match elt.get("typeName") {
            Some(t) => t,
            None => return,
        };
        let base_type_range = match get_range(type_name) {
            Some(r) => r,
            None => return,
        };
        let mut type_end = base_type_range.1;

        // Extend typeEnd past the closing `)` for parameterised types like
        // `TIMESTAMP(3)` / `NUMERIC(10,2)`. The last typmod's range ends just
        // before the `)`, so scan forward to consume it.
        if let Some(typmods) = type_name.get("typmods").and_then(Value::as_array)
            && let Some(last_typmod) = typmods.last()
            && let Some((_, typmod_end)) = get_range(last_typmod)
        {
            let src_len = ctx.source.len();
            let mut pos = typmod_end;
            while pos < src_len {
                if ctx.source.ascii_at(pos) == Some(b')') {
                    type_end = pos + 1;
                    break;
                }
                pos += 1;
            }
        }

        // Extend typeEnd past `[…]` array-dimension suffixes. The parser signals
        // presence via a non-empty `arrayBounds` list; the individual entries
        // carry no usable ranges, so consume `[\d*]` groups from source text.
        let has_array_bounds = type_name
            .get("arrayBounds")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty());
        if has_array_bounds {
            let src_len = ctx.source.len();
            let mut pos = type_end;
            loop {
                if ctx.source.ascii_at(pos) != Some(b'[') {
                    break;
                }
                let mut p = pos + 1;
                while p < src_len && ctx.source.ascii_at(p).is_some_and(|c| c.is_ascii_digit()) {
                    p += 1;
                }
                if ctx.source.ascii_at(p) == Some(b']') {
                    pos = p + 1;
                    type_end = pos;
                } else {
                    break;
                }
            }
        }

        // Find the last Constraint child with a valid range.
        let last_constraint_range: Option<(u32, u32)> = elt
            .get("constraints")
            .and_then(Value::as_array)
            .and_then(|cs| {
                cs.iter()
                    .filter(|c| is_type(c, "Constraint"))
                    .filter_map(get_range)
                    .next_back()
            });

        let rewrite_end = match last_constraint_range {
            Some((_, end)) => end,
            None => type_end,
        };
        let has_constraints = last_constraint_range.is_some();

        // The column definition must be entirely on one line.
        let start_pos = ctx.source.position(col_range.0);
        let end_pos = ctx.source.position(rewrite_end);
        if start_pos.line != end_pos.line {
            return;
        }

        // Reject if two column definitions share a line.
        if !seen_lines.insert(start_pos.line) {
            return;
        }

        // Reject if an inline comment lives inside the rewrite span — it would
        // be clobbered. Comments AFTER rewrite_end (e.g. trailing `-- …`) are
        // fine because the rule never touches that part of the line.
        let span_text = ctx.source.slice(col_range.0, rewrite_end);
        if span_text.contains("--") || span_text.contains("/*") {
            return;
        }

        // Type text (potentially extended for `(n)` / `[]`).
        let type_text = ctx.source.slice(base_type_range.0, type_end);
        let type_len = (type_end - base_type_range.0) as usize;

        // Constraint text: everything from end-of-type to end-of-last-constraint,
        // with whitespace normalised (matches upstream `.replace(/\s+/g, " ").trim()`).
        let constraints_text = if has_constraints {
            let raw = ctx.source.slice(type_end, rewrite_end);
            normalize_whitespace(raw.as_str())
        } else {
            CompactString::default()
        };

        slots.push(Slot {
            name: CompactString::from(colname),
            name_len: colname.len(), // identifiers are ASCII
            type_text: CompactString::from(type_text.as_str()),
            type_len,
            constraints_text,
            has_constraints,
            rewrite_start: col_range.0,
            rewrite_end,
        });
    }

    if slots.len() < 2 {
        return;
    }

    let max_name = slots.iter().map(|s| s.name_len).max().unwrap_or(0);
    let max_type = slots.iter().map(|s| s.type_len).max().unwrap_or(0);

    // Build expected text per slot and detect misalignment.
    let mut slot_expected: SmallVec<[Option<CompactString>; 8]> = SmallVec::new();
    let mut first_misaligned: Option<usize> = None;
    let mut last_misaligned: Option<usize> = None;

    for (idx, slot) in slots.iter().enumerate() {
        // `namePart.padEnd(maxName) + " ".repeat(gap) + typePart + …`
        let mut expected = CompactString::from(slot.name.as_str());
        for _ in 0..max_name.saturating_sub(slot.name_len) {
            expected.push(' ');
        }
        for _ in 0..gap {
            expected.push(' ');
        }
        expected.push_str(slot.type_text.as_str());
        if slot.has_constraints {
            // Pad type to maxType only when there are constraints to follow.
            for _ in 0..max_type.saturating_sub(slot.type_len) {
                expected.push(' ');
            }
            for _ in 0..gap {
                expected.push(' ');
            }
            expected.push_str(slot.constraints_text.as_str());
        }

        let current = ctx.source.slice(slot.rewrite_start, slot.rewrite_end);
        if current == expected.as_str() {
            slot_expected.push(None);
        } else {
            if first_misaligned.is_none() {
                first_misaligned = Some(idx);
            }
            last_misaligned = Some(idx);
            slot_expected.push(Some(expected));
        }
    }

    let (first_idx, last_idx) = match (first_misaligned, last_misaligned) {
        (Some(f), Some(l)) => (f, l),
        _ => return,
    };

    // Build a single combined fix spanning from the first to the last misaligned
    // slot. Source text between slot rewrite-spans (commas, newlines, indents)
    // is preserved verbatim.
    let combined_start = slots[first_idx].rewrite_start;
    let combined_end = slots[last_idx].rewrite_end;
    let mut replacement = CompactString::default();
    let mut cursor = combined_start;

    for (idx, slot) in slots.iter().enumerate() {
        if idx < first_idx || idx > last_idx {
            continue;
        }
        if cursor < slot.rewrite_start {
            let between = ctx.source.slice(cursor, slot.rewrite_start);
            replacement.push_str(between.as_str());
        }
        match &slot_expected[idx] {
            Some(exp) => replacement.push_str(exp.as_str()),
            None => {
                let orig = ctx.source.slice(slot.rewrite_start, slot.rewrite_end);
                replacement.push_str(orig.as_str());
            }
        }
        cursor = slot.rewrite_end;
    }

    // Report at the first misaligned slot's rewrite span.
    let first_slot = &slots[first_idx];
    let start_pos = ctx.source.position(first_slot.rewrite_start);
    let end_pos = ctx.source.position(first_slot.rewrite_end);
    let loc = DiagnosticLoc {
        start_line: start_pos.line,
        start_column: start_pos.column,
        end_line: end_pos.line,
        end_column: end_pos.column,
    };
    let fix = DiagnosticFix {
        start: combined_start,
        end: combined_end,
        replacement,
    };
    ctx.report_loc(loc, "misaligned", SmallVec::new(), Some(fix));
}
