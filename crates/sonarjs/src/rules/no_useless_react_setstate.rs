//! Rule `no-useless-react-setstate` (SonarJS key S6443).
//!
//! Clean-room port from public RSPEC S6443 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! React's `useState` returns a `[state, setter]` pair. Calling the setter with
//! the current value of its own paired state variable is a no-op: React compares
//! the next value with the current one (`Object.is`) and bails out of the
//! re-render. Such a call (`setV(v)`) is therefore dead code and almost always a
//! mistake — the author meant to pass a derived or different value.
//!
//! ## Zero-FP pair-matching detection
//!
//! Detection is scoped to a single function body. Two passes run:
//!
//! 1. Collect pairs from the body's DIRECT `VariableDeclarator`s whose `init` is
//!    a call to the identifier `useState` (or the member `React.useState`) and
//!    whose `id` is a two-element array pattern of plain binding identifiers
//!    `[stateName, setterName]`. Each `(stateName, setterName)` pair is recorded.
//!
//! 2. Scan EVERY descendant `CallExpression` of the body — including calls nested
//!    inside event handlers, effects, `.map` callbacks, and JSX — for a call
//!    whose callee is a plain identifier equal to a recorded `setterName` and
//!    whose SINGLE argument is a plain identifier equal to that pair's
//!    `stateName`. The call's span is reported.
//!
//! Because both names in a pair are bound by the same `const [state, setter] =
//! useState(...)` destructuring, the state binding is `const`, so an exact
//! `setter(state)` match is a guaranteed no-op — zero false positives. Pairs are
//! recorded per body, so a setter from one component cannot flag a call that uses
//! another component's state. Nested components declare their own pairs and are
//! scanned independently when the traversal reaches them.
//!
//! ## Flagged
//! ```jsx
//! function C() {
//!   const [v, setV] = useState(0);
//!   return <button onClick={() => setV(v)} />; // no-op: passes current state
//! }
//! ```
//!
//! ## Not Flagged
//! ```jsx
//! function C() {
//!   const [v, setV] = useState(0);
//!   setV(v + 1); // a different (derived) value
//!   setV(other); // a different variable
//!   setV();      // no argument
//!   setV(v, x);  // extra argument
//!   return null;
//! }
//! ```

use oxc_ast::ast::{BindingPattern, CallExpression, Expression, Statement};
use oxc_ast_visit::{Visit, walk};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-useless-react-setstate";

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

/// Returns the `(state, setter)` binding names of a `useState` destructuring
/// `const [state, setter] = useState(...)`, or `None` when the binding shape
/// does not match (both elements must be plain binding identifiers).
fn use_state_pair<'a>(id: &'a BindingPattern<'a>) -> Option<(&'a str, &'a str)> {
    let BindingPattern::ArrayPattern(array) = id else {
        return None;
    };
    if array.elements.len() < 2 {
        return None;
    }
    let Some(BindingPattern::BindingIdentifier(state)) = &array.elements[0] else {
        return None;
    };
    let Some(BindingPattern::BindingIdentifier(setter)) = &array.elements[1] else {
        return None;
    };
    Some((state.name.as_str(), setter.name.as_str()))
}

/// Walks a function body collecting the spans of every `setter(state)` no-op
/// call for a known set of `useState` `(state, setter)` pairs.
struct SetstateCallCollector<'a, 'p> {
    pairs: &'p [(&'a str, &'a str)],
    reports: SmallVec<[Span; 4]>,
}

impl<'a, 'p> SetstateCallCollector<'a, 'p> {
    /// Records `call`'s span when it is a `setter(state)` no-op for some pair.
    fn collect_call(&mut self, call: &CallExpression<'a>) {
        let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
            return;
        };
        if call.arguments.len() != 1 {
            return;
        }
        let Some(arg_expr) = call.arguments[0].as_expression() else {
            return;
        };
        let Expression::Identifier(arg) = arg_expr.get_inner_expression() else {
            return;
        };
        for (state, setter) in self.pairs {
            if callee.name.as_str() == *setter && arg.name.as_str() == *state {
                self.reports.push(call.span);
            }
        }
    }
}

impl<'a, 'p> Visit<'a> for SetstateCallCollector<'a, 'p> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.collect_call(call);
        walk::walk_call_expression(self, call);
    }
}

impl<'a> Scanner<'a> {
    pub(crate) fn check_no_useless_react_setstate(&mut self, statements: &[Statement<'a>]) {
        // Pass 1: collect (state, setter) pairs declared directly in this body.
        let mut pairs: SmallVec<[(&str, &str); 4]> = SmallVec::new();
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
                if let Some(pair) = use_state_pair(&declarator.id) {
                    pairs.push(pair);
                }
            }
        }
        if pairs.is_empty() {
            return;
        }

        // Pass 2: scan all descendant calls (handlers, effects, JSX, ...) for a
        // `setter(state)` no-op against any recorded pair.
        let mut collector = SetstateCallCollector {
            pairs: &pairs,
            reports: SmallVec::new(),
        };
        for stmt in statements {
            collector.visit_statement(stmt);
        }
        for span in collector.reports {
            self.report(RULE_NAME, "noUselessReactSetstate", span);
        }
    }
}
