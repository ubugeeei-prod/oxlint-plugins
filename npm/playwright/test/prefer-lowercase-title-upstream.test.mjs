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
  readFileSync(join(packageRoot, 'test/fixtures/prefer-lowercase-title-v2.11.0.json'), 'utf8'),
);
const ruleName = 'prefer-lowercase-title';
const rule = plugin.rules[ruleName];
const validCases = fixture.suites.flatMap((suite) =>
  suite.valid.map((testCase, index) => ({
    ...testCase,
    label: `${suite.name} valid ${index + 1}`,
  })),
);
const invalidCases = fixture.suites.flatMap((suite) =>
  suite.invalid.map((testCase, index) => ({
    ...testCase,
    label: `${suite.name} invalid ${index + 1}`,
  })),
);

describe('playwright prefer-lowercase-title upstream v2.11.0 fixtures', () => {
  it('pins exact upstream sources, previous-version drift, and authored inventory', () => {
    expect(fixture.__generated).toEqual({
      source: 'eslint-plugin-playwright',
      version: '2.11.0',
      sourceCommit: 'b6d3e5dac73c8aad4d5e62a933105579c319655f',
      license: 'MIT',
      tool: 'tools/tasks/sync-playwright-prefer-lowercase-title-tests.ts',
      sourceFiles: [
        'src/rules/prefer-lowercase-title.ts',
        'src/rules/prefer-lowercase-title.test.ts',
        'docs/rules/prefer-lowercase-title.md',
      ],
      sourceHashes: {
        'src/rules/prefer-lowercase-title.ts':
          '351bf97eb91d65da36f4e1bd079b399c786f621c06421d49f34c3c78b8246769',
        'src/rules/prefer-lowercase-title.test.ts':
          'eae061c77c8c796bacb80e77e7add65344671e77bad521232516c9958fbde3a9',
        'docs/rules/prefer-lowercase-title.md':
          '1fb0cf8238204964071adb4dbc5aa9be6200aafa3321e287573f99cf9b33738d',
      },
      previousVersionAudit: {
        version: '2.10.4',
        sourceCommit: '894c0ec261763bb1e073b276c70bbf88b4ebad39',
        changedFiles: [],
      },
      inventory: {
        suites: 5,
        valid: 55,
        invalid: 27,
        diagnostics: 28,
        fixable: 27,
      },
    });
    expect(validCases).toHaveLength(55);
    expect(invalidCases).toHaveLength(27);
  });

  it.each(validCases)('$label is valid through native and plugin paths', (testCase) => {
    expect(nativeDiagnostics(testCase)).toEqual([]);
    expect(adapterReports(testCase)).toEqual([]);
  });

  it.each(invalidCases)('$label matches exact diagnostics and fixes', (testCase) => {
    const diagnostics = nativeDiagnostics(testCase);
    const reports = adapterReports(testCase);

    expect(
      diagnostics.map((diagnostic) => ({
        messageId: diagnostic.messageId,
        data: compactData(diagnostic.data),
        loc: diagnostic.loc,
      })),
    ).toEqual(testCase.expectedDiagnostics);
    expect(
      reports.map((report) => ({
        messageId: report.messageId,
        data: compactData(report.data),
        loc: {
          startLine: report.loc.start.line,
          startColumn: report.loc.start.column,
          endLine: report.loc.end.line,
          endColumn: report.loc.end.column,
        },
      })),
    ).toEqual(testCase.expectedDiagnostics);

    expect(diagnostics.every((diagnostic) => diagnostic.fix)).toBe(true);
    expect(reports.every((report) => report.appliedFix)).toBe(true);
    expect(applyNativeFixes(testCase.code, diagnostics)).toBe(testCase.expectedOutput);
    expect(applyAdapterFixes(testCase.code, reports)).toBe(testCase.expectedOutput);
    expect(
      reports.map((report) =>
        rule.meta.messages[report.messageId].replace('{{method}}', report.data.method),
      ),
    ).toEqual(
      testCase.expectedDiagnostics.map(
        (diagnostic) => `\`${diagnostic.data.method}\`s should begin with lowercase`,
      ),
    );
  });

  it('exposes the exact upstream metadata, schema, and autofix contract', () => {
    expect(rule.meta).toMatchObject({
      type: 'suggestion',
      docs: {
        description: 'Enforce lowercase test names',
        recommended: false,
        url: 'https://github.com/mskelton/eslint-plugin-playwright/tree/main/docs/rules/prefer-lowercase-title.md',
      },
      fixable: 'code',
      messages: {
        unexpectedLowercase: '`{{method}}`s should begin with lowercase',
      },
      schema: [
        {
          additionalProperties: false,
          properties: {
            allowedPrefixes: {
              additionalItems: false,
              items: { type: 'string' },
              type: 'array',
            },
            ignore: {
              additionalItems: false,
              items: { enum: ['test.describe', 'test'] },
              type: 'array',
            },
            ignoreTopLevelDescribe: {
              default: false,
              type: 'boolean',
            },
          },
          type: 'object',
        },
      ],
    });
    expect(rule.meta.hasSuggestions).toBeUndefined();
  });

  it('applies options together without leaking state across top-level siblings', () => {
    const code = [
      "test.describe('Top', () => {",
      "  test.describe('Nested', () => { test('Case', () => {}); });",
      '});',
      "test.describe('Sibling', () => {});",
      "test('GET /health', () => {});",
      "test('POST /health', () => {});",
    ].join('\n');
    const testCase = {
      code,
      options: [
        {
          allowedPrefixes: ['GET'],
          ignoreTopLevelDescribe: true,
        },
      ],
      settings: null,
    };
    const diagnostics = nativeDiagnostics(testCase);

    expect(diagnostics.map((diagnostic) => diagnostic.data.method)).toEqual([
      'test.describe',
      'test',
      'test',
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.loc.startLine)).toEqual([2, 2, 6]);
    expect(applyNativeFixes(code, diagnostics)).toContain("test.describe('nested'");
    expect(applyNativeFixes(code, diagnostics)).toContain("test('case'");
    expect(applyNativeFixes(code, diagnostics)).toContain("test('pOST /health'");
    expect(applyNativeFixes(code, diagnostics)).toContain("test.describe('Top'");
    expect(applyNativeFixes(code, diagnostics)).toContain("test.describe('Sibling'");
  });

  it('supports arbitrary import aliases, configured globals, and transitive extend aliases', () => {
    const code = [
      'import { test as scenario } from "another-runner";',
      'const later = custom.extend({});',
      'const custom = scenario.extend({}).extend({});',
      "scenario('Imported', () => {});",
      "custom.describe('Extended', () => {});",
      "later.only('Forward', () => {});",
      "it('Global', () => {});",
    ].join('\n');
    const testCase = {
      code,
      options: [],
      settings: { playwright: { globalAliases: { test: ['it'] } } },
    };
    const diagnostics = nativeDiagnostics(testCase);

    expect(diagnostics.map((diagnostic) => diagnostic.loc.startLine)).toEqual([4, 5, 6, 7]);
    expect(diagnostics.map((diagnostic) => diagnostic.data.method)).toEqual([
      'test',
      'test.describe',
      'test',
      'test',
    ]);
    expect(adapterReports(testCase)).toHaveLength(4);
  });

  it('matches decoded strings, raw templates, dynamic titles, and JavaScript UTF-16 casing', () => {
    const code = [
      '"🧪"; test("\\u0046oo", () => {});',
      'test(`\\u0046oo`, () => {});',
      'test(`Dynamic ${name}`, () => {});',
      'test("İstanbul", () => {});',
      'test("𐐀eseret", () => {});',
      'test("Éclair", () => {});',
    ].join('\n');
    const testCase = { code, options: [], settings: null };
    const diagnostics = nativeDiagnostics(testCase);

    expect(diagnostics.map((diagnostic) => diagnostic.loc.startLine)).toEqual([1, 4, 6]);
    expect(diagnostics.map((diagnostic) => diagnostic.fix.replacement)).toEqual([
      'foo',
      'i\u0307stanbul',
      'éclair',
    ]);
    expect(diagnostics[0].fix.start).toBe(code.indexOf('"\\u0046oo"') + 1);
    expect(diagnostics[1].fix.start).toBe(code.indexOf('İstanbul'));
    expect(applyNativeFixes(code, diagnostics)).toBe(
      code
        .replace('"\\u0046oo"', '"foo"')
        .replace('"İstanbul"', '"i\u0307stanbul"')
        .replace('"Éclair"', '"éclair"'),
    );
  });

  it('fails closed for malformed input and ignores non-test call shapes', () => {
    expect(
      nativeDiagnostics({
        code: [
          "random('Title', () => {});",
          "foo.test('Title', () => {});",
          "test.step('Title', () => {});",
        ].join('\n'),
        options: [],
        settings: null,
      }),
    ).toEqual([]);
    expect(
      nativeDiagnostics({
        code: 'test("Broken',
        options: [],
        settings: null,
      }),
    ).toEqual([]);
  });

  it('reports and fixes a TypeScript file through real Oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-lowercase-'));
    try {
      const source = [
        'type Fixture = { title: string };',
        'const fixture: Fixture = { title: "demo" };',
        'test("Works with types", () => { void fixture; });',
        '',
      ].join('\n');
      writeFileSync(join(tempDir, 'fixture.spec.ts'), source);
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [{ name: 'playwright', specifier: join(packageRoot, 'index.js') }],
          rules: { 'playwright/prefer-lowercase-title': 'error' },
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
        code: 'playwright(prefer-lowercase-title)',
        message: '`test`s should begin with lowercase',
      });

      const fix = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--fix', '--quiet', 'fixture.spec.ts'],
        { cwd: tempDir, encoding: 'utf8' },
      );
      expect(fix.status).toBe(0);
      expect(fix.stderr).toBe('');
      expect(readFileSync(join(tempDir, 'fixture.spec.ts'), 'utf8')).toBe(
        source.replace('"Works with types"', '"works with types"'),
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

function nativeDiagnostics(testCase) {
  return scanPlaywright(testCase.code, 'fixture.spec.ts', scanOptions(testCase)).filter(
    (diagnostic) => diagnostic.ruleName === ruleName,
  );
}

function scanOptions(testCase) {
  const configured = testCase.options?.[0] ?? {};
  const testAliases = testCase.settings?.playwright?.globalAliases?.test;
  return {
    allowedPrefixes: configured.allowedPrefixes,
    ignore: configured.ignore,
    ignoreTopLevelDescribe: configured.ignoreTopLevelDescribe,
    ...(Array.isArray(testAliases) ? { testAliases } : {}),
  };
}

function adapterReports(testCase) {
  const reports = [];
  const sourceCode = {
    text: testCase.code,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    filename: 'fixture.spec.ts',
    options: testCase.options,
    settings: testCase.settings,
    sourceCode,
    report(descriptor) {
      reports.push({
        ...descriptor,
        appliedFix:
          typeof descriptor.fix === 'function'
            ? descriptor.fix({
                replaceTextRange(range, replacementText) {
                  return { range, replacementText };
                },
              })
            : null,
      });
    },
  });
  visitor.Program({ type: 'Program', range: [0, testCase.code.length] });
  return reports;
}

function compactData(data) {
  return Object.fromEntries(
    Object.entries(data).filter(
      ([, value]) => value !== undefined && value !== null && value !== '',
    ),
  );
}

function applyNativeFixes(source, diagnostics) {
  return applyFixes(
    source,
    diagnostics.map((diagnostic) => ({
      range: [diagnostic.fix.start, diagnostic.fix.end],
      replacementText: diagnostic.fix.replacement,
    })),
  );
}

function applyAdapterFixes(source, reports) {
  return applyFixes(
    source,
    reports.map((report) => report.appliedFix),
  );
}

function applyFixes(source, fixes) {
  return [...fixes]
    .sort((left, right) => right.range[0] - left.range[0])
    .reduce(
      (output, fix) =>
        `${output.slice(0, fix.range[0])}${fix.replacementText}${output.slice(fix.range[1])}`,
      source,
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
