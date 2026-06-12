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
  'consistent-interface',
  'consistent-spacing-between-blocks',
  'handle-done-callback',
  'max-top-level-suites',
  'no-async-suite',
  'no-empty-title',
  'no-exclusive-tests',
  'no-exports',
  'no-global-tests',
  'no-hooks',
  'no-hooks-for-single-case',
  'no-identical-title',
  'no-mocha-arrows',
  'no-nested-tests',
  'no-pending-tests',
  'no-return-and-callback',
  'no-return-from-async',
  'no-setup-in-describe',
  'no-sibling-hooks',
  'no-synchronous-tests',
  'no-top-level-hooks',
  'prefer-arrow-callback',
  'valid-suite-title',
  'valid-test-title',
];

const invalidCases = [
  [
    'consistent-interface',
    'TDD call while BDD is required',
    'suite("x", function () {});\n',
    ['Unexpected use of TDD interface instead of BDD'],
    [{ interface: 'BDD' }],
  ],
  [
    'consistent-spacing-between-blocks',
    'adjacent tests',
    'describe("x", function () {\n  it("one", function () {});\n  it("two", function () {});\n});\n',
    ['Expected line break before this statement.'],
  ],
  [
    'handle-done-callback',
    'unused done callback',
    'it("x", function (done) {});\n',
    ['Expected "done" callback to be handled.'],
  ],
  [
    'max-top-level-suites',
    'two top-level suites',
    'describe("a", function () {});\ndescribe("b", function () {});\n',
    ['The number of top-level suites is more than 1.'],
  ],
  [
    'no-async-suite',
    'async suite callback',
    'describe("x", async function () {});\n',
    ['Unexpected async function in describe()'],
  ],
  [
    'no-empty-title',
    'empty title',
    'it("", function () {});\n',
    ['Unexpected empty test description.'],
  ],
  [
    'no-exclusive-tests',
    'only modifier',
    'it.only("x", function () {});\n',
    ['Unexpected exclusive mocha test.'],
  ],
  [
    'no-exports',
    'export in test file',
    'it("x", function () {});\nexport const value = 1;\n',
    ['Unexpected export from a test file'],
  ],
  [
    'no-global-tests',
    'top-level test',
    'it("x", function () {});\n',
    ['Unexpected global mocha test.'],
  ],
  [
    'no-hooks',
    'hook inside suite',
    'describe("x", function () {\n  before(function () {});\n  it("a", function () {});\n  it("b", function () {});\n});\n',
    ['Unexpected use of Mocha `before()` hook'],
  ],
  [
    'no-hooks-for-single-case',
    'single test suite hook',
    'describe("x", function () {\n  before(function () {});\n  it("a", function () {});\n});\n',
    ['Unexpected use of Mocha `before()` hook for a single test case'],
  ],
  [
    'no-identical-title',
    'duplicate test titles',
    'describe("x", function () {\n  it("a", function () {});\n  it("a", function () {});\n});\n',
    ['Unexpected use of duplicate Mocha title `a`'],
  ],
  ['no-mocha-arrows', 'arrow callback', 'it("x", () => {});\n', ['Unexpected arrow function.']],
  [
    'no-nested-tests',
    'nested test',
    'it("x", function () { it("y", function () {}); });\n',
    ['Unexpected test nested inside another test.'],
  ],
  ['no-pending-tests', 'skip modifier', 'it.skip("x");\n', ['Unexpected pending mocha test.']],
  [
    'no-return-and-callback',
    'return with callback',
    'it("x", function (done) { return fetch("/"); });\n',
    ['Unexpected use of `return` in a test with callback'],
  ],
  [
    'no-return-from-async',
    'return from async',
    'it("x", async function () { return fetch("/"); });\n',
    ['Unexpected use of `return` in a test with an async function'],
  ],
  [
    'no-setup-in-describe',
    'setup call in suite',
    'describe("x", function () {\n  helper();\n  it("a", function () {});\n});\n',
    ['Unexpected function call in describe block.'],
  ],
  [
    'no-sibling-hooks',
    'duplicate hooks',
    'describe("x", function () {\n  before(function () {});\n  before(function () {});\n  it("a", function () {});\n});\n',
    ['Unexpected use of duplicate Mocha `before()` hook'],
  ],
  [
    'no-synchronous-tests',
    'sync test',
    'it("x", function () {});\n',
    ['Unexpected synchronous test.'],
  ],
  [
    'no-top-level-hooks',
    'top-level hook',
    'before(function () {});\n',
    ['Unexpected use of Mocha `before()` hook outside of a test suite'],
  ],
  [
    'prefer-arrow-callback',
    'function callback',
    'it("x", function () { doThing(); });\n',
    ['Unexpected function expression.'],
  ],
  [
    'valid-suite-title',
    'bad suite title',
    'describe("bad", function () {});\n',
    ['Invalid "describe()" description found.'],
    [{ pattern: '^Suite' }],
  ],
  [
    'valid-test-title',
    'bad test title',
    'it("bad", function () {});\n',
    ['Invalid "it()" description found.'],
    [{ pattern: '^should' }],
  ],
];

function runRule(ruleName, sourceText, options = [], filename = 'fixture.test.js') {
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

function renderMessage(ruleName, report) {
  return plugin.rules[ruleName].meta.messages[report.messageId].replace(
    '{{message}}',
    report.data.message,
  );
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

function runOxlint(ruleName, code, options) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'mocha-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.test.js');
    const configPath = join(temp, 'oxlint.config.jsonc');
    const ruleConfig = options == null ? 'error' : ['error', options];

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'mocha',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`mocha/${ruleName}`]: ruleConfig,
        },
      }),
    );

    const result = spawnSync(
      oxlint,
      ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
      {
        encoding: 'utf8',
      },
    );
    const payload = result.stdout.trim() === '' ? { diagnostics: [] } : JSON.parse(result.stdout);

    return {
      diagnostics: payload.diagnostics ?? [],
      status: result.status,
      stderr: result.stderr,
    };
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

describe('mocha plugin shape', () => {
  it('exposes all ported rules', () => {
    expect(plugin.meta?.name).toBe('mocha');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedMochaRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanMocha).toBe('function');
  });

  it('ships upstream-compatible configs', () => {
    expect(plugin.configs.recommended.rules).toMatchObject({
      'mocha/handle-done-callback': 'error',
      'mocha/max-top-level-suites': ['error', { limit: 1 }],
      'mocha/no-exclusive-tests': 'warn',
      'mocha/no-hooks': 'off',
      'mocha/no-return-from-async': 'off',
      'mocha/consistent-spacing-between-blocks': 'error',
    });
    expect(plugin.configs.all.rules['mocha/consistent-interface']).toEqual([
      'error',
      { interface: 'BDD' },
    ]);
    expect(plugin.configs.recommended.languageOptions.globals).toMatchObject({
      describe: false,
      it: false,
      beforeEach: false,
    });
  });
});

describe('mocha rules through direct adapter harness', () => {
  it.each(invalidCases)('reports %s: %s', (ruleName, _name, code, messages, options = []) => {
    const reports = runRule(ruleName, code, options);

    expect(reports.map((report) => renderMessage(ruleName, report))).toEqual(messages);
  });

  it('honors selected rule options', () => {
    expect(
      runRule('no-hooks', 'describe("x", function () { before(function () {}); });', [
        { allow: ['before()'] },
      ]),
    ).toEqual([]);
    expect(
      runRule('handle-done-callback', 'it.skip("x", function (done) {});', [
        { ignorePending: true },
      ]),
    ).toEqual([]);
    expect(
      runRule('prefer-arrow-callback', 'it("x", function named() {});', [
        { allowNamedFunctions: true },
      ]),
    ).toEqual([]);
    expect(
      runRule('prefer-arrow-callback', 'it("x", function () { this.timeout(1000); });'),
    ).toEqual([]);
  });
});

describe('mocha rules through oxlint jsPlugins', () => {
  it.each(invalidCases)(
    'reports %s through the CLI: %s',
    (ruleName, _name, code, _messages, options = []) => {
      const actualRuleName = /** @type {string} */ (ruleName);
      const result = runOxlint(actualRuleName, code, options[0]);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(result.diagnostics).toHaveLength(1);
      expect(result.diagnostics[0].code).toBe(`mocha(${actualRuleName})`);
    },
  );
});
