import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { scanPlaywright } from '../api.js';
import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(here);
const workspaceRoot = resolve(packageRoot, '../..');
const fixture = JSON.parse(readFileSync(join(here, 'fixtures', 'thresholds-v2.10.4.json'), 'utf8'));
const validCases = fixture.suites.flatMap((suite) =>
  suite.valid.map((testCase, index) => ({ rule: suite.rule, index, testCase })),
);
const invalidCases = fixture.suites.flatMap((suite) =>
  suite.invalid.map((testCase, index) => ({ rule: suite.rule, index, testCase })),
);

describe('eslint-plugin-playwright v2.10.4 numeric threshold replay', () => {
  it('pins the complete authored inventory and exact source hashes', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-playwright',
      version: '2.10.4',
      sourceCommit: '894c0ec261763bb1e073b276c70bbf88b4ebad39',
      license: 'MIT',
      tool: 'tools/tasks/sync-playwright-threshold-tests.ts',
      inventory: {
        suites: 3,
        valid: 58,
        invalid: 42,
        diagnostics: 45,
      },
    });
    expect(fixture.__generated.sourceFiles).toHaveLength(6);
    expect(Object.values(fixture.__generated.sourceHashes)).toHaveLength(6);
    expect(
      Object.values(fixture.__generated.sourceHashes).every((hash) => /^[\da-f]{64}$/u.test(hash)),
    ).toBe(true);
  });

  it.each(validCases)(
    '$rule accepts authored valid case $index in both entry points',
    ({ rule, testCase }) => {
      expect(nativeDiagnostics(rule, testCase)).toEqual([]);
      expect(adapterReports(rule, testCase)).toEqual([]);
    },
  );

  it.each(invalidCases)('$rule reproduces authored invalid case $index', ({ rule, testCase }) => {
    const native = nativeDiagnostics(rule, testCase);
    const adapter = adapterReports(rule, testCase);
    expect(native.map(({ messageId }) => messageId)).toEqual(
      testCase.expectedDiagnostics.map(({ messageId }) => messageId),
    );
    expect(adapter).toEqual(
      native.map(({ messageId, data, loc }) => ({
        messageId,
        data,
        loc: {
          start: {
            line: loc.startLine,
            column: loc.startColumn,
          },
          end: {
            line: loc.endLine,
            column: loc.endColumn,
          },
        },
      })),
    );
    for (const [index, expected] of testCase.expectedDiagnostics.entries()) {
      if (expected.loc) {
        expect(native[index].loc).toMatchObject(expected.loc);
      }
      for (const [key, value] of Object.entries(expected.data)) {
        expect(native[index].data[key]).toBe(String(value));
      }
      expectThresholdData(rule, native[index]);
    }
    expect(
      adapter.map((report) =>
        renderMessage(plugin.rules[rule].meta.messages[report.messageId], report.data),
      ),
    ).toEqual(
      native.map((diagnostic) =>
        renderMessage(plugin.rules[rule].meta.messages[diagnostic.messageId], diagnostic.data),
      ),
    );
  });

  it('exposes the exact upstream messages, descriptions, types, and schemas', () => {
    expect(plugin.rules['max-expects'].meta).toMatchObject({
      type: 'suggestion',
      docs: {
        description: 'Enforces a maximum number assertion calls in a test body',
        recommended: false,
      },
      messages: {
        exceededMaxAssertion:
          'Too many assertion calls ({{ count }}) - maximum allowed is {{ max }}',
      },
      schema: [maximumSchema('max', 1, 'integer')],
    });
    expect(plugin.rules['max-nested-describe'].meta).toMatchObject({
      type: 'suggestion',
      docs: {
        description: 'Enforces a maximum depth to nested describe calls',
        recommended: true,
      },
      messages: {
        exceededMaxDepth:
          'Maximum describe call depth exceeded ({{ depth }}). Maximum allowed is {{ max }}.',
      },
      schema: [maximumSchema('max', 0, 'integer')],
    });
    expect(plugin.rules['require-top-level-describe'].meta).toMatchObject({
      type: 'suggestion',
      docs: {
        description: 'Require test cases and hooks to be inside a `test.describe` block',
        recommended: false,
      },
      messages: {
        tooManyDescribes: 'There should not be more than {{amount}} describe{{s}} at the top level',
        unexpectedHook: 'All hooks must be wrapped in a describe block.',
        unexpectedTest: 'All test cases must be wrapped in a describe block.',
      },
      schema: [maximumSchema('maxTopLevelDescribes', 1, 'number')],
    });
  });

  it('rejects malformed direct API threshold values', () => {
    const source = 'test("case", () => {});';
    expect(() => scanPlaywright(source, 'fixture.spec.ts', { maxExpects: 0 })).toThrow(
      'maxExpects must be an integer greater than or equal to 1.',
    );
    expect(() => scanPlaywright(source, 'fixture.spec.ts', { maxExpects: 1.5 })).toThrow(
      'maxExpects must be an integer greater than or equal to 1.',
    );
    expect(() => scanPlaywright(source, 'fixture.spec.ts', { maxNestedDescribe: -1 })).toThrow(
      'maxNestedDescribe must be an integer greater than or equal to 0.',
    );
    expect(() =>
      scanPlaywright(source, 'fixture.spec.ts', { maxTopLevelDescribes: Number.POSITIVE_INFINITY }),
    ).toThrow('maxTopLevelDescribes must be a finite number greater than or equal to 1.');
  });

  it('preserves exact UTF-16 locations and complete assertion data', () => {
    const source = [
      'const marker = "🧪";',
      'test("case", () => {',
      '  expect(1).toBe(1);',
      '  expect(2).toBe(2);',
      '});',
    ].join('\n');
    expect(
      scanPlaywright(source, 'fixture.spec.ts', { maxExpects: 1 }).filter(
        ({ ruleName }) => ruleName === 'max-expects',
      ),
    ).toMatchObject([
      {
        messageId: 'exceededMaxAssertion',
        data: { count: '2', max: '1' },
        loc: {
          startLine: 4,
          startColumn: 2,
          endLine: 4,
          endColumn: 19,
        },
      },
    ]);
  });

  it('supports configured, imported, and test.extend aliases in TypeScript sources', () => {
    const source = [
      'import { test as scenario, expect as assuming } from "@playwright/test";',
      'const custom = scenario.extend({});',
      'it("global", () => { verify(1).toBe(1); verify(2).toBe(2); });',
      'scenario.describe("outer", () => { custom.describe("inner", () => {}); });',
      'custom.beforeAll(() => {});',
      'assuming(1).toBe(1);',
    ].join('\n');
    const diagnostics = scanPlaywright(source, 'fixture.spec.ts', {
      testAliases: ['it'],
      expectAliases: ['verify'],
      maxExpects: 1,
      maxNestedDescribe: 1,
    });
    expect(
      diagnostics
        .filter(({ ruleName }) =>
          ['max-expects', 'max-nested-describe', 'require-top-level-describe'].includes(ruleName),
        )
        .map(({ ruleName, messageId }) => [ruleName, messageId]),
    ).toEqual([
      ['require-top-level-describe', 'unexpectedTest'],
      ['max-expects', 'exceededMaxAssertion'],
      ['max-nested-describe', 'exceededMaxDepth'],
      ['require-top-level-describe', 'unexpectedHook'],
    ]);
  });

  it('is inert on malformed input and keeps rule selection isolated', () => {
    expect(scanPlaywright('test("broken', 'fixture.spec.ts', { maxExpects: 1 })).toEqual([]);
    expect(
      runRule(
        'max-nested-describe',
        'test("top", () => { expect(1).toBe(1); expect(2).toBe(2); });',
        [{ max: 0 }],
      ),
    ).toEqual([]);
  });

  it('runs all three rules through real Oxlint on TypeScript', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-thresholds-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.spec.ts'),
        [
          'import { test as scenario, expect as assuming } from "@playwright/test";',
          'scenario("top", () => { assuming(1).toBe(1); assuming(2).toBe(2); });',
          'scenario.describe("outer", () => { scenario.describe("inner", () => {}); });',
        ].join('\n'),
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [{ name: 'playwright', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'playwright/max-expects': ['error', { max: 1 }],
            'playwright/max-nested-describe': ['error', { max: 1 }],
            'playwright/require-top-level-describe': 'error',
          },
        }),
      );
      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.spec.ts'],
        { cwd: tempDir, encoding: 'utf8' },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics.map(({ code, message }) => [code, message])).toEqual([
        ['playwright(max-expects)', 'Too many assertion calls (2) - maximum allowed is 1'],
        [
          'playwright(max-nested-describe)',
          'Maximum describe call depth exceeded (2). Maximum allowed is 1.',
        ],
        [
          'playwright(require-top-level-describe)',
          'All test cases must be wrapped in a describe block.',
        ],
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

function nativeDiagnostics(rule, testCase) {
  return scanPlaywright(testCase.code, 'fixture.spec.ts', scanOptions(rule, testCase)).filter(
    ({ ruleName }) => ruleName === rule,
  );
}

function adapterReports(rule, testCase) {
  return runRule(rule, testCase.code, testCase.options, testCase.settings);
}

function runRule(ruleName, sourceText, options = [], settings = null) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename: 'fixture.spec.ts',
    options,
    settings: settings ?? {},
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function scanOptions(rule, testCase) {
  const configured = testCase.options[0] ?? {};
  const globalAliases = testCase.settings?.playwright?.globalAliases ?? {};
  return {
    ...(rule === 'max-expects' ? { maxExpects: configured.max } : {}),
    ...(rule === 'max-nested-describe' ? { maxNestedDescribe: configured.max } : {}),
    ...(rule === 'require-top-level-describe'
      ? { maxTopLevelDescribes: configured.maxTopLevelDescribes }
      : {}),
    ...(Array.isArray(globalAliases.expect) ? { expectAliases: globalAliases.expect } : {}),
    ...(Array.isArray(globalAliases.test) ? { testAliases: globalAliases.test } : {}),
  };
}

function expectThresholdData(rule, diagnostic) {
  if (rule === 'max-expects') {
    expect(diagnostic.data).toMatchObject({
      count: expect.stringMatching(/^\d+$/u),
      max: expect.stringMatching(/^\d+$/u),
    });
  } else if (rule === 'max-nested-describe') {
    expect(diagnostic.data).toMatchObject({
      depth: expect.stringMatching(/^\d+$/u),
      max: expect.stringMatching(/^\d+$/u),
    });
  } else if (diagnostic.messageId === 'tooManyDescribes') {
    expect(diagnostic.data).toMatchObject({
      amount: expect.stringMatching(/^\d+(?:\.\d+)?$/u),
      s: expect.stringMatching(/^s?$/u),
    });
  }
}

function maximumSchema(property, minimum, type) {
  return {
    additionalProperties: false,
    properties: {
      [property]: { minimum, type },
    },
    type: 'object',
  };
}

function renderMessage(template, data) {
  return template.replace(/\{\{\s*(\w+)\s*\}\}/gu, (_match, key) => data?.[key] ?? '');
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
