//! Rule `no-undefined-assignment` (SonarJS key S2138).
//!
//! Clean-room port. Explicitly assigning the bare identifier `undefined` to a
//! variable or property is a code smell. When a developer writes `x = undefined`
//! they intend to clear the value; the conventional idiom for that is `null`, or
//! simply omitting the initializer / leaving the variable unset. The literal
//! identifier `undefined` is a global property whose value *can* be shadowed in
//! pre-ES5 environments, and even in modern code the explicit assignment adds
//! noise without benefit.
//!
//! This port flags any `AssignmentExpression` with the plain `=` operator whose
//! right-hand side (unwrapped through parentheses) is the bare identifier
//! `undefined`. Variable declarator initializers (`let x = undefined;`) are a
//! documented follow-up and are deliberately NOT flagged here to keep the rule
//! focused.
//!
//! Exemptions:
//! - `void 0` — a recognised idiom for the undefined value;
//! - function calls (`x = foo()`) — only the identifier `undefined` is flagged;
//! - comparison uses (`x === undefined`) — read-only, not an assignment;
//! - compound operators (`x += undefined`) — flagging those is a follow-up.
//!
//! Behaviour is reproduced from the public RSPEC description (S2138) only; no
//! upstream source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `x = undefined;`
//! - `obj.prop = undefined;`
//!
//! ## Not flagged
//! - `let x = undefined;` — declarator initializer (follow-up)
//! - `x = null;` — correct conventional idiom
//! - `x = void 0;` — recognised idiom for undefined
//! - `if (x === undefined) {}` — comparison, not an assignment
//! - `x = foo();` — call expression, not the bare `undefined` identifier

use oxc_ast::ast::{AssignmentExpression, Expression};
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-undefined-assignment";

impl<'a> Scanner<'a> {
    /// Reports an `AssignmentExpression` with `=` whose right-hand side is the
    /// bare identifier `undefined`.
    pub(crate) fn check_no_undefined_assignment(&mut self, assign: &AssignmentExpression<'a>) {
        if assign.operator != AssignmentOperator::Assign {
            return;
        }
        let Expression::Identifier(ident) = assign.right.get_inner_expression() else {
            return;
        };
        if ident.name.as_str() == "undefined" {
            self.report(RULE_NAME, "noUndefinedAssignment", assign.span);
        }
    }
}
