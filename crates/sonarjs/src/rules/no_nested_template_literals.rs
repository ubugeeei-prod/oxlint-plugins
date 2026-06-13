//! Rule `no-nested-template-literals` (SonarJS key S4624).
//!
//! Clean-room port. Reports a template literal that appears inside the
//! interpolated expressions of another template literal, because such nesting
//! is hard to read. Behaviour is reproduced from the public RSPEC description
//! only; no upstream source, tests, fixtures, or message strings were consulted
//! or copied.
//!
//! Semantics: a template literal is flagged when at least one enclosing
//! template literal is open on the traversal stack, so every nested literal is
//! reported at its own location regardless of nesting depth or any intervening
//! expressions (including function bodies).

use oxc_ast::ast::TemplateLiteral;
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-nested-template-literals";

impl Scanner<'_> {
    pub(crate) fn check_no_nested_template_literals(&mut self, template: &TemplateLiteral<'_>) {
        if self.template_literal_depth > 0 {
            self.report(RULE_NAME, "nestedTemplateLiteral", template.span());
        }
    }
}
