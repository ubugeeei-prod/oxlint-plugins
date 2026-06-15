import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

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

const invalidCases = [
  ['consistent-spacing-between-blocks', 'test("one", () => {});\ntest("two", () => {});\n'],
  ['expect-expect', 'test("x", async ({ page }) => { await page.click("button"); });\n'],
  [
    'max-expects',
    'test("x", () => { expect(a).toBe(1); expect(b).toBe(2); expect(c).toBe(3); });\n',
  ],
  ['max-nested-describe', 'test.describe("outer", () => { test.describe("inner", () => {}); });\n'],
  ['missing-playwright-await', 'test("x", async ({ page }) => { page.click("button"); });\n'],
  ['no-commented-out-tests', '// test("commented", () => {});\n'],
  ['no-conditional-expect', 'test("x", () => { if (ready) { expect(value).toBe(1); } });\n'],
  ['no-conditional-in-test', 'test("x", () => { if (ready) doThing(); });\n'],
  ['no-duplicate-hooks', 'test.beforeEach(() => {});\ntest.beforeEach(() => {});\n'],
  ['no-duplicate-slow', 'test.slow();\ntest.slow();\n'],
  ['no-element-handle', 'let handle: ElementHandle;\n'],
  ['no-eval', 'page.$eval("button", (el) => el.textContent);\n'],
  ['no-focused-test', 'test.only("focused", () => {});\n'],
  ['no-force-option', 'page.click("button", { force: true });\n'],
  ['no-get-by-title', 'page.getByTitle("Title");\n'],
  ['no-hooks', 'test.beforeEach(() => {});\n'],
  [
    'no-nested-step',
    'test.step("outer", async () => { await test.step("inner", async () => {}); });\n',
  ],
  ['no-networkidle', 'page.goto("/", { waitUntil: "networkidle" });\n'],
  ['no-nth-methods', 'page.locator("li").nth(1);\n'],
  ['no-page-pause', 'page.pause();\n'],
  ['no-raw-locators', 'page.locator("button");\n'],
  ['no-restricted-locators', 'page.getByText("Forbidden");\n', [['getByText']]],
  ['no-restricted-matchers', 'expect(value).toBeTruthy();\n', [{ toBeTruthy: null }]],
  ['no-restricted-roles', 'page.getByRole("button");\n', [['button']]],
  ['no-skipped-test', 'test.skip("skipped", () => {});\n'],
  ['no-slowed-test', 'test.slow("slow", () => {});\n'],
  ['no-standalone-expect', 'expect(value).toBe(1);\n'],
  ['no-unsafe-references', 'const value = 1;\npage.evaluate(() => value);\n'],
  ['no-unused-locators', 'const locator = page.locator("button");\n'],
  ['no-useless-await', 'await page.locator("button");\n'],
  ['no-useless-not', 'expect(locator).not.toBeVisible();\n'],
  ['no-wait-for-navigation', 'page.waitForNavigation();\n'],
  ['no-wait-for-selector', 'page.waitForSelector("button");\n'],
  ['no-wait-for-timeout', 'page.waitForTimeout(1000);\n'],
  ['prefer-comparison-matcher', 'expect(count > 1).toBe(true);\n'],
  ['prefer-equality-matcher', 'expect(count === 1).toBe(true);\n'],
  ['prefer-hooks-in-order', 'test.afterEach(() => {});\ntest.beforeEach(() => {});\n'],
  [
    'prefer-hooks-on-top',
    'test.describe("x", () => { test("case", () => {}); test.beforeEach(() => {}); });\n',
  ],
  ['prefer-locator', 'page.fill("input", "value");\n'],
  ['prefer-lowercase-title', 'test("Should be lowercase", () => {});\n'],
  ['prefer-native-locators', 'page.locator("text=Submit");\n'],
  ['prefer-strict-equal', 'expect(value).toEqual({});\n'],
  ['prefer-to-be', 'expect(value).toEqual(true);\n'],
  ['prefer-to-contain', 'expect(items.includes(value)).toBe(true);\n'],
  ['prefer-to-have-count', 'expect(await rows.count()).toBe(2);\n'],
  ['prefer-to-have-length', 'expect(items.length).toBe(2);\n'],
  ['prefer-web-first-assertions', 'expect(await locator.isVisible()).toBe(true);\n'],
  ['require-hook', 'const user = createUser();\ntest("uses user", () => {});\n'],
  ['require-soft-assertions', 'expect(value).toBe(1);\n'],
  ['require-tags', 'test("missing tag", () => {});\n'],
  ['require-to-pass-timeout', 'expect(async () => {}).toPass();\n'],
  ['require-to-throw-message', 'expect(() => fn()).toThrow();\n'],
  ['require-top-level-describe', 'test("top level", () => {});\n'],
  ['valid-describe-callback', 'test.describe("no callback");\n'],
  ['valid-expect', 'expect(value);\n'],
  ['valid-expect-in-promise', 'Promise.resolve().then(() => expect(value).toBe(1));\n'],
  ['valid-test-tags', 'test("@bad tag", () => {});\n'],
  ['valid-title', 'test("", () => {});\n'],
];

function runRule(ruleName, sourceText, options = [], filename = 'fixture.spec.ts') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename,
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((a, b) => a.localeCompare(b));

  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }

  return candidates[candidates.length - 1];
}

describe('playwright plugin adapter', () => {
  it('exposes rules and recommended configs', () => {
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.configs['flat/recommended'].rules).toHaveProperty(
      'playwright/no-focused-test',
      'error',
    );
    expect(plugin.configs.recommended.rules).not.toHaveProperty('playwright/max-expects');
    expect(plugin.configs['playwright-test'].plugins).toEqual(['playwright']);
  });

  it.each(invalidCases)('reports %s through direct createOnce', (ruleName, code, options = []) => {
    const reports = runRule(ruleName, code, options);

    expect(reports).toHaveLength(1);
    expect(plugin.rules[ruleName].meta.messages[reports[0].messageId]).toBe(
      'Unexpected Playwright pattern.',
    );
  });

  it('suppresses restricted rules without configured options', () => {
    expect(runRule('no-restricted-locators', 'page.getByText("Forbidden");\n')).toHaveLength(0);
    expect(runRule('no-restricted-matchers', 'expect(value).toBeTruthy();\n')).toHaveLength(0);
    expect(runRule('no-restricted-roles', 'page.getByRole("button");\n')).toHaveLength(0);
  });

  it('loads through oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.spec.ts'),
        'test("x", async ({ page }) => { page.click("button"); });\n',
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: 'playwright',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'playwright/missing-playwright-await': 'error',
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.spec.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics).toHaveLength(1);
      expect(payload.diagnostics[0].message).toBe('Unexpected Playwright pattern.');
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
