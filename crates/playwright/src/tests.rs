use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{
    PlaywrightOptions, RULE_NAMES, Restriction, TagPattern, TitlePattern, TitlePatternOptions,
    ValidTestTagsOptions, ValidTitleOptions, implemented_playwright_rule_names, scan_playwright,
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
test("@bad tag", { tag: "bad" }, () => {});
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
        ..PlaywrightOptions::default()
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
        ..PlaywrightOptions::default()
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

#[test]
fn valid_title_reports_exact_data_utf16_ranges_and_byte_fixes() {
    let source = concat!(
        "const marker = \"🧪\";\n",
        "test(\" test scenario \", () => {});\n",
        "test(\"test duplicate\", () => {});\n",
    );
    let diagnostics = scan_playwright(source, "fixture.spec.ts")
        .into_iter()
        .filter(|diagnostic| diagnostic.rule_name == "valid-title")
        .collect::<SmallVec<[_; 4]>>();

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect::<SmallVec<[_; 4]>>()
            .as_slice(),
        ["accidentalSpace", "duplicatePrefix"]
    );
    assert_eq!(
        (
            diagnostics[0].loc.start_line,
            diagnostics[0].loc.start_column,
            diagnostics[0].loc.end_line,
            diagnostics[0].loc.end_column,
        ),
        (2, 5, 2, 22)
    );
    let fix = diagnostics[0].fix.as_ref().expect("space fix");
    assert_eq!(
        &source[fix.start as usize..fix.end as usize],
        "\" test scenario \""
    );
    assert_eq!(fix.replacement, "\"test scenario\"");
    assert_eq!(
        diagnostics[1]
            .fix
            .as_ref()
            .map(|fix| fix.replacement.as_str()),
        Some("\"duplicate\"")
    );
}

#[test]
fn valid_title_supports_lookahead_patterns_custom_messages_and_call_groups() {
    let patterns = TitlePatternOptions {
        describe: Some(title_pattern(
            r"(?:#(?!unit|e2e))\w+",
            Some("invalid describe kind"),
        )),
        step: Some(title_pattern(
            r"(?:#(?!unit|e2e))\w+",
            Some("invalid step kind"),
        )),
        test: Some(title_pattern(
            r"(?:#(?!unit|e2e))\w+",
            Some("invalid test kind"),
        )),
    };
    let options = PlaywrightOptions {
        valid_title: ValidTitleOptions {
            must_not_match: patterns,
            ..ValidTitleOptions::default()
        },
        ..PlaywrightOptions::default()
    };
    let source = concat!(
        "test.describe(\"suite #wrong\", () => {});\n",
        "test(\"case #wrong\", () => {});\n",
        "test.step(\"action #wrong\", () => {});\n",
    );
    let diagnostics = scan_playwright_with_options(source, "fixture.spec.ts", &options)
        .into_iter()
        .filter(|diagnostic| diagnostic.rule_name == "valid-title")
        .collect::<SmallVec<[_; 4]>>();

    assert_eq!(diagnostics.len(), 3);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "mustNotMatchCustom")
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.data.function_name.as_deref())
            .collect::<SmallVec<[_; 4]>>()
            .as_slice(),
        [Some("describe"), Some("test"), Some("step")]
    );
    assert_eq!(
        diagnostics[0].data.pattern.as_deref(),
        Some(r"/(?:#(?!unit|e2e))\w+/u")
    );
}

#[test]
fn valid_test_tags_supports_exact_and_regex_lists_with_all_message_data() {
    let options = PlaywrightOptions {
        valid_test_tags: ValidTestTagsOptions {
            allowed_tags: [
                tag_pattern("@smoke", false),
                tag_pattern(r"^@team-\d+$", true),
            ]
            .into_iter()
            .collect(),
            disallowed_tags: Default::default(),
        },
        ..PlaywrightOptions::default()
    };
    let source = concat!(
        "test(\"@unknown title\", { tag: [\"@smoke\", \"@team-42\", \"bad\"] }, () => {});\n",
        "test.step(\"@team-nope step\", () => {});\n",
    );
    let diagnostics = scan_playwright_with_options(source, "fixture.spec.ts", &options)
        .into_iter()
        .filter(|diagnostic| diagnostic.rule_name == "valid-test-tags")
        .collect::<SmallVec<[_; 8]>>();

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect::<SmallVec<[_; 8]>>()
            .as_slice(),
        ["unknownTag", "invalidTagFormat", "unknownTag"]
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.data.tag.as_deref())
            .collect::<SmallVec<[_; 8]>>()
            .as_slice(),
        [Some("@unknown"), None, Some("@team-nope")]
    );
}

#[test]
fn pattern_rules_resolve_every_test_alias_form_and_ignore_malformed_input() {
    let source = concat!(
        "import { test as scenario } from \"@playwright/test\";\n",
        "const extended = scenario.extend({});\n",
        "scenario(\"test imported\", () => {});\n",
        "extended(\"test extended\", () => {});\n",
        "it(\"test configured\", () => {});\n",
    );
    let options = PlaywrightOptions {
        test_aliases: [CompactString::from("it")].into_iter().collect(),
        ..PlaywrightOptions::default()
    };
    let diagnostics = scan_playwright_with_options(source, "fixture.spec.ts", &options)
        .into_iter()
        .filter(|diagnostic| diagnostic.rule_name == "valid-title")
        .collect::<SmallVec<[_; 4]>>();

    assert_eq!(diagnostics.len(), 3);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "duplicatePrefix")
    );
    assert!(scan_playwright_with_options("test(\"", "fixture.spec.ts", &options).is_empty());
}

fn representative_options() -> PlaywrightOptions {
    PlaywrightOptions {
        restricted_locators: [restriction("getByText", None)].into_iter().collect(),
        restricted_matchers: [restriction("toBeTruthy", None)].into_iter().collect(),
        restricted_roles: [restriction("button", None)].into_iter().collect(),
        expect_aliases: Default::default(),
        ..PlaywrightOptions::default()
    }
}

fn restriction(value: &str, message: Option<&str>) -> Restriction {
    Restriction {
        value: CompactString::from(value),
        message: message.map(CompactString::from),
    }
}

fn title_pattern(source: &str, message: Option<&str>) -> TitlePattern {
    TitlePattern {
        source: CompactString::from(source),
        message: message.map(CompactString::from),
    }
}

fn tag_pattern(source: &str, is_regex: bool) -> TagPattern {
    TagPattern {
        source: CompactString::from(source),
        flags: CompactString::from("u"),
        is_regex,
    }
}
