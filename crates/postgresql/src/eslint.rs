//! `parseForESLint` assembly, ported from upstream `src/parse.ts` +
//! `src/visitorKeys.ts`.
//!
//! Turns the enriched statement list produced by [`crate::parse`] into the
//! `{ ast, visitorKeys, scopeManager }` shape upstream's custom ESLint parser
//! returns. The AST is a `Program` node carrying the manipulated statements,
//! the lexed `tokens`, and the `comments`; `visitorKeys` is derived from the
//! AST exactly as upstream does; `scopeManager` is always `null` (upstream does
//! not build one).
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serde_json interop boundary: this assembly layer mirrors upstream's JS object/array semantics and operates directly on serde_json's owned String/Vec. The carton-collection policy governs rule hot-path state, not this boundary."
)]

use serde_json::{Map, Value, json};

use crate::embedded_code::attach_embedded_code;
use crate::parse::parse;
use crate::text::Position;
use crate::tokenize::{Comment, CommentKind, Token, TokenKind};

/// Parse `source_text` and return the `parseForESLint` result serialized as a
/// JSON string. The NAPI boundary of `npm/postgresql-eslint-parser` hands this
/// to JavaScript, which `JSON.parse`s it back into the object an ESLint custom
/// parser must return.
pub fn parse_for_eslint_json(source_text: &str) -> String {
    parse_for_eslint(source_text).to_string()
}

/// Parse `source_text` and return only the `ast` half of the result, mirroring
/// upstream's `parse(code)` convenience export.
pub fn parse_ast(source_text: &str) -> Value {
    let mut result = parse_for_eslint(source_text);
    result
        .get_mut("ast")
        .map(Value::take)
        .unwrap_or(Value::Null)
}

/// Parse `source_text` and return the upstream `parseForESLint` result:
/// `{ "ast": Program, "visitorKeys": { … }, "scopeManager": null }`.
pub fn parse_for_eslint(source_text: &str) -> Value {
    let parsed = parse(source_text);
    // UTF-16 code-unit length, mirroring JS `code.length` (the parser indexes in
    // UTF-16 units throughout, like the upstream TypeScript parser).
    let len = parsed.source.len();

    let program_range = json!([0, len]);
    let program_loc = json!({
        "start": { "line": 1, "column": 0 },
        "end": position_to_json(parsed.source.position(len)),
    });

    let body = match &parsed.error {
        // A syntax error collapses the whole program to a single SQLParseError
        // node spanning the program, exactly like upstream's catch branch.
        Some(error) => vec![json!({
            "type": "SQLParseError",
            "range": program_range.clone(),
            "loc": program_loc.clone(),
            "error": error.message,
            "raw": source_text,
        })],
        None => parsed.statements,
    };

    let tokens: Vec<Value> = parsed.tokens.iter().map(token_to_json).collect();
    let comments: Vec<Value> = parsed.comments.iter().map(comment_to_json).collect();

    let mut program = Value::Object({
        let mut map = Map::new();
        map.insert("type".to_string(), json!("Program"));
        map.insert("range".to_string(), program_range);
        map.insert("loc".to_string(), program_loc);
        map.insert("body".to_string(), Value::Array(body));
        map.insert("tokens".to_string(), Value::Array(tokens));
        map.insert("comments".to_string(), Value::Array(comments));
        map
    });

    // EmbeddedCode is attached after the program is assembled so it can mirror
    // upstream, which runs `attachEmbeddedCode(program, …)` last.
    attach_embedded_code(&mut program, &parsed.tokens, &parsed.source);

    let visitor_keys = build_visitor_keys(&program);

    json!({
        "ast": program,
        "visitorKeys": visitor_keys,
        "scopeManager": Value::Null,
    })
}

fn token_kind_str(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::String => "String",
        TokenKind::Operator => "Operator",
        TokenKind::Punctuator => "Punctuator",
        TokenKind::Keyword => "Keyword",
        TokenKind::Identifier => "Identifier",
        TokenKind::Numeric => "Numeric",
    }
}

fn token_to_json(token: &Token) -> Value {
    json!({
        "type": token_kind_str(token.kind),
        "value": token.value,
        "range": [token.start, token.end],
        "loc": {
            "start": position_to_json(token.start_pos),
            "end": position_to_json(token.end_pos),
        },
    })
}

fn comment_to_json(comment: &Comment) -> Value {
    let ty = match comment.kind {
        CommentKind::Line => "Line",
        CommentKind::Block => "Block",
    };
    json!({
        "type": ty,
        "value": comment.value,
        "range": [comment.start, comment.end],
        "loc": {
            "start": position_to_json(comment.start_pos),
            "end": position_to_json(comment.end_pos),
        },
    })
}

fn position_to_json(position: Position) -> Value {
    json!({ "line": position.line, "column": position.column })
}

// ---------------------------------------------------------------------------
// visitorKeys
// ---------------------------------------------------------------------------

const SKIP_KEYS: [&str; 4] = ["type", "range", "loc", "parent"];

/// Base visitor keys merged on top of the derived ones, mirroring
/// `BASE_VISITOR_KEYS` in upstream `src/visitorKeys.ts`. These are always
/// present even when no node of the type appears in a given file.
fn base_visitor_keys() -> [(&'static str, &'static [&'static str]); 5] {
    [
        ("Program", &["body", "tokens", "comments"]),
        ("SQLStatement", &["statement"]),
        ("SQLParseError", &[]),
        ("SQLProcedure", &[]),
        ("EmbeddedCode", &[]),
    ]
}

/// Derive the visitor-key map by walking the AST, unioning the child keys seen
/// across every node of each type, then merging the base keys on top.
///
/// The serialized AST carries no `parent` back-pointers, so it is a finite tree
/// and needs no cycle guard (upstream's `WeakSet` exists only because its nodes
/// are parent-linked).
pub fn build_visitor_keys(ast: &Value) -> Value {
    let mut visitor_keys: Map<String, Value> = Map::new();
    traverse_visitor_keys(ast, &mut visitor_keys);

    for (ty, keys) in base_visitor_keys() {
        visitor_keys.insert(
            ty.to_string(),
            Value::Array(keys.iter().map(|k| json!(k)).collect()),
        );
    }

    Value::Object(visitor_keys)
}

fn traverse_visitor_keys(node: &Value, visitor_keys: &mut Map<String, Value>) {
    let Value::Object(object) = node else {
        return;
    };

    let ty = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    if !visitor_keys.contains_key(&ty) {
        visitor_keys.insert(ty.clone(), Value::Array(Vec::new()));
    }

    // Upstream's `buildVisitorKeys` walks JS objects, whose key iteration order
    // puts integer-index keys first (ascending), then the remaining keys in
    // insertion order. Array-valued wrappers spread into `"0"`, `"1"`, … keys
    // (see `manipulate::add_types`) rely on that ordering, so reproduce it here
    // rather than using serde_json's raw insertion order.
    for key in js_key_order(object) {
        if SKIP_KEYS.contains(&key) {
            continue;
        }
        let Some(value) = object.get(key) else {
            continue;
        };
        match value {
            Value::Array(items) => {
                let contains_object = items.iter().any(Value::is_object);
                // Upstream adds the key when the array holds an object *or* is
                // simply non-empty (so non-empty primitive arrays count too).
                if contains_object || !items.is_empty() {
                    add_visitor_key(visitor_keys, &ty, key);
                }
                if contains_object {
                    for item in items {
                        traverse_visitor_keys(item, visitor_keys);
                    }
                }
            }
            Value::Object(_) => {
                add_visitor_key(visitor_keys, &ty, key);
                traverse_visitor_keys(value, visitor_keys);
            }
            _ => {}
        }
    }
}

/// Return `object`'s keys in JavaScript property-iteration order: canonical
/// array-index keys (`"0"`, `"1"`, …) first in ascending numeric order, then the
/// remaining keys in insertion order. Mirrors the order upstream's JS
/// `Object.entries`/`Object.keys` yields.
fn js_key_order(object: &Map<String, Value>) -> Vec<&str> {
    let mut index_keys: Vec<(u32, &str)> = Vec::new();
    let mut other_keys: Vec<&str> = Vec::new();
    for key in object.keys() {
        match array_index(key) {
            Some(index) => index_keys.push((index, key.as_str())),
            None => other_keys.push(key.as_str()),
        }
    }
    index_keys.sort_unstable_by_key(|(index, _)| *index);
    let mut ordered: Vec<&str> = index_keys.into_iter().map(|(_, key)| key).collect();
    ordered.extend(other_keys);
    ordered
}

/// Parse `key` as a canonical JS array index (`"0"` or a non-zero-leading run of
/// digits within `u32`), matching V8's integer-key fast path. Returns `None` for
/// non-index keys (e.g. `"typmod"`, or `"01"`).
fn array_index(key: &str) -> Option<u32> {
    if key == "0" {
        return Some(0);
    }
    if key.starts_with('0') || key.is_empty() {
        return None;
    }
    if !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // V8 array indices run `0..=2^32-2`; the string `"4294967295"` (u32::MAX) is
    // a named property, not an index, so exclude it.
    key.parse::<u32>().ok().filter(|&index| index < u32::MAX)
}

fn add_visitor_key(visitor_keys: &mut Map<String, Value>, ty: &str, key: &str) {
    let entry = visitor_keys
        .entry(ty.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(keys) = entry
        && !keys.iter().any(|existing| existing.as_str() == Some(key))
    {
        keys.push(Value::String(key.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::parse_for_eslint;

    #[test]
    fn basic_select_program_shape() {
        let result = parse_for_eslint("SELECT * FROM users;");
        let ast = &result["ast"];
        assert_eq!(ast["type"], "Program");
        assert_eq!(ast["range"], serde_json::json!([0, 20]));
        assert_eq!(ast["loc"]["end"]["column"], 20);
        assert_eq!(ast["body"][0]["type"], "SelectStmt");
        assert_eq!(ast["tokens"][0]["type"], "Keyword");
        assert_eq!(ast["tokens"][0]["value"], "SELECT");
        assert_eq!(ast["comments"], serde_json::json!([]));
        assert!(result["scopeManager"].is_null());

        let vk = &result["visitorKeys"];
        assert_eq!(
            vk["Program"],
            serde_json::json!(["body", "tokens", "comments"])
        );
        assert_eq!(vk["ColumnRef"], serde_json::json!(["fields"]));
        assert_eq!(vk["A_Star"], serde_json::json!([]));
    }

    #[test]
    fn syntax_error_becomes_parse_error_node() {
        let result = parse_for_eslint("SELECT FROM WHERE )(");
        let body = &result["ast"]["body"];
        assert_eq!(body[0]["type"], "SQLParseError");
        assert_eq!(body[0]["raw"], "SELECT FROM WHERE )(");
        assert!(body[0]["error"].is_string());
    }
}
