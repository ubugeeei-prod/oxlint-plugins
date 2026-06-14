//! Rule `no-function-declaration-in-block` (SonarJS key S1530).
//!
//! Clean-room port. A function *declaration* nested directly inside a block
//! (an `if`/`else` body, a loop body, a `try`/`catch` body, or a bare block)
//! is confusing: historically its scope and hoisting behaviour differed between
//! engines and between sloppy and strict mode. Code that needs a function in
//! such a position should use a function *expression* assigned to a variable,
//! or move the declaration to the top level of the enclosing function or
//! module.
//!
//! **Flagged** — a `function` declaration that is a direct statement of a
//! block:
//! - `if (cond) { function f() {} }`
//! - `for (;;) { function f() {} }`
//! - `{ function f() {} }` (a bare block).
//!
//! **Not flagged**:
//! - `function f() {}` at the top level of a module or function body — the body
//!   of a function is not a block statement, so a declaration there is allowed.
//! - `if (cond) { const f = function () {}; }` — a function expression, not a
//!   declaration.
//! - `declare function f(): void;` inside a block — an ambient TypeScript
//!   declaration produces no runtime binding.
//!
//! Narrow form: only declarations whose immediate parent is a block statement
//! are reported; function declarations appearing directly in a `switch` case
//! body are a documented follow-up. Behaviour is reproduced from the public
//! RSPEC description (S1530) only; no upstream source, tests, fixtures, or
//! message strings were consulted or copied.

use oxc_ast::ast::{BlockStatement, Statement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-function-declaration-in-block";

impl Scanner<'_> {
    pub(crate) fn check_no_function_declaration_in_block(&mut self, block: &BlockStatement<'_>) {
        for statement in &block.body {
            let Statement::FunctionDeclaration(func) = statement else {
                continue;
            };
            if func.declare {
                continue;
            }
            self.report(RULE_NAME, "noFunctionDeclarationInBlock", func.span);
        }
    }
}
