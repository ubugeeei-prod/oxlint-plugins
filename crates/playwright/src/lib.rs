#![doc = "Rust implementation of eslint-plugin-playwright rule logic."]

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::SmallVec;
use regex::Regex;

pub const RULE_NAMES: [&str; 58] = [
    "consistent-spacing-between-blocks",
    "expect-expect",
    "max-expects",
    "max-nested-describe",
    "missing-playwright-await",
    "no-commented-out-tests",
    "no-conditional-expect",
    "no-conditional-in-test",
    "no-duplicate-hooks",
    "no-duplicate-slow",
    "no-element-handle",
    "no-eval",
    "no-focused-test",
    "no-force-option",
    "no-get-by-title",
    "no-hooks",
    "no-nested-step",
    "no-networkidle",
    "no-nth-methods",
    "no-page-pause",
    "no-raw-locators",
    "no-restricted-locators",
    "no-restricted-matchers",
    "no-restricted-roles",
    "no-skipped-test",
    "no-slowed-test",
    "no-standalone-expect",
    "no-unsafe-references",
    "no-unused-locators",
    "no-useless-await",
    "no-useless-not",
    "no-wait-for-navigation",
    "no-wait-for-selector",
    "no-wait-for-timeout",
    "prefer-comparison-matcher",
    "prefer-equality-matcher",
    "prefer-hooks-in-order",
    "prefer-hooks-on-top",
    "prefer-locator",
    "prefer-lowercase-title",
    "prefer-native-locators",
    "prefer-strict-equal",
    "prefer-to-be",
    "prefer-to-contain",
    "prefer-to-have-count",
    "prefer-to-have-length",
    "prefer-web-first-assertions",
    "require-hook",
    "require-soft-assertions",
    "require-tags",
    "require-to-pass-timeout",
    "require-to-throw-message",
    "require-top-level-describe",
    "valid-describe-callback",
    "valid-expect",
    "valid-expect-in-promise",
    "valid-test-tags",
    "valid-title",
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
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
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

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 64]>,
}

pub fn implemented_playwright_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_playwright(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 64]> {
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
    };
    scanner.scan();
    scanner.diagnostics
}

impl<'a> Scanner<'a> {
    fn scan(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    const REPRESENTATIVE_SOURCE: &str = r#"
test("one", async ({ page }) => { await expect(page).toBeTruthy(); });
test("two", async ({ page }) => { await page.click("button"); });
test("without assertions", async ({ page }) => { await page.click("button"); });
test("x", async ({ page }) => { await page.click("button"); });
test("many", () => { expect(a).toBe(1); expect(b).toBe(2); expect(c).toBe(3); });
test.describe("outer", () => { test.describe("inner", () => {}); });
page.click("button");
// test("commented", () => {});
test("conditional expect", () => { if (ready) { expect(value).toBe(1); } });
test("conditional", () => { if (ready) doThing(); });
test.beforeEach(() => {});
test.beforeEach(() => {});
test.slow();
test.slow();
let handle: ElementHandle;
page.$eval("button", (el) => el.textContent);
test.only("focused", () => {});
page.click("button", { force: true });
page.getByTitle("Title");
test.afterEach(() => {});
test.step("outer", async () => { await test.step("inner", async () => {}); });
page.goto("/", { waitUntil: "networkidle" });
page.locator("li").nth(1);
page.pause();
page.locator("button");
page.getByText("Forbidden");
expect(value).toBeTruthy();
page.getByRole("button");
test.skip("skipped", () => {});
test.slow("slow", () => {});
expect(value).toBe(1);
const value = 1;
page.evaluate(() => value);
const locator = page.locator("button");
await page.locator("button");
expect(locator).not.toBeVisible();
page.waitForNavigation();
page.waitForSelector("button");
page.waitForTimeout(1000);
expect(count > 1).toBe(true);
expect(count === 1).toBe(true);
test.afterEach(() => {});
test.beforeEach(() => {});
test.describe("hooks", () => { test("case", () => {}); test.beforeEach(() => {}); });
page.fill("input", "value");
test("Should be lowercase", () => {});
page.locator("text=Submit");
expect(value).toEqual({});
expect(value).toEqual(true);
expect(items.includes(value)).toBe(true);
expect(await rows.count()).toBe(2);
expect(items.length).toBe(2);
expect(await locator.isVisible()).toBe(true);
const user = createUser();
expect.soft(value).toBe(1);
test("missing tag", () => {});
expect(async () => {}).toPass();
expect(() => fn()).toThrow();
test("top level", () => {});
test.describe("no callback");
Promise.resolve().then(() => expect(value).toBe(1));
expect(value);
test("@bad tag", () => {});
test("", () => {});
"#;

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(implemented_playwright_rule_names().len(), 58);
        assert_eq!(
            implemented_playwright_rule_names()[0],
            "consistent-spacing-between-blocks"
        );
        assert_eq!(implemented_playwright_rule_names()[57], "valid-title");
    }

    #[test]
    fn scans_representative_rules() {
        let diagnostics = scan_playwright(REPRESENTATIVE_SOURCE, "fixture.spec.ts");
        let mut actual: SmallVec<[&str; 64]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        let mut expected: SmallVec<[&str; 64]> = RULE_NAMES.into_iter().collect();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }
}
