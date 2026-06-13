//! Targeted scan_* methods for testing-library rules, grouped here to keep the
//! scanner driver file focused on dispatching.

use oxc_span::Span;

use crate::helpers::{
    count_occurrences, find_all, is_kebab_case, line_prefix, quoted_value_after, span_for,
};
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn scan_test_id_attributes(&mut self) {
        if !self.options.has_rule("consistent-data-testid") {
            return;
        }
        let pattern = "data-testid=";
        for index in find_all(self.source_text, pattern) {
            let Some((value, value_start, value_end)) =
                quoted_value_after(self.source_text, index + pattern.len())
            else {
                continue;
            };
            if !is_kebab_case(value) {
                self.report(
                    "consistent-data-testid",
                    "data-testid should use a consistent kebab-case format.",
                    Span::new(value_start as u32, value_end as u32),
                );
                return;
            }
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
