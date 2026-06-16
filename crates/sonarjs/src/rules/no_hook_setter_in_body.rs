//! Rule `no-hook-setter-in-body` (SonarJS key S6442).
//!
//! Clean-room port from public RSPEC S6442 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Calling a React `useState` setter directly in a component's body schedules a
//! state update on every render. Because that update causes another render, the
//! component re-renders forever. The setter should instead be called from an
//! event handler, an effect, or another callback so that it runs in response to
//! a discrete event rather than on every render.
//!
//! ## Zero-FP function-local detection
//!
//! Detection is scoped to a single function body and never crosses scope
//! boundaries. For each function (or block-bodied arrow function), two passes
//! run over the body's DIRECT statements only:
//!
//! 1. Collect setter names: a `VariableDeclarator` whose `init` is a call to the
//!    identifier `useState` (or the member `React.useState`) and whose `id` is an
//!    array pattern with at least two elements where the second element is a
//!    plain binding identifier. That binding's name is recorded as a setter for
//!    this body.
//! 2. Flag calls: a direct top-level `ExpressionStatement` whose expression is a
//!    call to a plain identifier matching one of the recorded setter names.
//!
//! Because only the direct statements of the same body are inspected, a setter
//! call nested inside an inner function/arrow (event handler, `useEffect`
//! callback, `.map`), inside an `if`/loop/`try`, or inside JSX is never reached
//! and therefore never flagged. Setters are recorded per body, so a setter from
//! one component cannot flag a call in another.
//!
//! ## Flagged
//! ```jsx
//! function C() {
//!   const [v, setV] = useState(0);
//!   setV(1); // direct call in the component body
//!   return null;
//! }
//! ```
//!
//! ## Not Flagged
//! ```jsx
//! function C() {
//!   const [v, setV] = useState(0);
//!   const onClick = () => setV(1); // inside an event handler
//!   useEffect(() => setV(1));      // inside an effect callback
//!   if (cond) setV(1);            // inside a conditional
//!   return null;
//! }
//! ```

use oxc_ast::ast::{BindingPattern, Expression, Statement};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-hook-setter-in-body";

/// Returns `true` when `expr` is a call to `useState` or `React.useState`.
fn is_use_state_call(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expr else {
        return false;
    };
    match call.callee.get_inner_expression() {
        Expression::Identifier(ident) => ident.name.as_str() == "useState",
        Expression::StaticMemberExpression(member) => {
            member.property.name.as_str() == "useState"
                && matches!(member.object.get_inner_expression(),
                    Expression::Identifier(obj) if obj.name.as_str() == "React")
        }
        _ => false,
    }
}

/// Returns the name of the second array-pattern element (the setter binding) of
/// a `useState` destructuring, or `None` when the binding shape does not match.
fn use_state_setter_name<'a>(id: &'a BindingPattern<'_>) -> Option<&'a str> {
    let BindingPattern::ArrayPattern(array) = id else {
        return None;
    };
    if array.elements.len() < 2 {
        return None;
    }
    let Some(BindingPattern::BindingIdentifier(setter)) = &array.elements[1] else {
        return None;
    };
    Some(setter.name.as_str())
}

impl<'a> Scanner<'a> {
    pub(crate) fn check_no_hook_setter_in_body(&mut self, statements: &[Statement<'a>]) {
        // Pass 1: collect useState setter names declared directly in this body.
        let mut setters: SmallVec<[&str; 4]> = SmallVec::new();
        for stmt in statements {
            let Statement::VariableDeclaration(decl) = stmt else {
                continue;
            };
            for declarator in &decl.declarations {
                let Some(init) = &declarator.init else {
                    continue;
                };
                if !is_use_state_call(init) {
                    continue;
                }
                if let Some(name) = use_state_setter_name(&declarator.id) {
                    setters.push(name);
                }
            }
        }
        if setters.is_empty() {
            return;
        }

        // Pass 2: flag a direct top-level expression statement that calls a
        // recorded setter by its plain identifier name.
        for stmt in statements {
            let Statement::ExpressionStatement(expr_stmt) = stmt else {
                continue;
            };
            let Expression::CallExpression(call) = expr_stmt.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                continue;
            };
            if setters.contains(&callee.name.as_str()) {
                self.report(RULE_NAME, "noHookSetterInBody", call.span);
            }
        }
    }
}
