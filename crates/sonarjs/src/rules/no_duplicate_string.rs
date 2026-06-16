//! Rule `no-duplicate-string` (SonarJS key S1192).
//!
//! Clean-room port. Reports a string literal value that appears at least
//! `threshold` (default 3) times in a file, where the value has at least
//! 10 characters, contains a non-word character (non-[A-Za-z0-9_]), and is
//! not the excluded literal `"application/json"`. String literals used as
//! import/export module sources or as JSX attribute values are not counted.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{Expression, StringLiteral};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-duplicate-string";

fn qualifies(value: &str) -> bool {
    if value.chars().count() < 10 {
        return false;
    }
    if value == "application/json" {
        return false;
    }
    value
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || c == '_'))
}

impl<'a> Scanner<'a> {
    pub(crate) fn record_string_literal(&mut self, lit: &StringLiteral<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        self.string_literals.push((lit.value.as_str(), lit.span));
    }

    pub(crate) fn finalize_no_duplicate_string(&mut self) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let literals = self.string_literals.clone();
        let threshold = self.options.no_duplicate_string_threshold;
        let mut seen: SmallVec<[(&'a str, Span, u32); 16]> = SmallVec::new();
        for (value, span) in &literals {
            if self.excluded_string_starts.contains(&span.start) {
                continue;
            }
            if !qualifies(value) {
                continue;
            }
            let pos = seen.iter().position(|e| e.0 == *value);
            if let Some(i) = pos {
                seen[i].2 += 1;
            } else {
                seen.push((*value, *span, 1));
            }
        }
        let mut to_report: SmallVec<[Span; 8]> = SmallVec::new();
        for e in &seen {
            if e.2 >= threshold {
                to_report.push(e.1);
            }
        }
        for span in to_report {
            self.report(RULE_NAME, "duplicateString", span);
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn exclude_string(&mut self, lit: &StringLiteral<'_>) {
        self.excluded_string_starts.push(lit.span.start);
    }

    pub(crate) fn exclude_string_expression(&mut self, expr: &Expression<'_>) {
        if let Expression::StringLiteral(lit) = expr {
            self.exclude_string(lit);
        }
    }
}
