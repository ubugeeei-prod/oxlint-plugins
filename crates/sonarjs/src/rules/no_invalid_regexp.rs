//! Rule `no-invalid-regexp` (SonarJS key S5856).
//!
//! Regular expressions should be syntactically valid. A regex *literal*
//! (`/.../`) that is invalid is already a JS syntax error and never appears
//! as an AST node, so this rule targets the **`RegExp` constructor** called
//! with a **string-literal** pattern: `new RegExp("...")` or `RegExp("...")`.
//!
//! If the first argument is not a string literal (variable, template literal,
//! regex literal, etc.) the rule is silent — it cannot statically validate
//! the pattern and reporting would produce false positives.
//!
//! The second argument (flags), when present and a string literal, is used
//! during validation; some patterns are only invalid under certain flag
//! combinations (e.g. `[\d-\D]` with `u`). If the second argument exists but
//! is NOT a string literal the rule validates the pattern with **empty flags**,
//! the safe choice that avoids false positives from dynamic flags while still
//! catching flag-independent syntax errors.
//!
//! Behaviour reproduced from public RSPEC S5856 only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.
//!
//! **Flagged**:
//! - `new RegExp("[")` — unclosed bracket
//! - `RegExp("(")` — unclosed group
//! - `new RegExp("a", "z")` — unknown flag `z`
//!
//! **Not flagged**:
//! - `new RegExp("\\d+")` — cooked value `\d+`, valid digit class
//! - `new RegExp("abc")` — valid literal pattern
//! - `new RegExp(someVar)` — dynamic argument, skipped
//! - `new RegExp("a{2,3}")` — valid bounded quantifier

use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, CallExpression, Expression, NewExpression};
use oxc_regular_expression::{LiteralParser, Options as RegExpOptions};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-invalid-regexp";

/// Extracts the flags string from the second argument of the call/new args slice.
/// Returns `""` if there is no second argument or if it is not a string literal.
fn flags_from_args<'a>(args: &[Argument<'a>]) -> &'a str {
    let Some(second) = args.get(1) else {
        return "";
    };
    match second {
        Argument::StringLiteral(flags_lit) => flags_lit.value.as_str(),
        _ => "",
    }
}

/// Returns `true` when the arguments represent an invalid `RegExp` call.
fn is_invalid_regexp(args: &[Argument<'_>]) -> bool {
    let Some(first) = args.first() else {
        return false;
    };
    let Argument::StringLiteral(pattern_lit) = first else {
        return false;
    };
    let pattern = pattern_lit.value.as_str();
    let flags = flags_from_args(args);
    let allocator = Allocator::default();
    LiteralParser::new(&allocator, pattern, Some(flags), RegExpOptions::default())
        .parse()
        .is_err()
}

impl Scanner<'_> {
    /// Checks a bare `RegExp(...)` function call for an invalid string-literal
    /// pattern or flags argument.
    pub(crate) fn check_no_invalid_regexp_call(&mut self, it: &CallExpression<'_>) {
        let Expression::Identifier(callee) = it.callee.get_inner_expression() else {
            return;
        };
        if callee.name != "RegExp" {
            return;
        }
        if is_invalid_regexp(&it.arguments) {
            self.report(RULE_NAME, "invalidRegExp", it.span);
        }
    }

    /// Checks a `new RegExp(...)` constructor expression for an invalid
    /// string-literal pattern or flags argument.
    pub(crate) fn check_no_invalid_regexp_new(&mut self, it: &NewExpression<'_>) {
        let Expression::Identifier(callee) = it.callee.get_inner_expression() else {
            return;
        };
        if callee.name != "RegExp" {
            return;
        }
        if is_invalid_regexp(&it.arguments) {
            self.report(RULE_NAME, "invalidRegExp", it.span);
        }
    }
}
