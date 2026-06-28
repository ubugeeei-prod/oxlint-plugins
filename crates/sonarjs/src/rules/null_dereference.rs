//! Rule `null-dereference` (SonarJS key S2259).
//!
//! Clean-room port. A null pointer should never be dereferenced: reading a
//! property of — or calling a method on — a value that is `null` or `undefined`
//! throws a `TypeError` at runtime and is virtually always a bug.
//!
//! ## Narrow form
//!
//! The full SonarJS rule relies on a symbolic-execution / nullability dataflow
//! engine to prove that some *variable* can hold `null`/`undefined` at a
//! dereference site (e.g. inside an `if (x === null) { x.foo(); }` branch).
//! This Oxlint port has no type checker, dataflow, or nullability engine, so it
//! cannot reproduce that analysis without false positives.
//!
//! Instead it enforces the unambiguous, analysis-independent core of S2259: a
//! *member access whose object is syntactically the literal `null` or the
//! `undefined` global*. These dereferences always throw at runtime:
//!
//! ```js
//! null.foo;        // Noncompliant: always a TypeError
//! null.foo();      // Noncompliant (callee `null.foo` is the dereference)
//! undefined.bar;   // Noncompliant
//! (null).baz;      // Noncompliant (parentheses are stripped)
//! ```
//!
//! Only the *static* member form (`.name`) is handled, since computed member
//! access (`null["x"]`) is a distinct node type that the shared traversal does
//! not currently visit; it is a documented follow-up. Dereferences through
//! variables that *might* be null are intentionally NOT reported — that is the
//! dataflow part this port deliberately under-approximates, preferring zero
//! false positives.
//!
//! The `undefined` case matches the global identifier by name. Locally
//! shadowing `undefined` (`let undefined = obj; undefined.x`) is exceedingly
//! rare, itself non-conformant, and flagged by other rules; treating it as a
//! dereference of the global is an accepted, documented limitation.
//!
//! Behaviour is reproduced from the public RSPEC description (S2259) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, StaticMemberExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "null-dereference";

impl Scanner<'_> {
    pub(crate) fn check_null_dereference(&mut self, member: &StaticMemberExpression<'_>) {
        // `null?.foo` / `undefined?.foo` short-circuit to `undefined` and never
        // throw, so optional member access is never a null dereference.
        if member.optional {
            return;
        }
        let is_null_like = match member.object.get_inner_expression() {
            Expression::NullLiteral(_) => true,
            Expression::Identifier(ident) => ident.name.as_str() == "undefined",
            _ => false,
        };
        if !is_null_like {
            return;
        }
        self.report(RULE_NAME, "nullDereference", member.span);
    }
}
