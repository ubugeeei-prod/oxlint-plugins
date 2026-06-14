//! Rule `no-alphabetical-sort` (SonarJS key S2871).
//!
//! Clean-room port. `Array.prototype.sort()` and `Array.prototype.toSorted()`,
//! when called without a compare function, coerce each element to a string and
//! order them by their UTF-16 code units. For arrays of numbers (or other
//! non-string values) this is almost never what the author intends — e.g.
//! `[1, 30, 4, 21, 100000].sort()` yields `[1, 100000, 21, 30, 4]`. The rule
//! asks the author to supply an explicit compare function.
//!
//! This port reports a `.sort()` / `.toSorted()` call that has **no arguments**
//! (i.e. no compare function) AND whose receiver is *provably* an array: either
//! a direct array-literal receiver (`[3, 1, 2].sort()`) or an identifier that
//! resolves via semantic analysis to a never-reassigned binding whose
//! initializer is an array literal (`const a = [3, 1, 2]; a.sort();`). Unlike
//! `no-misleading-array-reverse`, a fresh array-literal receiver IS flagged
//! here: the defect is the missing comparator, independent of aliasing.
//!
//! When the receiver's array-ness cannot be proven (function parameters,
//! imports, reassigned variables, member access, call results, etc.) the call
//! is conservatively skipped to avoid false positives — the upstream rule is
//! type-aware, and without TypeScript type information we deliberately under-
//! report rather than risk flagging non-array `.sort()` methods.
//!
//! Behaviour is reproduced from the public SonarSource rule documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `[3, 1, 2].sort();` — array literal receiver, no compare function
//! - `const a = [3, 1, 2]; a.sort();` — resolved array variable, no comparator
//! - `[3, 1, 2].toSorted();` — same defect on the copying variant
//!
//! ## Not flagged
//! - `arr.sort((x, y) => x - y);` — a compare function is provided
//! - `['b', 'a'].sort();` — every element is a string literal; alphabetical
//!   order is the expected sort for strings (matches the type-aware upstream
//!   rule, which exempts string arrays)
//! - `obj.sort();` where `obj` is not a provable array
//! - `foo();` — not a `sort`/`toSorted` call

use oxc_ast::ast::{ArrayExpression, ArrayExpressionElement, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-alphabetical-sort";

/// Returns `true` when `element` is a string-literal array element.
fn is_string_literal_element(element: &ArrayExpressionElement<'_>) -> bool {
    matches!(element, ArrayExpressionElement::StringLiteral(_))
}

/// Decides whether an array-literal receiver makes a comparator-less sort a
/// likely defect. An array whose every element is a string literal is exempt:
/// alphabetical (UTF-16) ordering is the natural, expected sort for strings,
/// matching the type-aware upstream rule which does not flag string arrays. Any
/// other element kind (number, identifier, spread, …) — or a mix — keeps it
/// flagged. An empty array literal is exempt (nothing to sort surprisingly).
fn array_should_be_flagged(arr: &ArrayExpression<'_>) -> bool {
    !arr.elements.iter().all(is_string_literal_element)
}

impl<'a> Scanner<'a> {
    /// Reports a `.sort()` / `.toSorted()` call with no compare function whose
    /// receiver is a provable array.
    pub(crate) fn check_no_alphabetical_sort(&mut self, call: &CallExpression<'a>) {
        if !call.arguments.is_empty() {
            return;
        }
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "sort" && method != "toSorted" {
            return;
        }
        if !self.sort_receiver_is_array(member.object.get_inner_expression()) {
            return;
        }
        self.report(RULE_NAME, "provideCompareFunction", call.span);
    }

    /// Conservatively decides whether `receiver` is a provable array that should
    /// be flagged: a direct array literal, or an identifier resolving to an
    /// array-literal initializer — and, in either case, not an all-string-literal
    /// array (those are exempt; see [`array_should_be_flagged`]).
    fn sort_receiver_is_array(&self, receiver: &Expression<'a>) -> bool {
        match receiver {
            Expression::ArrayExpression(arr) => array_should_be_flagged(arr),
            Expression::Identifier(ident) => match self.resolve_identifier_initializer(ident) {
                Some(init) => match init.get_inner_expression() {
                    Expression::ArrayExpression(arr) => array_should_be_flagged(arr),
                    _ => false,
                },
                None => false,
            },
            _ => false,
        }
    }
}
