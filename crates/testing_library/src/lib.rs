#![doc = "Rust implementation of eslint-plugin-testing-library rule logic."]
#![allow(
    clippy::disallowed_types,
    reason = "The native scanner stores compact NAPI-facing diagnostics and scans short source slices."
)]

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 29] = [
    "await-async-events",
    "await-async-queries",
    "await-async-utils",
    "consistent-data-testid",
    "no-await-sync-events",
    "no-await-sync-queries",
    "no-container",
    "no-debugging-utils",
    "no-dom-import",
    "no-global-regexp-flag-in-query",
    "no-manual-cleanup",
    "no-node-access",
    "no-promise-in-fire-event",
    "no-render-in-lifecycle",
    "no-test-id-queries",
    "no-unnecessary-act",
    "no-wait-for-multiple-assertions",
    "no-wait-for-side-effects",
    "no-wait-for-snapshot",
    "prefer-explicit-assert",
    "prefer-find-by",
    "prefer-implicit-assert",
    "prefer-presence-queries",
    "prefer-query-by-disappearance",
    "prefer-query-matchers",
    "prefer-screen-queries",
    "prefer-user-event",
    "prefer-user-event-setup",
    "render-result-naming-convention",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message: CompactString,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestingLibraryOptions {
    pub rule_names: SmallVec<[CompactString; 29]>,
    pub test_id_pattern: CompactString,
}

impl Default for TestingLibraryOptions {
    fn default() -> Self {
        Self {
            rule_names: RULE_NAMES
                .iter()
                .map(|rule_name| CompactString::from(*rule_name))
                .collect(),
            test_id_pattern: CompactString::from("kebab-case"),
        }
    }
}

impl TestingLibraryOptions {
    fn has_rule(&self, rule_name: &str) -> bool {
        self.rule_names.iter().any(|name| name == rule_name)
    }
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }
}

pub fn implemented_testing_library_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_testing_library(
    source_text: &str,
    filename: &str,
    options: &TestingLibraryOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
        options,
    };
    scanner.scan();
    scanner.diagnostics
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 32]>,
    options: &'a TestingLibraryOptions,
}

impl<'a> Scanner<'a> {
    fn scan(&mut self) {
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

    fn report(&mut self, rule_name: &'static str, message: &'static str, span: Span) {
        if self.options.has_rule(rule_name) {
            self.diagnostics.push(Diagnostic {
                rule_name,
                message: message.into(),
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

    fn scan_test_id_attributes(&mut self) {
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

    fn scan_global_regex_queries(&mut self) {
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

    fn scan_fire_event_promises(&mut self) {
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

    fn scan_render_in_lifecycle(&mut self) {
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

    fn scan_wait_for_rules(&mut self) {
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

    fn scan_prefer_explicit_assert(&mut self) {
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

    fn scan_expect_query_matchers(&mut self) {
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

    fn scan_prefer_query_by_disappearance(&mut self) {
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

    fn scan_prefer_screen_queries(&mut self) {
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

    fn scan_render_result_names(&mut self) {
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

    fn find_pattern_span(&self, pattern: &str) -> Option<Span> {
        self.source_text
            .find(pattern)
            .map(|index| span_for(index, pattern.len()))
    }

    fn is_handled(&self, index: usize) -> bool {
        let prefix = line_prefix(self.source_text, index);
        prefix.contains("await ") || prefix.contains("return ") || prefix.contains("void ")
    }
}

fn span_for(index: usize, len: usize) -> Span {
    Span::new(index as u32, (index + len) as u32)
}

fn find_all<'a>(source_text: &'a str, pattern: &'a str) -> impl Iterator<Item = usize> + 'a {
    source_text.match_indices(pattern).map(|(index, _)| index)
}

fn line_prefix(source_text: &str, index: usize) -> &str {
    let line_start = source_text[..index].rfind('\n').map_or(0, |line| line + 1);
    &source_text[line_start..index]
}

fn quoted_value_after(source_text: &str, start: usize) -> Option<(&str, usize, usize)> {
    let quote = source_text.as_bytes().get(start).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let value_start = start + 1;
    let value_end = source_text[value_start..]
        .find(quote as char)
        .map(|offset| value_start + offset)?;
    Some((&source_text[value_start..value_end], value_start, value_end))
}

fn is_kebab_case(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn count_occurrences(source_text: &str, pattern: &str) -> usize {
    source_text.match_indices(pattern).count()
}

#[cfg(test)]
mod tests {
    use super::{
        TestingLibraryOptions, implemented_testing_library_rule_names, scan_testing_library,
    };

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(implemented_testing_library_rule_names().len(), 29);
        assert!(implemented_testing_library_rule_names().contains(&"await-async-events"));
        assert!(
            implemented_testing_library_rule_names().contains(&"render-result-naming-convention")
        );
    }

    #[test]
    fn scans_representative_rules() {
        let diagnostics = scan_testing_library(
            r#"
import { fireEvent } from '@testing-library/dom';
const { getByText } = render(<Button data-testid="BadId" />);
userEvent.click(button);
await fireEvent.click(button);
screen.getByText(/Save/g);
cleanup();
container.querySelector('.button');
waitFor(() => { expect(a).toBe(1); expect(b).toBe(2); fireEvent.click(button); expect(screen.getByText('x')).toBeInTheDocument(); });
waitForElementToBeRemoved(() => screen.getByText('gone'));
const result = render(<Button />);
"#,
            "fixture.test.tsx",
            &TestingLibraryOptions::default(),
        );
        let rules: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        assert!(rules.contains(&"await-async-events"));
        assert!(rules.contains(&"consistent-data-testid"));
        assert!(rules.contains(&"no-await-sync-events"));
        assert!(rules.contains(&"no-dom-import"));
        assert!(rules.contains(&"no-global-regexp-flag-in-query"));
        assert!(rules.contains(&"no-manual-cleanup"));
        assert!(rules.contains(&"no-container"));
        assert!(rules.contains(&"no-node-access"));
        assert!(rules.contains(&"no-wait-for-multiple-assertions"));
        assert!(rules.contains(&"no-wait-for-side-effects"));
        assert!(rules.contains(&"prefer-find-by"));
        assert!(rules.contains(&"prefer-query-by-disappearance"));
        assert!(rules.contains(&"prefer-user-event"));
        assert!(rules.contains(&"prefer-screen-queries"));
        assert!(rules.contains(&"render-result-naming-convention"));
    }

    #[test]
    fn accepts_awaited_async_interactions() {
        let options = TestingLibraryOptions {
            rule_names: ["await-async-events".into()].into_iter().collect(),
            ..TestingLibraryOptions::default()
        };
        assert!(
            scan_testing_library(
                "await userEvent.click(button);",
                "fixture.test.ts",
                &options
            )
            .is_empty()
        );
    }
}
