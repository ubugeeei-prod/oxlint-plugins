use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{
    PlaywrightOptions, RULE_NAMES, Restriction, implemented_playwright_rule_names, scan_playwright,
    scan_playwright_with_options,
};

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
    let diagnostics = scan_playwright_with_options(
        REPRESENTATIVE_SOURCE,
        "fixture.spec.ts",
        &representative_options(),
    );
    let mut actual: SmallVec<[&str; 64]> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();
    let mut expected: SmallVec<[&str; 64]> = RULE_NAMES.into_iter().collect();
    actual.sort_unstable();
    actual.dedup();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn restricted_rules_honor_lists_custom_messages_and_exact_utf16_ranges() {
    let source = concat!(
        "const café = page.getByText(\"Submit\");\n",
        "expect(café).not.toBeTruthy();\n",
        "page.getByRole(`progressbar`, { name: \"Loading\" });\n",
    );
    let options = PlaywrightOptions {
        restricted_locators: [restriction("getByText", None)].into_iter().collect(),
        restricted_matchers: [restriction(
            "not.toBeTruthy",
            Some("Prefer a positive assertion"),
        )]
        .into_iter()
        .collect(),
        restricted_roles: [restriction(
            "progressbar",
            Some("Assert the loaded content"),
        )]
        .into_iter()
        .collect(),
        expect_aliases: Default::default(),
    };

    let diagnostics = scan_playwright_with_options(source, "fixture.ts", &options);
    let restricted = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_name.starts_with("no-restricted-"))
        .collect::<SmallVec<[_; 4]>>();

    assert_eq!(restricted.len(), 3);
    assert_eq!(restricted[0].rule_name, "no-restricted-locators");
    assert_eq!(restricted[0].message_id, "restricted");
    assert_eq!(restricted[0].data.method.as_deref(), Some("getByText"));
    assert_eq!(restricted[0].loc.start_column, 13);
    assert_eq!(restricted[0].loc.end_column, 37);

    assert_eq!(restricted[1].rule_name, "no-restricted-matchers");
    assert_eq!(restricted[1].message_id, "restrictedWithMessage");
    assert_eq!(restricted[1].data.message, "Prefer a positive assertion");
    assert_eq!(restricted[1].loc.start_column, 13);
    assert_eq!(restricted[1].loc.end_column, 27);

    assert_eq!(restricted[2].rule_name, "no-restricted-roles");
    assert_eq!(restricted[2].message_id, "restrictedWithMessage");
    assert_eq!(restricted[2].data.role.as_deref(), Some("progressbar"));
    assert_eq!(restricted[2].loc.start_column, 0);
    assert_eq!(restricted[2].loc.end_column, 50);
}

#[test]
fn restricted_rules_support_computed_names_aliases_and_last_duplicate_wins() {
    let source = concat!(
        "page[\"getByTestId\"](\"button\");\n",
        "assuming(value)[`not`][\"toBe\"]();\n",
        "page[`getByRole`](role);\n",
        "import { expect as assuming } from \"@playwright/test\";\n",
    );
    let options = PlaywrightOptions {
        restricted_locators: [
            restriction("getByTestId", Some("old")),
            restriction("getByTestId", Some("Use a role")),
        ]
        .into_iter()
        .collect(),
        restricted_matchers: [
            restriction("not", None),
            restriction("not.toBe", Some("Use a positive matcher")),
        ]
        .into_iter()
        .collect(),
        restricted_roles: [restriction("button", None)].into_iter().collect(),
        expect_aliases: Default::default(),
    };

    let diagnostics = scan_playwright_with_options(source, "fixture.ts", &options);
    let restricted = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.rule_name.starts_with("no-restricted-"))
        .collect::<SmallVec<[_; 4]>>();

    assert_eq!(restricted.len(), 3);
    assert_eq!(restricted[0].data.message, "Use a role");
    assert_eq!(restricted[1].data.restriction.as_deref(), Some("not"));
    assert_eq!(restricted[2].data.restriction.as_deref(), Some("not.toBe"));
    assert!(
        restricted
            .iter()
            .all(|diagnostic| diagnostic.rule_name != "no-restricted-roles")
    );
}

#[test]
fn restricted_rules_are_inert_without_options_and_on_parse_errors() {
    let source = concat!(
        "page.getByText(\"Forbidden\");\n",
        "expect(value).toBeTruthy();\n",
        "page.getByRole(\"button\");\n",
    );
    assert!(
        scan_playwright(source, "fixture.ts")
            .iter()
            .all(|diagnostic| !diagnostic.rule_name.starts_with("no-restricted-"))
    );
    assert!(
        scan_playwright_with_options("page.getByText(", "fixture.ts", &representative_options(),)
            .is_empty()
    );
}

fn representative_options() -> PlaywrightOptions {
    PlaywrightOptions {
        restricted_locators: [restriction("getByText", None)].into_iter().collect(),
        restricted_matchers: [restriction("toBeTruthy", None)].into_iter().collect(),
        restricted_roles: [restriction("button", None)].into_iter().collect(),
        expect_aliases: Default::default(),
    }
}

fn restriction(value: &str, message: Option<&str>) -> Restriction {
    Restriction {
        value: CompactString::from(value),
        message: message.map(CompactString::from),
    }
}
