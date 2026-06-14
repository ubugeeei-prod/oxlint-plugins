//! Port of `align-values`: vertically align tuple positions across the rows of
//! a multi-row `INSERT ... VALUES (...)`.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "Layout rule that reconstructs and pads source text for VALUES alignment, working with serde_json's owned String/Vec at the rule/source-text boundary."
)]
#![allow(clippy::needless_range_loop)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::is_type;
use crate::text::Source;
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

const DEFAULT_GAP: usize = 1;

struct Row {
    item_texts: Vec<String>,
    rewrite_start: u32,
    rewrite_end: u32,
}

fn gap_option(options: &Value, default: usize) -> usize {
    options
        .get(0)
        .and_then(|o| o.get("gap"))
        .and_then(Value::as_u64)
        .map_or(default, |g| g as usize)
}

fn pad_end(s: &str, width: usize) -> String {
    let len = s.chars().count();
    let mut out = s.to_owned();
    if len < width {
        out.push_str(&" ".repeat(width - len));
    }
    out
}

fn visit_range(node: &Value, min: &mut Option<u32>, max: &mut Option<u32>) {
    match node {
        Value::Object(map) => {
            if let Some(arr) = map.get("range").and_then(Value::as_array)
                && let (Some(a), Some(b)) = (
                    arr.first().and_then(Value::as_u64),
                    arr.get(1).and_then(Value::as_u64),
                )
                && a != 0
            {
                let a = a as u32;
                let b = b as u32;
                if min.is_none_or(|m| a < m) {
                    *min = Some(a);
                }
                if max.is_none_or(|m| b > m) {
                    *max = Some(b);
                }
            }
            for (k, v) in map {
                if !matches!(k.as_str(), "parent" | "range" | "loc") {
                    visit_range(v, min, max);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                visit_range(item, min, max);
            }
        }
        _ => {}
    }
}

fn full_source_range(node: &Value) -> Option<(u32, u32)> {
    let mut min = None;
    let mut max = None;
    visit_range(node, &mut min, &mut max);
    match (min, max) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
}

fn line_text(source: &Source, offset: u32) -> String {
    let len = source.len();
    let clamped = offset.min(len);
    let mut s = clamped;
    while s > 0 && source.ascii_at(s - 1) != Some(b'\n') {
        s -= 1;
    }
    let mut e = clamped;
    while e < len && source.ascii_at(e) != Some(b'\n') {
        e += 1;
    }
    source.slice(s, e)
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "InsertStmt") {
        return;
    }
    let Some(select) = node.get("selectStmt") else {
        return;
    };
    let Some(lists) = select.get("valuesLists").and_then(Value::as_array) else {
        return;
    };
    if lists.len() < 2 {
        return;
    }
    let gap = gap_option(ctx.options, DEFAULT_GAP);

    let mut rows: Vec<Row> = Vec::new();
    let mut column_count: Option<usize> = None;
    for list in lists {
        let Some(items) = list.get("items").and_then(Value::as_array) else {
            return;
        };
        if items.is_empty() {
            return;
        }
        match column_count {
            None => column_count = Some(items.len()),
            Some(c) => {
                if items.len() != c {
                    return;
                }
            }
        }
        let mut ranges: Vec<(u32, u32)> = Vec::new();
        for it in items {
            let Some(r) = full_source_range(it) else {
                return;
            };
            ranges.push(r);
        }
        let first = ranges[0];
        let last = ranges[ranges.len() - 1];
        let start_loc = ctx.source.position(first.0);
        let end_loc = ctx.source.position(last.1);
        if start_loc.line != end_loc.line {
            return;
        }
        let line = line_text(ctx.source, first.0);
        if line.contains("--") || line.contains("/*") {
            return;
        }
        let item_texts: Vec<String> = ranges
            .iter()
            .map(|(s, e)| ctx.source.slice(*s, *e))
            .collect();
        rows.push(Row {
            item_texts,
            rewrite_start: first.0,
            rewrite_end: last.1,
        });
    }
    let Some(column_count) = column_count else {
        return;
    };
    if rows.len() < 2 {
        return;
    }

    let mut widths: Vec<usize> = vec![0; column_count];
    for row in &rows {
        for i in 0..column_count {
            let w = row.item_texts[i].chars().count();
            if w > widths[i] {
                widths[i] = w;
            }
        }
    }

    let mut rewrites: Vec<(u32, u32, String)> = Vec::new();
    for row in &rows {
        let mut expected = String::new();
        for i in 0..column_count {
            let text = &row.item_texts[i];
            if i == column_count - 1 {
                expected.push_str(text);
            } else {
                let pad_to = widths[i] + 1 + gap;
                let mut seg = String::new();
                seg.push_str(text);
                seg.push(',');
                expected.push_str(&pad_end(&seg, pad_to));
            }
        }
        let current = ctx.source.slice(row.rewrite_start, row.rewrite_end);
        if current == expected {
            continue;
        }
        rewrites.push((row.rewrite_start, row.rewrite_end, expected));
    }
    if rewrites.is_empty() {
        return;
    }

    let first = &rewrites[0];
    let sp = ctx.source.position(first.0);
    let ep = ctx.source.position(first.1);
    let loc = DiagnosticLoc {
        start_line: sp.line,
        start_column: sp.column,
        end_line: ep.line,
        end_column: ep.column,
    };

    let min_start = rewrites[0].0;
    let max_end = rewrites[rewrites.len() - 1].1;
    let mut replacement = String::new();
    let mut cursor = min_start;
    for (s, e, r) in &rewrites {
        replacement.push_str(&ctx.source.slice(cursor, *s));
        replacement.push_str(r);
        cursor = *e;
    }
    replacement.push_str(&ctx.source.slice(cursor, max_end));

    let fix = Some(DiagnosticFix {
        start: min_start,
        end: max_end,
        replacement: CompactString::from(replacement.as_str()),
    });
    ctx.report_loc(loc, "misaligned", SmallVec::new(), fix);
}
