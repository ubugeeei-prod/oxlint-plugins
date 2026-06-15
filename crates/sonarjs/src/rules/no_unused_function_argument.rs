//! Rule `no-unused-function-argument` (SonarJS key S1172).
//!
//! Clean-room port. SonarJS S1172 flags function parameters that are never
//! referenced AND sit at the trailing end of the parameter list. Parameters
//! that are unused but appear earlier in the list than a used parameter cannot
//! be safely removed (removing them would shift argument positions at call
//! sites), so they are deliberately not flagged.
//!
//! ## Scope and guards
//!
//! - Only functions with a body are analysed; TypeScript declare/abstract
//!   overloads have no body and are skipped automatically.
//! - If the function has a rest element (`...rest`), the whole function is
//!   skipped (zero-FP: the rest element absorbs trailing arguments).
//! - If ANY parameter uses a destructuring pattern (`ObjectPattern` or
//!   `ArrayPattern`), the whole function is skipped (conservative: tracking
//!   individual bindings inside a destructure requires substantially more
//!   analysis and the risk of false positives is high).
//! - Parameters whose name begins with `_` are treated as intentionally unused
//!   placeholders; they stop the backwards walk (so earlier params are not
//!   flagged even if they are technically unused).
//! - Requires semantic analysis; when semantic data is absent nothing is
//!   emitted.
//!
//! ## Flagged
//! - `function f(a, b) { return a; }` — `b` is trailing unused → flagged
//! - `const g = (x, y, z) => x + y;` — `z` trailing unused → flagged
//!
//! ## Not flagged
//! - `function f(a, b) { return a + b; }` — all params used
//! - `function f(a, b) { return b; }` — `a` unused but not trailing (b used)
//! - `function f(_unused) {}` — underscore-prefixed, exempt
//! - `function f(...args) {}` — rest element, entire function skipped
//! - `function f({ x }) {}` — destructuring, entire function skipped
//! - `function f(a) { return inner(); function inner() { return a; } }` — used
//!   in nested function; semantic resolves all references regardless of nesting

use oxc_ast::ast::{ArrowFunctionExpression, BindingPattern, FormalParameters, Function};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-unused-function-argument";

impl<'a> Scanner<'a> {
    /// Entry point for regular function declarations and expressions.
    pub(crate) fn check_no_unused_function_argument_fn(&mut self, func: &Function<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        // Skip functions without a body (declare/abstract/overload signatures).
        if func.body.is_none() {
            return;
        }
        self.check_trailing_unused_params(&func.params);
    }

    /// Entry point for arrow function expressions.
    pub(crate) fn check_no_unused_function_argument_arrow(
        &mut self,
        arrow: &ArrowFunctionExpression<'a>,
    ) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        self.check_trailing_unused_params(&arrow.params);
    }

    /// Walk the parameter list from the end, reporting each consecutive
    /// trailing parameter that has zero references (stopping at the first
    /// used param, underscore-prefixed param, or non-simple-binding param).
    fn check_trailing_unused_params(&mut self, params: &FormalParameters<'a>) {
        // Zero-FP: rest element absorbs any number of trailing args.
        if params.rest.is_some() {
            return;
        }
        let scoping = match self.scoping {
            Some(s) => s,
            None => return,
        };
        // Zero-FP: any destructuring pattern complicates usage analysis.
        for param in &params.items {
            match &param.pattern {
                BindingPattern::BindingIdentifier(_) => {}
                _ => return,
            }
        }
        // Walk backwards from last parameter.
        for param in params.items.iter().rev() {
            let bi = match &param.pattern {
                BindingPattern::BindingIdentifier(bi) => bi,
                // All patterns were verified to be BindingIdentifier above;
                // this arm is unreachable but required for exhaustiveness.
                _ => return,
            };
            // Underscore-prefixed params are intentional placeholders; stop.
            if bi.name.starts_with('_') {
                break;
            }
            let symbol_id = match bi.symbol_id.get() {
                Some(id) => id,
                // Symbol not resolved (semantic absent or parse-only); stop.
                None => break,
            };
            if !scoping.symbol_is_unused(symbol_id) {
                // This param has at least one reference; stop the backwards walk.
                break;
            }
            self.report(RULE_NAME, "unusedFunctionArgument", bi.span);
        }
    }
}
