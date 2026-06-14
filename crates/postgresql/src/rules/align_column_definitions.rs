//! Port of `align-column-definitions`: vertically align `name type constraints`
//! across the single-line `ColumnDef` rows of a `CREATE TABLE`.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "Layout rule that reconstructs and pads source text for column alignment, working with serde_json's owned String/Vec at the rule/source-text boundary."
)]

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::ast::{array_field, is_type};
use crate::text::Source;
use crate::tokenize::{Token, tokenize};
use crate::{DiagnosticFix, DiagnosticLoc, RuleContext};

const DEFAULT_GAP: usize = 2;

struct Slot {
    name: String,
    type_text: String,
    constraints_text: String,
    rewrite_start: u32,
    rewrite_end: u32,
}

fn range_of(node: &Value) -> Option<(u32, u32)> {
    let arr = node.get("range")?.as_array()?;
    let a = arr.first()?.as_u64()? as u32;
    let b = arr.get(1)?.as_u64()? as u32;
    Some((a, b))
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

fn collapse_ws(s: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_owned()
}

fn consume_array_suffix(source: &Source, start: u32) -> u32 {
    let mut p = start;
    loop {
        if source.ascii_at(p) == Some(b'[') {
            let mut q = p + 1;
            while source.ascii_at(q).is_some_and(|c| c.is_ascii_digit()) {
                q += 1;
            }
            if source.ascii_at(q) == Some(b']') {
                p = q + 1;
                continue;
            }
        }
        break;
    }
    p
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateStmt") {
        return;
    }
    let gap = gap_option(ctx.options, DEFAULT_GAP);
    let Some(elts) = array_field(node, "tableElts") else {
        return;
    };
    if elts.is_empty() {
        return;
    }

    let mut slots: Vec<Slot> = Vec::new();
    let mut seen_lines: Vec<u32> = Vec::new();
    let mut tokens: Option<Vec<Token>> = None;

    for elt in elts {
        if !is_type(elt, "ColumnDef") {
            continue;
        }
        let Some(col_range) = range_of(elt) else {
            return;
        };
        let type_name = elt.get("typeName");
        let Some(base_type_range) = type_name.and_then(range_of) else {
            return;
        };
        let mut type_end = base_type_range.1;

        if let Some(last_typmod_range) = type_name
            .and_then(|t| t.get("typmods"))
            .and_then(Value::as_array)
            .and_then(|typmods| typmods.last())
            .and_then(range_of)
        {
            if tokens.is_none() {
                tokens = Some(tokenize(ctx.source).tokens);
            }
            if let Some(toks) = &tokens
                && let Some(close) = toks
                    .iter()
                    .find(|t| t.start >= last_typmod_range.1 && t.value == ")")
            {
                type_end = close.end;
            }
        }

        if type_name
            .and_then(|t| t.get("arrayBounds"))
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
        {
            type_end = consume_array_suffix(ctx.source, type_end);
        }

        let type_range = (base_type_range.0, type_end);

        let constraints: Vec<&Value> = elt
            .get("constraints")
            .and_then(Value::as_array)
            .map(|cs| cs.iter().filter(|c| is_type(c, "Constraint")).collect())
            .unwrap_or_default();
        let last_constraint_range = constraints.last().and_then(|c| range_of(c));
        let rewrite_end = last_constraint_range.map_or(type_range.1, |r| r.1);

        let start_loc = ctx.source.position(col_range.0);
        let end_loc = ctx.source.position(rewrite_end);
        if start_loc.line != end_loc.line {
            return;
        }
        if seen_lines.contains(&start_loc.line) {
            return;
        }
        seen_lines.push(start_loc.line);

        let rewrite_span = ctx.source.slice(col_range.0, rewrite_end);
        if rewrite_span.contains("--") || rewrite_span.contains("/*") {
            return;
        }

        let type_text = ctx.source.slice(type_range.0, type_range.1);
        let constraints_text = match last_constraint_range {
            Some(r) => collapse_ws(&ctx.source.slice(type_range.1, r.1)),
            None => String::new(),
        };
        let name = elt
            .get("colname")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();

        slots.push(Slot {
            name,
            type_text,
            constraints_text,
            rewrite_start: col_range.0,
            rewrite_end,
        });
    }

    if slots.len() < 2 {
        return;
    }

    let max_name = slots
        .iter()
        .map(|s| s.name.chars().count())
        .max()
        .unwrap_or(0);
    let max_type = slots
        .iter()
        .map(|s| s.type_text.chars().count())
        .max()
        .unwrap_or(0);
    let gap_spaces = " ".repeat(gap);

    let mut rewrites: Vec<(u32, u32, String)> = Vec::new();
    for slot in &slots {
        let name_part = pad_end(&slot.name, max_name);
        let mut expected = String::new();
        expected.push_str(&name_part);
        expected.push_str(&gap_spaces);
        if slot.constraints_text.is_empty() {
            expected.push_str(&slot.type_text);
        } else {
            expected.push_str(&pad_end(&slot.type_text, max_type));
            expected.push_str(&gap_spaces);
            expected.push_str(&slot.constraints_text);
        }
        let current = ctx.source.slice(slot.rewrite_start, slot.rewrite_end);
        if current == expected {
            continue;
        }
        rewrites.push((slot.rewrite_start, slot.rewrite_end, expected));
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
