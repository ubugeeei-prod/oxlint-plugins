//! Rule `no-extra-arguments` (SonarJS key S930).
//!
//! Clean-room port. Calling a function with more positional arguments than the
//! function declares as parameters is almost always a bug — the extra values are
//! silently ignored by the runtime. This rule flags such calls.
//!
//! ## Conservative scope (zero-false-positive design)
//!
//! The rule is semantic: it resolves the callee to a function definition.
//! Resolution is performed ONLY when the callee is an `Identifier` that resolves
//! (via `resolve_identifier_initializer`) to a `const`/`let`/`var`-initialised
//! `FunctionExpression` or `ArrowFunctionExpression`. Calls to unresolved callees,
//! method calls, computed callees, or function declarations are deliberately NOT
//! checked. Function declarations (e.g. `function f(a) {}`) are noted as a
//! follow-up improvement; implementing them would require resolving the identifier
//! through `scoping.symbol_declaration` → `AstKind::Function`, which adds
//! complexity and is left for a later PR.
//!
//! Additional zero-FP guards (skip the call when any holds):
//! - The called function's parameter list contains a rest element (`...rest`).
//!   A rest parameter collects any number of extra arguments, so the count
//!   comparison is meaningless.
//! - One of the call's arguments is a spread element (`...arr`). A spread makes
//!   the effective argument count unknown at static analysis time.
//! - The source text of the called function contains the word `"arguments"`.
//!   A function body that uses the `arguments` object may intentionally rely on
//!   extra arguments; this is a crude but zero-FP-safe text scan.
//!
//! The diagnostic is reported at the `CallExpression` span and the declared
//! parameter count is included in the diagnostic data (`value`) for use in the
//! message template.
//!
//! Behaviour is reproduced from the public RSPEC S930 documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `const f = function(a){}; f(1, 2);` — 1 declared param, 2 call args
//! - `const g = (a) => a; g(1, 2, 3);` — 1 declared param, 3 call args
//!
//! ## Not flagged
//! - `const f = (a, b) => {}; f(1, 2);` — exact arg/param match, not flagged
//! - `const f = (a) => {}; f(1);` — fewer args than params
//! - `const f = (...args) => {}; f(1, 2, 3);` — rest element absorbs extras
//! - `const f = function(){ return arguments.length; }; f(1, 2);` — uses `arguments`
//! - `g(1, 2);` where `g` is unresolved — callee cannot be resolved
//! - `const f = (a) => {}; f(...arr);` — spread argument, count is unknown

use compact_str::ToCompactString;
use oxc_ast::ast::{CallExpression, Expression};

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "no-extra-arguments";

impl<'a> Scanner<'a> {
    /// Reports a call expression that passes more arguments than the called
    /// function declares parameters. See module doc for conservative scope.
    pub(crate) fn check_no_extra_arguments(&mut self, call: &CallExpression<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        // Only handle simple identifier callees that we can resolve.
        let ident = match call.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident,
            _ => return,
        };
        // Resolve identifier to its const/let/var initializer expression.
        let init = match self.resolve_identifier_initializer(ident) {
            Some(e) => e,
            None => return,
        };
        // Extract params and function span from FunctionExpression or ArrowFunctionExpression.
        let (params, func_span) = match init.get_inner_expression() {
            Expression::FunctionExpression(func) => (&func.params, func.span),
            Expression::ArrowFunctionExpression(func) => (&func.params, func.span),
            _ => return,
        };
        // Zero-FP guard: rest element absorbs any number of extra args.
        if params.rest.is_some() {
            return;
        }
        // Zero-FP guard: spread argument makes argument count unknown at call site.
        for arg in &call.arguments {
            if arg.is_spread() {
                return;
            }
        }
        // Early exit when argument count does not exceed param count.
        let param_count = params.items.len();
        if call.arguments.len() <= param_count {
            return;
        }
        // Zero-FP guard: function source text references the `arguments` object.
        // A function that reads `arguments` may intentionally consume extra args.
        // Crude text scan; may produce false negatives (aliases of `arguments`)
        // but never a false positive.
        if self.text(func_span).contains("arguments") {
            return;
        }
        let data = DiagnosticData {
            value: Some(param_count.to_compact_string()),
        };
        self.report_with_data(RULE_NAME, "extraArguments", data, call.span, None);
    }
}
