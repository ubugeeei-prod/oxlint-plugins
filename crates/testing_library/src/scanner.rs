//! Driver for testing-library rules. Heavy per-rule logic lives in `checks.rs`.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{find_all, line_prefix, span_for};
use crate::types::{Diagnostic, LineIndex, TestingLibraryOptions};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) filename: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
    pub(crate) options: &'a TestingLibraryOptions,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan(&mut self) {
        self.report_unawaited_patterns(
            "await-async-events",
            &["userEvent.", "user."],
            "Async user-event interactions should be awaited.",
        );
        self.report_unawaited_patterns(
            "await-async-queries",
            &["findBy", "findAllBy"],
            "Async Testing Library queries should be awaited.",
        );
        self.report_unawaited_patterns(
            "await-async-utils",
            &["waitFor(", "waitForElementToBeRemoved("],
            "Async Testing Library utilities should be awaited.",
        );
        self.scan_test_id_attributes();
        self.report_patterns(
            "no-await-sync-events",
            &["await fireEvent."],
            "Do not await synchronous fireEvent calls.",
        );
        self.report_patterns(
            "no-await-sync-queries",
            &[
                "await screen.getBy",
                "await screen.queryBy",
                "await getBy",
                "await queryBy",
            ],
            "Do not await synchronous Testing Library queries.",
        );
        self.report_patterns(
            "no-container",
            &["container.", "baseElement."],
            "Avoid direct container access. Prefer Testing Library queries.",
        );
        self.report_patterns(
            "no-debugging-utils",
            &["screen.debug(", "debug(", "logTestingPlaygroundURL("],
            "Do not leave Testing Library debugging utilities in tests.",
        );
        self.report_patterns(
            "no-dom-import",
            &["'@testing-library/dom'", "\"@testing-library/dom\""],
            "Do not import DOM Testing Library directly.",
        );
        self.scan_global_regex_queries();
        self.report_patterns(
            "no-manual-cleanup",
            &["cleanup("],
            "Do not call cleanup manually.",
        );
        self.report_patterns(
            "no-node-access",
            &[
                ".firstChild",
                ".lastChild",
                ".childNodes",
                ".children",
                ".parentElement",
                ".querySelector(",
                ".querySelectorAll(",
                ".closest(",
            ],
            "Avoid direct Node access. Prefer Testing Library queries.",
        );
        self.scan_fire_event_promises();
        self.scan_render_in_lifecycle();
        self.report_patterns(
            "no-test-id-queries",
            &["ByTestId("],
            "Avoid data-testid queries when user-visible queries are available.",
        );
        self.report_patterns(
            "no-unnecessary-act",
            &["act("],
            "Avoid unnecessary act wrappers around Testing Library utilities.",
        );
        self.scan_wait_for_rules();
        self.scan_prefer_explicit_assert();
        self.scan_expect_query_matchers();
        self.scan_prefer_query_by_disappearance();
        self.scan_prefer_screen_queries();
        self.report_patterns(
            "prefer-user-event",
            &["fireEvent."],
            "Prefer userEvent over fireEvent.",
        );
        self.report_patterns(
            "prefer-user-event-setup",
            &[
                "userEvent.click(",
                "userEvent.type(",
                "userEvent.hover(",
                "userEvent.keyboard(",
            ],
            "Prefer userEvent.setup() over direct userEvent calls.",
        );
        self.scan_render_result_names();
    }

    pub(crate) fn report(&mut self, rule_name: &'static str, message: &'static str, span: Span) {
        self.report_message(rule_name, message.into(), span);
    }

    pub(crate) fn report_message(
        &mut self,
        rule_name: &'static str,
        message: CompactString,
        span: Span,
    ) {
        if self.options.has_rule(rule_name) {
            self.diagnostics.push(Diagnostic {
                rule_name,
                message,
                loc: self.line_index.loc_for_span(self.source_text, span),
            });
        }
    }

    fn report_patterns(
        &mut self,
        rule_name: &'static str,
        patterns: &[&str],
        message: &'static str,
    ) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        for pattern in patterns {
            if let Some(span) = self.find_pattern_span(pattern) {
                self.report(rule_name, message, span);
                return;
            }
        }
    }

    fn report_unawaited_patterns(
        &mut self,
        rule_name: &'static str,
        patterns: &[&str],
        message: &'static str,
    ) {
        if !self.options.has_rule(rule_name) {
            return;
        }
        for pattern in patterns {
            for index in find_all(self.source_text, pattern) {
                if !self.is_handled(index) {
                    self.report(rule_name, message, span_for(index, pattern.len()));
                    return;
                }
            }
        }
    }

    pub(crate) fn find_pattern_span(&self, pattern: &str) -> Option<Span> {
        self.source_text
            .find(pattern)
            .map(|index| span_for(index, pattern.len()))
    }

    fn is_handled(&self, index: usize) -> bool {
        let prefix = line_prefix(self.source_text, index);
        prefix.contains("await ") || prefix.contains("return ") || prefix.contains("void ")
    }
}
