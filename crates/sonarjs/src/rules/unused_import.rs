//! Rule `unused-import` (SonarJS key S1128).
//!
//! Clean-room port. Behavior derived solely from the public RSPEC S1128
//! ("Unnecessary imports should be removed"): an imported binding that is
//! never referenced anywhere in the module is dead code and should be removed.
//!
//! ## Scope and guards
//!
//! - Requires semantic analysis. Reference resolution (whether a binding is
//!   ever used) depends on scope/symbol data; when semantic data is absent
//!   nothing is emitted.
//! - Side-effect imports (`import "./styles.css";`) have no specifiers and are
//!   never flagged — they are imported for their side effects, not a binding.
//! - Each import specifier is considered independently. The local binding of a
//!   named (`import { foo }`), default (`import bar`), or namespace
//!   (`import * as ns`) specifier is flagged when its symbol has zero
//!   references in the module.
//! - Reference resolution is trusted from the semantic model, which correctly
//!   accounts for uses in nested scopes, JSX, and TypeScript type positions.
//!   No heuristics are layered on top.
//!
//! ## Flagged
//! - `import { foo } from 'x';` — `foo` never used → flagged
//! - `import bar from 'x';` — default binding never used → flagged
//! - `import * as ns from 'x';` — namespace binding never used → flagged
//! - `import { used, dead } from 'x'; used();` — only `dead` is flagged
//!
//! ## Not flagged
//! - `import { foo } from 'x'; foo();` — binding is referenced
//! - `import 'x';` — side-effect import, no specifiers
//! - `import { C } from 'x'; const f = () => <C />;` — used in JSX/nested scope;
//!   semantic resolves all references regardless of nesting

use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "unused-import";

impl<'a> Scanner<'a> {
    pub(crate) fn check_unused_import(&mut self, it: &ImportDeclaration<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let scoping = match self.scoping {
            Some(s) => s,
            // Semantic absent: cannot prove a binding is unused → emit nothing.
            None => return,
        };
        // Side-effect import (`import "x";`): no bindings to flag.
        let specifiers = match &it.specifiers {
            Some(specifiers) => specifiers,
            None => return,
        };
        for specifier in specifiers {
            let local = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(s) => &s.local,
                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => &s.local,
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => &s.local,
            };
            let symbol_id = match local.symbol_id.get() {
                Some(id) => id,
                // Binding not resolved by semantic; skip this specifier.
                None => continue,
            };
            if scoping.symbol_is_unused(symbol_id) {
                self.report(RULE_NAME, "unusedImport", local.span);
            }
        }
    }
}
