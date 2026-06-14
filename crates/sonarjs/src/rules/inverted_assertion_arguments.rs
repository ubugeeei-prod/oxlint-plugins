//! Rule `inverted-assertion-arguments` (SonarJS key S3415).
//!
//! Clean-room port. Chai's `assert` interface takes its arguments in a fixed
//! order: for a binary assertion such as `assert.equal(actual, expected)` the
//! value under test (`actual`) comes first and the expected value — typically a
//! literal constant — comes second. Passing them inverted (the expected literal
//! first) still passes or fails correctly, but produces a confusing failure
//! message in which the "actual" and "expected" labels are swapped.
//!
//! Only the equality-comparison methods of the `assert` interface (`equal`,
//! `notEqual`, `strictEqual`, `notStrictEqual`, `deepEqual`, `notDeepEqual`,
//! `deepStrictEqual`, `notDeepStrictEqual`) are checked, because they are the
//! ones that follow the `(actual, expected)` convention. Non-equality
//! assertions (`include`, `isAbove`, `lengthOf`, …) legitimately take a literal
//! first argument and are never flagged.
//!
//! **Flagged** — a `CallExpression` whose callee is the static member expression
//! `assert.<equalityMethod>` (the object being the bare identifier `assert`,
//! Chai's `assert` interface) with at least two arguments where the FIRST
//! argument is a literal constant and the SECOND argument is NOT a literal
//! constant. The asymmetry is what signals an inversion:
//! - `assert.equal(42, x);`
//! - `assert.strictEqual('foo', bar);`
//!
//! **Not flagged**:
//! - `assert.equal(x, 42);` — arguments already in actual/expected order.
//! - `assert.equal(1, 2);` — both literals; nothing identifies which is the
//!   actual value, so this is not treated as an inversion (conservative).
//! - `assert.equal(x, y);` — neither argument is a literal.
//! - `assert.ok(x);` — a single argument has no actual/expected pair.
//! - `assert.include('foobar', x);` — `include` is not an equality method; its
//!   first argument is the haystack, so a literal there is legitimate.
//! - `foo(42, x);` — not an `assert` member call.
//! - `chai.assert.equal(42, x);` — the object of the callee is itself a member
//!   expression, not the bare identifier `assert`; out of scope for this
//!   syntactic check.
//! - `assert.equal(...xs, x);` — a spread element is not a plain expression.
//!
//! ## What counts as a literal constant
//! Numeric, string, boolean, null, BigInt, and regular-expression literals; a
//! template literal with no `${...}` substitutions; and the `undefined`
//! identifier. These are the unambiguous compile-time constants a developer
//! would write as the *expected* value.
//!
//! Behaviour is reproduced from the public RSPEC S3415 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "inverted-assertion-arguments";

/// Returns whether `method` is a Chai/Node `assert` equality-comparison method
/// that follows the `(actual, expected)` argument convention. Only these are
/// checked: for non-equality assertions (`include`, `isAbove`, `lengthOf`,
/// `property`, `match`, …) a literal first argument is perfectly legitimate
/// (e.g. `assert.include('foobar', x)`), so flagging them would be a false
/// positive.
fn is_equality_assertion_method(method: &str) -> bool {
    matches!(
        method,
        "equal"
            | "notEqual"
            | "strictEqual"
            | "notStrictEqual"
            | "deepEqual"
            | "notDeepEqual"
            | "deepStrictEqual"
            | "notDeepStrictEqual"
    )
}

/// Returns whether `expr` is a literal compile-time constant: a numeric,
/// string, boolean, null, BigInt, or regular-expression literal; a template
/// literal with no substitutions; or the `undefined` identifier.
fn is_literal_constant(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::Identifier(ident) => ident.name == "undefined",
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_inverted_assertion_arguments(&mut self, call: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let Expression::Identifier(object) = member.object.get_inner_expression() else {
            return;
        };
        if object.name != "assert" {
            return;
        }
        if !is_equality_assertion_method(member.property.name.as_str()) {
            return;
        }
        let [first, second, ..] = call.arguments.as_slice() else {
            return;
        };
        if first.is_spread() || second.is_spread() {
            return;
        }
        let (Some(first_expr), Some(second_expr)) = (first.as_expression(), second.as_expression())
        else {
            return;
        };
        if is_literal_constant(first_expr) && !is_literal_constant(second_expr) {
            self.report(RULE_NAME, "invertedArguments", call.span);
        }
    }
}
