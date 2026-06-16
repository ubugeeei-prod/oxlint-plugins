//! Rule `no-globals-shadowing` (SonarJS key S2137).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC description
//! for S2137 ("Special identifiers should not be bound or assigned") only; no
//! upstream SonarJS source, tests, fixtures, or message strings were consulted
//! or copied.
//!
//! ## What is flagged
//!
//! Binding or assigning the special JavaScript identifiers `eval` and
//! `arguments`, or the global value properties `undefined`, `NaN`, and
//! `Infinity`, is error-prone — and for `eval`/`arguments` it is outright
//! forbidden in strict mode. Rebinding these names hides a fundamental part of
//! the language from the surrounding code and almost always indicates a bug.
//!
//! ### Protected name set
//!
//! This rule targets a deliberately conservative core set:
//!
//! `eval`, `arguments`, `undefined`, `NaN`, `Infinity`.
//!
//! The RSPEC text also alludes to a broader set of global objects/functions
//! whose shadowing is debatable (and which overlap with `no-built-in-override`,
//! S2424). We intentionally **under-report** that broader set here: the five
//! names above cover every documented Noncompliant example and keep the rule
//! zero-false-positive.
//!
//! ## Three contexts
//!
//! 1. **Binding** (`visit_binding_identifier`): any `BindingIdentifier` whose
//!    name is protected. This covers `let`/`const`/`var` declarations, function
//!    and class names, function-expression names, function parameters, and
//!    `catch` parameters. E.g. `let eval;`, `function arguments() {}`,
//!    `const y = function eval() {};`, `function x(eval) {}`,
//!    `try {} catch (arguments) {}`.
//!
//! 2. **Assignment** (`visit_assignment_expression`): a bare assignment whose
//!    left-hand side is a plain identifier target with a protected name.
//!    E.g. `eval = 17;`. Member-expression targets (`window.eval = ...`) are
//!    not flagged — only bare identifier targets.
//!
//! 3. **Update** (`visit_update_expression`): an increment/decrement whose
//!    operand is a plain identifier with a protected name. E.g. `arguments++`,
//!    `++eval`.
//!
//! ## Reads are not flagged
//!
//! Only *binding* and *assignment/update targets* are reported. Plain reads of
//! these identifiers are perfectly fine and are never flagged: a bare reference
//! (`var x = undefined;`), or a member access such as `arguments.length` inside
//! a function, produces no diagnostic.
//!
//! ## Flagged
//!
//! ```js
//! eval = 17;                       // assignment
//! arguments++;                     // update
//! ++eval;                          // update
//! let eval;                        // binding
//! try {} catch (arguments) {}      // catch binding
//! function x(eval) {}              // param binding
//! function arguments() {}          // function-name binding
//! const y = function eval() {};    // function-expression-name binding
//! ```
//!
//! ## Not flagged
//!
//! ```js
//! var x = undefined;                          // read
//! function f() { return arguments.length; }   // read of arguments
//! let result;                                 // unprotected name
//! ```

use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, BindingIdentifier, SimpleAssignmentTarget,
    UpdateExpression,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-globals-shadowing";

/// The conservative core set of special identifiers protected by S2137.
///
/// This is a deliberate under-report of the broader global set the RSPEC
/// alludes to; these five names cover every documented example and keep the
/// rule zero-false-positive.
fn is_protected_special_identifier(name: &str) -> bool {
    matches!(
        name,
        "eval" | "arguments" | "undefined" | "NaN" | "Infinity"
    )
}

impl Scanner<'_> {
    pub(crate) fn check_no_globals_shadowing_binding(&mut self, id: &BindingIdentifier<'_>) {
        if !is_protected_special_identifier(id.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noGlobalsShadowing", id.span);
    }

    pub(crate) fn check_no_globals_shadowing_assignment(
        &mut self,
        assign: &AssignmentExpression<'_>,
    ) {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
            return;
        };
        if !is_protected_special_identifier(id.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noGlobalsShadowing", assign.span);
    }

    pub(crate) fn check_no_globals_shadowing_update(&mut self, update: &UpdateExpression<'_>) {
        let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &update.argument else {
            return;
        };
        if !is_protected_special_identifier(id.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noGlobalsShadowing", update.span);
    }
}
