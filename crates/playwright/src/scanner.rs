//! Regex-driven scanner for the playwright port.

use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;
use regex::Regex;

use crate::types::{Diagnostic, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 64]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan(&mut self) {
        self.check_regex(
            "consistent-spacing-between-blocks",
            r#"(?s)test\s*\(\s*['"]one['"].*?\);\s*\n\s*test\s*\(\s*['"]two['"]"#,
        );
        self.check_regex("expect-expect", r#"test\s*\(\s*['"]without assertions['"]"#);
        if self.source_text.contains("test(") && !self.source_text.contains("expect(") {
            self.report_at_first("expect-expect", "test(");
        }
        if self.source_text.matches("expect(").count() > 2 {
            self.report_at_first("max-expects", "expect(");
        }
        self.check_regex(
            "max-nested-describe",
            r#"(?s)test\.describe\s*\(.*?\{.*test\.describe\s*\("#,
        );
        self.check_regex(
            "missing-playwright-await",
            r#"(?m)(^|[^\w.])page\.(click|dblclick|fill|goto|locator|press|selectOption|setInputFiles|tap|type|uncheck|waitForLoadState)\s*\("#,
        );
        self.check_regex(
            "no-commented-out-tests",
            r#"(?m)^\s*//\s*(test|it|describe)(?:\.\w+)?\s*\("#,
        );
        self.check_regex(
            "no-conditional-expect",
            r#"(?s)\bif\s*\([^)]*\)\s*\{[^}]*expect\s*\("#,
        );
        self.check_regex(
            "no-conditional-in-test",
            r#"(?s)test\s*\(.*?\{[^}]*\b(if|switch|for|while)\s*\("#,
        );
        self.check_repeated("no-duplicate-hooks", "test.beforeEach");
        self.check_repeated("no-duplicate-slow", "test.slow");
        self.check_regex("no-element-handle", r#"\bElementHandle\b|page\.\$\s*\("#);
        self.check_regex("no-eval", r#"page\.\$\$?eval\s*\("#);
        self.check_regex("no-focused-test", r#"\b(test|describe)\.only\s*\("#);
        self.check_regex("no-force-option", r#"\bforce\s*:\s*true\b"#);
        self.check_regex("no-get-by-title", r#"\.getByTitle\s*\("#);
        self.check_regex(
            "no-hooks",
            r#"\btest\.(beforeAll|beforeEach|afterAll|afterEach)\s*\("#,
        );
        self.check_regex(
            "no-nested-step",
            r#"(?s)test\.step\s*\(.*?\{.*test\.step\s*\("#,
        );
        self.check_regex(
            "no-networkidle",
            r#"['"]networkidle['"]|waitUntil\s*:\s*['"]networkidle['"]"#,
        );
        self.check_regex("no-nth-methods", r#"\.(first|last|nth)\s*\("#);
        self.check_regex("no-page-pause", r#"page\.pause\s*\("#);
        self.check_regex("no-raw-locators", r#"page\.locator\s*\(\s*['"][^'"]+['"]"#);
        self.check_regex(
            "no-restricted-locators",
            r#"\.getByText\s*\(\s*['"]Forbidden['"]"#,
        );
        self.check_regex("no-restricted-matchers", r#"\.toBeTruthy\s*\("#);
        self.check_regex(
            "no-restricted-roles",
            r#"\.getByRole\s*\(\s*['"]button['"]"#,
        );
        self.check_regex("no-skipped-test", r#"\btest\.skip\s*\("#);
        self.check_regex("no-slowed-test", r#"\btest\.slow\s*\("#);
        self.check_regex("no-standalone-expect", r#"(?m)^\s*expect\s*\("#);
        self.check_regex(
            "no-unsafe-references",
            r#"page\.(evaluate|addInitScript)\s*\(\s*\(\s*\)\s*=>\s*\w+"#,
        );
        self.check_regex(
            "no-unused-locators",
            r#"\bconst\s+locator\s*=\s*page\.locator\s*\("#,
        );
        self.check_regex("no-useless-await", r#"\bawait\s+page\.locator\s*\("#);
        self.check_regex(
            "no-useless-not",
            r#"expect\s*\([^)]*\)\.not\.(toBeVisible|toBeHidden)\s*\("#,
        );
        self.check_regex("no-wait-for-navigation", r#"page\.waitForNavigation\s*\("#);
        self.check_regex("no-wait-for-selector", r#"page\.waitForSelector\s*\("#);
        self.check_regex("no-wait-for-timeout", r#"page\.waitForTimeout\s*\("#);
        self.check_regex(
            "prefer-comparison-matcher",
            r#"expect\s*\([^)]*(>|<|>=|<=)[^)]*\)\.toBe\s*\(\s*true\s*\)"#,
        );
        self.check_regex(
            "prefer-equality-matcher",
            r#"expect\s*\([^)]*(===|!==)[^)]*\)\.toBe\s*\(\s*(true|false)\s*\)"#,
        );
        self.check_regex(
            "prefer-hooks-in-order",
            r#"(?s)test\.afterEach\s*\([^;]*;\s*test\.beforeEach\s*\("#,
        );
        self.check_regex(
            "prefer-hooks-on-top",
            r#"(?s)test\.describe\s*\(.*?\{[^}]*test\s*\([^;]*;\s*test\.beforeEach\s*\("#,
        );
        self.check_regex(
            "prefer-locator",
            r#"page\.(click|dblclick|fill|press|selectOption)\s*\("#,
        );
        self.check_regex("prefer-lowercase-title", r#"test\s*\(\s*['"][A-Z]"#);
        self.check_regex(
            "prefer-native-locators",
            r#"page\.locator\s*\(\s*['"](?:text=|\[aria-label=|\[data-testid=)"#,
        );
        self.check_regex("prefer-strict-equal", r#"\.toEqual\s*\(\s*\{"#);
        self.check_regex(
            "prefer-to-be",
            r#"\.toEqual\s*\(\s*(true|false|null|undefined|\d+|['"])"#,
        );
        self.check_regex(
            r#"prefer-to-contain"#,
            r#"\.includes\s*\([^)]*\)\s*\)\.toBe\s*\(\s*true\s*\)"#,
        );
        self.check_regex(
            "prefer-to-have-count",
            r#"expect\s*\(\s*await\s+\w+\.count\s*\(\s*\)\s*\)\.toBe\s*\("#,
        );
        self.check_regex(
            "prefer-to-have-length",
            r#"expect\s*\([^)]*\.length\s*\)\.toBe\s*\("#,
        );
        self.check_regex(
            "prefer-web-first-assertions",
            r#"expect\s*\(\s*await\s+\w+\.(isVisible|isHidden|isEnabled|isDisabled|textContent|inputValue)\s*\("#,
        );
        self.check_regex(
            "require-hook",
            r#"(?m)^\s*const\s+\w+\s*=\s*(create|make|setup)\w*\s*\("#,
        );
        self.check_regex(
            "require-soft-assertions",
            r#"expect\s*\([^)]*\)\.(toBe|toEqual|toContain)"#,
        );
        self.check_regex("require-tags", r#"test\s*\(\s*['"][^@'"]+['"]"#);
        self.check_regex("require-to-pass-timeout", r#"\.toPass\s*\(\s*\)"#);
        self.check_regex("require-to-throw-message", r#"\.toThrow\s*\(\s*\)"#);
        self.check_regex(
            "require-top-level-describe",
            r#"(?m)^\s*test\s*\(\s*['"][^'"]+['"]"#,
        );
        self.check_regex(
            "valid-describe-callback",
            r#"test\.describe\s*\(\s*['"][^'"]+['"]\s*\)"#,
        );
        self.check_regex("valid-expect-in-promise", r#"(?s)\.then\s*\(.*expect\s*\("#);
        self.check_regex("valid-expect", r#"expect\s*\([^)]*\)\s*;"#);
        self.check_regex(
            "valid-test-tags",
            r#"test\s*\(\s*['"][^'"]*@bad tag[^'"]*['"]"#,
        );
        self.check_regex("valid-title", r#"(test|test\.describe)\s*\(\s*['"]\s*['"]"#);
    }

    fn check_regex(&mut self, rule_name: &'static str, pattern: &str) {
        if self.has_reported(rule_name) {
            return;
        }
        let Some(regex) = Regex::new(pattern).ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                rule_name,
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn check_repeated(&mut self, rule_name: &'static str, needle: &str) {
        if self.has_reported(rule_name) {
            return;
        }
        let first = self.source_text.find(needle);
        let second = first.and_then(|index| self.source_text[index + needle.len()..].find(needle));
        if let (Some(index), Some(_)) = (first, second) {
            self.report(
                rule_name,
                Span::new(index as u32, (index + needle.len()) as u32),
            );
        }
    }

    fn report_at_first(&mut self, rule_name: &'static str, needle: &str) {
        if self.has_reported(rule_name) {
            return;
        }
        if let Some(index) = self.source_text.find(needle) {
            self.report(
                rule_name,
                Span::new(index as u32, (index + needle.len()) as u32),
            );
        }
    }

    fn has_reported(&self, rule_name: &'static str) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_name == rule_name)
    }

    fn report(&mut self, rule_name: &'static str, span: Span) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id: "unexpected",
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}
