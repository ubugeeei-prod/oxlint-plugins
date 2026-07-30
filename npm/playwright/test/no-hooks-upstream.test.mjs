import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { scanPlaywright } from '../api.js';
import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const fixture = JSON.parse(
  readFileSync(join(packageRoot, 'test/fixtures/no-hooks-v2.11.0.json'), 'utf8'),
);
const ruleName = 'no-hooks';
const rule = plugin.rules[ruleName];

describe('playwright no-hooks upstream v2.11.0 fixtures', () => {
  it('pins exact upstream sources, previous-version drift, and authored inventory', () => {
    expect(fixture.__generated).toEqual({
      source: 'eslint-plugin-playwright',
      version: '2.11.0',
      sourceCommit: 'b6d3e5dac73c8aad4d5e62a933105579c319655f',
      license: 'MIT',
      tool: 'tools/tasks/sync-playwright-no-hooks-tests.ts',
      sourceFiles: [
        'src/rules/no-hooks.ts',
        'src/rules/no-hooks.test.ts',
        'docs/rules/no-hooks.md',
      ],
      sourceHashes: {
        'src/rules/no-hooks.ts': 'bfa7ad98ee3e395919654904c5eaf1dc64c0dfb24ebd07321d9247a9d7a98c57',
        'src/rules/no-hooks.test.ts':
          '94d9bc99d3db616b81d8dff8355bf2bb23a4a69262b3a2806819e744868b9a07',
        'docs/rules/no-hooks.md':
          'aef2a1119c63d0469fd99b98882b2953ee450a8c63c6de5b99ca0bfeb01d3e7e',
      },
      previousVersionAudit: {
        version: '2.10.4',
        sourceCommit: '894c0ec261763bb1e073b276c70bbf88b4ebad39',
        changedFiles: [],
      },
      inventory: {
        suites: 1,
        valid: 5,
        invalid: 6,
        diagnostics: 6,
        fixable: 0,
      },
    });
  });

  it.each(
    fixture.valid.map((testCase, index) => ({
      ...testCase,
      label: `valid ${index + 1}`,
    })),
  )('$label is valid through native and plugin paths', (testCase) => {
    expect(nativeDiagnostics(testCase)).toEqual([]);
    expect(adapterReports(testCase)).toEqual([]);
  });

  it.each(
    fixture.invalid.map((testCase, index) => ({
      ...testCase,
      label: `invalid ${index + 1}`,
    })),
  )('$label matches exact authored messages, data, and no-fix contract', (testCase) => {
    const diagnostics = nativeDiagnostics(testCase);
    const reports = adapterReports(testCase);

    expect(
      diagnostics.map(({ messageId, data }) => ({
        messageId,
        data: compactData(data),
      })),
    ).toEqual(testCase.expectedDiagnostics);
    expect(
      reports.map(({ messageId, data }) => ({
        messageId,
        data: compactData(data),
      })),
    ).toEqual(testCase.expectedDiagnostics);
    expect(reports.map(({ loc }) => loc)).toEqual(
      diagnostics.map(({ loc }) => ({
        start: { line: loc.startLine, column: loc.startColumn },
        end: { line: loc.endLine, column: loc.endColumn },
      })),
    );
    expect(diagnostics.every((diagnostic) => diagnostic.fix == null)).toBe(true);
    expect(reports.every((report) => report.fix === undefined)).toBe(true);
    expect(
      reports.map((report) =>
        rule.meta.messages[report.messageId].replace('{{ hookName }}', report.data.hookName),
      ),
    ).toEqual(
      testCase.expectedDiagnostics.map(
        (diagnostic) => `Unexpected '${diagnostic.data.hookName}' hook`,
      ),
    );
  });

  it('exposes exact upstream metadata, schema, and non-fixable contract', () => {
    expect(rule.meta).toMatchObject({
      type: 'suggestion',
      docs: {
        description: 'Disallow setup and teardown hooks',
        recommended: false,
        url: 'https://github.com/mskelton/eslint-plugin-playwright/tree/main/docs/rules/no-hooks.md',
      },
      messages: {
        unexpectedHook: "Unexpected '{{ hookName }}' hook",
      },
      schema: [
        {
          additionalProperties: false,
          properties: {
            allow: {
              contains: ['beforeAll', 'beforeEach', 'afterAll', 'afterEach'],
              type: 'array',
            },
          },
          type: 'object',
        },
      ],
    });
    expect(rule.meta.fixable).toBeUndefined();
    expect(rule.meta.hasSuggestions).toBeUndefined();
  });

  it('reports complete call ranges and canonical data for every member and bare hook', () => {
    const code = [
      '"🧪"; test.beforeAll(() => {});',
      'test["beforeEach"](() => {});',
      'test[`afterAll`]();',
      'afterEach(() => {});',
    ].join('\n');
    const diagnostics = nativeDiagnostics({ code, options: [], settings: null });

    expect(diagnostics.map(({ data, loc }) => [data.hookName, loc])).toEqual([
      ['beforeAll', { startLine: 1, startColumn: 6, endLine: 1, endColumn: 30 }],
      ['beforeEach', { startLine: 2, startColumn: 0, endLine: 2, endColumn: 28 }],
      ['afterAll', { startLine: 3, startColumn: 0, endLine: 3, endColumn: 18 }],
      ['afterEach', { startLine: 4, startColumn: 0, endLine: 4, endColumn: 19 }],
    ]);
  });

  it('supports configured globals, import aliases, test imports, and transitive extend aliases', () => {
    const code = [
      'import { test as scenario, beforeAll as setupSuite } from "another-runner";',
      'const later = custom.extend({});',
      'const custom = scenario["extend"]({})[`extend`]({});',
      'setupEach(() => {});',
      'setupSuite(() => {});',
      'scenario.beforeEach(() => {});',
      'custom.afterAll(() => {});',
      'later[`afterEach`](() => {});',
    ].join('\n');
    const testCase = {
      code,
      options: [],
      settings: {
        playwright: {
          globalAliases: {
            beforeEach: ['setupEach'],
          },
        },
      },
    };
    const diagnostics = nativeDiagnostics(testCase);

    expect(diagnostics.map(({ data }) => data.hookName)).toEqual([
      'beforeEach',
      'beforeAll',
      'beforeEach',
      'afterAll',
      'afterEach',
    ]);
    expect(diagnostics.map(({ loc }) => loc.startLine)).toEqual([4, 5, 6, 7, 8]);
    expect(adapterReports(testCase)).toHaveLength(5);
  });

  it('honors allow after alias resolution without suppressing other hooks', () => {
    const testCase = {
      code: [
        'setupEach(() => {});',
        'test.beforeEach(() => {});',
        'test.afterEach(() => {});',
        'afterAll(() => {});',
      ].join('\n'),
      options: [{ allow: ['beforeEach', 'afterAll'] }],
      settings: {
        playwright: {
          globalAliases: {
            beforeEach: ['setupEach'],
          },
        },
      },
    };
    expect(nativeDiagnostics(testCase).map(({ data }) => data.hookName)).toEqual(['afterEach']);
    expect(adapterReports(testCase)).toHaveLength(1);
  });

  it('ignores unrelated members, invalid chains, references, and dynamic properties', () => {
    const code = [
      'subject.beforeEach();',
      'runner.afterAll(() => {});',
      'test.describe.beforeEach(() => {});',
      'test.beforeEach.extra(() => {});',
      'test.beforeEach;',
      'test[hookName](() => {});',
    ].join('\n');
    expect(nativeDiagnostics({ code, options: [], settings: null })).toEqual([]);
  });

  it('fails closed for malformed input and isolates rule selection', () => {
    expect(
      nativeDiagnostics({
        code: 'test.beforeEach(',
        options: [],
        settings: null,
      }),
    ).toEqual([]);
    expect(runRule('prefer-lowercase-title', 'test.beforeEach(() => {});', [], null)).toEqual([]);
  });

  it('reports TypeScript through real Oxlint and stays unchanged under --fix', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-no-hooks-'));
    try {
      const source = [
        'type Setup = () => void;',
        'const setup: Setup = () => {};',
        'test.beforeEach(() => setup());',
        '',
      ].join('\n');
      writeFileSync(join(tempDir, 'fixture.spec.ts'), source);
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [{ name: 'playwright', specifier: join(packageRoot, 'index.js') }],
          rules: { 'playwright/no-hooks': 'error' },
        }),
      );

      const lint = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.spec.ts'],
        { cwd: tempDir, encoding: 'utf8' },
      );
      const payload = JSON.parse(lint.stdout);
      expect(lint.status).toBe(1);
      expect(lint.stderr).toBe('');
      expect(payload.diagnostics).toHaveLength(1);
      expect(payload.diagnostics[0]).toMatchObject({
        code: 'playwright(no-hooks)',
        message: "Unexpected 'beforeEach' hook",
      });

      const fix = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--fix', '--quiet', 'fixture.spec.ts'],
        { cwd: tempDir, encoding: 'utf8' },
      );
      expect(fix.status).toBe(1);
      expect(readFileSync(join(tempDir, 'fixture.spec.ts'), 'utf8')).toBe(source);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

function nativeDiagnostics(testCase) {
  return scanPlaywright(testCase.code, 'fixture.spec.ts', scanOptions(testCase)).filter(
    ({ ruleName: diagnosticRule }) => diagnosticRule === ruleName,
  );
}

function scanOptions(testCase) {
  const configured = testCase.options?.[0] ?? {};
  const globalAliases = testCase.settings?.playwright?.globalAliases;
  return {
    allowedHooks: configured.allow,
    hookAliases: globalAliases,
    ...(Array.isArray(globalAliases?.test) ? { testAliases: globalAliases.test } : {}),
  };
}

function adapterReports(testCase) {
  return runRule(ruleName, testCase.code, testCase.options, testCase.settings);
}

function runRule(selectedRule, sourceText, options = [], settings = null) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[selectedRule].createOnce({
    filename: 'fixture.spec.ts',
    options,
    settings,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function compactData(data) {
  return Object.fromEntries(
    Object.entries(data).filter(
      ([, value]) => value !== undefined && value !== null && value !== '',
    ),
  );
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((left, right) => left.localeCompare(right));
  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }
  return candidates[candidates.length - 1];
}
