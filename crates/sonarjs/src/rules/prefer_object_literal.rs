//! Rule `prefer-object-literal` (SonarJS key S2428).
//!
//! Clean-room port. An object should be created and populated in a single
//! object literal rather than declared as an empty object and then filled in
//! with property assignments. When a variable is declared with an empty object
//! literal `{}` and the immediately following sibling statement assigns to a
//! property of that same variable, the declaration is reported.
//!
//! ```js
//! let person = {};        // Noncompliant
//! person.name = "John";
//! ```
//!
//! Compliant:
//!
//! ```js
//! let person = { name: "John" };
//! ```
//!
//! **Flagged**: a single-declarator `VariableDeclaration` whose initializer is
//! an empty object literal `{}` and whose directly following sibling statement
//! (within the same statement list: a program, block, function body, or switch
//! case) is an `=` assignment to a static or computed member of the same
//! variable (`person.x = ...` or `person['x'] = ...`).
//!
//! **Not flagged**:
//! - a declaration already initialized as a non-empty literal `{ x: 1 }`.
//! - a declaration whose initializer is not an object literal (`new Object()`,
//!   `getObj()`, ...).
//! - a declaration not immediately followed by a property assignment to it
//!   (e.g. the next statement reads or passes the variable, or is unrelated).
//! - a multi-declarator declaration (`let a = {}, b = 1;`).
//!
//! The declared variable and the following assignment are adjacent sibling
//! statements in the same scope, so matching the variable by name is sound:
//! no shadowing can occur between two consecutive statements of one list.
//!
//! Behaviour is reproduced from the public RSPEC description (S2428) and the
//! eslint-plugin-sonarjs rule documentation only; no upstream source, tests,
//! fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentTarget, BindingPattern, Expression, Statement};
use oxc_span::Span;
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-object-literal";

/// If `stmt` declares exactly one variable with a plain binding identifier
/// initialized to an empty object literal `{}`, returns its name and the
/// declaration span. Otherwise returns `None`.
fn empty_object_decl<'a>(stmt: &'a Statement) -> Option<(&'a str, Span)> {
    let Statement::VariableDeclaration(decl) = stmt else {
        return None;
    };
    if decl.declarations.len() != 1 {
        return None;
    }
    let declarator = &decl.declarations[0];
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    let Some(Expression::ObjectExpression(obj)) = &declarator.init else {
        return None;
    };
    if !obj.properties.is_empty() {
        return None;
    }
    Some((id.name.as_str(), decl.span))
}

/// Returns `true` when `stmt` is an `=` assignment to a static or computed
/// member (`name.prop` / `name['prop']`) of the variable `name`.
fn is_property_assign_to(stmt: &Statement, name: &str) -> bool {
    let Statement::ExpressionStatement(expr_stmt) = stmt else {
        return false;
    };
    let Expression::AssignmentExpression(assign) = &expr_stmt.expression else {
        return false;
    };
    if assign.operator != AssignmentOperator::Assign {
        return false;
    }
    let object = match &assign.left {
        AssignmentTarget::StaticMemberExpression(member) => &member.object,
        AssignmentTarget::ComputedMemberExpression(member) => &member.object,
        _ => return false,
    };
    let Expression::Identifier(id) = object else {
        return false;
    };
    id.name.as_str() == name
}

impl Scanner<'_> {
    pub(crate) fn check_prefer_object_literal(&mut self, statements: &[Statement<'_>]) {
        for (index, statement) in statements.iter().enumerate() {
            let Some((name, span)) = empty_object_decl(statement) else {
                continue;
            };
            let Some(next) = statements.get(index + 1) else {
                continue;
            };
            if is_property_assign_to(next, name) {
                self.report(RULE_NAME, "preferObjectLiteral", span);
            }
        }
    }
}
