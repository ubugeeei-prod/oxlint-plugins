//! Rule `no-session-cookies-on-static-assets` (SonarJS key S8441).
//!
//! Clean-room port from the public RSPEC S8441 behavioral description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! In an Express application the session middleware (`express-session` /
//! `cookie-session`) issues a `Set-Cookie` header for every request it handles.
//! When that middleware is mounted *before* a static-file handler
//! (`express.static(...)` / `serve-static`), requests for static assets
//! (images, scripts, stylesheets) also flow through the session middleware and
//! needlessly receive a fresh session cookie. This wastes bandwidth and widens
//! the surface for session-fixation style problems. Static assets should be
//! served *before* the session middleware is registered so their responses
//! never carry a session cookie.
//!
//! ## Zero-false-positive subset
//!
//! Within a single statement list (a module body, a block, or a function body)
//! this port:
//!
//! 1. Collects the local bindings introduced *in that same statement list* for
//!    the distinctive modules `express-session` / `cookie-session` (session),
//!    `serve-static` (static), and `express`, via either an `import` declaration
//!    or a `const x = require('...')` declarator.
//! 2. Scans the statement list in source order for `<app>.use(<factory>(...))`
//!    registrations. A session registration is `app.use(session(...))` where
//!    `session` is a collected session binding. A static registration is
//!    `app.use(express.static(...))` (or a collected `serve-static` binding
//!    call).
//! 3. Reports a static registration only when a session registration for the
//!    *same* receiver object (compared by source text) was already seen earlier
//!    in the same statement list.
//!
//! Restricting recognition to these distinctive Express module names, requiring
//! the bindings to be visible in the very same statement list, and requiring the
//! session-then-static source ordering on an identical receiver keeps the check
//! effectively false-positive free. The reverse ordering (static before
//! session), a missing session registration, a missing static registration, and
//! registrations on different receivers are all deliberately not flagged.
//!
//! ## Flagged
//! ```js
//! const express = require('express');
//! const session = require('express-session');
//! const app = express();
//! app.use(session({ secret: 's' }));
//! app.use(express.static('public')); // Noncompliant: static after session
//! ```
//!
//! ## Not flagged
//! ```js
//! app.use(express.static('public')); // static first
//! app.use(session({ secret: 's' }));
//! ```

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, ImportDeclarationSpecifier, Statement,
};
use oxc_span::GetSpan;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-session-cookies-on-static-assets";

/// Classification of an imported/required module name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ModuleKind {
    Session,
    Static,
    Express,
    Other,
}

/// Classification of an `app.use(<factory>(...))` middleware factory.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FactoryKind {
    Session,
    Static,
    Other,
}

fn classify_module(source: &str) -> ModuleKind {
    match source {
        "express-session" | "cookie-session" => ModuleKind::Session,
        "serve-static" => ModuleKind::Static,
        "express" => ModuleKind::Express,
        _ => ModuleKind::Other,
    }
}

/// Returns the string argument of a `require('...')` initializer, if `init` is
/// exactly that shape.
fn require_source<'b>(init: &'b Expression<'_>) -> Option<&'b str> {
    let Expression::CallExpression(call) = init.get_inner_expression() else {
        return None;
    };
    let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
        return None;
    };
    if callee.name.as_str() != "require" {
        return None;
    }
    let Some(Argument::StringLiteral(source)) = call.arguments.first() else {
        return None;
    };
    Some(source.value.as_str())
}

/// Classifies the middleware factory call passed to `app.use(...)`.
fn factory_kind(
    factory: &CallExpression<'_>,
    session_bindings: &[&str],
    static_bindings: &[&str],
    express_bindings: &[&str],
) -> FactoryKind {
    match factory.callee.get_inner_expression() {
        // `session(...)` / `serveStatic(...)`.
        Expression::Identifier(id) => {
            let name = id.name.as_str();
            if session_bindings.contains(&name) {
                FactoryKind::Session
            } else if static_bindings.contains(&name) {
                FactoryKind::Static
            } else {
                FactoryKind::Other
            }
        }
        // `express.static(...)`: the `static` property on the express binding (or
        // the literal `express` identifier, which is unambiguous).
        Expression::StaticMemberExpression(member) => {
            if member.property.name.as_str() == "static"
                && let Expression::Identifier(obj) = member.object.get_inner_expression()
            {
                let obj_name = obj.name.as_str();
                if obj_name == "express" || express_bindings.contains(&obj_name) {
                    return FactoryKind::Static;
                }
            }
            FactoryKind::Other
        }
        _ => FactoryKind::Other,
    }
}

impl<'a> Scanner<'a> {
    pub(crate) fn check_no_session_cookies_on_static_assets(
        &mut self,
        statements: &[Statement<'a>],
    ) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }

        // Pass 1: collect the session / static / express bindings introduced in
        // this statement list (via `import` or `const x = require('...')`).
        let mut session_bindings: SmallVec<[&str; 4]> = SmallVec::new();
        let mut static_bindings: SmallVec<[&str; 4]> = SmallVec::new();
        let mut express_bindings: SmallVec<[&str; 2]> = SmallVec::new();

        let mut record = |kind: ModuleKind, name: &'a str| match kind {
            ModuleKind::Session => session_bindings.push(name),
            ModuleKind::Static => static_bindings.push(name),
            ModuleKind::Express => express_bindings.push(name),
            ModuleKind::Other => {}
        };

        for stmt in statements {
            match stmt {
                Statement::ImportDeclaration(decl) => {
                    let kind = classify_module(decl.source.value.as_str());
                    if kind == ModuleKind::Other {
                        continue;
                    }
                    let Some(specifiers) = &decl.specifiers else {
                        continue;
                    };
                    for specifier in specifiers {
                        let local = match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(s) => s.local.name.as_str(),
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                s.local.name.as_str()
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                s.local.name.as_str()
                            }
                        };
                        record(kind, local);
                    }
                }
                Statement::VariableDeclaration(decl) => {
                    for declarator in &decl.declarations {
                        let BindingPattern::BindingIdentifier(ident) = &declarator.id else {
                            continue;
                        };
                        let Some(init) = &declarator.init else {
                            continue;
                        };
                        let Some(source) = require_source(init) else {
                            continue;
                        };
                        record(classify_module(source), ident.name.as_str());
                    }
                }
                _ => {}
            }
        }

        // Nothing distinctive in scope: cannot prove the anti-pattern.
        if session_bindings.is_empty() {
            return;
        }

        // Pass 2: ordered scan of `<app>.use(<factory>(...))` registrations.
        let mut session_apps: SmallVec<[&str; 2]> = SmallVec::new();
        for stmt in statements {
            let Statement::ExpressionStatement(expr_stmt) = stmt else {
                continue;
            };
            let Expression::CallExpression(call) = expr_stmt.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression()
            else {
                continue;
            };
            if member.property.name.as_str() != "use" {
                continue;
            }
            let app_text = self.text(member.object.span());

            for arg in &call.arguments {
                let Some(arg_expr) = arg.as_expression() else {
                    continue;
                };
                let Expression::CallExpression(factory) = arg_expr.get_inner_expression() else {
                    continue;
                };
                match factory_kind(
                    factory,
                    &session_bindings,
                    &static_bindings,
                    &express_bindings,
                ) {
                    FactoryKind::Session => {
                        if !session_apps.contains(&app_text) {
                            session_apps.push(app_text);
                        }
                    }
                    FactoryKind::Static => {
                        if session_apps.contains(&app_text) {
                            self.report(RULE_NAME, "noSessionCookiesOnStaticAssets", call.span);
                            break;
                        }
                    }
                    FactoryKind::Other => {}
                }
            }
        }
    }
}
