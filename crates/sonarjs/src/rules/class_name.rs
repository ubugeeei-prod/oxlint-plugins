//! Rule `class-name` (SonarJS key S101).
//!
//! Clean-room port. Class names should follow the conventional PascalCase
//! style, which (among other things) requires the name to begin with an
//! uppercase letter. A class whose name starts with anything else reads like a
//! variable or function and obscures that it is a constructor.
//!
//! ## Narrow form
//!
//! The default SonarJS convention is the regular expression
//! `^[A-Z][a-zA-Z0-9]*$`. This port enforces the unambiguous, configuration-
//! independent part of that convention — the name must start with an uppercase
//! ASCII letter:
//!
//! ```js
//! class myClass {}   // Noncompliant: starts with a lowercase letter
//! class _Helper {}   // Noncompliant: starts with an underscore
//! class MyClass {}   // Compliant
//! ```
//!
//! Restricting the check to the first character guarantees no false positives
//! regardless of the exact configured format. Violations only in the remainder
//! of the name (for example an embedded underscore, `My_Class`) are a documented
//! follow-up. Anonymous classes (`export default class {}`) have no name and are
//! never reported.
//!
//! Behaviour is reproduced from the public RSPEC description (S101) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::Class;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "class-name";

impl Scanner<'_> {
    pub(crate) fn check_class_name(&mut self, class: &Class<'_>) {
        let Some(id) = &class.id else {
            return;
        };
        let name = id.name.as_str();
        if name.starts_with(|c: char| c.is_ascii_uppercase()) {
            return;
        }
        self.report(RULE_NAME, "className", id.span);
    }
}
