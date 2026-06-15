//! Rule `declarations-in-global-scope` (SonarJS key S3798).
//!
//! Clean-room port. In module code, top-level `function` declarations and
//! `var` declarations create module/global-scope bindings. The rule reports
//! those declarations and encourages moving them into a local scope or binding
//! them explicitly to the global object.
//!
//! Observed behaviour reproduced:
//! - Report named top-level function declarations, including `export function`
//!   and named `export default function`.
//! - Report `var` declarations at module scope, including those nested inside
//!   top-level control-flow blocks because `var` is hoisted to the module scope.
//! - Do not report `let`/`const`, anonymous default function exports, or
//!   `var x = require("...")` CommonJS import bindings.
//! - Do not report declarations inside functions/arrow functions or class
//!   static blocks.

use oxc_ast::ast::{
    CallExpression, Declaration, ExportDefaultDeclarationKind, Expression, Function, Program,
    Statement, VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "declarations-in-global-scope";

impl Scanner<'_> {
    pub(crate) fn check_declarations_in_global_scope_program(&mut self, program: &Program<'_>) {
        for statement in &program.body {
            match statement {
                Statement::FunctionDeclaration(function) => {
                    self.check_global_function_declaration(function);
                }
                Statement::ExportNamedDeclaration(export) => {
                    if let Some(Declaration::FunctionDeclaration(function)) = &export.declaration {
                        self.check_global_function_declaration(function);
                    }
                }
                Statement::ExportDefaultDeclaration(export) => {
                    if let ExportDefaultDeclarationKind::FunctionDeclaration(function) =
                        &export.declaration
                    {
                        self.check_global_function_declaration(function);
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn check_declarations_in_global_scope_var(
        &mut self,
        declaration: &VariableDeclaration<'_>,
    ) {
        if declaration.kind != VariableDeclarationKind::Var {
            return;
        }

        for declarator in &declaration.declarations {
            if is_require_binding(declarator) {
                continue;
            }
            self.report(RULE_NAME, "defineLocally", declarator.id.span());
        }
    }

    fn check_global_function_declaration(&mut self, function: &Function<'_>) {
        if function.declare || function.id.is_none() {
            return;
        }
        self.report(RULE_NAME, "defineLocally", function.span);
    }
}

fn is_require_binding(declarator: &VariableDeclarator<'_>) -> bool {
    let Some(init) = &declarator.init else {
        return false;
    };
    let Expression::CallExpression(call) = init.get_inner_expression() else {
        return false;
    };
    is_require_call(call)
}

fn is_require_call(call: &CallExpression<'_>) -> bool {
    if call.arguments.len() != 1 {
        return false;
    }
    let Expression::Identifier(identifier) = call.callee.get_inner_expression() else {
        return false;
    };
    identifier.name.as_str() == "require"
}
