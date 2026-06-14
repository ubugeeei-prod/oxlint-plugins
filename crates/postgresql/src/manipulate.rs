//! AST enrichment ported from upstream `src/manipulate.ts`.
//!
//! libpg_query's JSON wraps every node as a single-key object
//! (`{"SelectStmt": { … }}`) and gives each node only a start `location` (a
//! UTF-8 byte offset). This module rewrites that tree in place to the shape the
//! rules consume:
//!   * [`add_types`] collapses each `{Type: body}` wrapper into `body` with a
//!     `type` field, and tags the bare `Alias` node libpg_query emits unwrapped.
//!   * [`add_location`] resolves every node's `range`/`loc` bottom-up from the
//!     token stream and child spans.
//!   * The statement bounds are then overridden from libpg_query's
//!     `stmt_location`/`stmt_len`, `[0,0]` fallbacks are repaired from the
//!     nearest ancestor, and `Alias` ranges are recovered from the tokens.
//!
//! Note on a faithful simplification: upstream stores parent links and, as a
//! last resort in `addLocation`, walks them looking for an already-located
//! ancestor (`getParentLocation`). In a single post-order pass no ancestor is
//! ever located yet when a descendant reaches that step, so it always falls
//! through to the `[0,0]` placeholder that `repair_fallback_locations` fixes.
//! We therefore skip parent links entirely and emit the `[0,0]` placeholder
//! directly — same output, no back-pointers.

#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serde_json / libpg_query interop boundary: this parser layer mirrors upstream's JS object/array semantics and operates directly on serde_json's owned String/Vec. The carton-collection policy governs rule hot-path state, not this boundary."
)]

use oxlint_plugins_carton::FastHashMap;
use serde_json::{Map, Value, json};

use crate::text::{Position, Source};
use crate::tokenize::Token;

const SPECIAL_KEYS: [&str; 4] = ["parent", "type", "range", "loc"];

#[derive(Clone, Copy)]
struct Loc {
    start_off: u32,
    start: Position,
    end_off: u32,
    end: Position,
}

fn is_special(key: &str) -> bool {
    SPECIAL_KEYS.contains(&key)
}

// libpg_query emits some nodes (notably `Alias`) as bare objects with no
// single-key wrapper. Detect them by their characteristic fields and assign the
// canonical type explicitly, matching upstream `detectBareNodeType`.
fn detect_bare_node_type(node: &Map<String, Value>) -> Option<&'static str> {
    if node.get("aliasname").is_some_and(Value::is_string) {
        return Some("Alias");
    }
    None
}

/// Collapse `{Type: body}` wrappers into `body` with a `type` field.
pub fn add_types(node: &mut Value) {
    match node {
        Value::Array(items) => {
            for item in items {
                add_types(item);
            }
        }
        Value::Object(map) => {
            let has_type = map.contains_key("type");
            if !has_type {
                if let Some(bare) = detect_bare_node_type(map) {
                    map.insert("type".to_string(), Value::String(bare.to_string()));
                } else if let Some(type_key) = map
                    .iter()
                    .find(|(k, v)| !is_special(k) && v.is_object())
                    .map(|(k, _)| k.clone())
                {
                    // Inline the wrapped body up into this node.
                    if let Some(Value::Object(body)) = map.remove(&type_key) {
                        map.insert("type".to_string(), Value::String(type_key));
                        for (k, v) in body {
                            map.insert(k, v);
                        }
                    }
                }
            }
            for (k, v) in map.iter_mut() {
                if !is_special(k) && (v.is_object() || v.is_array()) {
                    add_types(v);
                }
            }
        }
        _ => {}
    }
}

fn update_min_max(
    min: &mut Option<Loc>,
    max: &mut Option<Loc>,
    new_min: Option<Loc>,
    new_max: Option<Loc>,
) {
    if let (Some(nmin), Some(nmax)) = (new_min, new_max) {
        if min.is_none_or(|m| nmin.start_off < m.start_off) {
            *min = Some(nmin);
        }
        if max.is_none_or(|m| nmax.end_off > m.end_off) {
            *max = Some(nmax);
        }
    }
}

fn position_loc(source: &Source, off: u32) -> Loc {
    let p = source.position(off);
    Loc {
        start_off: off,
        start: p,
        end_off: off,
        end: p,
    }
}

fn set_node_location(
    map: &mut Map<String, Value>,
    start: u32,
    sp: Position,
    end: u32,
    ep: Position,
) {
    map.insert("range".to_string(), json!([start, end]));
    map.insert(
        "loc".to_string(),
        json!({
            "start": { "line": sp.line, "column": sp.column },
            "end": { "line": ep.line, "column": ep.column },
        }),
    );
}

fn has_range_and_loc(map: &Map<String, Value>) -> bool {
    map.get("range").is_some_and(|v| !v.is_null()) && map.get("loc").is_some_and(|v| !v.is_null())
}

struct LocationResolver<'a> {
    start_end: &'a FastHashMap<u32, Loc>,
    source: &'a Source,
}

impl LocationResolver<'_> {
    /// Bottom-up location resolution. Returns the (min-start, max-end) spans
    /// observed in this subtree, or `None` when nothing was anchored.
    fn add_location(&self, node: &mut Value) -> (Option<Loc>, Option<Loc>) {
        let mut min: Option<Loc> = None;
        let mut max: Option<Loc> = None;

        match node {
            Value::Array(items) => {
                for item in items {
                    let (cmin, cmax) = self.add_location(item);
                    update_min_max(&mut min, &mut max, cmin, cmax);
                }
                return (min, max);
            }
            Value::Object(_) => {}
            _ => return (min, max),
        }

        // Recurse into object/array-valued properties (mirroring JS, where
        // arrays are `typeof === "object"` so they recurse too).
        if let Value::Object(map) = node {
            let keys: Vec<String> = map
                .iter()
                .filter(|(k, v)| !is_special(k) && (v.is_object() || v.is_array()))
                .map(|(k, _)| k.clone())
                .collect();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    let (cmin, cmax) = self.add_location(child);
                    update_min_max(&mut min, &mut max, cmin, cmax);
                }
            }
        }

        let map = match node {
            Value::Object(map) => map,
            _ => return (min, max),
        };

        // libpg_query uses negative `location` (usually -1) for synthetic nodes;
        // treat those as "no location". `location` is a UTF-8 byte offset.
        let raw_location = map.get("location").and_then(Value::as_i64);
        let location = raw_location
            .filter(|n| *n >= 0)
            .and_then(|n| self.source.byte_to_unit(n));
        if raw_location.is_some() {
            map.remove("location");
        }

        if let Some(loc_off) = location {
            if let Some(tok) = self.start_end.get(&loc_off) {
                let tok_loc = Loc {
                    start_off: tok.start_off,
                    start: tok.start,
                    end_off: tok.end_off,
                    end: tok.end,
                };
                set_node_location(map, tok.start_off, tok.start, tok.end_off, tok.end);
                update_min_max(&mut min, &mut max, Some(tok_loc), Some(tok_loc));
            } else {
                let p = self.source.position(loc_off);
                set_node_location(map, loc_off, p, loc_off, p);
                let point = position_loc(self.source, loc_off);
                update_min_max(&mut min, &mut max, Some(point), Some(point));
            }
        }

        if !has_range_and_loc(map)
            && let (Some(mn), Some(mx)) = (min, max)
        {
            set_node_location(map, mn.start_off, mn.start, mx.end_off, mx.end);
        }

        // A typed node still without a span resolves to the `[0,0]` placeholder
        // (see module note on why parent-walking never helps here).
        if map.contains_key("type") && !has_range_and_loc(map) {
            set_node_location(
                map,
                0,
                Position { line: 1, column: 0 },
                0,
                Position { line: 1, column: 0 },
            );
        }

        (min, max)
    }
}

fn build_start_end(tokens: &[Token]) -> FastHashMap<u32, Loc> {
    let mut map = FastHashMap::default();
    for t in tokens {
        map.insert(
            t.start,
            Loc {
                start_off: t.start,
                start: t.start_pos,
                end_off: t.end,
                end: t.end_pos,
            },
        );
    }
    map
}

fn range_of(map: &Map<String, Value>) -> Option<(u32, u32)> {
    let arr = map.get("range")?.as_array()?;
    let a = arr.first()?.as_u64()? as u32;
    let b = arr.get(1)?.as_u64()? as u32;
    Some((a, b))
}

fn is_fallback_range(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|a| a.len() == 2 && a[0] == json!(0) && a[1] == json!(0))
}

// Replace `[0,0]` placeholder ranges with the nearest located ancestor's span,
// so inline `eslint-disable` directives and reports resolve inside the
// enclosing statement instead of at line 1, column 0.
fn repair_fallback_locations(node: &mut Value, ancestor_range: (u32, u32), ancestor_loc: &Value) {
    match node {
        Value::Array(items) => {
            for item in items {
                repair_fallback_locations(item, ancestor_range, ancestor_loc);
            }
        }
        Value::Object(map) => {
            let mut inherited_range = ancestor_range;
            let mut inherited_loc = ancestor_loc.clone();

            let is_fallback = map.get("range").is_some_and(is_fallback_range);
            if is_fallback {
                map.insert(
                    "range".to_string(),
                    json!([ancestor_range.0, ancestor_range.1]),
                );
                map.insert("loc".to_string(), ancestor_loc.clone());
            } else if let (Some(r), Some(l)) = (map.get("range").cloned(), map.get("loc").cloned())
                && let Some(arr) = r.as_array()
                && l.is_object()
                && arr.len() == 2
                && let (Some(a), Some(b)) = (arr[0].as_u64(), arr[1].as_u64())
            {
                inherited_range = (a as u32, b as u32);
                inherited_loc = l;
            }

            let keys: Vec<String> = map
                .iter()
                .filter(|(k, v)| !is_special(k) && (v.is_object() || v.is_array()))
                .map(|(k, _)| k.clone())
                .collect();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    repair_fallback_locations(child, inherited_range, &inherited_loc);
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Alias range recovery (port of `resolveAliasRanges` / `resolveAliasNodeRanges`)
// ---------------------------------------------------------------------------

fn unquote_identifier(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].replace("\"\"", "\"")
    } else {
        value.to_string()
    }
}

fn first_token_at_or_after(tokens: &[Token], position: u32) -> usize {
    let mut lo = 0usize;
    let mut hi = tokens.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if tokens[mid].start >= position {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

fn set_range_and_loc(map: &mut Map<String, Value>, start: u32, end: u32, source: &Source) {
    let sp = source.position(start);
    let ep = source.position(end);
    set_node_location(map, start, sp, end, ep);
}

fn resolve_alias_node_ranges(
    alias: &mut Map<String, Value>,
    parent_range: Option<(u32, u32)>,
    tokens: &[Token],
    source: &Source,
) {
    let Some(aliasname) = alias
        .get("aliasname")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };

    let search_from = parent_range.map_or(0, |r| r.0);
    let start_idx = first_token_at_or_after(tokens, search_from);
    let target = aliasname.to_lowercase();

    let mut alias_token_idx: Option<usize> = None;
    for (i, token) in tokens.iter().enumerate().skip(start_idx) {
        if !token.is_identifier_like() {
            continue;
        }
        // Skip identifiers inside the parent's range — those are the relation /
        // function being aliased, not the alias name.
        if let Some(pr) = parent_range
            && token.end <= pr.1
        {
            continue;
        }
        if unquote_identifier(&token.value).to_lowercase() == target {
            alias_token_idx = Some(i);
            break;
        }
    }
    let Some(alias_token_idx) = alias_token_idx else {
        return;
    };

    let alias_token = &tokens[alias_token_idx];
    let mut end_range = (alias_token.start, alias_token.end);

    let colnames_len = alias
        .get("colnames")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if colnames_len > 0
        && let Some(open) = tokens.get(alias_token_idx + 1)
        && open.kind == crate::tokenize::TokenKind::Punctuator
        && open.value == "("
    {
        let mut col_segments: Vec<Vec<&Token>> = Vec::new();
        let mut current: Vec<&Token> = Vec::new();
        let mut depth = 1i32;
        let mut close_idx: Option<usize> = None;
        let mut i = alias_token_idx + 2;
        while i < tokens.len() && depth > 0 {
            let t = &tokens[i];
            if t.kind == crate::tokenize::TokenKind::Punctuator && t.value == "(" {
                depth += 1;
                current.push(t);
            } else if t.kind == crate::tokenize::TokenKind::Punctuator && t.value == ")" {
                depth -= 1;
                if depth == 0 {
                    close_idx = Some(i);
                    col_segments.push(std::mem::take(&mut current));
                } else {
                    current.push(t);
                }
            } else if t.kind == crate::tokenize::TokenKind::Punctuator
                && t.value == ","
                && depth == 1
            {
                col_segments.push(std::mem::take(&mut current));
            } else {
                current.push(t);
            }
            i += 1;
        }
        if let Some(close_idx) = close_idx {
            end_range = (tokens[close_idx].start, tokens[close_idx].end);
            if let Some(Value::Array(colnames)) = alias.get_mut("colnames") {
                for (idx, col_node) in colnames.iter_mut().enumerate() {
                    let Some(segment) = col_segments.get(idx) else {
                        break;
                    };
                    if segment.is_empty() {
                        continue;
                    }
                    if let Value::Object(col_map) = col_node {
                        let first = segment[0];
                        let last = segment[segment.len() - 1];
                        set_range_and_loc(col_map, first.start, last.end, source);
                    }
                }
            }
        }
    }

    set_range_and_loc(alias, alias_token.start, end_range.1, source);
}

fn resolve_alias_ranges(node: &mut Value, tokens: &[Token], source: &Source) {
    match node {
        Value::Array(items) => {
            for item in items {
                resolve_alias_ranges(item, tokens, source);
            }
        }
        Value::Object(map) => {
            let parent_range = range_of(map);
            if let Some(Value::Object(alias)) = map.get_mut("alias")
                && alias.get("type").and_then(Value::as_str) == Some("Alias")
            {
                resolve_alias_node_ranges(alias, parent_range, tokens, source);
            }
            let keys: Vec<String> = map
                .iter()
                .filter(|(k, v)| !is_special(k) && (v.is_object() || v.is_array()))
                .map(|(k, _)| k.clone())
                .collect();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    resolve_alias_ranges(child, tokens, source);
                }
            }
        }
        _ => {}
    }
}

/// Turn libpg_query's raw `stmts` array into the enriched statement nodes the
/// rules walk. `raw` is the JSON value returned by [`crate::ffi::parse_to_json`].
pub fn manipulate(raw: &Value, tokens: &[Token], source: &Source) -> Vec<Value> {
    let start_end = build_start_end(tokens);
    let resolver = LocationResolver {
        start_end: &start_end,
        source,
    };
    let mut result = Vec::new();

    let Some(stmts) = raw.get("stmts").and_then(Value::as_array) else {
        return result;
    };

    for stmt in stmts {
        let Some(mut stmt_node) = stmt.get("stmt").cloned() else {
            continue;
        };
        add_types(&mut stmt_node);
        resolver.add_location(&mut stmt_node);

        let stmt_location = stmt
            .get("stmt_location")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let stmt_len = stmt.get("stmt_len").and_then(Value::as_i64).unwrap_or(0);
        if stmt_len > 0
            && let (Some(start_char), Some(end_char)) = (
                source.byte_to_unit(stmt_location),
                source.byte_to_unit(stmt_location + stmt_len),
            )
            && let Value::Object(map) = &mut stmt_node
        {
            set_range_and_loc(map, start_char, end_char, source);
        }

        let stmt_bounds = if let Value::Object(map) = &stmt_node {
            range_of(map).zip(map.get("loc").cloned())
        } else {
            None
        };
        if let Some((range, loc)) = stmt_bounds {
            repair_fallback_locations(&mut stmt_node, range, &loc);
        }

        resolve_alias_ranges(&mut stmt_node, tokens, source);
        result.push(stmt_node);
    }

    result
}
