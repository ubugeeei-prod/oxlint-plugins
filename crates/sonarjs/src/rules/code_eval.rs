//! Rule `code-eval` (SonarJS key S1523).
//!
//! Clean-room port. Dynamic code execution via `eval(...)` or the `Function`
//! constructor (`new Function(...)` / `Function(...)`) introduces serious
//! security risks: any string passed to these APIs is parsed and executed as
//! code, which can open the door to code-injection attacks and makes static
//! analysis impossible.
//!
//! **Flagged** — a call or `new` expression whose callee is one of the bare
//! identifiers `eval` or `Function`:
//! - `eval("x + 1")` — direct call to the global eval.
//! - `new Function("a", "return a")` — Function constructor via `new`.
//! - `Function("return 42")` — Function constructor called directly.
//!
//! **Not flagged**:
//! - `window.eval(x)` — member-expression callee; only bare identifiers are
//!   checked (syntactic check only, no scope analysis).
//! - `foo.eval(x)` — same: member-expression callee.
//! - `function eval() {}` — a declaration is not a call; only call and new
//!   expressions are checked.
//! - `setTimeout("code", 1000)` — implied-eval patterns are out of scope for
//!   this rule.
//!
//! Behaviour is reproduced from the public RSPEC description (S1523) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "code-eval";

impl Scanner<'_> {
    pub(crate) fn check_code_eval_call(&mut self, expr: &CallExpression<'_>) {
        let Expression::Identifier(callee) = expr.callee.get_inner_expression() else {
            return;
        };
        let name = callee.name.as_str();
        if name == "eval" || name == "Function" {
            self.report(RULE_NAME, "codeEval", expr.span);
        }
    }

    pub(crate) fn check_code_eval_new(&mut self, expr: &NewExpression<'_>) {
        let Expression::Identifier(callee) = expr.callee.get_inner_expression() else {
            return;
        };
        if callee.name.as_str() == "Function" {
            self.report(RULE_NAME, "codeEval", expr.span);
        }
    }
}
