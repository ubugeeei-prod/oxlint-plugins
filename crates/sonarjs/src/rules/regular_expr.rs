//! Rule `regular-expr` (SonarJS key S4784).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC description
//! of S4784 ("Using regular expressions is security-sensitive") ONLY; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! S4784 is a **security hotspot**: complex regular expressions can be used to
//! mount a Regular expression Denial of Service (ReDoS) attack, where a crafted
//! input forces super-linear backtracking. Rather than prove a pattern is
//! exploitable, a hotspot surfaces *hardcoded* regular expressions that look
//! complex enough to warrant a manual security review.
//!
//! This rule is **deprecated upstream** in favour of its modern successor
//! [`slow-regex`] (Sonar key S5852, already ported in this crate), which
//! detects the specific super-linear nested-quantifier shape rather than
//! flagging every complex pattern for review. It remains part of the plugin, so
//! it is ported here faithfully to its documented heuristic.
//!
//! ### Documented heuristic
//! A hardcoded pattern is *sensitive* when BOTH hold:
//! - it has **at least 3 characters**, AND
//! - it contains **at least 2** occurrences of the characters `*`, `+`, or `{`
//!   (counted together).
//!
//! The count is a raw character count over the pattern text; escaped
//! occurrences are not distinguished from literal ones (the documented
//! heuristic is a plain character tally). Example sensitive pattern: `(a+)*`.
//!
//! ### Three flagged shapes (hardcoded patterns only)
//! - a regex literal: `/(a+)+b/`
//! - `new RegExp("(a+)+b")` — constructor with a string-literal pattern
//! - `str.search("(a+)+b")`, `str.match("(a+)+b")`, `str.split("(a+)+b")` — a
//!   `.search`/`.match`/`.split` call with a string-literal first argument
//!
//! Dynamic values (variables, template literals, computed expressions) are
//! never analyzed: only literals can be tallied statically, and reporting
//! dynamic values would be false-positive-prone.
//!
//! **Flagged**:
//! - `/(a+)+b/` — 4 chars, two `+`
//! - `new RegExp("(a+)*")` — 5 chars, one `+` and one `*`
//! - `str.match("a{2}{3}")` — two `{`
//!
//! **Not flagged**:
//! - `/ab/` — fewer than 2 of `*`/`+`/`{`
//! - `/a+/` — only one `+`
//! - `/*/` — fewer than 3 characters
//! - `new RegExp(pattern)` — dynamic argument, skipped
//! - `str.replace("(a+)+b", "")` — `.replace` is not one of the targeted methods
//!
//! [`slow-regex`]: crate::rules::slow_regex

use oxc_ast::ast::{Argument, CallExpression, Expression, NewExpression, RegExpLiteral};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "regular-expr";

/// Returns `true` when `pattern` matches the documented S4784 heuristic: at
/// least 3 characters AND at least 2 occurrences of `*`, `+`, or `{` combined.
fn pattern_is_sensitive(pattern: &str) -> bool {
    if pattern.chars().count() < 3 {
        return false;
    }
    let quantifier_chars = pattern
        .chars()
        .filter(|&c| c == '*' || c == '+' || c == '{')
        .count();
    quantifier_chars >= 2
}

/// Extracts the string-literal value of the first argument, or `None` when the
/// first argument is missing or not a string literal (dynamic, skipped).
fn first_string_arg<'a>(args: &'a [Argument<'a>]) -> Option<&'a str> {
    match args.first()? {
        Argument::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

impl Scanner<'_> {
    /// Flags a regex literal whose pattern text matches the S4784 heuristic.
    pub(crate) fn check_regular_expr_literal(&mut self, it: &RegExpLiteral<'_>) {
        if pattern_is_sensitive(it.regex.pattern.text.as_str()) {
            self.report(RULE_NAME, "regularExpr", it.span);
        }
    }

    /// Flags `new RegExp("<sensitive>")` with a hardcoded string-literal pattern.
    pub(crate) fn check_regular_expr_new(&mut self, it: &NewExpression<'_>) {
        let Expression::Identifier(callee) = it.callee.get_inner_expression() else {
            return;
        };
        if callee.name != "RegExp" {
            return;
        }
        if first_string_arg(&it.arguments).is_some_and(pattern_is_sensitive) {
            self.report(RULE_NAME, "regularExpr", it.span);
        }
    }

    /// Flags `str.search/match/split("<sensitive>")` with a hardcoded
    /// string-literal first argument.
    pub(crate) fn check_regular_expr_call(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let property = member.property.name.as_str();
        if !matches!(property, "search" | "match" | "split") {
            return;
        }
        if first_string_arg(&it.arguments).is_some_and(pattern_is_sensitive) {
            self.report(RULE_NAME, "regularExpr", it.span);
        }
    }
}
