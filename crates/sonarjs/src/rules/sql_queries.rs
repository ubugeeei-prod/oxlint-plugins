//! Rule `sql-queries` (SonarJS key S2077).
//!
//! Clean-room port from the public RSPEC S2077 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Building a SQL query string by mixing runtime values into the statement text
//! (string concatenation or template interpolation) is the classic shape of a
//! SQL-injection vulnerability: untrusted data lands inside the query grammar
//! instead of being bound as a parameter. SonarJS raises a security hotspot when
//! a SQL statement is assembled dynamically; the safe pattern is a constant
//! statement with parameter placeholders bound separately.
//!
//! ## Zero-FP subset
//!
//! There is no type information or taint/dataflow engine available here, so this
//! port flags only the syntactic shape that is unambiguously a *formatted* SQL
//! statement. A node is reported when BOTH of these hold:
//!
//!  - it is built at runtime, i.e. a template literal with at least one
//!    interpolation (`` `... ${x} ...` ``) or a string concatenation
//!    (`"..." + x`), and
//!  - its STATIC text begins (ignoring leading whitespace) with a SQL command
//!    keyword as a whole word — `SELECT`, `INSERT`, `UPDATE`, `DELETE`, or
//!    `MERGE` — AND also contains a second SQL clause keyword (`FROM`, `WHERE`,
//!    `INTO`, `VALUES`, `JOIN`, or `SET`).
//!
//! Requiring two distinct SQL keywords in the constant part makes the match
//! effectively free of false positives: ordinary prose such as
//! `` `Update ${count} records` `` (no second keyword) or `"Select an option: " + n`
//! is not flagged, while real statements such as
//! `` `SELECT * FROM t WHERE id = ${id}` `` are.
//!
//! ## Flagged
//! ```js
//! const q = `SELECT name FROM users WHERE id = ${userId}`;       // interpolation
//! db.query("DELETE FROM sessions WHERE token = '" + token + "'"); // concatenation
//! ```
//!
//! ## Not flagged
//! ```js
//! const q = `SELECT name FROM users WHERE id = 1`;   // static template, no ${}
//! const q = "SELECT name FROM users";                // constant string literal
//! const msg = `Update ${count} rows`;                // only one SQL keyword
//! const q = "INSERT INTO t VALUES " + "(1, 2)";      // right side is constant too
//! ```
//!
//! ## Follow-up (out of scope here)
//! Resolving a query assembled across a variable (`const part = "...WHERE..."; db.query("SELECT " + part + x)`),
//! and SQL whose command keyword sits after an interpolation, are intentionally
//! out of scope to keep the check false-positive-free.

use oxc_ast::ast::{BinaryExpression, Expression, TemplateLiteral};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "sql-queries";

/// SQL command keywords that may begin a statement.
const SQL_STARTERS: [&str; 5] = ["SELECT", "INSERT", "UPDATE", "DELETE", "MERGE"];
/// Secondary SQL clause keywords; at least one must also appear in the static
/// text so that ordinary prose beginning with a command word is not flagged.
const SQL_SECONDARIES: [&str; 6] = ["FROM", "WHERE", "INTO", "VALUES", "JOIN", "SET"];

/// Case-insensitive ASCII substring search without allocating.
fn ci_contains(haystack: &str, needle: &str) -> bool {
    let needle = needle.as_bytes();
    let n = needle.len();
    if n == 0 {
        return true;
    }
    let hay = haystack.as_bytes();
    if hay.len() < n {
        return false;
    }
    hay.windows(n).any(|w| w.eq_ignore_ascii_case(needle))
}

/// Returns `true` when `text`, after trimming leading whitespace, starts with a
/// SQL command keyword as a whole word.
fn starts_with_sql_command(text: &str) -> bool {
    let trimmed = text.trim_start();
    let bytes = trimmed.as_bytes();
    SQL_STARTERS.iter().any(|kw| {
        let kw = kw.as_bytes();
        bytes.len() >= kw.len()
            && bytes[..kw.len()].eq_ignore_ascii_case(kw)
            && bytes.get(kw.len()).is_none_or(|b| b.is_ascii_whitespace())
    })
}

/// Decides whether the static text of a formatted query looks like SQL: it must
/// begin with a command keyword and contain a second clause keyword.
fn looks_like_sql(static_text: &str) -> bool {
    starts_with_sql_command(static_text)
        && SQL_SECONDARIES
            .iter()
            .any(|kw| ci_contains(static_text, kw))
}

impl Scanner<'_> {
    /// Flags a template literal with interpolations whose static parts form a
    /// SQL statement (attached to `visit_template_literal`).
    pub(crate) fn check_sql_queries(&mut self, it: &TemplateLiteral<'_>) {
        if it.expressions.is_empty() {
            return;
        }
        // The command keyword must begin the first static chunk; any chunk may
        // carry a secondary keyword. Checking chunks individually avoids an
        // allocation and never splits a keyword across an interpolation.
        let Some(first) = it.quasis.first() else {
            return;
        };
        if !starts_with_sql_command(first.value.raw.as_str()) {
            return;
        }
        let has_secondary = it.quasis.iter().any(|q| {
            let raw = q.value.raw.as_str();
            SQL_SECONDARIES.iter().any(|kw| ci_contains(raw, kw))
        });
        if !has_secondary {
            return;
        }
        self.report(RULE_NAME, "safeQuery", it.span);
    }

    /// Flags a string concatenation whose leading literal is a SQL statement and
    /// whose right operand is a runtime value (attached to
    /// `visit_binary_expression`). Restricting the left operand to a string
    /// literal reports a nested `+` chain exactly once, at its innermost node.
    pub(crate) fn check_sql_queries_concat(&mut self, it: &BinaryExpression<'_>) {
        if it.operator != BinaryOperator::Addition {
            return;
        }
        let Expression::StringLiteral(left) = it.left.get_inner_expression() else {
            return;
        };
        if !looks_like_sql(left.value.as_str()) {
            return;
        }
        // A constant `"..." + "..."` is not a formatted query.
        if matches!(
            it.right.get_inner_expression(),
            Expression::StringLiteral(_)
        ) {
            return;
        }
        self.report(RULE_NAME, "safeQuery", it.span);
    }
}
