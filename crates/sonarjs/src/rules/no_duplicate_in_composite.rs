//! Rule `no-duplicate-in-composite` (SonarJS key S4621).
//!
//! Clean-room port. Reports a type member that appears more than once in a
//! TypeScript union (`A | B | A`) or intersection (`A & B & A`) type. Only the
//! second and later occurrences of a repeated member are flagged; the first
//! occurrence is left alone. Members are compared by their source text.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::TSType;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-duplicate-in-composite";

impl<'a> Scanner<'a> {
    pub(crate) fn check_no_duplicate_in_composite(&mut self, types: &[TSType<'a>]) {
        let mut seen: SmallVec<[&'a str; 8]> = SmallVec::new();
        let mut duplicates: SmallVec<[Span; 4]> = SmallVec::new();
        for member in types {
            let t = self.text(member.span());
            if seen.iter().any(|s| *s == t) {
                duplicates.push(member.span());
            } else {
                seen.push(t);
            }
        }
        for span in duplicates {
            self.report(RULE_NAME, "duplicateType", span);
        }
    }
}
