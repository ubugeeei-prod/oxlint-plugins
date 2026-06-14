//! Rule `no-misleading-array-reverse` (SonarJS key S4043).
//!
//! Clean-room port. Most of JavaScript's Array methods return an altered copy
//! while leaving the source array intact; `reverse()` and `sort()` do not — they
//! mutate the receiver in place *and* return a reference to that same array.
//! Consuming the return value (assigning it, initialising a variable with it)
//! is misleading because a maintainer may assume the original array is left
//! untouched, when in fact it has been mutated.
//!
//! This rule reports when the return value of a `.reverse()` or `.sort()` call
//! is **assigned** — i.e. it is the initializer of a variable declarator or the
//! right-hand side of a plain `=` assignment — AND the receiver is *provably* a
//! reference to a pre-existing array (an identifier that resolves to a
//! never-reassigned binding whose initializer is an array literal). A bare
//! side-effecting statement (`arr.reverse();`) is fine and never flagged.
//!
//! The receiver must be a named array variable, not a freshly-created array
//! expression: reversing a fresh copy (`[...a].reverse()`, `[1, 2].reverse()`)
//! mutates nothing the caller still holds elsewhere, so it is not misleading and
//! is deliberately NOT flagged. When the receiver's array-ness cannot be proven
//! (function parameters, imports, reassigned variables, member access, call
//! results) the call is conservatively skipped to avoid false positives.
//!
//! Behaviour is reproduced from the public SonarSource rule documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `const a = [1, 2, 3]; const b = a.reverse();` — assigns the mutated array
//! - `const a = [1, 2, 3]; let b; b = a.sort();` — same via plain assignment
//!
//! ## Not flagged
//! - `const a = [1, 2, 3]; a.reverse();` — side-effect statement, value unused
//! - `const b = [1, 2].reverse();` — receiver is a fresh array literal
//! - `function f(a) { return a.reverse(); }` — `a` is a parameter, not provably an array
//! - `const b = obj.reverse();` — receiver is not a provable array

use oxc_ast::ast::Expression;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-misleading-array-reverse";

impl<'a> Scanner<'a> {
    /// Reports when `value` is a `.reverse()` / `.sort()` call whose receiver is
    /// a provable array reference, meaning the (consumed) return value is the
    /// in-place-mutated original array.
    pub(crate) fn check_no_misleading_array_reverse(&mut self, value: &Expression<'a>) {
        let Expression::CallExpression(call) = value.get_inner_expression() else {
            return;
        };
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "reverse" && method != "sort" {
            return;
        }
        let Expression::Identifier(ident) = member.object.get_inner_expression() else {
            return;
        };
        let Some(init) = self.resolve_identifier_initializer(ident) else {
            return;
        };
        if !matches!(init.get_inner_expression(), Expression::ArrayExpression(_)) {
            return;
        }
        self.report(RULE_NAME, "misleadingReverse", call.span);
    }
}
