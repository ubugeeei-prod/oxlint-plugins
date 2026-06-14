//! Parse entry point: SQL text → enriched statement nodes + token stream.
//!
//! Mirrors upstream `parseForESLint` (`src/parse.ts`): tokenize, parse via
//! libpg_query, then enrich the raw JSON tree with `type`/`range`/`loc`. A
//! syntax error yields an empty statement list plus a captured [`ParseError`],
//! matching upstream's single `SQLParseError` node (rules other than
//! `no-syntax-error` simply report nothing on unparseable input).
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serde_json / libpg_query interop boundary: this parser layer mirrors upstream's JS string semantics and works with owned String/Vec. The carton-collection policy governs rule hot-path state, not this boundary."
)]

use serde_json::Value;

use crate::ffi::parse_to_json;
use crate::manipulate::manipulate;
use crate::text::Source;
use crate::tokenize::{Comment, Token, Tokenized, tokenize};

#[derive(Clone, Debug)]
pub struct ParseError {
    pub message: String,
}

pub struct Parsed {
    pub source: Source,
    pub tokens: Vec<Token>,
    pub comments: Vec<Comment>,
    pub statements: Vec<Value>,
    pub error: Option<ParseError>,
}

pub fn parse(source_text: &str) -> Parsed {
    let source = Source::new(source_text);
    let Tokenized { tokens, comments } = tokenize(&source);

    match parse_to_json(source_text) {
        Ok(json) => match serde_json::from_str::<Value>(&json) {
            Ok(raw) => {
                let statements = manipulate(&raw, &tokens, &source);
                Parsed {
                    source,
                    tokens,
                    comments,
                    statements,
                    error: None,
                }
            }
            Err(err) => Parsed {
                source,
                tokens,
                comments,
                statements: Vec::new(),
                error: Some(ParseError {
                    message: err.to_string(),
                }),
            },
        },
        Err(message) => Parsed {
            source,
            tokens,
            comments,
            statements: Vec::new(),
            error: Some(ParseError { message }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn select_star_has_located_res_target() {
        let parsed = parse("SELECT * FROM users");
        assert!(parsed.error.is_none());
        assert_eq!(parsed.statements.len(), 1);
        let stmt = &parsed.statements[0];
        assert_eq!(stmt["type"], "SelectStmt");
        let target = &stmt["targetList"][0];
        assert_eq!(target["type"], "ResTarget");
        // `SELECT *` — the ResTarget starts at column 7 (offset 7).
        assert_eq!(target["loc"]["start"]["line"], 1);
        assert_eq!(target["loc"]["start"]["column"], 7);
    }

    #[test]
    fn syntax_error_is_captured() {
        let parsed = parse("SELECT FROM )(");
        assert!(parsed.error.is_some());
        assert!(parsed.statements.is_empty());
    }
}
