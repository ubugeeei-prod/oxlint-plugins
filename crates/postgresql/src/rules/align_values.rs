//! Port of `align-values`: align column values vertically inside multi-row
//! `INSERT ... VALUES (...)`. Produces an autofix.
//!
//! The rule visits `InsertStmt` nodes whose `selectStmt.valuesLists` contains
//! at least two rows. Each row must be single-line and comment-free. When rows
//! are not consistently padded to the widest value per column, one error is
//! reported at the first misaligned row with a single combined fix that rewrites
//! every misaligned row.

use serde_json::Value;

use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};
use oxlint_plugins_carton::{CompactString, SmallVec};

const DEFAULT_GAP: usize = 1;

/// Walk every descendant collecting `range` values; return `[min_start, max_end]`.
/// Skips ranges where start == 0 (libpg_query "no location" placeholder).
/// Mirrors upstream `getFullSourceRange`.
fn get_full_source_range(node: &Value) -> Option<(u32, u32)> {
    let mut min = u32::MAX;
    let mut max = 0u32;
    let mut found = false;
    walk_range(node, &mut min, &mut max, &mut found);
    if found { Some((min, max)) } else { None }
}

fn walk_range(node: &Value, min: &mut u32, max: &mut u32, found: &mut bool) {
    match node {
        Value::Object(map) => {
            if let Some(range) = map.get("range").and_then(Value::as_array)
                && range.len() == 2
            {
                let s = range[0].as_u64().unwrap_or(0) as u32;
                let e = range[1].as_u64().unwrap_or(0) as u32;
                if s != 0 {
                    *found = true;
                    if s < *min {
                        *min = s;
                    }
                    if e > *max {
                        *max = e;
                    }
                }
            }
            for (k, v) in map {
                if matches!(k.as_str(), "parent" | "range" | "loc") {
                    continue;
                }
                walk_range(v, min, max, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_range(item, min, max, found);
            }
        }
        _ => {}
    }
}

/// Return true if the source line containing `offset` has an inline comment
/// marker (`--` or `/*`). Mirrors the upstream `lineText.includes("--") ||
/// lineText.includes("/*")` check.
fn line_has_comment(source: &crate::text::Source, offset: u32) -> bool {
    let mut line_start = offset;
    while line_start > 0 && source.ascii_at(line_start - 1) != Some(b'\n') {
        line_start -= 1;
    }
    let src_len = source.len();
    let mut line_end = offset;
    while line_end < src_len && source.ascii_at(line_end) != Some(b'\n') {
        line_end += 1;
    }
    let line_text = source.slice(line_start, line_end);
    line_text.contains("--") || line_text.contains("/*")
}

/// Per-row data gathered during the first pass.
struct RowInfo {
    /// UTF-16 [start, end) ranges of each item in the row.
    item_ranges: SmallVec<[(u32, u32); 8]>,
    /// Start of the first item — the rewrite span's inclusive lower bound.
    rewrite_start: u32,
    /// End of the last item — the rewrite span's exclusive upper bound.
    rewrite_end: u32,
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if node.get("type").and_then(Value::as_str) != Some("InsertStmt") {
        return;
    }

    let gap = ctx
        .options
        .get(0)
        .and_then(|o| o.get("gap"))
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_GAP as u64) as usize;

    let select_stmt = match node.get("selectStmt") {
        Some(s) => s,
        None => return,
    };
    let values_lists = match select_stmt.get("valuesLists").and_then(Value::as_array) {
        Some(l) if l.len() >= 2 => l,
        _ => return,
    };

    let mut rows: SmallVec<[RowInfo; 8]> = SmallVec::new();
    let mut column_count: Option<usize> = None;

    for list in values_lists {
        let items = match list.get("items").and_then(Value::as_array) {
            Some(i) if !i.is_empty() => i,
            _ => return,
        };
        let col_count = items.len();
        match column_count {
            None => column_count = Some(col_count),
            Some(c) if c != col_count => return,
            _ => {}
        }

        let mut item_ranges: SmallVec<[(u32, u32); 8]> = SmallVec::new();
        for item in items {
            match get_full_source_range(item) {
                Some(r) => item_ranges.push(r),
                None => return,
            }
        }

        let first_range = item_ranges[0];
        let last_range = match item_ranges.last() {
            Some(&r) => r,
            None => return,
        };

        // All items in a row must lie on the same source line.
        let first_pos = ctx.source.position(first_range.0);
        let last_pos = ctx.source.position(last_range.1);
        if first_pos.line != last_pos.line {
            return;
        }

        // Inline comments would be clobbered by a per-row rewrite.
        if line_has_comment(ctx.source, first_range.0) {
            return;
        }

        rows.push(RowInfo {
            item_ranges,
            rewrite_start: first_range.0,
            rewrite_end: last_range.1,
        });
    }

    let col_count = match column_count {
        Some(c) if rows.len() >= 2 => c,
        _ => return,
    };

    // Per-column maximum width in UTF-16 units (range end − start).
    let mut widths: SmallVec<[usize; 8]> = SmallVec::new();
    for _ in 0..col_count {
        widths.push(0usize);
    }
    for row in &rows {
        for (i, &(s, e)) in row.item_ranges.iter().enumerate() {
            let w = (e - s) as usize;
            if w > widths[i] {
                widths[i] = w;
            }
        }
    }

    // Build the expected aligned text for every row; track which ones differ.
    let mut row_expected: SmallVec<[Option<CompactString>; 8]> = SmallVec::new();
    let mut first_misaligned: Option<usize> = None;
    let mut last_misaligned: Option<usize> = None;

    for (row_idx, row) in rows.iter().enumerate() {
        let mut expected = CompactString::default();
        for (i, &(s, e)) in row.item_ranges.iter().enumerate() {
            let item_text = ctx.source.slice(s, e);
            if i == col_count - 1 {
                // Last column: emit the value with no trailing padding.
                expected.push_str(item_text.as_str());
            } else {
                // Non-last column: `"text," padEnd(widths[i] + 1 + gap)`.
                // widths[i] + 1 accounts for the comma; + gap is the spacing.
                let pad_to = widths[i] + 1 + gap;
                let item_len = (e - s) as usize; // UTF-16 units
                let with_comma = item_len + 1; // item + ','
                expected.push_str(item_text.as_str());
                expected.push(',');
                for _ in 0..pad_to.saturating_sub(with_comma) {
                    expected.push(' ');
                }
            }
        }

        let current = ctx.source.slice(row.rewrite_start, row.rewrite_end);
        if current == expected.as_str() {
            row_expected.push(None);
        } else {
            if first_misaligned.is_none() {
                first_misaligned = Some(row_idx);
            }
            last_misaligned = Some(row_idx);
            row_expected.push(Some(expected));
        }
    }

    let (first_idx, last_idx) = match (first_misaligned, last_misaligned) {
        (Some(f), Some(l)) => (f, l),
        _ => return,
    };

    // Build one combined fix spanning from the first to the last misaligned row.
    // Source text between row rewrite-spans (commas, newlines, leading spaces,
    // opening parens) is preserved verbatim; only misaligned row bodies change.
    let combined_start = rows[first_idx].rewrite_start;
    let combined_end = rows[last_idx].rewrite_end;
    let mut replacement = CompactString::default();
    let mut cursor = combined_start;

    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx < first_idx || row_idx > last_idx {
            continue;
        }
        if cursor < row.rewrite_start {
            let between = ctx.source.slice(cursor, row.rewrite_start);
            replacement.push_str(between.as_str());
        }
        match &row_expected[row_idx] {
            Some(exp) => replacement.push_str(exp.as_str()),
            None => {
                let orig = ctx.source.slice(row.rewrite_start, row.rewrite_end);
                replacement.push_str(orig.as_str());
            }
        }
        cursor = row.rewrite_end;
    }

    // Report at the first misaligned row's own rewrite span.
    let first_row = &rows[first_idx];
    let start_pos = ctx.source.position(first_row.rewrite_start);
    let end_pos = ctx.source.position(first_row.rewrite_end);
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
