//! Targeted scan_* methods for testing-library rules, grouped here to keep the
//! scanner driver file focused on dispatching.

use std::fmt::Write as _;

use oxc_span::Span;
use oxlint_plugins_carton::CompactString;
use regex::Regex;

use crate::helpers::{count_occurrences, find_all, line_prefix, quoted_value_after, span_for};
use crate::scanner::Scanner;

const FILENAME_PLACEHOLDER: &str = "{fileName}";

impl<'a> Scanner<'a> {
    pub(crate) fn scan_test_id_attributes(&mut self) {
        if !self.options.has_rule("consistent-data-testid") {
            return;
        }
        // Upstream defaults `testIdPattern` to "" which compiles to `//` and
        // matches every value, so an unset pattern never reports.
        if self.options.test_id_pattern.is_empty() {
            return;
        }
        let file_name = derive_file_name(self.filename).unwrap_or_default();
        let resolved = self
            .options
            .test_id_pattern
            .replacen(FILENAME_PLACEHOLDER, file_name, 1);
        let Ok(regex) = Regex::new(&resolved) else {
            return;
        };

        // Collect first so the immutable borrows of `self` end before we report.
        let mut reports: Vec<(CompactString, Span)> = Vec::new();
        for attr in &self.options.test_id_attribute {
            let mut needle = CompactString::new("");
            let _ = write!(needle, "{attr}=");
            for index in find_all(self.source_text, &needle) {
                let Some((value, value_start, value_end)) =
                    quoted_value_after(self.source_text, index + needle.len())
                else {
                    continue;
                };
                if regex.is_match(value) {
                    continue;
                }
                let message = self.options.custom_message.clone().unwrap_or_else(|| {
                    let mut message = CompactString::new("");
                    let _ = write!(message, "`{attr}` \"{value}\" should match `/{resolved}/`");
                    message
                });
                reports.push((message, Span::new(value_start as u32, value_end as u32)));
            }
        }
        for (message, span) in reports {
            self.report_message("consistent-data-testid", message, span);
        }
    }

    pub(crate) fn scan_global_regex_queries(&mut self) {
        if !self.options.has_rule("no-global-regexp-flag-in-query") {
            return;
        }
        for prefix in ["ByText(/", "ByLabelText(/", "ByRole(/", "ByTestId(/"] {
            for index in find_all(self.source_text, prefix) {
                let tail = &self.source_text[index..self.source_text.len().min(index + 160)];
                if tail.contains("/g") || tail.contains("/gi") || tail.contains("/gu") {
                    self.report(
                        "no-global-regexp-flag-in-query",
                        "Do not use global regular expressions in Testing Library queries.",
                        span_for(index, prefix.len()),
                    );
                    return;
                }
            }
        }
    }

    pub(crate) fn scan_fire_event_promises(&mut self) {
        if !self.options.has_rule("no-promise-in-fire-event") {
            return;
        }
        for index in find_all(self.source_text, "fireEvent.") {
            let tail = &self.source_text[index..self.source_text.len().min(index + 220)];
            if tail.contains("await ") || tail.contains(".then(") || tail.contains("Promise.") {
                self.report(
                    "no-promise-in-fire-event",
                    "Do not pass promises to fireEvent calls.",
                    span_for(index, "fireEvent.".len()),
                );
                return;
            }
        }
    }

    pub(crate) fn scan_render_in_lifecycle(&mut self) {
        if !self.options.has_rule("no-render-in-lifecycle") {
            return;
        }
        for hook in ["beforeEach(", "beforeAll("] {
            for index in find_all(self.source_text, hook) {
                let tail = &self.source_text[index..self.source_text.len().min(index + 320)];
                if tail.contains("render(") {
                    self.report(
                        "no-render-in-lifecycle",
                        "Do not call render in setup lifecycle hooks.",
                        span_for(index, hook.len()),
                    );
                    return;
                }
            }
        }
    }

    pub(crate) fn scan_wait_for_rules(&mut self) {
        for index in find_all(self.source_text, "waitFor(") {
            let tail = &self.source_text[index..self.source_text.len().min(index + 420)];
            if count_occurrences(tail, "expect(") > 1 {
                self.report(
                    "no-wait-for-multiple-assertions",
                    "Avoid multiple assertions inside waitFor.",
                    span_for(index, "waitFor(".len()),
                );
            }
            if tail.contains("fireEvent.")
                || tail.contains("userEvent.")
                || tail.contains("render(")
            {
                self.report(
                    "no-wait-for-side-effects",
                    "Avoid side effects inside waitFor.",
                    span_for(index, "waitFor(".len()),
                );
            }
            if tail.contains("toMatchSnapshot(") || tail.contains("toMatchInlineSnapshot(") {
                self.report(
                    "no-wait-for-snapshot",
                    "Avoid snapshots inside waitFor.",
                    span_for(index, "waitFor(".len()),
                );
            }
            if tail.contains("getBy") || tail.contains("getAllBy") {
                self.report(
                    "prefer-find-by",
                    "Prefer findBy queries instead of waitFor with getBy queries.",
                    span_for(index, "waitFor(".len()),
                );
            }
        }
    }

    pub(crate) fn scan_prefer_explicit_assert(&mut self) {
        if !self.options.has_rule("prefer-explicit-assert") {
            return;
        }
        for prefix in ["screen.getBy", "screen.findBy", "getBy", "findBy"] {
            for index in find_all(self.source_text, prefix) {
                if !line_prefix(self.source_text, index).contains("expect(") {
                    self.report(
                        "prefer-explicit-assert",
                        "Wrap standalone queries in an explicit assertion.",
                        span_for(index, prefix.len()),
                    );
                    return;
                }
            }
        }
    }

    pub(crate) fn scan_expect_query_matchers(&mut self) {
        for index in find_all(self.source_text, "expect(") {
            let tail = &self.source_text[index..self.source_text.len().min(index + 260)];
            if tail.contains("getBy") && tail.contains(".toBeInTheDocument(") {
                self.report(
                    "prefer-implicit-assert",
                    "Prefer implicit getBy/findBy assertions when checking presence.",
                    span_for(index, "expect(".len()),
                );
            }
            if tail.contains("queryBy") && tail.contains(".toBeInTheDocument(") {
                self.report(
                    "prefer-presence-queries",
                    "Use presence queries that match the assertion.",
                    span_for(index, "expect(".len()),
                );
            }
            if tail.contains("queryBy")
                && (tail.contains(".toBeNull(") || tail.contains(".not.toBeInTheDocument("))
            {
                self.report(
                    "prefer-query-matchers",
                    "Prefer query matchers that describe element absence.",
                    span_for(index, "expect(".len()),
                );
            }
        }
    }

    pub(crate) fn scan_prefer_query_by_disappearance(&mut self) {
        if !self.options.has_rule("prefer-query-by-disappearance") {
            return;
        }
        for index in find_all(self.source_text, "waitForElementToBeRemoved(") {
            let tail = &self.source_text[index..self.source_text.len().min(index + 260)];
            if tail.contains("getBy") || tail.contains("findBy") {
                self.report(
                    "prefer-query-by-disappearance",
                    "Prefer queryBy queries when waiting for disappearance.",
                    span_for(index, "waitForElementToBeRemoved(".len()),
                );
                return;
            }
        }
    }

    pub(crate) fn scan_prefer_screen_queries(&mut self) {
        if !self.options.has_rule("prefer-screen-queries") {
            return;
        }
        for pattern in [
            "const { getBy",
            "const { queryBy",
            "const { findBy",
            "let { getBy",
        ] {
            if let Some(span) = self.find_pattern_span(pattern) {
                self.report(
                    "prefer-screen-queries",
                    "Prefer screen queries over destructured render queries.",
                    span,
                );
                return;
            }
        }
    }

    pub(crate) fn scan_render_result_names(&mut self) {
        if !self.options.has_rule("render-result-naming-convention") {
            return;
        }
        for pattern in [
            "const result = render(",
            "const wrapper = render(",
            "const rendered = render(",
        ] {
            if let Some(span) = self.find_pattern_span(pattern) {
                self.report(
                    "render-result-naming-convention",
                    "Use a conventional name for render results.",
                    span,
                );
                return;
            }
        }
    }
}

/// Derive the `{fileName}` substitution the way upstream `consistent-data-testid`
/// does: take the last path segment, drop its extension, and fall back to the
/// parent directory when the file is `index`. Bracketed names (e.g. Next.js
/// `[id].tsx`) yield no file name.
fn derive_file_name(filename: &str) -> Option<&str> {
    let mut segments: Vec<&str> = filename.split('/').collect();
    let file_with_ext = segments.pop()?;
    if file_with_ext.contains('[') || file_with_ext.contains(']') {
        return None;
    }
    let parent = segments.pop();
    let name = file_with_ext.split('.').next().unwrap_or("");
    if name == "index" { parent } else { Some(name) }
}
