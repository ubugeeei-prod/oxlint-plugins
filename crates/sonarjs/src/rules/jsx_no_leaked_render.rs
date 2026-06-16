//! Rule `jsx-no-leaked-render` (SonarJS key S6439).
//!
//! Clean-room port. Flags a logical-AND expression used to conditionally render
//! JSX where the left operand is numeric. In React, `{value && <JSX/>}` renders
//! the left operand itself when it is falsy but not omitted; a numeric `0` is
//! falsy yet still rendered as the text `0` in the output. The canonical bug is
//! `{cart.length && <Cart/>}`, which renders `0` when the array is empty instead
//! of rendering nothing.
//!
//! Behaviour derived from public RSPEC S6439 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged (zero-false-positive subset)
//!
//! The expression is reported only when **all** of the following hold:
//! - the operator is `&&` (`LogicalOperator::And`), and
//! - the right operand (after unwrapping parentheses / non-null assertions) is a
//!   `JSXElement` or `JSXFragment`, and
//! - the left operand (after the same unwrapping) is either
//!   - a static member access whose property name is `length`
//!     (the classic `.length && <JSX/>` numeric leak), or
//!   - a numeric literal.
//!
//! ```jsx
//! {items.length && <List/>}
//! {0 && <X/>}
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! // explicit boolean comparison — already safe
//! {items.length > 0 && <List/>}
//! {count !== 0 && <X/>}
//!
//! // plain identifier or non-`length` member — cannot prove numeric without types
//! {show && <X/>}
//! {props.visible && <X/>}
//!
//! // `||` operator, not `&&`
//! {items.length || <X/>}
//!
//! // right side is not JSX
//! cond && doThing();
//! ```

use oxc_ast::ast::{Expression, LogicalExpression};
use oxc_syntax::operator::LogicalOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "jsx-no-leaked-render";

impl Scanner<'_> {
    pub(crate) fn check_jsx_no_leaked_render(&mut self, expr: &LogicalExpression<'_>) {
        if !matches!(expr.operator, LogicalOperator::And) {
            return;
        }

        // The right operand must render JSX for the leak to matter.
        if !matches!(
            expr.right.get_inner_expression(),
            Expression::JSXElement(_) | Expression::JSXFragment(_)
        ) {
            return;
        }

        // The left operand must be provably numeric in a zero-false-positive
        // sense: either a `.length` member access or a numeric literal.
        let left_is_numeric = match expr.left.get_inner_expression() {
            Expression::StaticMemberExpression(member) => member.property.name == "length",
            Expression::NumericLiteral(_) => true,
            _ => false,
        };
        if !left_is_numeric {
            return;
        }

        self.report(RULE_NAME, "jsxNoLeakedRender", expr.span);
    }
}
