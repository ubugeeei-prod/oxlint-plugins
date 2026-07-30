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
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'expect-expect-v2.10.4.json'), 'utf8'),
);

describe('eslint-plugin-playwright v2.10.4 expect-expect replay', () => {
  it('pins the complete authored inventory and exact source hashes', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-playwright',
      version: '2.10.4',
      sourceCommit: '894c0ec261763bb1e073b276c70bbf88b4ebad39',
      license: 'MIT',
      tool: 'tools/tasks/sync-playwright-expect-expect-tests.ts',
      sourceFiles: [
        'src/rules/expect-expect.ts',
        'src/rules/expect-expect.test.ts',
        'docs/rules/expect-expect.md',
      ],
      sourceHashes: {
        'src/rules/expect-expect.ts':
          '110a10377a4185fd95330efd0c098e9475ddfed98f7da9ef276b5654e3958112',
        'src/rules/expect-expect.test.ts':
          '2515bb44c8c914a229d34bed3dff38669361dc8f1c6332a82598f4a32f0df092',
        'docs/rules/expect-expect.md':
          '7b4613f0c991066aab1a861a06b475d15686f8a0ec1ee068264ec7c4bc2de97e',
      },
      inventory: {
        valid: 32,
        invalid: 8,
        diagnostics: 8,
      },
    });
  });

  it.each(fixture.suite.valid)(
    'accepts authored valid case %# through native and adapter entry points',
    (testCase) => {
      expect(nativeDiagnostics(testCase)).toEqual([]);
      expect(adapterReports(testCase)).toEqual([]);
    },
  );

  it.each(fixture.suite.invalid)(
    'reproduces authored invalid case %# with exact contract',
    (testCase) => {
      const native = nativeDiagnostics(testCase);
      const adapter = adapterReports(testCase);
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
        expect(native[index]).toMatchObject({
          messageId: expected.messageId,
          data: { message: '' },
          ...(expected.loc ? { loc: expected.loc } : {}),
        });
      }
    },
  );

  it('exposes the exact upstream metadata and no fixer', () => {
    expect(plugin.rules['expect-expect'].meta).toMatchObject({
      type: 'problem',
      docs: {
        description: 'Enforce assertion to be made in a test body',
        recommended: true,
        url: 'https://github.com/mskelton/eslint-plugin-playwright/tree/main/docs/rules/expect-expect.md',
      },
      messages: {
        noAssertions: 'Test has no assertions',
      },
      schema: [
        {
          additionalProperties: false,
          properties: {
            assertFunctionNames: {
              items: [{ type: 'string' }],
              type: 'array',
            },
            assertFunctionPatterns: {
              items: [{ type: 'string' }],
              type: 'array',
            },
          },
          type: 'object',
        },
      ],
    });
    expect(plugin.rules['expect-expect'].meta.fixable).toBeUndefined();
  });

  it('matches only terminal identifier names like upstream dig()', () => {
    const source = [
      'test("direct", () => assertCustomCondition());',
      'test("member", () => page.assertCustomCondition());',
      'test("computed identifier", () => page[assertCustomCondition]());',
      'test("computed string", () => page["assertCustomCondition"]());',
      'test("nonterminal", () => page.assertCustomCondition.factory());',
    ].join('\n');
    expect(
      scanPlaywright(source, 'fixture.spec.ts', {
        assertFunctionNames: ['assertCustomCondition'],
      })
        .filter(({ ruleName }) => ruleName === 'expect-expect')
        .map(({ loc }) => loc.startLine),
    ).toEqual([4, 5]);
  });

  it('supports multiple exact names and regular-expression patterns together', () => {
    const source = [
      'test("exact", () => myCustomAssert());',
      'test("prefix", () => verifyElementVisible());',
      'test("suffix", () => anotherAssertion());',
      'test("lookahead", () => ensureElement());',
      'test("backreference", () => checkcheck());',
      'test("missing", () => checkNothing());',
    ].join('\n');
    expect(
      scanPlaywright(source, 'fixture.spec.ts', {
        assertFunctionNames: ['myCustomAssert'],
        assertFunctionPatterns: ['^verify.*', '.*Assertion$', '^ensure(?=Element)', '^(check)\\1$'],
      })
        .filter(({ ruleName }) => ruleName === 'expect-expect')
        .map(({ loc }) => loc.startLine),
    ).toEqual([6]);
  });

  it('validates pattern syntax at the direct and plugin adapter boundaries', () => {
    const source = 'test("case", () => {});';
    expect(() =>
      scanPlaywright(source, 'fixture.spec.ts', {
        assertFunctionPatterns: ['(?<unterminated'],
      }),
    ).toThrow(SyntaxError);
    expect(() => runRule(source, [{ assertFunctionPatterns: ['(?<unterminated'] }])).toThrow(
      SyntaxError,
    );
  });

  it('supports global aliases, arbitrary named import aliases, and chained extends', () => {
    const source = [
      'import { test as scenario, expect as assuming } from "another-runner";',
      'const custom = scenario.extend({}).extend({});',
      'const chained = custom.extend({});',
      'scenario("import", () => assuming(true).toBeDefined());',
      'chained("extended", () => expect(true).toBeDefined());',
      'it("global", () => verify(true).toBeDefined());',
    ].join('\n');
    expect(
      runRule(source, [], {
        playwright: {
          globalAliases: {
            expect: ['verify'],
            test: ['it'],
          },
        },
      }),
    ).toEqual([]);
  });

  it('preserves upstream outermost-test ancestry semantics for nested tests', () => {
    const source = [
      'test("outer", () => {',
      '  test("inner", () => {',
      '    expect(true).toBeDefined();',
      '  });',
      '});',
    ].join('\n');
    expect(runRule(source)).toMatchObject([
      {
        messageId: 'noAssertions',
        loc: {
          start: { line: 2, column: 2 },
          end: { line: 2, column: 6 },
        },
      },
    ]);
  });

  it('counts assertions in steps and nested callbacks while isolating sibling tests', () => {
    const source = [
      'test.describe.configure({ mode: "parallel" });',
      'test.skip(true);',
      'test("step", async () => {',
      '  await test.step("inside", async () => expect(true).toBeDefined());',
      '});',
      'test("callback", () => Promise.resolve().then(() => expect(true).toBeDefined()));',
      'test("empty sibling", () => {});',
    ].join('\n');
    expect(
      nativeDiagnostics({ code: source, options: [], settings: null }).map(({ messageId, loc }) => [
        messageId,
        loc.startLine,
      ]),
    ).toEqual([['noAssertions', 7]]);
  });

  it('uses UTF-16 columns, fails closed on malformed input, and keeps rule selection isolated', () => {
    expect(
      nativeDiagnostics({
        code: '"🧪"; test("empty", () => {});',
        options: [],
        settings: null,
      }),
    ).toMatchObject([
      {
        messageId: 'noAssertions',
        loc: {
          startLine: 1,
          startColumn: 6,
          endLine: 1,
          endColumn: 10,
        },
      },
    ]);
    expect(
      scanPlaywright('test("broken', 'fixture.spec.ts').filter(
        ({ ruleName }) => ruleName === 'expect-expect',
      ),
    ).toEqual([]);
    expect(runNamedRule('max-expects', 'test("empty", () => {});')).toEqual([]);
  });

  it('runs options and exact diagnostics through real Oxlint on TypeScript', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-expect-expect-'));
    try {
      writeFileSync(
        join(tempDir, 'fixture.spec.ts'),
        [
          'const label: string = "case";',
          'test("empty", () => console.log(label));',
          'test("named", () => assertCustomCondition());',
          'test("pattern", () => verifyElementVisible());',
        ].join('\n'),
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [{ name: 'playwright', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'playwright/expect-expect': [
              'error',
              {
                assertFunctionNames: ['assertCustomCondition'],
                assertFunctionPatterns: ['^verify.*'],
              },
            ],
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
      expect(
        payload.diagnostics.map(({ code, message, labels }) => [
          code,
          message,
          labels[0].span.line,
          labels[0].span.column,
        ]),
      ).toEqual([['playwright(expect-expect)', 'Test has no assertions', 2, 1]]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

function nativeDiagnostics(testCase) {
  return scanPlaywright(testCase.code, 'fixture.spec.ts', scanOptions(testCase)).filter(
    ({ ruleName }) => ruleName === 'expect-expect',
  );
}

function adapterReports(testCase) {
  return runRule(testCase.code, testCase.options, testCase.settings);
}

function runRule(sourceText, options = [], settings = null) {
  return runNamedRule('expect-expect', sourceText, options, settings);
}

function runNamedRule(ruleName, sourceText, options = [], settings = null) {
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

function scanOptions(testCase) {
  const configured = testCase.options[0] ?? {};
  const globalAliases = testCase.settings?.playwright?.globalAliases ?? {};
  return {
    assertFunctionNames: configured.assertFunctionNames,
    assertFunctionPatterns: configured.assertFunctionPatterns,
    ...(Array.isArray(globalAliases.expect) ? { expectAliases: globalAliases.expect } : {}),
    ...(Array.isArray(globalAliases.test) ? { testAliases: globalAliases.test } : {}),
  };
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
