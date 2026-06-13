//! Rule `arguments-usage` (SonarJS key S3513).
//!
//! Clean-room port. Reports every use of the `arguments` object and suggests
//! switching to rest parameters (`...args`), which are explicit, scoped to the
//! function they are declared in, and work with arrow functions.
//!
//! ## Detection strategy
//!
//! The hook `visit_identifier_reference` is used because, in Oxc's AST, a
//! *use* of a variable is represented as `Expression::Identifier(IdentifierReference)`.
//! This means:
//!
//! - `obj.arguments` — the property name is an `IdentifierName` inside a static
//!   member expression, **not** an `IdentifierReference`, so it is correctly skipped.
//! - `function arguments() {}` — the function name is a `BindingIdentifier`, also
//!   not flagged.
//! - Arrow functions do not have their own `arguments` binding; any `arguments`
//!   reference inside an arrow function refers to the enclosing function's object,
//!   which is still the `arguments` object and is correctly flagged.
//!
//! ## Syntactic check
//!
//! This is a purely syntactic check. In the rare (and invalid in strict mode)
//! case where a programmer declares a local binding literally named `arguments`,
//! it will still be flagged. The trade-off is simplicity and zero false negatives
//! for the common case.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::IdentifierReference;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "arguments-usage";

impl Scanner<'_> {
    pub(crate) fn check_arguments_usage(&mut self, ident: &IdentifierReference<'_>) {
        if ident.name.as_str() != "arguments" {
            return;
        }
        self.report(RULE_NAME, "argumentsUsage", ident.span);
    }
}
