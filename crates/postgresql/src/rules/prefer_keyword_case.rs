//! Port of `prefer-keyword-case`: enforce a consistent case for SQL keyword
//! tokens, exempting identifier-positioned tokens (column/table/constraint
//! names, type names by default, dotted field access, INSERT column lists,
//! IndexElem / Constraint key columns).
//!
//! This is a program-level rule: the scan engine walks per-statement nodes and
//! does not expose the token stream or a `Program` node to rules, so when
//! invoked once with `node == null` (uses_parse_error) it re-derives both by
//! calling `crate::parse::parse` on the reconstructed source. Tokens exist even
//! on a parse error (matching upstream `sourceCode.ast.tokens`); the AST-derived
//! exemptions simply collapse to nothing then, as upstream.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "Program-level keyword-case rule: reconstructs source text and re-derives the AST/token stream, mirroring upstream's JS source-text manipulation over serde_json's owned String/Vec."
)]
#![allow(clippy::if_same_then_else)]

use oxlint_plugins_carton::{CompactString, FastHashSet, SmallVec};
use serde_json::{Value, json};

use crate::tokenize::TokenKind;
use crate::{DiagnosticDatum, DiagnosticFix, DiagnosticLoc, RuleContext};

const START_TYPES: [&str; 2] = ["ColumnDef", "Constraint"];
const RANGE_TYPES: [&str; 2] = ["ColumnRef", "RangeVar"];
const TYPE_NAME_TYPES: [&str; 2] = ["names", "TypeName"];

struct Positions {
    start_ids: FastHashSet<u32>,
    range_ids: Vec<(u32, u32)>,
    scoped: Vec<(String, (u32, u32))>,
    type_starts: FastHashSet<u32>,
}

fn range_of(map: &serde_json::Map<String, Value>) -> Option<(u32, u32)> {
    let arr = map.get("range")?.as_array()?;
    let a = arr.first()?.as_u64()? as u32;
    let b = arr.get(1)?.as_u64()? as u32;
    Some((a, b))
}

fn collect(
    node: &Value,
    parent_type: Option<&str>,
    stmt_range: Option<(u32, u32)>,
    pos: &mut Positions,
) {
    if let Value::Array(items) = node {
        for item in items {
            collect(item, parent_type, stmt_range, pos);
        }
        return;
    }
    let Value::Object(map) = node else {
        return;
    };
    let ty = map.get("type").and_then(Value::as_str);
    let mut next_stmt = stmt_range;
    if let Some(t) = ty
        && let Some((s, e)) = range_of(map)
    {
        let scoped = (s, e);
        if stmt_range.is_none() && parent_type == Some("Program") {
            next_stmt = Some(scoped);
        }
        if START_TYPES.contains(&t) {
            pos.start_ids.insert(s);
        } else if RANGE_TYPES.contains(&t) {
            pos.range_ids.push(scoped);
        } else if TYPE_NAME_TYPES.contains(&t) {
            pos.type_starts.insert(s);
        } else if t == "String" && parent_type != Some("A_Const") {
            pos.range_ids.push(scoped);
        } else if t == "ResTarget" && map.get("val").is_none_or(Value::is_null) {
            pos.range_ids.push(scoped);
        } else if t == "IndexElem" {
            if let (Some(name), Some(sr)) = (map.get("name").and_then(Value::as_str), stmt_range) {
                pos.scoped.push((name.to_owned(), sr));
            }
        } else if t == "Constraint"
            && let Some(sr) = stmt_range
        {
            for key in ["keys", "pk_attrs", "fk_attrs"] {
                if let Some(arr) = map.get(key).and_then(Value::as_array) {
                    for item in arr {
                        if item.get("type").and_then(Value::as_str) == Some("String")
                            && let Some(sv) = item.get("sval").and_then(Value::as_str)
                        {
                            pos.scoped.push((sv.to_owned(), sr));
                        }
                    }
                }
            }
        }
    }
    let current = ty.or(parent_type);
    for (k, v) in map {
        if !matches!(k.as_str(), "parent" | "range" | "loc") {
            collect(v, current, next_stmt, pos);
        }
    }
}

fn in_range(s: u32, e: u32, ranges: &[(u32, u32)]) -> bool {
    ranges.iter().any(|&(a, b)| s >= a && e <= b)
}

fn transform(value: &str, upper: bool) -> String {
    if upper {
        value.to_uppercase()
    } else {
        value.to_lowercase()
    }
}

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !node.is_null() {
        return;
    }
    let case_upper = !matches!(
        ctx.options
            .get(0)
            .and_then(|o| o.get("case"))
            .and_then(Value::as_str),
        Some("lower")
    );
    let types_upper: Option<bool> = match ctx
        .options
        .get(0)
        .and_then(|o| o.get("types"))
        .and_then(Value::as_str)
    {
        Some("upper") => Some(true),
        Some("lower") => Some(false),
        _ => None,
    };

    let src = ctx.source.slice(0, ctx.source.len());
    let len = ctx.source.len();
    let crate::parse::Parsed {
        tokens, statements, ..
    } = crate::parse::parse(&src);
    let program = json!({ "type": "Program", "range": [0, len], "body": statements });

    let mut pos = Positions {
        start_ids: FastHashSet::default(),
        range_ids: Vec::new(),
        scoped: Vec::new(),
        type_starts: FastHashSet::default(),
    };
    collect(&program, Some("Program"), None, &mut pos);

    let mut scoped_starts: FastHashSet<u32> = FastHashSet::default();
    if !pos.scoped.is_empty() {
        for token in &tokens {
            if token.kind != TokenKind::Keyword {
                continue;
            }
            let tv = token.value.to_lowercase();
            for (name, range) in &pos.scoped {
                if *name == tv && token.start >= range.0 && token.end <= range.1 {
                    scoped_starts.insert(token.start);
                    break;
                }
            }
        }
    }

    let mut field_starts: FastHashSet<u32> = FastHashSet::default();
    for i in 1..tokens.len() {
        let token = &tokens[i];
        if token.kind != TokenKind::Keyword {
            continue;
        }
        let prev = &tokens[i - 1];
        if prev.kind == TokenKind::Punctuator
            && prev.value == "."
            && (i < 2 || tokens[i - 2].kind != TokenKind::Punctuator || tokens[i - 2].value != ".")
        {
            field_starts.insert(token.start);
        }
    }

    for token in &tokens {
        if token.kind != TokenKind::Keyword {
            continue;
        }
        if pos.start_ids.contains(&token.start) {
            continue;
        }
        if in_range(token.start, token.end, &pos.range_ids) {
            continue;
        }
        if scoped_starts.contains(&token.start) {
            continue;
        }
        if field_starts.contains(&token.start) {
            continue;
        }

        let (desired, is_upper_msg) = if pos.type_starts.contains(&token.start) {
            match types_upper {
                None => continue,
                Some(up) => (transform(&token.value, up), up),
            }
        } else {
            (transform(&token.value, case_upper), case_upper)
        };
        if token.value == desired {
            continue;
        }
        let loc = DiagnosticLoc {
            start_line: token.start_pos.line,
            start_column: token.start_pos.column,
            end_line: token.end_pos.line,
            end_column: token.end_pos.column,
        };
        let mut data: SmallVec<[DiagnosticDatum; 2]> = SmallVec::new();
        data.push(DiagnosticDatum {
            key: CompactString::from("actual"),
            value: CompactString::from(token.value.as_str()),
        });
        data.push(DiagnosticDatum {
            key: CompactString::from("expected"),
            value: CompactString::from(desired.as_str()),
        });
        let message_id = if is_upper_msg {
            "expectedUpper"
        } else {
            "expectedLower"
        };
        let fix = Some(DiagnosticFix {
            start: token.start,
            end: token.end,
            replacement: CompactString::from(desired.as_str()),
        });
        ctx.report_loc(loc, message_id, data, fix);
    }
}
