//! Rule `no-array-delete` (SonarJS key S2870).
//!
//! Clean-room port. Using the `delete` operator on an array element
//! (`delete arr[i]`) removes the value at that index but leaves a *hole*: the
//! array's `length` is unchanged and iteration sees `undefined` at that
//! position. To actually remove an element and re-index the remainder, callers
//! should use `Array.prototype.splice()` (or rebuild the array). Applying
//! `delete` to an array element is therefore a likely defect.
//!
//! The upstream rule is type-aware: it flags a `delete` whose target object has
//! an array type. We have NO TypeScript type information, so this port
//! CONSERVATIVELY flags only when the deleted object is *provably* an array: a
//! direct array-literal object (`delete [1, 2][0]`) or an identifier that
//! resolves via semantic analysis to a never-reassigned binding whose
//! initializer is an array literal (`const a = [1, 2]; delete a[0]`). Any target
//! whose array-ness cannot be proven (function parameters, imports, reassigned
//! variables, member access, call results, etc.) is deliberately NOT flagged, so
//! we under-report relative to the type-aware upstream rule rather than risk
//! false positives. This under-reporting is a known parity risk.
//!
//! Only *computed* member access (`arr[i]`) is targeted — that is element
//! deletion by index. A static property delete (`arr.foo`) is not array-element
//! deletion and is not flagged.
//!
//! Reported at the `UnaryExpression` span (the whole `delete arr[i]`).
//!
//! Behaviour is reproduced from the public RSPEC description (S2870) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `const a = [1, 2, 3]; delete a[0];` — resolved array variable, computed
//! - `delete [1, 2][0];` — direct array-literal object, computed
//!
//! ## Not flagged
//! - `const o = { x: 1 }; delete o.x;` — object property, not an array element
//! - `const o = {}; delete o['x'];` — non-array computed target
//! - `delete a.foo;` — static member access, not element deletion
//! - `function f(p) { delete p[0]; }` — `p` is not a provable array

use oxc_ast::ast::{Expression, UnaryExpression};
use oxc_syntax::operator::UnaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-array-delete";

impl<'a> Scanner<'a> {
    /// Reports `delete arr[i]` when `arr` is a provable array.
    pub(crate) fn check_no_array_delete(&mut self, expr: &UnaryExpression<'a>) {
        if expr.operator != UnaryOperator::Delete {
            return;
        }
        let Expression::ComputedMemberExpression(member) = expr.argument.get_inner_expression()
        else {
            return;
        };
        if !self.delete_target_is_array(member.object.get_inner_expression()) {
            return;
        }
        self.report(RULE_NAME, "noArrayDelete", expr.span);
    }

    /// Conservatively decides whether `object` is a provable array: a direct
    /// array literal, or an identifier resolving to an array-literal initializer.
    fn delete_target_is_array(&self, object: &Expression<'a>) -> bool {
        match object {
            Expression::ArrayExpression(_) => true,
            Expression::Identifier(ident) => match self.resolve_identifier_initializer(ident) {
                Some(init) => matches!(init.get_inner_expression(), Expression::ArrayExpression(_)),
                None => false,
            },
            _ => false,
        }
    }
}
