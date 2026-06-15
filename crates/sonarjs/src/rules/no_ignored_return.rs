//! Rule `no-ignored-return` (SonarJS key S2201).
//!
//! Clean-room port. The upstream `eslint-plugin-sonarjs` rule is type-aware
//! (it uses TypeScript type information to determine the receiver type). This
//! port implements a **zero-false-positive narrow subset**: it reports only
//! when the call receiver is a **literal** of an unambiguously-known type —
//! a string literal, a numeric literal, or an array expression — so the
//! receiver's type is statically certain without type inference. Calls through
//! a non-literal receiver (`foo.trim()`, `arr.map(fn)`) are never flagged,
//! because without type information we cannot confirm the receiver is actually
//! a built-in and not a user-defined class with a same-named method that
//! legitimately produces side effects.
//!
//! **Flagged**: an `ExpressionStatement` whose `expression` is a
//! `CallExpression` whose callee is a `StaticMemberExpression` where:
//!  - the object is a string literal and the method is a known pure
//!    `String.prototype` method, e.g. `"hello".trim();`
//!  - the object is a numeric literal and the method is a known pure
//!    `Number.prototype` method, e.g. `(1.5).toFixed(2);`
//!  - the object is an array expression and the method is a known pure
//!    non-mutating `Array.prototype` method, e.g. `[1, 2].map(x => x);`
//!
//! **Not flagged**:
//!  - `const y = "hello".trim();` — the return value is captured.
//!  - `foo.trim();` — non-literal receiver; type is unknown.
//!  - `arr.push(1);` — `push` is a mutating method with a side effect.
//!  - `[1, 2].sort();` — `sort` mutates in place; side effect is the point.
//!
//! Behaviour is reproduced from the public RSPEC description (S2201,
//! "Return values from functions without side effects should not be ignored")
//! only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied. The literal-receiver narrowing is this port's own
//! design to achieve zero false positives in the absence of type information.

use oxc_ast::ast::{Expression, ExpressionStatement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-ignored-return";

/// Returns `true` when `name` is a known pure `String.prototype` method whose
/// return value must not be discarded. All `String.prototype` methods are pure
/// because JS strings are immutable; the only observable result is the return
/// value.
fn is_pure_string_method(name: &str) -> bool {
    matches!(
        name,
        "trim"
            | "trimStart"
            | "trimEnd"
            | "trimLeft"
            | "trimRight"
            | "toUpperCase"
            | "toLowerCase"
            | "toLocaleLowerCase"
            | "toLocaleUpperCase"
            | "charAt"
            | "charCodeAt"
            | "codePointAt"
            | "indexOf"
            | "lastIndexOf"
            | "includes"
            | "startsWith"
            | "endsWith"
            | "slice"
            | "substring"
            | "substr"
            | "replace"
            | "replaceAll"
            | "match"
            | "matchAll"
            | "search"
            | "split"
            | "concat"
            | "repeat"
            | "padStart"
            | "padEnd"
            | "normalize"
            | "at"
            | "toString"
            | "valueOf"
    )
}

/// Returns `true` when `name` is a known pure `Number.prototype` method whose
/// return value must not be discarded.
fn is_pure_number_method(name: &str) -> bool {
    matches!(
        name,
        "toFixed" | "toPrecision" | "toExponential" | "toString" | "toLocaleString" | "valueOf"
    )
}

/// Returns `true` when `name` is a known pure (non-mutating) `Array.prototype`
/// method whose return value must not be discarded. Mutating methods (`push`,
/// `pop`, `splice`, `sort`, `reverse`, `fill`, `shift`, `unshift`,
/// `copyWithin`) are intentionally excluded because they produce observable
/// side effects on the array, so discarding their return value may be
/// intentional.
fn is_pure_array_method(name: &str) -> bool {
    matches!(
        name,
        "map"
            | "filter"
            | "find"
            | "findIndex"
            | "findLast"
            | "findLastIndex"
            | "every"
            | "some"
            | "reduce"
            | "reduceRight"
            | "slice"
            | "concat"
            | "flat"
            | "flatMap"
            | "join"
            | "indexOf"
            | "lastIndexOf"
            | "includes"
            | "at"
            | "entries"
            | "keys"
            | "values"
            | "toString"
    )
}

impl Scanner<'_> {
    pub(crate) fn check_no_ignored_return(&mut self, stmt: &ExpressionStatement<'_>) {
        let Expression::CallExpression(call) = stmt.expression.get_inner_expression() else {
            return;
        };
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let method_name = member.property.name.as_str();
        let is_pure = match member.object.get_inner_expression() {
            Expression::StringLiteral(_) => is_pure_string_method(method_name),
            Expression::NumericLiteral(_) => is_pure_number_method(method_name),
            Expression::ArrayExpression(_) => is_pure_array_method(method_name),
            _ => false,
        };
        if !is_pure {
            return;
        }
        self.report(RULE_NAME, "ignoredReturn", stmt.span);
    }
}
