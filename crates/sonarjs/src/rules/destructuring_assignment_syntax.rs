//! Rule `destructuring-assignment-syntax` (SonarJS key S3514).
//!
//! Clean-room port. When multiple consecutive single-declarator `const` or
//! `let` statements in the same statement list each extract a property from
//! the *same* plain object identifier—and the bound name equals the property
//! name—the code should use a single destructuring declaration instead.
//!
//! ```js
//! const a = obj.a;
//! const b = obj.b;  // Noncompliant: both extract from `obj` with matching names
//! ```
//!
//! Suggest: `const { a, b } = obj;`
//!
//! **Flagged**: starting from the *second* consecutive matching declarator in
//! such a run. Each declaration from the second onward is reported so the user
//! can see every statement that belongs to the group.
//!
//! **Not flagged**:
//! - A lone `const x = obj.x;` with no adjacent partner.
//! - `const x = obj.y;` where the binding name (`x`) differs from the property
//!   name (`y`); renaming destructuring (`{ y: x }`) is less readable and out
//!   of scope.
//! - `const x = a.b.c;` where the source is itself a member expression (chained
//!   access), not a plain identifier.
//! - Computed member access (`obj['key']` or `obj[i]`).
//! - Multi-declarator declarations (`const a = obj.a, b = obj.b;`).
//! - Mixed sources: `const a = foo.a; const b = bar.b;` — different base objects.
//! - Non-consecutive matching declarations separated by other statements.
//!
//! Behaviour is reproduced from the public RSPEC description (S3514) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{BindingPattern, Expression, Statement};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "destructuring-assignment-syntax";

/// If `stmt` is a single-declarator `const`/`let` declaration whose
/// initializer is a static member expression `IDENT.prop` where `IDENT` is a
/// plain (non-chained) identifier and the bound name equals the property name,
/// returns the base identifier name. Otherwise returns `None`.
fn destructurable_object_source<'b>(stmt: &Statement<'b>) -> Option<&'b str> {
    let Statement::VariableDeclaration(decl) = stmt else {
        return None;
    };
    if decl.declarations.len() != 1 {
        return None;
    }
    let declarator = &decl.declarations[0];
    let BindingPattern::BindingIdentifier(binding_id) = &declarator.id else {
        return None;
    };
    let Some(Expression::StaticMemberExpression(member)) = &declarator.init else {
        return None;
    };
    // Source must be a plain identifier (no chained access like `a.b.c`)
    let Expression::Identifier(object_id) = &member.object else {
        return None;
    };
    // Binding name must equal property name for zero-false-positive detection:
    // `const a = obj.a` is eligible; `const x = obj.a` is not.
    if binding_id.name.as_str() != member.property.name.as_str() {
        return None;
    }
    Some(object_id.name.as_str())
}

impl Scanner<'_> {
    pub(crate) fn check_destructuring_assignment_syntax<'b>(
        &mut self,
        statements: &[Statement<'b>],
    ) {
        let mut prev_source: Option<&'b str> = None;

        for statement in statements {
            if let Some(source) = destructurable_object_source(statement) {
                if prev_source == Some(source) {
                    // This declaration extends a consecutive group — report it.
                    // Keep `prev_source` unchanged so further consecutive
                    // declarations with the same base are also reported.
                    let Statement::VariableDeclaration(decl) = statement else {
                        unreachable!()
                    };
                    self.report(RULE_NAME, "useDestructuring", decl.span);
                } else {
                    // First declaration of a potential new group.
                    prev_source = Some(source);
                }
            } else {
                // Non-matching statement — any ongoing group is broken.
                prev_source = None;
            }
        }
    }
}
