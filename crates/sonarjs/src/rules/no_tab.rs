//! Rule `no-tab` (SonarJS key S105).
//!
//! Clean-room port. Tab characters render inconsistently across editors and
//! tools — different environments show them as different numbers of spaces,
//! breaking visual alignment. Only spaces should be used for indentation and
//! spacing within source files.
//!
//! This rule is RAW TEXT based: it scans the raw source text for tab bytes
//! (0x09). For each source line that contains at least one tab, exactly one
//! diagnostic is emitted, located at the FIRST tab on that line. Tabs found
//! anywhere on the line — including inside string literals or comments — are
//! counted, because the rule is about the physical characters in the file, not
//! about semantic context.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-tab";

impl Scanner<'_> {
    pub(crate) fn check_no_tab(&mut self) {
        let bytes = self.source_text.as_bytes();
        let mut spans: SmallVec<[Span; 8]> = SmallVec::new();
        let mut line_has_tab = false;
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => line_has_tab = false,
                b'\t' if !line_has_tab => {
                    line_has_tab = true;
                    spans.push(Span::new(i as u32, i as u32 + 1));
                }
                _ => {}
            }
            i += 1;
        }
        for span in spans {
            self.report(RULE_NAME, "noTab", span);
        }
    }
}
