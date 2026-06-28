//! Rule `stateful-regex` (SonarJS key S6351).
//!
//! Clean-room port. A regular expression that carries the global (`g`) or
//! sticky (`y`) flag is *stateful*: each successful call to
//! `RegExp.prototype.test()` or `RegExp.prototype.exec()` advances the
//! engine's hidden `lastIndex` cursor, so the *next* call resumes from where
//! the previous one stopped instead of from the start of the input.
//!
//! That cursor only causes surprising bugs when the same `RegExp` object is
//! reused, which happens when it is stored in a variable and later invoked:
//!
//! ```js
//! const isNumeric = /\d+/g;
//! isNumeric.test("42");  // true
//! isNumeric.test("42");  // false! lastIndex carried over from the first call
//! ```
//!
//! ## Narrow, false-positive-free subset
//!
//! To stay free of false positives without a dataflow/liveness engine this
//! port only reports a `recv.test(...)` or `recv.exec(...)` call when:
//!
//! 1. `recv` is a plain identifier (not an inline literal — an inline
//!    `/\d/g.test(x)` builds a fresh, stateless object each time), and
//! 2. that identifier resolves (via Oxc semantic scoping, so shadowing is
//!    handled by symbol identity) to a `const`/`let`/`var` declarator whose
//!    initializer is a regex literal — or `new RegExp(pattern, flags)` — that
//!    carries the `g` or `y` flag.
//!
//! Inline literals, dynamically built flag strings, parameters, and regexes
//! without `g`/`y` are intentionally not reported. We under-report rather than
//! over-report.
//!
//! Behaviour is reproduced from the public RSPEC description (S6351) and
//! independently authored examples only; no upstream source, tests, fixtures,
//! or message strings were consulted or copied.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, CallExpression, Expression, RegExpFlags};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "stateful-regex";

/// Returns `true` if the variable-declarator initializer `init` evaluates to a
/// regular expression carrying the global (`g`) or sticky (`y`) flag.
fn init_is_stateful_regex(init: &Expression<'_>) -> bool {
    match init.get_inner_expression() {
        Expression::RegExpLiteral(lit) => {
            lit.regex.flags.intersects(RegExpFlags::G | RegExpFlags::Y)
        }
        Expression::NewExpression(new_expr) => {
            // `new RegExp(pattern, "<flags>")` with a string-literal flags
            // argument containing `g` or `y`.
            let Expression::Identifier(callee) = new_expr.callee.get_inner_expression() else {
                return false;
            };
            if callee.name != "RegExp" {
                return false;
            }
            let Some(Argument::StringLiteral(flags)) = new_expr.arguments.get(1) else {
                return false;
            };
            flags.value.contains('g') || flags.value.contains('y')
        }
        _ => false,
    }
}

impl<'a> Scanner<'a> {
    pub(crate) fn check_stateful_regex(&mut self, call: &CallExpression<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        // Must be `<object>.test(...)` or `<object>.exec(...)`.
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "test" && method != "exec" {
            return;
        }
        // The receiver must be a plain identifier reference (a stored regex),
        // never an inline literal which would be stateless.
        let Expression::Identifier(ident) = member.object.get_inner_expression() else {
            return;
        };

        let (Some(scoping), Some(nodes)) = (self.scoping, self.nodes) else {
            return;
        };
        let Some(reference_id) = ident.reference_id.get() else {
            return;
        };
        let Some(symbol_id) = scoping.get_reference(reference_id).symbol_id() else {
            return;
        };
        let decl_node_id = scoping.symbol_declaration(symbol_id);
        let AstKind::VariableDeclarator(declarator) = nodes.get_node(decl_node_id).kind() else {
            return;
        };
        let Some(init) = declarator.init.as_ref() else {
            return;
        };
        if !init_is_stateful_regex(init) {
            return;
        }
        self.report(RULE_NAME, "statefulRegex", call.span);
    }
}
