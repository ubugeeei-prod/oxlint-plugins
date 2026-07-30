import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { scanPlaywright } from '../api.js';
import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(here);
const workspaceRoot = resolve(packageRoot, '../..');
const fixture = JSON.parse(readFileSync(join(here, 'fixtures', 'patterns-v2.10.4.json'), 'utf8'));
const validCases = fixture.suites.flatMap((suite) =>
  suite.valid.map((testCase, index) => ({
    suite: suite.name,
    rule: suite.rule,
    index,
    testCase,
  })),
);
const invalidCases = fixture.suites.flatMap((suite) =>
  suite.invalid.map((testCase, index) => ({
    suite: suite.name,
    rule: suite.rule,
    index,
    testCase,
  })),
);
const fixCases = invalidCases.filter(({ testCase }) => testCase.output !== undefined);

describe('eslint-plugin-playwright v2.10.4 title and tag replay', () => {
  it('pins the complete authored inventory and exact source hashes', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-playwright',
      version: '2.10.4',
      sourceCommit: '894c0ec261763bb1e073b276c70bbf88b4ebad39',
      license: 'MIT',
      tool: 'tools/tasks/sync-playwright-pattern-tests.ts',
      inventory: {
        suites: 10,
        valid: 141,
        invalid: 106,
        diagnostics: 116,
      },
    });
    expect(fixture.__generated.sourceFiles).toHaveLength(4);
    expect(Object.values(fixture.__generated.sourceHashes)).toHaveLength(4);
    expect(
      Object.values(fixture.__generated.sourceHashes).every((hash) => /^[\da-f]{64}$/u.test(hash)),
    ).toBe(true);
  });

  it.each(validCases)(
    '$rule / $suite accepts authored valid case $index in both entry points',
    ({ rule, testCase }) => {
      expect(nativeDiagnostics(rule, testCase)).toEqual([]);
      expect(adapterDiagnostics(rule, testCase)).toEqual([]);
    },
  );

  it.each(invalidCases)(
    '$rule / $suite matches authored invalid case $index',
    ({ rule, testCase }) => {
      const native = nativeDiagnostics(rule, testCase);
      const adapter = adapterDiagnostics(rule, testCase);
      expect(native.map(({ messageId }) => messageId)).toEqual(
        testCase.expectedDiagnostics.map(({ messageId }) => messageId),
      );
      expect(adapter.map(({ messageId }) => messageId)).toEqual(
        testCase.expectedDiagnostics.map(({ messageId }) => messageId),
      );
      for (const [index, expected] of testCase.expectedDiagnostics.entries()) {
        expect(native[index].data).toMatchObject(expected.data);
        expect(adapter[index].data).toMatchObject(expected.data);
        if (expected.loc) {
          expect(native[index].loc).toMatchObject(expected.loc);
          expect(adapter[index].loc).toMatchObject(expected.loc);
        }
      }
      expect(
        adapterReports(rule, testCase).map((report) =>
          renderMessage(plugin.rules[rule].meta.messages[report.messageId], report.data),
        ),
      ).toEqual(
        native.map((diagnostic) =>
          renderMessage(plugin.rules[rule].meta.messages[diagnostic.messageId], diagnostic.data),
        ),
      );
    },
  );

  it.each(fixCases)(
    '$rule / $suite reproduces authored output and reaches a fixpoint for case $index',
    ({ rule, testCase }) => {
      const expected = testCase.output === null ? testCase.code : testCase.output;
      expect(applyNativeFixes(rule, testCase)).toBe(expected);
      expect(applyAdapterFixes(rule, testCase)).toBe(expected);
      const fixedCase = { ...testCase, code: expected };
      const secondNativePass = applyNativeFixes(rule, fixedCase);
      const secondAdapterPass = applyAdapterFixes(rule, fixedCase);
      expect(secondNativePass).toBe(expected);
      expect(secondAdapterPass).toBe(expected);
    },
  );

  it('exposes the exact upstream messages, fixability, and schemas', () => {
    expect(plugin.rules['valid-test-tags'].meta).toMatchObject({
      type: 'problem',
      messages: {
        disallowedTag: 'Tag "{{tag}}" is not allowed',
        invalidTagFormat: 'Tag must start with @',
        invalidTagValue: 'Tag must be a string or array of strings',
        unknownTag: 'Unknown tag "{{tag}}"',
      },
      schema: [
        {
          additionalProperties: false,
          properties: {
            allowedTags: tagListSchema(),
            disallowedTags: tagListSchema(),
          },
          type: 'object',
        },
      ],
    });
    expect(plugin.rules['valid-title'].meta).toMatchObject({
      fixable: 'code',
      messages: {
        accidentalSpace: 'should not have leading or trailing spaces',
        disallowedWord: '"{{ word }}" is not allowed in test titles',
        duplicatePrefix: 'should not have duplicate prefix',
        emptyTitle: '{{ functionName }} should not have an empty title',
        mustMatch: '{{ functionName }} should match {{ pattern }}',
        mustMatchCustom: '{{ message }}',
        mustNotMatch: '{{ functionName }} should not match {{ pattern }}',
        mustNotMatchCustom: '{{ message }}',
        titleMustBeString: 'Title must be a string',
      },
    });
  });

  it('rejects mutually exclusive and malformed tag configuration exactly', () => {
    expect(() =>
      scanPlaywright("test('x', () => {})", 'fixture.spec.ts', {
        validTestTags: { allowedTags: ['@ok'], disallowedTags: ['@bad'] },
      }),
    ).toThrow('The allowedTags and disallowedTags options cannot be used together');
    expect(() =>
      scanPlaywright("test('x', () => {})", 'fixture.spec.ts', {
        validTestTags: { allowedTags: ['missing-at'] },
      }),
    ).toThrow('Invalid tag "missing-at" in configuration: tags must start with @');
    expect(() =>
      scanPlaywright("test('x', () => {})", 'fixture.spec.ts', {
        validTitle: { mustMatch: '[' },
      }),
    ).toThrow(SyntaxError);
  });

  it('preserves exact UTF-16 locations and fix ranges after astral Unicode', () => {
    const source = 'const marker = "🧪"; test(" test title ", () => {});';
    const diagnostics = scanPlaywright(source, 'fixture.spec.ts', {
      validTitle: {},
    }).filter(({ ruleName }) => ruleName === 'valid-title');
    expect(diagnostics).toMatchObject([
      {
        messageId: 'accidentalSpace',
        loc: { startLine: 1, startColumn: 26, endLine: 1, endColumn: 40 },
        fix: { start: 26, end: 40, replacement: '"test title"' },
      },
    ]);
    expect(applyFixSet(source, diagnostics)).toBe(
      'const marker = "🧪"; test("test title", () => {});',
    );
  });

  it('supports configured, imported, and test.extend aliases in TypeScript sources', () => {
    const sources = [
      ['it("test bad", () => {});', { testAliases: ['it'], validTitle: {} }],
      [
        'import { test as scenario } from "@playwright/test"; scenario("test bad", () => {});',
        { validTitle: {} },
      ],
      ['const scenario = test.extend({}); scenario("test bad", () => {});', { validTitle: {} }],
    ];
    for (const [source, options] of sources) {
      expect(
        scanPlaywright(source, 'fixture.spec.ts', options)
          .filter(({ ruleName }) => ruleName === 'valid-title')
          .map(({ messageId }) => messageId),
      ).toEqual(['duplicatePrefix']);
    }
  });

  it('is inert on malformed input and keeps rule selection isolated', () => {
    expect(scanPlaywright('test("unterminated', 'fixture.spec.ts', { validTitle: {} })).toEqual([]);
    const reports = adapterReports('valid-test-tags', {
      code: 'test("", { tag: "bad" }, () => {});',
      options: [{}],
      settings: null,
    });
    expect(reports.map(({ messageId }) => messageId)).toEqual(['invalidTagFormat']);
  });

  it('runs both option families and iterative fixes through real Oxlint on TypeScript', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-patterns-'));
    try {
      const source =
        'import { test as scenario } from "@playwright/test";\n' +
        'scenario("test broken", { tag: "bad" }, () => {});\n';
      const sourcePath = join(tempDir, 'fixture.spec.ts');
      writeFileSync(sourcePath, source);
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [{ name: 'playwright', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'playwright/valid-title': ['error', {}],
            'playwright/valid-test-tags': ['error', { allowedTags: ['@e2e'] }],
          },
        }),
      );
      const result = spawnSync(
        findOxlintCli(),
        [
          '--config',
          'oxlint.config.jsonc',
          '--fix',
          '--quiet',
          '--format',
          'json',
          'fixture.spec.ts',
        ],
        { cwd: tempDir, encoding: 'utf8' },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(readFileSync(sourcePath, 'utf8')).toContain(
        'scenario("broken", { tag: "bad" }, () => {});',
      );
      expect(payload.diagnostics).toMatchObject([
        {
          code: 'playwright(valid-test-tags)',
          message: 'Tag must start with @',
        },
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

function nativeDiagnostics(rule, testCase) {
  return scanPlaywright(testCase.code, 'fixture.spec.ts', scanOptions(rule, testCase))
    .filter((diagnostic) => diagnostic.ruleName === rule)
    .map(({ messageId, data, loc, fix }) => ({ messageId, data, loc, fix }));
}

function adapterReports(rule, testCase, sourceText = testCase.code) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[rule].createOnce({
    filename: 'fixture.spec.ts',
    options: testCase.options ?? [],
    settings: testCase.settings ?? {},
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function adapterDiagnostics(rule, testCase) {
  return adapterReports(rule, testCase).map(({ messageId, data, loc, fix }) => ({
    messageId,
    data,
    loc: {
      startLine: loc.start.line,
      startColumn: loc.start.column,
      endLine: loc.end.line,
      endColumn: loc.end.column,
    },
    fix,
  }));
}

function scanOptions(rule, testCase) {
  const options = testCase.options ?? [];
  const testAliases = testCase.settings?.playwright?.globalAliases?.test;
  return {
    ...(rule === 'valid-title' ? { validTitle: options[0] } : {}),
    ...(rule === 'valid-test-tags' ? { validTestTags: options[0] } : {}),
    ...(Array.isArray(testAliases) ? { testAliases } : {}),
  };
}

function applyNativeFixes(rule, testCase) {
  return applyUntilStable(testCase.code, (source) =>
    scanPlaywright(source, 'fixture.spec.ts', scanOptions(rule, testCase)).filter(
      (diagnostic) => diagnostic.ruleName === rule,
    ),
  );
}

function applyAdapterFixes(rule, testCase) {
  return applyUntilStable(testCase.code, (source) =>
    adapterReports(rule, testCase, source).flatMap((report) => {
      if (typeof report.fix !== 'function') return [];
      return [
        {
          fix: report.fix({
            replaceTextRange(range, replacement) {
              return { range, replacement };
            },
          }),
        },
      ];
    }),
  );
}

function applyUntilStable(initialSource, diagnosticsForSource) {
  let source = initialSource;
  for (let pass = 0; pass < 10; pass += 1) {
    const next = applyFixSet(source, diagnosticsForSource(source));
    if (next === source) return source;
    source = next;
  }
  throw new Error('Fixes did not converge after 10 passes.');
}

function applyFixSet(source, diagnostics) {
  const fixes = diagnostics
    .flatMap((diagnostic) => {
      if (!diagnostic.fix) return [];
      if (Array.isArray(diagnostic.fix)) return diagnostic.fix;
      if (diagnostic.fix.range) {
        return [
          {
            start: diagnostic.fix.range[0],
            end: diagnostic.fix.range[1],
            replacement: diagnostic.fix.replacement,
          },
        ];
      }
      return [diagnostic.fix];
    })
    .sort((left, right) => left.start - right.start || left.end - right.end);
  let cursor = 0;
  let output = '';
  for (const fix of fixes) {
    if (fix.start < cursor) continue;
    output += source.slice(cursor, fix.start);
    output += fix.replacement;
    cursor = fix.end;
  }
  output += source.slice(cursor);
  return output;
}

function tagListSchema() {
  return {
    items: {
      oneOf: [
        { type: 'string' },
        {
          additionalProperties: false,
          properties: { source: { type: 'string' } },
          type: 'object',
        },
      ],
    },
    type: 'array',
  };
}

function renderMessage(template, data) {
  return template.replace(/\{\{\s*(\w+)\s*\}\}/gu, (_match, key) => data[key] ?? '');
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((left, right) => left.localeCompare(right));
  if (candidates.length === 0) throw new Error('Could not find oxlint CLI.');
  return candidates.at(-1);
}
