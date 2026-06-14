//! Rule `no-for-in-iterable` (SonarJS key S4139).
//!
//! Clean-room port. A `for...in` loop iterates over the enumerable property
//! *keys* of an object — including inherited ones, and always as strings — which
//! is almost never what is wanted when iterating an array or other iterable. For
//! those, `for...of` (or plain index iteration) is the intended construct: it
//! yields the values in order and ignores non-index enumerable properties. Using
//! `for...in` on an array is therefore a likely defect.
//!
//! The upstream rule is type-aware: it flags a `for...in` whose right-hand side
//! has an array, string, or other iterable type. We have NO TypeScript type
//! information, so this port CONSERVATIVELY flags only when the iterated value is
//! *provably* an array: a direct array-literal right-hand side (`for (x in [1])`)
//! or an identifier that resolves via semantic analysis to a never-reassigned
//! binding whose initializer is an array literal (`const a = [1]; for (x in a)`).
//! Strings, typed arrays, and other iterables — and any array whose array-ness
//! cannot be proven (parameters, imports, reassigned variables, member access,
//! call results) — are deliberately NOT flagged, so we under-report relative to
//! upstream rather than risk false positives. This under-reporting is a known
//! parity risk against the type-aware upstream rule.
//!
//! Reported at the `ForInStatement` span.
//!
//! Behaviour is reproduced from the public SonarSource rule documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `for (const i in [1, 2, 3]) {}` — direct array-literal right-hand side
//! - `const a = [1, 2, 3]; for (const i in a) {}` — resolved array variable
//!
//! ## Not flagged
//! - `for (const k in obj) {}` where `obj` is `{ a: 1 }` — object, not an array
//! - `function f(p) { for (const k in p) {} }` — `p` is not a provable array
//! - `for (const x of [1, 2, 3]) {}` — a `for...of` loop, not `for...in`

use oxc_ast::ast::{Expression, ForInStatement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-for-in-iterable";

impl<'a> Scanner<'a> {
    /// Reports a `for...in` loop whose iterated value is a provable array.
    pub(crate) fn check_no_for_in_iterable(&mut self, stmt: &ForInStatement<'a>) {
        if !self.for_in_right_is_array(stmt.right.get_inner_expression()) {
            return;
        }
        self.report(RULE_NAME, "noForInIterable", stmt.span);
    }

    /// Conservatively decides whether `expr` is a provable array: a direct array
    /// literal, or an identifier resolving to an array-literal initializer.
    fn for_in_right_is_array(&self, expr: &Expression<'a>) -> bool {
        match expr {
            Expression::ArrayExpression(_) => true,
            Expression::Identifier(ident) => match self.resolve_identifier_initializer(ident) {
                Some(init) => matches!(init.get_inner_expression(), Expression::ArrayExpression(_)),
                None => false,
            },
            _ => false,
        }
    }
}
