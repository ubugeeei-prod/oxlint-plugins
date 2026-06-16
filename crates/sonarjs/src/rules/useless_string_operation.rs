//! Rule `useless-string-operation` (SonarJS key S1154, deprecated but retained
//! in the plugin).
//!
//! Clean-room port. Behaviour is reproduced from the public SonarSource RSPEC
//! documentation for S1154 only; no upstream source, tests, fixtures, or
//! message strings were consulted or copied.
//!
//! JavaScript strings are immutable, so every string method returns a **new**
//! string rather than mutating the receiver in place. Calling such a method as
//! a bare statement and discarding its result is therefore almost always a bug:
//! the author likely believed the call mutated the string. The public RSPEC
//! example is:
//!
//! ```js
//! str.toUpperCase(); // Noncompliant — result ignored, str is unchanged
//! str = str.toUpperCase(); // Compliant — the returned value is kept
//! ```
//!
//! ## Zero-false-positive subset
//!
//! Without type information we cannot prove that an arbitrary receiver is a
//! string. To stay zero-false-positive this port flags a call only when BOTH
//! hold:
//!
//! 1. The call appears at **statement level** as the discarded expression of an
//!    `ExpressionStatement` (its returned value is provably thrown away — it is
//!    not assigned, returned, passed as an argument, or part of a larger
//!    expression).
//! 2. The callee is a `StaticMemberExpression` whose property name is one of a
//!    fixed allowlist of **string-specific** pure methods — methods that exist
//!    only on `String.prototype` and are not also present on `Array.prototype`
//!    or commonly defined by user code.
//!
//! The allowlist: `toUpperCase`, `toLowerCase`, `toLocaleUpperCase`,
//! `toLocaleLowerCase`, `trim`, `trimStart`, `trimEnd`, `trimLeft`,
//! `trimRight`, `padStart`, `padEnd`, `charAt`, `charCodeAt`, `codePointAt`,
//! `normalize`, `substring`, `substr`, `repeat`.
//!
//! ## Why some method names are deliberately excluded
//!
//! Method names that are shared with `Array.prototype` (e.g. `slice`, `concat`,
//! `at`, `indexOf`, `includes`) or that are frequently user-defined / generic
//! (e.g. `replace`, `split`, `match`, `toString`) are **excluded** on purpose.
//! `arr.slice();` is a no-op on an array too, but the receiver could just as
//! easily be a custom object with a side-effecting `slice` method, so flagging
//! it risks false positives. Restricting the allowlist to methods that exist
//! only on strings keeps this conservative rule free of false positives at the
//! cost of under-reporting some genuine cases.
//!
//! ## Flagged
//! - `str.toUpperCase();` — string-specific pure method, result discarded
//! - `s.trim();` — same defect
//! - `name.padStart(4, "0");` — arguments are irrelevant; result is discarded
//!
//! ## Not flagged
//! - `str = str.toUpperCase();` — the returned value is assigned
//! - `return s.trim();` — the returned value is used
//! - `console.log(s.trim());` — the returned value is passed as an argument
//! - `arr.slice();` — `slice` is shared with arrays and is excluded
//! - `foo();` — not a member call
//! - `str.length;` — not a call

use oxc_ast::ast::{Expression, ExpressionStatement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "useless-string-operation";

/// Returns `true` when `name` is a pure method that exists only on
/// `String.prototype` (and not on `Array.prototype` or commonly user-defined),
/// so that discarding its result at statement level is a reliable defect
/// signal. See the module doc-comment for why array-shared / user-definable
/// names are deliberately omitted.
fn is_pure_string_method(name: &str) -> bool {
    matches!(
        name,
        "toUpperCase"
            | "toLowerCase"
            | "toLocaleUpperCase"
            | "toLocaleLowerCase"
            | "trim"
            | "trimStart"
            | "trimEnd"
            | "trimLeft"
            | "trimRight"
            | "padStart"
            | "padEnd"
            | "charAt"
            | "charCodeAt"
            | "codePointAt"
            | "normalize"
            | "substring"
            | "substr"
            | "repeat"
    )
}

impl<'a> Scanner<'a> {
    /// Reports a statement-level call to a string-specific pure method whose
    /// returned value is discarded.
    pub(crate) fn check_useless_string_operation(&mut self, it: &ExpressionStatement<'_>) {
        let Expression::CallExpression(call) = it.expression.get_inner_expression() else {
            return;
        };
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        if !is_pure_string_method(member.property.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "uselessStringOperation", call.span);
    }
}
