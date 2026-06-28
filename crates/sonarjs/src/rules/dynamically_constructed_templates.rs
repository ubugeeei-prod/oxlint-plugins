//! Rule `dynamically-constructed-templates` (SonarJS key S7790).
//!
//! Clean-room port from the public RSPEC S7790 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Angular fully trusts the markup it compiles for a component or directive: a
//! template is parsed for binding expressions which the framework then
//! executes. Building that template string dynamically — especially by mixing
//! in runtime values — bypasses Angular's built-in contextual escaping and
//! opens the door to template injection (a class of cross-site-scripting). The
//! safe pattern is a static template whose dynamic parts flow in through
//! property/interpolation bindings, never through string assembly.
//!
//! ## Zero-FP subset
//!
//! This port flags the distinctive Angular shape: a class decorator
//! `@Component(...)` or `@Directive(...)` (the callee identifier is matched
//! after unwrapping parentheses) whose first argument is an object literal
//! containing a `template` property whose value is *unambiguously* constructed
//! at runtime, namely:
//!
//! - a template literal with at least one interpolated expression
//!   (`` `<div>${userInput}</div>` ``), or
//! - a string concatenation (`'<div>' + userInput + '</div>'`).
//!
//! Both `Component` and `Directive` are essentially unique to Angular's
//! decorator API, and a `template` key built from interpolation or `+`
//! concatenation is a deliberate dynamic assembly, so this combination is
//! effectively zero-false-positive in practice. The offending `template`
//! property is reported.
//!
//! ## Flagged
//! ```js
//! @Component({ template: '<div>' + userInput + '</div>' })  // concatenation
//! @Component({ template: `<h1>${title}</h1>` })             // interpolation
//! @Directive({ template: header + body })                   // concatenation
//! ```
//!
//! ## Not flagged
//! ```js
//! @Component({ template: '<div></div>' })          // static string literal
//! @Component({ template: `<div></div>` })          // static template, no ${}
//! @Component({ templateUrl: dir + '/x.html' })     // not the `template` key
//! foo({ template: '<x>' + y })                     // callee not Component/Directive
//! ```
//!
//! Indirection through a variable (`const t = '<x>' + y; @Component({ template: t })`)
//! is intentionally out of scope to keep the check free of false positives; it
//! is a documented follow-up.

use oxc_ast::ast::{Class, Expression, ObjectPropertyKind, PropertyKey};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "dynamically-constructed-templates";

/// Returns `true` when `expr` is unambiguously assembled at runtime: a template
/// literal with interpolations, or a `+` string concatenation.
fn is_dynamically_constructed(expr: &Expression<'_>) -> bool {
    match expr.get_inner_expression() {
        Expression::TemplateLiteral(tpl) => !tpl.expressions.is_empty(),
        Expression::BinaryExpression(bin) => bin.operator == BinaryOperator::Addition,
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_dynamically_constructed_templates(&mut self, class: &Class<'_>) {
        for decorator in &class.decorators {
            // `@Component(...)` / `@Directive(...)`: the decorator expression is
            // a call to an identifier factory.
            let Expression::CallExpression(call) = decorator.expression.get_inner_expression()
            else {
                continue;
            };
            let Expression::Identifier(callee) = call.callee.get_inner_expression() else {
                continue;
            };
            if callee.name != "Component" && callee.name != "Directive" {
                continue;
            }
            let Some(Expression::ObjectExpression(obj)) =
                call.arguments.first().and_then(|arg| arg.as_expression())
            else {
                continue;
            };
            for prop in &obj.properties {
                let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                    continue;
                };
                let key = match &prop.key {
                    PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                    PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                    _ => continue,
                };
                if key != "template" {
                    continue;
                }
                if is_dynamically_constructed(&prop.value) {
                    self.report(RULE_NAME, "dynamicTemplate", prop.span);
                }
            }
        }
    }
}
