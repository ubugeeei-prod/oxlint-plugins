//! Rule `chai-determinate-assertion` (SonarJS key S6092).
//!
//! Clean-room port. Chai assertions should have only one reason to succeed.
//! Several negated or combined Chai assertions can pass for the *wrong* reason:
//! the assertion is non-deterministic or too permissive, so a green test does
//! not actually prove the property the author intended. For example,
//! `expect(x).to.not.throw(TypeError)` succeeds both when `x` throws nothing
//! *and* when `x` throws a different error type — two unrelated reasons — so it
//! fails to pin down a single deterministic condition.
//!
//! ## Expect-root gating (zero false positives)
//! Every match is gated on the assertion chain being rooted in a call to the
//! bare identifier `expect`. The [`analyze`] helper walks *down* the receiver
//! chain (descending through `StaticMemberExpression.object` and
//! `CallExpression.callee.object`, unwrapping parentheses via
//! `get_inner_expression`) and only reports when the chain bottoms out at
//! `expect(...)`. This keeps the rule conservative: it fires solely inside Chai
//! `expect(...)` assertion chains. The `.should` style (e.g.
//! `foo.should.not.throw(...)`) is intentionally out of scope and therefore
//! under-reported rather than risk false positives on unrelated member chains.
//!
//! ## Flagged patterns (all require an `expect(...)`-rooted chain)
//! 1. `expect(fn).to.not.throw(ReferenceError)` — negated `.throw` *with* an
//!    argument (succeeds when nothing is thrown OR a different error is thrown).
//! 2. `expect({a: 42}).to.not.include({b: 10, c: 20})` — negated `.include`
//!    whose argument is an object literal (succeeds if *any* key/value differs).
//! 3. `expect({a: 21}).to.not.have.property('b', 42)` — negated `.property`
//!    with two or more arguments (succeeds if the property is absent OR its
//!    value differs).
//! 4. `expect({a: 21}).to.not.have.ownPropertyDescriptor('b', {…})` — negated
//!    `.ownPropertyDescriptor` with two or more arguments.
//! 5. `expect([21, 42]).to.not.have.members([1, 2])` — negated `.members`.
//! 6. `expect(incThree).to.change(myObj, 'value').by(3)` — `.change(...).by(...)`
//!    (the `.by` assertion attached to a `.change` is non-deterministic).
//! 7. `expect(decThree).to.not.increase(myObj, 'value')` — negated `.increase`.
//! 8. `expect(incThree).to.not.decrease(myObj, 'value')` — negated `.decrease`.
//! 9. `expect(incThree).to.increase(myObj, 'value').but.not.by(1)` — negated
//!    `.by`.
//! 10. `expect(toCheck).to.not.be.finite` — the `.not.finite` property
//!     (use `.be.NaN` instead).
//!
//! ## Compliant counter-examples (not flagged)
//! - `expect(fn).to.throw(TypeError)` — `.throw` without `.not`.
//! - `expect({a: 21}).to.not.have.property('b')` — `.not.property` with ONE arg.
//! - `expect({a: 21}).to.not.have.ownPropertyDescriptor('b')` — ONE arg.
//! - `expect(incThree).to.increase(myObj, 'value').by(3)` — `.increase(...).by(...)`.
//! - `expect(decThree).to.decrease(myObj, 'value').by(3)` — `.decrease(...).by(...)`.
//! - `expect(doNothing).to.not.change(myObj, 'value')` — `.not.change` (no `.by`).
//! - `expect(toCheck).to.be.NaN` — `.be.NaN` instead of `.not.finite`.
//! - `foo.to.not.be.finite` — not rooted in `expect(...)`, so out of scope.
//!
//! Behaviour is reproduced from the public RSPEC S6092 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression, StaticMemberExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "chai-determinate-assertion";

/// Summary of a Chai assertion chain, produced by walking *down* a receiver
/// expression toward its root.
struct ChaiChain {
    /// The chain bottoms out at a `CallExpression` whose callee (after
    /// unwrapping parentheses) is the bare identifier `expect`.
    rooted_in_expect: bool,
    /// Some `StaticMemberExpression` in the chain has the property name `not`.
    negated: bool,
    /// Some `CallExpression` in the chain has a callee static-member property
    /// name `change`.
    contains_change_call: bool,
}

/// Walks DOWN the receiver chain starting at `expr` toward its root, recording
/// whether the chain is rooted in `expect(...)`, whether it contains a `.not`
/// member, and whether it contains a `.change(...)` call.
///
/// Unwraps each node with `get_inner_expression` (parentheses), descends into
/// `StaticMemberExpression.object` and `CallExpression.callee` (then its
/// `.object`), and stops when it reaches the `expect(...)` call (rooted) or a
/// node that is neither a member access nor a call (not rooted).
fn analyze(expr: &Expression<'_>) -> ChaiChain {
    let mut negated = false;
    let mut contains_change_call = false;
    let mut rooted_in_expect = false;
    let mut current = expr.get_inner_expression();

    loop {
        match current {
            Expression::StaticMemberExpression(member) => {
                if member.property.name == "not" {
                    negated = true;
                }
                current = member.object.get_inner_expression();
            }
            Expression::CallExpression(call) => {
                let callee = call.callee.get_inner_expression();
                match callee {
                    Expression::Identifier(ident) => {
                        if ident.name == "expect" {
                            rooted_in_expect = true;
                        }
                        break;
                    }
                    Expression::StaticMemberExpression(member) => {
                        if member.property.name == "change" {
                            contains_change_call = true;
                        }
                        current = member.object.get_inner_expression();
                    }
                    _ => break,
                }
            }
            _ => break,
        }
    }

    ChaiChain {
        rooted_in_expect,
        negated,
        contains_change_call,
    }
}

/// Returns whether the first argument of `call`, after unwrapping, is an object
/// literal expression.
fn first_arg_is_object(call: &CallExpression<'_>) -> bool {
    let Some(first) = call.arguments.first() else {
        return false;
    };
    let Some(expr) = first.as_expression() else {
        return false;
    };
    matches!(expr.get_inner_expression(), Expression::ObjectExpression(_))
}

impl Scanner<'_> {
    pub(crate) fn check_chai_determinate_assertion_call(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();
        let chain = analyze(&member.object);
        if !chain.rooted_in_expect {
            return;
        }
        let flag = match method {
            // (1) `.not.throw(arg)` — negated throw with at least one argument.
            "throw" => chain.negated && !it.arguments.is_empty(),
            // (2) `.not.include({...})` — negated include with an object literal.
            "include" => chain.negated && first_arg_is_object(it),
            // (3) `.not.property(a, b)` — negated property with 2+ arguments.
            "property" => chain.negated && it.arguments.len() >= 2,
            // (4) `.not.ownPropertyDescriptor(a, b)` — 2+ arguments.
            "ownPropertyDescriptor" => chain.negated && it.arguments.len() >= 2,
            // (5) `.not.members(...)`.
            "members" => chain.negated,
            // (7) `.not.increase(...)`.
            "increase" => chain.negated,
            // (8) `.not.decrease(...)`.
            "decrease" => chain.negated,
            // (9) `.not.by(...)`  OR  (6) `.change(...).by(...)`.
            "by" => chain.negated || chain.contains_change_call,
            _ => false,
        };
        if flag {
            self.report(RULE_NAME, "chaiDeterminateAssertion", it.span);
        }
    }

    pub(crate) fn check_chai_determinate_assertion_member(
        &mut self,
        it: &StaticMemberExpression<'_>,
    ) {
        // (10) `.not.finite` — the `finite` property on a negated expect chain.
        if it.property.name != "finite" {
            return;
        }
        let chain = analyze(&it.object);
        if chain.rooted_in_expect && chain.negated {
            self.report(RULE_NAME, "chaiDeterminateAssertion", it.span);
        }
    }
}
