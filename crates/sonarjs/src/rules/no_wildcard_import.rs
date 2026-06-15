//! Rule `no-wildcard-import` (SonarJS key S2208).
//!
//! Clean-room port. Reports an `import` declaration that pulls in a module's
//! entire namespace through a wildcard specifier (`import * as ns from 'mod'`),
//! which hurts readability and defeats tree-shaking. The diagnostic is reported
//! at the namespace specifier (`* as ns`). Named, default, and side-effect
//! imports are not affected; re-exports (`export * from 'mod'`) are a different
//! declaration that this rule does not touch.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-wildcard-import";

impl Scanner<'_> {
    pub(crate) fn check_no_wildcard_import(&mut self, decl: &ImportDeclaration<'_>) {
        let Some(specifiers) = &decl.specifiers else {
            return;
        };
        for specifier in specifiers {
            if let ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) = specifier {
                self.report(RULE_NAME, "noWildcardImport", namespace.span);
            }
        }
    }
}
