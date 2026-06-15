//! Rule `no-empty-test-file` (SonarJS key S2187).
//!
//! Clean-room port. A test file (whose base name contains `.test.` or `.spec.`
//! before the final extension) must contain at least one `it(...)` or
//! `test(...)` call expression (including their `.only`, `.skip`, and `.each`
//! variants). A test file with no such call expression is flagged at the
//! program span.
//!
//! ## Scope (conservative)
//!
//! - Only files whose base name contains `.test.` or `.spec.` are checked.
//! - A qualifying call is one whose callee is the bare identifier `it` or
//!   `test`, or a member expression `it.only`, `it.skip`, `it.each`,
//!   `test.only`, `test.skip`, or `test.each`.
//! - `describe(...)` alone does **not** count as a test.
//! - No semantic analysis is required.
//!
//! ## Flagged
//! - `import {x} from './x';` in `foo.test.ts` (no it/test)
//! - `describe('x', () => {});` in `a.spec.ts` (describe but no it/test)
//!
//! ## Not flagged
//! - `it('works', () => {});` in `foo.test.ts` (has a test)
//! - any content in `foo.ts` (not a test file)
//!
//! Behaviour is reproduced from the public RSPEC description (S2187) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression, Program};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-empty-test-file";

/// Returns `true` when the base name of `filename` contains `.test.` or
/// `.spec.`, indicating this is a test file.
fn is_test_filename(filename: &str) -> bool {
    let basename = match filename.rfind(['/', '\\']) {
        Some(pos) => &filename[pos + 1..],
        None => filename,
    };
    basename.contains(".test.") || basename.contains(".spec.")
}

/// Returns `true` when `callee` names an `it` or `test` call (including
/// `.only`, `.skip`, and `.each` modifiers).
fn is_test_callee(callee: &Expression<'_>) -> bool {
    match callee.get_inner_expression() {
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            name == "it" || name == "test"
        }
        Expression::StaticMemberExpression(member) => {
            let prop = member.property.name.as_str();
            if prop != "only" && prop != "skip" && prop != "each" {
                return false;
            }
            let Expression::Identifier(object) = member.object.get_inner_expression() else {
                return false;
            };
            let name = object.name.as_str();
            name == "it" || name == "test"
        }
        _ => false,
    }
}

impl Scanner<'_> {
    /// Records that at least one `it`/`test` call was seen in this file.
    pub(crate) fn check_no_empty_test_file_call(&mut self, call: &CallExpression<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        if !self.saw_test_call && is_test_callee(&call.callee) {
            self.saw_test_call = true;
        }
    }

    /// After the AST walk, flags the program if the file is a test file with
    /// no `it`/`test` call expression.
    pub(crate) fn finalize_no_empty_test_file(&mut self, program: &Program<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        if !is_test_filename(self.filename) {
            return;
        }
        if !self.saw_test_call {
            self.report(RULE_NAME, "emptyTestFile", program.span);
        }
    }
}
