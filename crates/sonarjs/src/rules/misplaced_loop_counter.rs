//! Rule `misplaced-loop-counter` (SonarJS key S1994).
//!
//! Clean-room port. A classic `for (init; test; update)` loop should advance,
//! in its UPDATE clause, at least one of the variables that its CONDITION
//! tests. When the update clause modifies only variable(s) that never appear in
//! the condition, the counter checked by the condition is never moved by the
//! header and the loop likely does not terminate as intended:
//!
//! ```js
//! for (let i = 0; i < 10; j++) {}   // Noncompliant: tests i, updates j
//! for (let i = 0; i < 10; i++) {}   // Compliant: tests i, updates i
//! ```
//!
//! ## Scope
//!
//! The rule compares two NAME sets:
//!
//! - The CONDITION set: every identifier name referenced anywhere in the test
//!   expression (operands of comparisons, the base of a member access such as
//!   `arr[i]`, call arguments, etc.). This is collected generously so that a
//!   counter used indirectly in the condition still counts as "tested".
//! - The UPDATE set: the bare identifiers written by the update clause — the
//!   target of an `UpdateExpression` (`i++`), of an `AssignmentExpression`
//!   (`i += 1`, `i = …`), and of each element of a comma `SequenceExpression`
//!   (`i++, j++`). Property writes (`o.i++`) are not bare identifiers and are
//!   never collected.
//!
//! An issue is raised, at the update clause, only when BOTH sets are non-empty
//! AND they are DISJOINT (no update name appears in the condition). When the
//! test has no identifier, the update has no bare-identifier target, or the two
//! sets overlap, nothing is reported.
//!
//! Name-based comparison is sound here because the condition and the update of
//! one for-statement reference the same enclosing scope (the for-header); there
//! is no shadowing between them. The rule concerns only the update clause —
//! modifications inside the loop body are out of scope (that is the separate
//! `updated-loop-counter` rule, S2310).
//!
//! Behaviour is reproduced from the public RSPEC description (S1994) and the
//! observable behaviour of the equivalent check only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentTarget, Expression, ForStatement, SimpleAssignmentTarget};
use oxc_span::GetSpan;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "misplaced-loop-counter";

/// Collects every identifier name referenced anywhere in `expr` (a loop's test
/// expression) into `out`, descending through the expression shapes that can
/// hold a counter reference. Duplicate names are not added twice.
fn collect_condition_names<'a>(expr: &Expression<'a>, out: &mut SmallVec<[&'a str; 4]>) {
    match expr.get_inner_expression() {
        Expression::Identifier(ident) => {
            let name = ident.name.as_str();
            if !out.contains(&name) {
                out.push(name);
            }
        }
        Expression::BinaryExpression(bin) => {
            collect_condition_names(&bin.left, out);
            collect_condition_names(&bin.right, out);
        }
        Expression::LogicalExpression(logical) => {
            collect_condition_names(&logical.left, out);
            collect_condition_names(&logical.right, out);
        }
        Expression::UnaryExpression(unary) => collect_condition_names(&unary.argument, out),
        Expression::ConditionalExpression(cond) => {
            collect_condition_names(&cond.test, out);
            collect_condition_names(&cond.consequent, out);
            collect_condition_names(&cond.alternate, out);
        }
        Expression::SequenceExpression(seq) => {
            for sub in &seq.expressions {
                collect_condition_names(sub, out);
            }
        }
        Expression::CallExpression(call) => {
            // The callee is the function being invoked, not a loop counter, so
            // it is deliberately not collected (otherwise a bare `cond()` test
            // would look like it "tests" the name `cond`). Only the arguments,
            // which may carry the counter (`f(i)`), are descended into.
            for arg in &call.arguments {
                if let Some(arg_expr) = arg.as_expression() {
                    collect_condition_names(arg_expr, out);
                }
            }
        }
        Expression::StaticMemberExpression(member) => {
            collect_condition_names(&member.object, out);
        }
        Expression::ComputedMemberExpression(member) => {
            collect_condition_names(&member.object, out);
            collect_condition_names(&member.expression, out);
        }
        _ => {}
    }
}

/// Collects the bare identifier name(s) written by an update clause into `out`:
/// the target of an `UpdateExpression` (`i++`), of an `AssignmentExpression`
/// (`i += 1`), and of each element of a `SequenceExpression` (`i++, j++`).
fn collect_update_names<'a>(expr: &Expression<'a>, out: &mut SmallVec<[&'a str; 4]>) {
    match expr {
        Expression::UpdateExpression(update) => {
            if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) = &update.argument {
                out.push(ident.name.as_str());
            }
        }
        Expression::AssignmentExpression(assign) => {
            if let AssignmentTarget::AssignmentTargetIdentifier(ident) = &assign.left {
                out.push(ident.name.as_str());
            }
        }
        Expression::SequenceExpression(seq) => {
            for sub in &seq.expressions {
                collect_update_names(sub, out);
            }
        }
        _ => {}
    }
}

impl Scanner<'_> {
    pub(crate) fn check_misplaced_loop_counter(&mut self, stmt: &ForStatement<'_>) {
        let Some(test) = &stmt.test else {
            return;
        };
        let Some(update) = &stmt.update else {
            return;
        };
        let mut update_names: SmallVec<[&str; 4]> = SmallVec::new();
        collect_update_names(update, &mut update_names);
        if update_names.is_empty() {
            return;
        }
        let mut condition_names: SmallVec<[&str; 4]> = SmallVec::new();
        collect_condition_names(test, &mut condition_names);
        if condition_names.is_empty() {
            return;
        }
        let overlaps = update_names
            .iter()
            .any(|name| condition_names.contains(name));
        if overlaps {
            return;
        }
        self.report(RULE_NAME, "misplacedCounter", update.span());
    }
}
