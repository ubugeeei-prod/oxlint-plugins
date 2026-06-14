//! EmbeddedCode attachment, ported from upstream `src/embeddedCode.ts`.
//!
//! A `CREATE FUNCTION` / `CREATE PROCEDURE` statement (both surface as
//! `CreateFunctionStmt` in libpg_query) carries its PL body as an `AS` option.
//! This module locates that body literal in the token stream, records its inner
//! range and quote style, and hangs an `EmbeddedCode` node off the statement as
//! `node.embeddedCode`, matching upstream.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serde_json interop boundary: mirrors upstream's JS object/array semantics directly on serde_json's owned String/Vec. The carton-collection policy governs rule hot-path state, not this boundary."
)]

use serde_json::{Value, json};

use crate::text::Source;
use crate::tokenize::{Token, TokenKind};

/// Attach `embeddedCode` to every top-level `CreateFunctionStmt` in `program`'s
/// body that has a recognizable language + body literal.
pub fn attach_embedded_code(program: &mut Value, tokens: &[Token], source: &Source) {
    let Some(body) = program.get_mut("body").and_then(Value::as_array_mut) else {
        return;
    };
    for node in body.iter_mut() {
        // Only CreateFunctionStmt carries a function body. `CREATE PROCEDURE`
        // surfaces under the same node in libpg_query (with `is_procedure: true`).
        if node.get("type").and_then(Value::as_str) != Some("CreateFunctionStmt") {
            continue;
        }
        attach_to_function_stmt(node, tokens, source);
    }
}

fn attach_to_function_stmt(stmt: &mut Value, tokens: &[Token], source: &Source) {
    let options = stmt.get("options");

    let Some(language) = find_def_elem(options, "language").and_then(read_language) else {
        return;
    };
    let Some(as_def_elem) = find_def_elem(options, "as") else {
        return;
    };
    let Some(body) = read_as_body(as_def_elem) else {
        return;
    };
    let Some(token_info) = find_body_token(tokens, body.start_search_from) else {
        return;
    };

    let start = token_info.inner_start;
    let end = token_info.inner_end;
    let embedded = json!({
        "type": "EmbeddedCode",
        "language": language,
        "source": body.source,
        "quoteStyle": token_info.quote_style,
        "range": [start, end],
        "loc": {
            "start": position_to_json(source, start),
            "end": position_to_json(source, end),
        },
    });

    if let Value::Object(map) = stmt {
        map.insert("embeddedCode".to_string(), embedded);
    }
}

/// Find a `DefElem` option with the given `defname` in an options array.
fn find_def_elem<'a>(options: Option<&'a Value>, defname: &str) -> Option<&'a Value> {
    let items = options?.as_array()?;
    items.iter().find(|option| {
        option.get("type").and_then(Value::as_str) == Some("DefElem")
            && option.get("defname").and_then(Value::as_str) == Some(defname)
    })
}

/// Read the `language` DefElem's lowercased string argument.
fn read_language(lang_def_elem: &Value) -> Option<String> {
    let arg = lang_def_elem.get("arg")?;
    if arg.get("type").and_then(Value::as_str) != Some("String") {
        return None;
    }
    let sval = arg.get("sval")?.as_str()?;
    Some(sval.to_lowercase())
}

struct AsBody {
    source: String,
    start_search_from: u32,
}

/// Read the function body from the `AS` DefElem. libpg_query exposes it as a
/// `List` of `String` items; a single-item list is a normal body, a two-item
/// list is the C-language `AS 'libname', 'symbol'` form which is not source we
/// can lint.
fn read_as_body(as_def_elem: &Value) -> Option<AsBody> {
    let arg = as_def_elem.get("arg")?;
    let items = arg.get("items")?.as_array()?;
    if items.len() != 1 {
        return None;
    }
    let first = &items[0];
    if first.get("type").and_then(Value::as_str) != Some("String") {
        return None;
    }
    let sval = first.get("sval")?.as_str()?;

    // The DefElem for `AS` covers only the AS keyword; start scanning for the
    // body literal from the end of its range.
    let start_search_from = as_def_elem
        .get("range")
        .and_then(Value::as_array)
        .and_then(|range| range.get(1))
        .and_then(Value::as_u64)
        .map_or(0, |end| end as u32);

    Some(AsBody {
        source: sval.to_string(),
        start_search_from,
    })
}

struct BodyTokenInfo {
    inner_start: u32,
    inner_end: u32,
    quote_style: &'static str,
}

/// Locate the `String` token holding the function body, scanning forward from
/// the `AS` keyword (PostgreSQL grammar allows only whitespace between `AS` and
/// the body literal).
fn find_body_token(tokens: &[Token], start_search_from: u32) -> Option<BodyTokenInfo> {
    for token in tokens {
        if token.kind != TokenKind::String {
            continue;
        }
        if token.start < start_search_from {
            continue;
        }

        let raw = token.value.as_str();
        if let Some(after_dollar) = raw.strip_prefix('$') {
            // Dollar-quote: $$...$$ or $tag$...$tag$. The opening tag is ASCII
            // by grammar (and bounded by PostgreSQL's 63-char identifier limit),
            // so byte and UTF-16 offsets coincide over it and the length fits a
            // u32. The closing `$` of the opening tag sits one past its index in
            // the `$`-stripped remainder.
            let tag_length = (after_dollar.find('$')? + 2) as u32;
            // The token always spans at least its opening tag, so subtracting
            // the tag length from `token.end` never underflows.
            debug_assert!(token.end >= token.start + tag_length);
            return Some(BodyTokenInfo {
                inner_start: token.start + tag_length,
                inner_end: token.end - tag_length,
                quote_style: "dollar",
            });
        }

        if raw.starts_with('\'') {
            // Single-quoted body. The absolute range spans the raw literal
            // contents (sourceMap support for `''` escapes is deferred upstream).
            return Some(BodyTokenInfo {
                inner_start: token.start + 1,
                inner_end: token.end - 1,
                quote_style: "single",
            });
        }

        // Any other String token here is unexpected — skip and keep looking.
    }
    None
}

fn position_to_json(source: &Source, offset: u32) -> Value {
    let position = source.position(offset);
    json!({ "line": position.line, "column": position.column })
}
