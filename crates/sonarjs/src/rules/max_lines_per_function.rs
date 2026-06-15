//! Rule `max-lines-per-function` (SonarJS key S138).
//!
//! Clean-room port. Reports a function whose code-line count exceeds the
//! configured threshold because very long functions are hard to read and
//! maintain.
//!
//! ## What counts as a "code line"
//!
//! The same definition as `max-lines`: a physical line that contains at least
//! one character that is NOT whitespace AND NOT inside a comment span.
//!
//! Code lines are counted over the function's own line span (from the first
//! byte of the function keyword/arrow to the closing brace).
//!
//! ## Exclusions
//!
//! 1. **IIFEs** — a function/arrow that is directly the `callee` of a
//!    `CallExpression` is never reported.
//! 2. **JSX-containing functions** — a function/arrow whose subtree contains
//!    any `JSXElement` or `JSXFragment` is never reported. This is a
//!    deliberately broad, zero-false-positive broadening of SonarJS's
//!    React-component exclusion (SonarJS excludes capitalized-name components
//!    that return JSX; excluding any JSX-containing function is a superset, so
//!    we never over-report — we only under-report some, which is acceptable).
//!
//! ## Threshold
//!
//! Mirrors SonarJS's configurable `maximum` option
//! (`self.options.max_lines_per_function_threshold`); when no option is
//! supplied the SonarJS default of **200** is used.
//!
//! A diagnostic is emitted when the code-line count is **strictly greater
//! than** the threshold.

use oxc_ast::ast::Expression;
use oxc_span::Span;

use crate::rules::max_lines::line_has_code;
use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "max-lines-per-function";

/// Counts the code lines the function occupies, from the line containing
/// `span.start` (the signature line) through the line containing the closing
/// brace, inclusive — matching SonarJS S138, which counts non-blank,
/// non-comment-only lines over `function.loc.start.line ..= end.line`. Blank
/// and comment-only lines are skipped (delegated to `line_has_code`).
fn count_code_lines_in_span(source: &str, comments: &[Span], span: Span) -> u32 {
    let span_start = span.start as usize;
    let span_end = span.end as usize;
    // Start at the beginning of the line that contains the function's first byte.
    let first_line_start = source[..span_start].rfind('\n').map_or(0, |i| i + 1);

    let mut count = 0u32;
    let mut line_start = first_line_start;
    for (offset, b) in source[first_line_start..].bytes().enumerate() {
        if b != b'\n' {
            continue;
        }
        // A line that begins at or after `span_end` is past the function.
        if line_start >= span_end {
            break;
        }
        let line_end = first_line_start + offset;
        if line_has_code(source, line_start, line_end, comments) {
            count += 1;
        }
        line_start = first_line_start + offset + 1;
    }
    // A final line with no trailing newline that still overlaps the span.
    if line_start < span_end
        && line_start < source.len()
        && line_has_code(source, line_start, source.len(), comments)
    {
        count += 1;
    }
    count
}

impl Scanner<'_> {
    /// Records the callee of a call when it is a function/arrow literal (an
    /// IIFE), so `max-lines-per-function` can skip reporting that function.
    pub(crate) fn record_iife_callee(&mut self, callee: &Expression<'_>) {
        let start = match callee.get_inner_expression() {
            Expression::FunctionExpression(func) => func.span.start,
            Expression::ArrowFunctionExpression(arrow) => arrow.span.start,
            _ => return,
        };
        self.iife_function_starts.push(start);
    }

    /// Marks the innermost open function/arrow frame as containing JSX.
    pub(crate) fn mark_jsx(&mut self) {
        if let Some(top) = self.jsx_function_stack.last_mut() {
            *top = true;
        }
    }

    /// Pops the JSX frame and reports if the function exceeds the threshold.
    pub(crate) fn check_max_lines_per_function(&mut self, span: Span) {
        let has_jsx = self.jsx_function_stack.pop().unwrap_or(true);
        if has_jsx {
            return;
        }
        if self.iife_function_starts.contains(&span.start) {
            return;
        }
        let count = count_code_lines_in_span(self.source_text, &self.comment_spans, span);
        if count <= self.options.max_lines_per_function_threshold {
            return;
        }
        self.report(RULE_NAME, "maxLinesPerFunction", span);
    }
}
