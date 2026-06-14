//! Rule `prefer-single-boolean-return` (SonarJS key S1126).
//!
//! Clean-room port. Reports an `if` statement whose `consequent` AND `else`
//! branch both consist solely of `return <boolean-literal>;`, because the
//! whole structure can be collapsed into a single `return <condition>;` (or
//! `return !<condition>;`).
//!
//! **Scope — explicit-`else` form only**: this rule targets the pattern where
//! the `else` keyword is present.  The implicit-else form
//! (`if (c) return true; return false;`) is outside the scope of this rule
//! and should be handled by a follow-up rule or a separate check.
//!
//! ## Flagged
//! - `if (c) { return true; } else { return false; }` — block branches
//! - `if (c) return true; else return false;`          — bare branches
//! - `if (c) { return false; } else { return true; }` — inverted
//! - `if (c) return true; else return true;`           — same literal
//!
//! ## Not flagged
//! - `if (c) { return true; }`                                      — no else
//! - `if (c) { return x; } else { return false; }`                  — non-literal
//! - `if (c) { foo(); } else { return false; }`                     — not a return
//! - `if (c) return true; else if (d) return false; else return t;` — else-if
//! - `if (c) { return true; bar(); } else { return false; }`        — 2-stmt block
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, IfStatement, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-single-boolean-return";

/// Returns `true` when `stmt` is `return <bool>;` directly, or a
/// `BlockStatement` containing exactly one such statement.
fn returns_boolean_literal(stmt: &Statement) -> bool {
    if let Statement::ReturnStatement(ret) = stmt {
        return matches!(
            ret.argument.as_ref().map(|a| a.get_inner_expression()),
            Some(Expression::BooleanLiteral(_))
        );
    }
    let Statement::BlockStatement(block) = stmt else {
        return false;
    };
    block.body.len() == 1 && returns_boolean_literal(&block.body[0])
}

impl Scanner<'_> {
    pub(crate) fn check_prefer_single_boolean_return(&mut self, if_stmt: &IfStatement<'_>) {
        let Some(alternate) = &if_stmt.alternate else {
            return;
        };
        if !returns_boolean_literal(&if_stmt.consequent) || !returns_boolean_literal(alternate) {
            return;
        }
        let start = if_stmt.span.start;
        self.report(
            RULE_NAME,
            "preferSingleBooleanReturn",
            Span::new(start, start + 2),
        );
    }
}
