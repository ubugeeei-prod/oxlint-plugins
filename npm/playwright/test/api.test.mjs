import { describe, expect, it } from 'vitest';

import { implementedPlaywrightRuleNames, scanPlaywright } from '../api.js';

const expectedRuleNames = [
  'consistent-spacing-between-blocks',
  'expect-expect',
  'max-expects',
  'max-nested-describe',
  'missing-playwright-await',
  'no-commented-out-tests',
  'no-conditional-expect',
  'no-conditional-in-test',
  'no-duplicate-hooks',
  'no-duplicate-slow',
  'no-element-handle',
  'no-eval',
  'no-focused-test',
  'no-force-option',
  'no-get-by-title',
  'no-hooks',
  'no-nested-step',
  'no-networkidle',
  'no-nth-methods',
  'no-page-pause',
  'no-raw-locators',
  'no-restricted-locators',
  'no-restricted-matchers',
  'no-restricted-roles',
  'no-skipped-test',
  'no-slowed-test',
  'no-standalone-expect',
  'no-unsafe-references',
  'no-unused-locators',
  'no-useless-await',
  'no-useless-not',
  'no-wait-for-navigation',
  'no-wait-for-selector',
  'no-wait-for-timeout',
  'prefer-comparison-matcher',
  'prefer-equality-matcher',
  'prefer-hooks-in-order',
  'prefer-hooks-on-top',
  'prefer-locator',
  'prefer-lowercase-title',
  'prefer-native-locators',
  'prefer-strict-equal',
  'prefer-to-be',
  'prefer-to-contain',
  'prefer-to-have-count',
  'prefer-to-have-length',
  'prefer-web-first-assertions',
  'require-hook',
  'require-soft-assertions',
  'require-tags',
  'require-to-pass-timeout',
  'require-to-throw-message',
  'require-top-level-describe',
  'valid-describe-callback',
  'valid-expect',
  'valid-expect-in-promise',
  'valid-test-tags',
  'valid-title',
];

const representativeSource = `
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
`;

describe('playwright native API', () => {
  it('exposes all eslint-plugin-playwright rule names', () => {
    expect(implementedPlaywrightRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans representative Playwright patterns for every rule', () => {
    const diagnostics = scanPlaywright(representativeSource, 'fixture.spec.ts');

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName).sort()).toEqual(
      [...expectedRuleNames].sort(),
    );
  });

  it('returns LSP-shaped locations', () => {
    const [diagnostic] = scanPlaywright(
      'test("x", async ({ page }) => { page.click("button"); });\n',
      'fixture.spec.ts',
    );

    expect(diagnostic).toMatchObject({
      ruleName: 'expect-expect',
      messageId: 'unexpected',
      loc: {
        startLine: 1,
        startColumn: 0,
        endLine: 1,
      },
    });
  });
});
