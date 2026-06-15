//! Rule `use-type-alias` (SonarJS key S4323).
//!
//! Clean-room port. A union or intersection type whose exact source text is
//! repeated at least `THRESHOLD` (3) times in a file should be extracted into a
//! named type alias to improve readability and maintainability. The first
//! occurrence of each such repeated composite type is reported.
//!
//! Type identity is by exact source text as written (order-sensitive): `A | B`
//! and `B | A` are distinct, as are differing-whitespace spellings. Both union
//! and intersection types are counted; every union always has at least two
//! members and every intersection at least two, so no extra arity check is
//! needed. Nested composite types are each recorded as their own entry.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "use-type-alias";

const THRESHOLD: u32 = 3;

impl<'a> Scanner<'a> {
    pub(crate) fn record_composite_type(&mut self, span: Span) {
        let text = self.text(span);
        self.composite_types.push((text, span));
    }

    pub(crate) fn finalize_use_type_alias(&mut self) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let types = self.composite_types.clone();
        let mut seen: SmallVec<[(&'a str, Span, u32); 16]> = SmallVec::new();
        for (text, span) in &types {
            let pos = seen.iter().position(|e| e.0 == *text);
            if let Some(i) = pos {
                seen[i].2 += 1;
            } else {
                seen.push((*text, *span, 1));
            }
        }
        let mut to_report: SmallVec<[Span; 8]> = SmallVec::new();
        for e in &seen {
            if e.2 >= THRESHOLD {
                to_report.push(e.1);
            }
        }
        for span in to_report {
            self.report(RULE_NAME, "useTypeAlias", span);
        }
    }
}
