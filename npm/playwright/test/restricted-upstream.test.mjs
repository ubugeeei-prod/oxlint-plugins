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
const fixture = JSON.parse(readFileSync(join(here, 'fixtures', 'restricted-v2.10.4.json'), 'utf8'));
const validCases = fixture.suites.flatMap((suite) =>
  suite.valid.map((testCase, index) => ({ rule: suite.rule, index, testCase })),
);
const invalidCases = fixture.suites.flatMap((suite) =>
  suite.invalid.map((testCase, index) => ({ rule: suite.rule, index, testCase })),
);

describe('eslint-plugin-playwright v2.10.4 restricted-rule replay', () => {
  it('pins every authored suite and source hash', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-playwright',
      version: '2.10.4',
      sourceCommit: '894c0ec261763bb1e073b276c70bbf88b4ebad39',
      license: 'MIT',
      tool: 'tools/tasks/sync-playwright-restricted-tests.ts',
      inventory: {
        suites: [
          {
            rule: 'no-restricted-locators',
            valid: 12,
            invalid: 15,
            diagnostics: 17,
          },
          {
            rule: 'no-restricted-matchers',
            valid: 20,
            invalid: 13,
            diagnostics: 13,
          },
          {
            rule: 'no-restricted-roles',
            valid: 16,
            invalid: 18,
            diagnostics: 20,
          },
        ],
        valid: 48,
        invalid: 46,
        diagnostics: 50,
      },
    });
    expect(fixture.__generated.sourceFiles).toHaveLength(6);
    expect(Object.values(fixture.__generated.sourceHashes)).toHaveLength(6);
    expect(
      Object.values(fixture.__generated.sourceHashes).every((hash) => /^[\da-f]{64}$/u.test(hash)),
    ).toBe(true);
  });

  it.each(validCases)('$rule accepts upstream valid case $index', ({ rule, testCase }) => {
    expect(nativeDiagnostics(rule, testCase)).toEqual([]);
    expect(adapterDiagnostics(rule, testCase)).toEqual([]);
  });

  it.each(invalidCases)(
    '$rule matches upstream invalid case $index exactly',
    ({ rule, testCase }) => {
      expect(nativeDiagnostics(rule, testCase)).toEqual(testCase.expectedDiagnostics);
      expect(adapterDiagnostics(rule, testCase)).toEqual(testCase.expectedDiagnostics);
      expect(
        adapterReports(rule, testCase).map((report) =>
          renderMessage(plugin.rules[rule].meta.messages[report.messageId], report.data),
        ),
      ).toEqual(
        testCase.expectedDiagnostics.map((diagnostic) =>
          renderMessage(plugin.rules[rule].meta.messages[diagnostic.messageId], diagnostic.data),
        ),
      );
    },
  );

  it('exposes the exact upstream messages and schemas', () => {
    expect(plugin.rules['no-restricted-locators'].meta).toMatchObject({
      messages: {
        restricted: 'Usage of `{{method}}` is disallowed',
        restrictedWithMessage: '{{message}}',
      },
      schema: [restrictedListSchema('type')],
    });
    expect(plugin.rules['no-restricted-matchers'].meta).toMatchObject({
      messages: {
        restricted: 'Use of `{{restriction}}` is disallowed',
        restrictedWithMessage: '{{message}}',
      },
      schema: [
        {
          additionalProperties: { type: ['string', 'null'] },
          type: 'object',
        },
      ],
    });
    expect(plugin.rules['no-restricted-roles'].meta).toMatchObject({
      messages: {
        restricted: 'Usage of role `{{role}}` in getByRole() is disallowed',
        restrictedWithMessage: '{{message}}',
      },
      schema: [restrictedListSchema('role')],
    });
  });

  it('honors lists and custom messages through a real Oxlint TSX run', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-playwright-restricted-'));
    try {
      const source = [
        'import { expect as assuming } from "@playwright/test";',
        'export const Example = () => <button>Submit</button>;',
        'page.getByTestId("submit");',
        'page.getByTitle("tooltip");',
        'assuming(value).not.toBeTruthy();',
        'page.getByRole("progressbar");',
        'page.getByRole("alert");',
        '',
      ].join('\n');
      writeFileSync(join(tempDir, 'fixture.spec.tsx'), source);
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
            'playwright/no-restricted-locators': [
              'error',
              [
                'getByTestId',
                {
                  type: 'getByTitle',
                  message: 'Prefer accessible locators',
                },
              ],
            ],
            'playwright/no-restricted-matchers': [
              'error',
              {
                'not.toBeTruthy': 'Prefer a positive matcher',
              },
            ],
            'playwright/no-restricted-roles': [
              'error',
              [
                'progressbar',
                {
                  role: 'alert',
                  message: 'Assert on specific content',
                },
              ],
            ],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.spec.tsx'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Usage of `getByTestId` is disallowed',
        'Prefer accessible locators',
        'Prefer a positive matcher',
        'Usage of role `progressbar` in getByRole() is disallowed',
        'Assert on specific content',
      ]);
      expect(payload.diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'playwright(no-restricted-locators)',
        'playwright(no-restricted-locators)',
        'playwright(no-restricted-matchers)',
        'playwright(no-restricted-roles)',
        'playwright(no-restricted-roles)',
      ]);
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('keeps all restricted rules inert when their option collection is empty', () => {
    const source = [
      'page.getByTestId("submit");',
      'expect(value).not.toBeTruthy();',
      'page.getByRole("progressbar");',
    ].join('\n');
    for (const rule of fixture.suites.map((suite) => suite.rule)) {
      expect(
        scanPlaywright(source, 'fixture.spec.ts', scanOptions(rule, { options: [] })).filter(
          (diagnostic) => diagnostic.ruleName === rule,
        ),
      ).toEqual([]);
    }
  });
});

function nativeDiagnostics(rule, testCase) {
  return scanPlaywright(testCase.code, 'fixture.spec.ts', scanOptions(rule, testCase))
    .filter((diagnostic) => diagnostic.ruleName === rule)
    .map(({ messageId, data, loc }) => ({ messageId, data, loc }));
}

function adapterReports(rule, testCase) {
  const reports = [];
  const sourceCode = {
    text: testCase.code,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[rule].createOnce({
    filename: 'fixture.spec.ts',
    options: testCase.options,
    settings: testCase.settings ?? {},
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, testCase.code.length] });
  return reports;
}

function adapterDiagnostics(rule, testCase) {
  return adapterReports(rule, testCase).map(({ messageId, data, loc }) => ({
    messageId,
    data,
    loc: {
      startLine: loc.start.line,
      startColumn: loc.start.column,
      endLine: loc.end.line,
      endColumn: loc.end.column,
    },
  }));
}

function scanOptions(rule, testCase) {
  const options = testCase.options ?? [];
  const expectAliases = testCase.settings?.playwright?.globalAliases?.expect;
  return {
    ...(rule === 'no-restricted-locators' ? { noRestrictedLocators: options[0] } : {}),
    ...(rule === 'no-restricted-matchers' ? { noRestrictedMatchers: options[0] } : {}),
    ...(rule === 'no-restricted-roles' ? { noRestrictedRoles: options[0] } : {}),
    ...(Array.isArray(expectAliases) ? { expectAliases } : {}),
  };
}

function restrictedListSchema(requiredProperty) {
  return {
    items: {
      oneOf: [
        { type: 'string' },
        {
          additionalProperties: false,
          properties: {
            message: { type: 'string' },
            [requiredProperty]: { type: 'string' },
          },
          required: [requiredProperty],
          type: 'object',
        },
      ],
    },
    type: 'array',
  };
}

function renderMessage(template, data) {
  return template.replace(/\{\{(\w+)\}\}/gu, (_match, key) => data[key] ?? '');
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
  return candidates.at(-1);
}
