//! SQL tokenizer ported from upstream `src/tokenize.ts`.
//!
//! A permissive lexer whose only job is to give every lexeme a source range, so
//! that [`crate::manipulate`] can resolve node spans (libpg_query reports only a
//! start `location` per node) and recover alias ranges from the token stream.
//! It is intentionally ASCII-structural: non-ASCII bytes that are not inside a
//! quoted string fall through to the skip branch, exactly as upstream does.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serde_json / libpg_query interop boundary: this parser layer mirrors upstream's JS string semantics and works with owned String/Vec. The carton-collection policy governs rule hot-path state, not this boundary."
)]

use crate::text::{Position, Source};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    String,
    Operator,
    Punctuator,
    Keyword,
    Identifier,
    Numeric,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    /// UTF-16 offset range `[start, end)`.
    pub start: u32,
    pub end: u32,
    pub start_pos: Position,
    pub end_pos: Position,
}

impl Token {
    pub fn is_identifier_like(&self) -> bool {
        matches!(self.kind, TokenKind::Identifier | TokenKind::Keyword)
    }
}

/// A `--` line comment or `/* */` block comment, in upstream's ESLint shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub kind: CommentKind,
    /// Comment text with its delimiters stripped (`--` for lines, `/* */` for
    /// blocks), matching upstream's `value`.
    pub value: String,
    /// UTF-16 offset range `[start, end)` of the whole comment (delimiters
    /// included).
    pub start: u32,
    pub end: u32,
    pub start_pos: Position,
    pub end_pos: Position,
}

/// The lexer output: lexeme tokens plus the comments stripped out between them.
pub struct Tokenized {
    pub tokens: Vec<Token>,
    pub comments: Vec<Comment>,
}

static KEYWORDS: phf::Set<&'static str> = phf::phf_set! {
    // basic
    "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP", "TABLE",
    "INDEX", "VIEW", "DATABASE", "SCHEMA", "COLUMN", "PRIMARY", "KEY", "FOREIGN",
    "REFERENCES", "CONSTRAINT", "UNIQUE", "CHECK", "DEFAULT", "AS", "BETWEEN",
    "CASE", "CAST", "EXISTS", "FALSE", "TRUE", "NOT", "NULL", "NULLS", "IS",
    "ISNULL", "NOTNULL", "AND", "OR", "ANY", "SOME", "IN", "LIKE", "ILIKE",
    "SIMILAR", "ESCAPE", "ASC", "DESC", "ORDER", "GROUP", "HAVING", "LIMIT",
    "OFFSET", "DISTINCT", "ALL", "EXCEPT", "INTERSECT", "UNION", "VALUES",
    "FROM", "INTO",
    // data type
    "INTEGER", "INT", "BIGINT", "SMALLINT", "DEC", "DECIMAL", "NUMERIC", "REAL",
    "DOUBLE", "PRECISION", "FLOAT", "VARCHAR", "CHAR", "TEXT", "BOOLEAN", "DATE",
    "TIME", "TIMESTAMP", "INTERVAL", "UUID", "JSON", "JSONB", "ARRAY",
    // join
    "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS", "NATURAL",
    "USING", "ON",
    // aggregate / window
    "COUNT", "SUM", "AVG", "MIN", "MAX", "OVER", "PARTITION", "WINDOW", "RANGE",
    "ROWS", "UNBOUNDED", "PRECEDING", "FOLLOWING", "CURRENT", "ROW",
    // procedure / PL/pgSQL
    "FUNCTION", "PROCEDURE", "RETURNS", "RETURN", "LANGUAGE", "PLPGSQL", "SQL",
    "IMMUTABLE", "STABLE", "VOLATILE", "SECURITY", "DEFINER", "INVOKER",
    "STRICT", "CALLED", "INPUT", "COST", "PARALLEL", "SAFE", "RESTRICTED",
    "UNSAFE", "DECLARE", "BEGIN", "END", "EXCEPTION", "WHEN", "RAISE", "NOTICE",
    "WARNING", "INFO", "LOG", "DEBUG", "EXECUTE", "PERFORM", "GET",
    "DIAGNOSTICS", "LOOP", "WHILE", "FOR", "FOREACH", "EXIT", "CONTINUE", "IF",
    "THEN", "ELSE", "ELSIF", "FOUND", "ROW_COUNT", "RESULT_OID", "PG_CONTEXT",
    "PG_DATATYPE_NAME", "PG_EXCEPTION_CONTEXT", "PG_EXCEPTION_DETAIL",
    "PG_EXCEPTION_HINT", "MESSAGE_TEXT", "RETURNED_SQLSTATE", "SCHEMA_NAME",
    "TABLE_NAME", "COLUMN_NAME", "CONSTRAINT_NAME", "PG_TYPE_NAME", "CALL", "DO",
    "BLOCK",
    // transaction
    "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE", "START", "TRANSACTION", "WORK",
    // permission
    "GRANT", "REVOKE", "PRIVILEGES", "USAGE", "CONNECT", "TEMPORARY", "TEMP",
    "TRIGGER", "RULE", "EVENT", "COMMENT",
    // other
    "ANALYSE", "ANALYZE", "AUTHORIZATION", "BINARY", "BOTH", "BY", "COLLATE",
    "COLLATION", "CONCURRENTLY", "CURRENT_CATALOG", "CURRENT_DATE",
    "CURRENT_ROLE", "CURRENT_SCHEMA", "CURRENT_TIME", "CURRENT_TIMESTAMP",
    "CURRENT_USER", "DEALLOCATE", "DEFERRABLE", "DEFERRED", "FETCH", "FREEZE",
    "LOCALTIME", "LOCALTIMESTAMP", "ONLY", "OVERLAPS", "PLACING", "SESSION_USER",
    "SET", "SYMMETRIC", "SYSTEM_USER", "TABLESAMPLE", "TO", "TRAILING", "USER",
    "VERBOSE", "WHERE", "WITH",
};

fn is_keyword(value: &str) -> bool {
    KEYWORDS.contains(value.to_ascii_uppercase().as_str())
}

fn is_ascii_space(b: u8) -> bool {
    // Mirror JS `/\s/` for the ASCII range (the only bytes the lexer inspects):
    // space, \t, \n, \r, \x0b (VT), \x0c (FF).
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_punct_or_operator(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')'
            | b','
            | b';'
            | b'.'
            | b'='
            | b'<'
            | b'>'
            | b'!'
            | b'+'
            | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'|'
            | b'&'
            | b':'
    )
}

fn is_punctuator(b: u8) -> bool {
    matches!(b, b'(' | b')' | b',' | b';' | b'.')
}

fn numeric_is_valid(lexeme: &str) -> bool {
    // Mirror `/^\d+(\.\d+)?([eE][+-]?\d+)?$/`.
    let bytes = lexeme.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < n && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
        *i > start
    };
    if !digits(&mut i) {
        return false;
    }
    if i < n && bytes[i] == b'.' {
        i += 1;
        if !digits(&mut i) {
            return false;
        }
    }
    if i < n && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        if !digits(&mut i) {
            return false;
        }
    }
    i == n
}

/// Tokenize `source`, returning the lexeme tokens and the comments scanned out
/// between them. Comments do not participate in span resolution but upstream
/// surfaces them on `Program.comments`, so they are retained here too.
pub fn tokenize(source: &Source) -> Tokenized {
    let mut tokens = Vec::new();
    let mut comments = Vec::new();
    let length = source.len();
    let mut i: u32 = 0;

    let push = |tokens: &mut Vec<Token>, kind: TokenKind, start: u32, end: u32| {
        tokens.push(Token {
            kind,
            value: source.slice(start, end),
            start,
            end,
            start_pos: source.position(start),
            end_pos: source.position(end),
        });
    };

    while i < length {
        let Some(byte) = source.ascii_at(i) else {
            // Non-ASCII unit (or astral surrogate): skip, like upstream's
            // unsupported-character branch.
            i += 1;
            continue;
        };

        // whitespace
        if is_ascii_space(byte) {
            i += 1;
            continue;
        }

        // line comment
        if byte == b'-' && source.ascii_at(i + 1) == Some(b'-') {
            let start = i;
            while i < length && source.ascii_at(i) != Some(b'\n') {
                i += 1;
            }
            // `i` stops at the newline (or EOF) and never exceeds `length`.
            comments.push(Comment {
                kind: CommentKind::Line,
                value: source.slice(start + 2, i), // strip leading "--"
                start,
                end: i,
                start_pos: source.position(start),
                end_pos: source.position(i),
            });
            continue;
        }

        // block comment
        if byte == b'/' && source.ascii_at(i + 1) == Some(b'*') {
            let start = i;
            i += 2;
            while i + 1 < length
                && !(source.ascii_at(i) == Some(b'*') && source.ascii_at(i + 1) == Some(b'/'))
            {
                i += 1;
            }
            i += 2;
            // Upstream computes the range from `code.slice(start, i).length`,
            // which clamps an unterminated comment's end to the source length.
            let end = i.min(length);
            comments.push(Comment {
                kind: CommentKind::Block,
                value: source.slice(start + 2, end.saturating_sub(2)), // strip "/*" and "*/"
                start,
                end,
                start_pos: source.position(start),
                end_pos: source.position(end),
            });
            continue;
        }

        // string / quoted-identifier literal
        if byte == b'\'' || byte == b'"' {
            let quote = byte;
            let start = i;
            i += 1;
            while i < length {
                let Some(c) = source.ascii_at(i) else {
                    i += 1;
                    continue;
                };
                if c == quote {
                    if source.ascii_at(i + 1) == Some(quote) {
                        i += 2; // escaped quote ('' or "")
                    } else {
                        i += 1;
                        break;
                    }
                } else if c == b'\\' {
                    i += 2; // backslash escape
                } else {
                    i += 1;
                }
            }
            push(&mut tokens, TokenKind::String, start, i);
            continue;
        }

        // dollar-quoted string literal: $$...$$ or $tag$...$tag$
        if byte == b'$' {
            let mut j = i + 1;
            if let Some(c) = source.ascii_at(j)
                && (c.is_ascii_alphabetic() || c == b'_')
            {
                j += 1;
                while let Some(c) = source.ascii_at(j) {
                    if c.is_ascii_alphanumeric() || c == b'_' {
                        j += 1;
                    } else {
                        break;
                    }
                }
            }
            if source.ascii_at(j) == Some(b'$') {
                let tag = source.slice(i, j + 1);
                let tag_len = j + 1 - i;
                let start = i;
                i = j + 1;
                while i < length {
                    if source.ascii_at(i) == Some(b'$') && source.slice(i, i + tag_len) == tag {
                        i += tag_len;
                        break;
                    }
                    i += 1;
                }
                push(&mut tokens, TokenKind::String, start, i);
                continue;
            }
            // Not a dollar-quote opener ($1 etc.): fall through to skip.
        }

        // punctuation or operator
        if is_punct_or_operator(byte) {
            let start = i;
            if i + 1 < length {
                let two = source.slice(i, i + 2);
                if matches!(two.as_str(), "<=" | ">=" | "<>" | "!=" | "::" | "||" | "&&") {
                    i += 2;
                    push(&mut tokens, TokenKind::Operator, start, i);
                    continue;
                }
            }
            i += 1;
            let kind = if is_punctuator(byte) {
                TokenKind::Punctuator
            } else {
                TokenKind::Operator
            };
            push(&mut tokens, kind, start, i);
            continue;
        }

        // identifier, keyword, numeric
        if is_word_char(byte) {
            let start = i;
            let kind = if byte.is_ascii_digit() {
                while let Some(c) = source.ascii_at(i) {
                    if c.is_ascii_digit() || c == b'.' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                if let Some(c) = source.ascii_at(i)
                    && (c == b'e' || c == b'E')
                {
                    i += 1;
                    if let Some(s) = source.ascii_at(i)
                        && (s == b'+' || s == b'-')
                    {
                        i += 1;
                    }
                    while let Some(c) = source.ascii_at(i) {
                        if c.is_ascii_digit() {
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
                if numeric_is_valid(&source.slice(start, i)) {
                    TokenKind::Numeric
                } else {
                    TokenKind::Identifier
                }
            } else {
                while let Some(c) = source.ascii_at(i) {
                    if is_word_char(c) {
                        i += 1;
                    } else {
                        break;
                    }
                }
                if is_keyword(&source.slice(start, i)) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                }
            };
            push(&mut tokens, kind, start, i);
            continue;
        }

        // unsupported character
        i += 1;
    }

    Tokenized { tokens, comments }
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, tokenize};
    use crate::text::Source;

    #[test]
    fn classifies_basic_tokens() {
        let src = Source::new("SELECT id FROM users WHERE id >= 1");
        let toks = tokenize(&src).tokens;
        let kinds: Vec<_> = toks.iter().map(|t| (t.kind, t.value.as_str())).collect();
        assert_eq!(kinds[0], (TokenKind::Keyword, "SELECT"));
        assert_eq!(kinds[1], (TokenKind::Identifier, "id"));
        assert_eq!(kinds[2], (TokenKind::Keyword, "FROM"));
        assert!(
            kinds
                .iter()
                .any(|(k, v)| *k == TokenKind::Operator && *v == ">=")
        );
        assert!(
            kinds
                .iter()
                .any(|(k, v)| *k == TokenKind::Numeric && *v == "1")
        );
    }

    #[test]
    fn dollar_quoted_string() {
        let src = Source::new("$$ a $tag$ b$$");
        let toks = tokenize(&src).tokens;
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::String);
    }

    #[test]
    fn collects_line_and_block_comments() {
        let src = Source::new("-- hi\nSELECT 1 /* note */");
        let out = tokenize(&src);
        assert_eq!(out.comments.len(), 2);
        assert_eq!(out.comments[0].kind, CommentKind::Line);
        assert_eq!(out.comments[0].value, " hi");
        assert_eq!((out.comments[0].start, out.comments[0].end), (0, 5));
        assert_eq!(out.comments[1].kind, CommentKind::Block);
        assert_eq!(out.comments[1].value, " note ");
    }
}
