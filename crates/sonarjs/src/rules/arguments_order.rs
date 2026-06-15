//! Rule `arguments-order` (SonarJS key S2234).
//!
//! Clean-room port. When a function call's arguments are identifiers whose
//! names match the called function's parameter names but are supplied in a
//! different order, this is almost certainly a bug — the programmer has
//! accidentally transposed arguments.
//!
//! ## Transposition definition
//!
//! Let N be the number of arguments in the call. The call is flagged when ALL
//! of the following hold simultaneously:
//!
//! 1. N ≥ 2 (single-argument calls cannot be transpositions).
//! 2. Every argument is a plain `Identifier` (no spreads, literals, member
//!    expressions, or nested calls).
//! 3. The function's parameter list contains no rest element, and every one
//!    of the first N parameters is a simple `BindingIdentifier` (no
//!    destructuring patterns or default values that introduce sub-bindings).
//! 4. The multiset of the N argument names equals the multiset of the first N
//!    parameter names — i.e. the arguments are exactly those parameter names,
//!    possibly reordered.
//! 5. The ordered sequences differ (at least one position i where
//!    arg[i].name ≠ param[i].name).
//!
//! If the arg-name set differs from the param-name set in any way (the caller
//! uses unrelated variable names), the call is **not** flagged — there is no
//! evidence of transposition.
//!
//! ## Callee resolution (zero-FP guards)
//!
//! - **Path 1** — callee is a `const`/`let`/`var`-bound `FunctionExpression`
//!   or `ArrowFunctionExpression`. Resolved via
//!   [`Scanner::resolve_identifier_initializer`], which already guards against
//!   mutated bindings, non-`VariableDeclarator` declaration sites, and
//!   destructuring bindings.
//! - **Path 2** — callee is a `function` declaration. Resolved via
//!   `scoping.symbol_declaration` → `AstKind::Function`. Skipped when the
//!   binding is mutated (the declaration may no longer be authoritative).
//!
//! When the callee cannot be resolved (dynamic expression, imported function,
//! method call, mutated binding, etc.) the call is silently skipped.
//!
//! ## Flagged
//!
//! ```js
//! function create(width, height) {}
//! const width = 10, height = 5;
//! create(height, width);  // ← transposition: height/width are swapped
//! ```
//!
//! ## Not flagged
//!
//! ```js
//! // Correct order — argument sequence matches parameter sequence.
//! function create(w, h) {} const w = 10, h = 5; create(w, h);
//!
//! // Unrelated variable names — multiset differs, no signal of transposition.
//! function create(width, height) {} const w = 10, h = 5; create(w, h);
//!
//! // Single argument — cannot be a transposition.
//! function move(x) {} const x = 1; move(x);
//!
//! // Spread argument — effective argument count unknown.
//! function f(a, b) {} f(...arr);
//!
//! // Rest parameter — function accepts any number of args.
//! function g(...rest) {} g(a, b);
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description (S2234,
//! "Arguments should be passed in the correct order") only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, BindingPattern, CallExpression, Expression};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "arguments-order";

impl<'a> Scanner<'a> {
    /// Checks whether the arguments of `call` are the callee's parameter names
    /// supplied in transposed order.
    pub(crate) fn check_arguments_order(&mut self, call: &CallExpression<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }

        // Only handle simple identifier callees that can be resolved.
        let ident = match call.callee.get_inner_expression() {
            Expression::Identifier(ident) => ident,
            _ => return,
        };

        // Need at least 2 arguments; single-argument calls cannot be transpositions.
        let arg_count = call.arguments.len();
        if arg_count < 2 {
            return;
        }

        // Every argument must be a plain identifier (no spreads, literals, etc.).
        let mut arg_names: SmallVec<[&str; 8]> = SmallVec::new();
        for arg in &call.arguments {
            match arg {
                Argument::Identifier(id) => {
                    arg_names.push(id.name.as_str());
                }
                _ => return,
            }
        }

        // --- Path 1: callee is a const/let/var-bound FunctionExpression or Arrow ---
        if let Some(init_expr) = self.resolve_identifier_initializer(ident) {
            let params = match init_expr.get_inner_expression() {
                Expression::FunctionExpression(func) => &func.params,
                Expression::ArrowFunctionExpression(arrow) => &arrow.params,
                _ => return,
            };
            // Skip when a rest element is present.
            if params.rest.is_some() {
                return;
            }
            // Need at least as many params as arguments.
            if params.items.len() < arg_count {
                return;
            }
            let mut param_names: SmallVec<[&str; 8]> = SmallVec::new();
            for param in params.items.iter().take(arg_count) {
                match &param.pattern {
                    BindingPattern::BindingIdentifier(bi) => {
                        param_names.push(bi.name.as_str());
                    }
                    _ => return,
                }
            }
            flag_if_transposed(self, &arg_names, &param_names, call);
            return;
        }

        // --- Path 2: callee is a function declaration ---
        let scoping = match self.scoping {
            Some(s) => s,
            None => return,
        };
        let nodes = match self.nodes {
            Some(n) => n,
            None => return,
        };
        let reference_id = match ident.reference_id.get() {
            Some(id) => id,
            None => return,
        };
        let symbol_id = match scoping.get_reference(reference_id).symbol_id() {
            Some(id) => id,
            None => return,
        };
        // A reassigned binding may no longer refer to the original declaration.
        if scoping.symbol_is_mutated(symbol_id) {
            return;
        }
        let decl_node_id = scoping.symbol_declaration(symbol_id);
        let func = match nodes.get_node(decl_node_id).kind() {
            AstKind::Function(f) => f,
            _ => return,
        };
        let params = &func.params;
        // Skip when a rest element is present.
        if params.rest.is_some() {
            return;
        }
        // Need at least as many params as arguments.
        if params.items.len() < arg_count {
            return;
        }
        let mut param_names: SmallVec<[&str; 8]> = SmallVec::new();
        for param in params.items.iter().take(arg_count) {
            match &param.pattern {
                BindingPattern::BindingIdentifier(bi) => {
                    param_names.push(bi.name.as_str());
                }
                _ => return,
            }
        }
        flag_if_transposed(self, &arg_names, &param_names, call);
    }
}

/// Reports `call` when `arg_names` is a non-identity permutation of `param_names`
/// (same multiset, different order — a transposition).
fn flag_if_transposed<'a>(
    scanner: &mut Scanner<'a>,
    arg_names: &SmallVec<[&str; 8]>,
    param_names: &SmallVec<[&str; 8]>,
    call: &CallExpression<'a>,
) {
    // Fast path: ordered sequences already match — nothing to report.
    if arg_names == param_names {
        return;
    }
    // Check multiset equality by sorting copies of both name lists.
    let mut sorted_args: SmallVec<[&str; 8]> = arg_names.clone();
    let mut sorted_params: SmallVec<[&str; 8]> = param_names.clone();
    sorted_args.sort_unstable();
    sorted_params.sort_unstable();
    if sorted_args == sorted_params {
        scanner.report(RULE_NAME, "argumentsOrder", call.span);
    }
}
