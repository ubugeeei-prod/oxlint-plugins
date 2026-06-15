//! Rule `no-identical-functions` (SonarJS key S4144).
//!
//! Clean-room port. Two functions in the same file whose parameter list and body
//! are byte-for-byte equal are almost always the result of a copy-paste mistake
//! or a missed abstraction opportunity. The second (and further) identical
//! function is reported.
//!
//! ## Identity definition (conservative, byte-exact)
//!
//! Two qualifying functions are "identical" iff the SOURCE TEXT of their
//! `[params..body]` region — from the opening `(` of the parameter list to the
//! closing `}` of the body block — is byte-for-byte equal. This includes the
//! parameter list and the body but NOT the function name, so `function a(){…}`
//! and `function b(){…}` with identical params+body ARE flagged as duplicates.
//!
//! This comparison is intentionally WHITESPACE-SENSITIVE: two functions whose
//! bodies differ only in whitespace or comments are NOT flagged. This is a
//! deliberate conservative trade-off (we prefer under-reporting over false
//! positives), matching the approach taken by other conservative rules in this
//! crate. The upstream SonarJS rule uses a structural/token-level comparison and
//! flags whitespace-differing bodies; this port does not.
//!
//! ## Scope
//!
//! Applies to every function with a block body: `FunctionDeclaration`,
//! `FunctionExpression`, and methods (all `Function` nodes), as well as
//! block-bodied arrow functions (expression-bodied arrows such as `x => x + 1`
//! are never flagged because they have no `{…}` body to compare).
//!
//! Only functions spanning at least `THRESHOLD` (3) lines are considered; smaller
//! functions are neither flagged nor recorded.
//!
//! ## Options follow-up
//!
//! Upstream exposes a `threshold` option (default 3). This port hardcodes 3 and
//! does not wire the options layer; a follow-up PR can add that.
//!
//! Behaviour reproduced from RSPEC S4144 only; no upstream source, tests,
//! fixtures, or message strings were consulted or copied.

use compact_str::ToCompactString;
use oxc_span::Span;

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "no-identical-functions";

/// Minimum number of lines a function must span to qualify for the check.
/// Options follow-up: upstream exposes a `threshold` option (default 3).
const THRESHOLD: u32 = 3;

impl<'a> Scanner<'a> {
    /// Core check for `no-identical-functions`.
    ///
    /// `params_start` is the byte offset of the `(` that opens the parameter
    /// list; `body_end` is the byte offset just past the closing `}` of the
    /// body block; `func_span` is the full span of the function node used for
    /// the line-count test and the diagnostic location.
    ///
    /// Call this BEFORE walking into the function so that source-order is
    /// preserved: any earlier identical function is already in `seen_function_impls`.
    pub(crate) fn check_no_identical_functions(
        &mut self,
        params_start: u32,
        body_end: u32,
        func_span: Span,
    ) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let loc = self.line_index.loc_for_span(self.source_text, func_span);
        let line_count = loc.end_line.saturating_sub(loc.start_line) + 1;
        if line_count < THRESHOLD {
            return;
        }
        let impl_text = self.text(Span::new(params_start, body_end));
        let earlier_line = self
            .seen_function_impls
            .iter()
            .find(|(text, _)| *text == impl_text)
            .map(|(_, line)| *line);
        if let Some(line) = earlier_line {
            let data = DiagnosticData {
                value: Some(line.to_compact_string()),
                format: None,
            };
            self.report_with_data(RULE_NAME, "identicalFunctions", data, func_span, None);
        }
        self.seen_function_impls.push((impl_text, loc.start_line));
    }
}
