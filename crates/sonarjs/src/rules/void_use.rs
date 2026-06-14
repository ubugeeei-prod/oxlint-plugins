//! Rule `void-use` (SonarJS key S3735).
//!
//! Clean-room port. The `void` operator evaluates its argument and
//! unconditionally returns `undefined`. Almost every use of `void` is a code
//! smell: the intent is either to discard a value (use a statement instead) or
//! to obtain `undefined` (write `undefined` directly in modern JS). Neither
//! purpose requires the operator.
//!
//! ## Single exemption: `void 0`
//!
//! The idiom `void 0` (and its parenthesised variant `void (0)`) is treated as
//! a no-op because it is the historical safe way to produce `undefined` and
//! appears in a large amount of transpiled / legacy output. Both forms are
//! exempt via `get_inner_expression()` unwrapping followed by a numeric-literal
//! zero check.
//!
//! **Noncompliant** (flagged):
//! ```js
//! void foo();      // discarding a return value — use a statement
//! void x;          // not `void 0` — no purpose
//! void 1;          // numeric literal but not 0
//! ```
//!
//! **Compliant** (not flagged):
//! ```js
//! void 0;          // canonical undefined idiom — exempt
//! void (0);        // parenthesised zero — exempt (unwrapped by get_inner_expression)
//! !x;              // different unary operator
//! typeof x;        // different unary operator
//! -x;              // different unary operator
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description (S3735) only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, UnaryExpression};
use oxc_syntax::operator::UnaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "void-use";

impl Scanner<'_> {
    pub(crate) fn check_void_use(&mut self, expr: &UnaryExpression<'_>) {
        if expr.operator != UnaryOperator::Void {
            return;
        }
        // Exempt `void 0` and `void (0)` — the canonical undefined idiom.
        match expr.argument.get_inner_expression() {
            Expression::NumericLiteral(n) if n.value == 0.0 => return,
            _ => {}
        }
        self.report(RULE_NAME, "voidUse", expr.span);
    }
}
