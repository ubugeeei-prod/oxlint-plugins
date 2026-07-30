import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import typescriptParser from '@typescript-eslint/parser';
import { Linter } from 'eslint';
import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE = 'indent';
const RULE_ID = `stylistic/${RULE}`;
const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = dirname(here);
const workspaceRoot = resolve(packageRoot, '../..');
const fixture = JSON.parse(readFileSync(join(here, 'fixtures', 'indent-v5.10.0.json'), 'utf8'));
const suites = fixture.suites.filter((suite) => suite.language !== 'css');
const validCases = suites.flatMap((suite) =>
  suite.valid.map((testCase, index) => ({
    suite: `${suite.name}:${suite.language}`,
    index,
    language: suite.language,
    testCase,
  })),
);
const invalidCases = suites.flatMap((suite) =>
  suite.invalid.map((testCase, index) => ({
    suite: `${suite.name}:${suite.language}`,
    index,
    language: suite.language,
    testCase,
  })),
);

function configFor(testCase, settings) {
  const parserOptions = testCase.parserOptions ?? {};
  const { sourceType: parserSourceType, ...restParserOptions } = parserOptions;
  return [
    {
      files: ['**/*.{js,jsx,ts,tsx}'],
      languageOptions: {
        ecmaVersion: parserOptions.ecmaVersion ?? 'latest',
        sourceType: parserSourceType ?? 'module',
        ...(testCase.parser === 'typescript' ? { parser: typescriptParser } : {}),
        parserOptions: restParserOptions,
      },
      plugins: {
        stylistic: { rules: { [RULE]: plugin.rules[RULE] } },
      },
      ...(settings ? { settings } : {}),
      rules: {
        [RULE_ID]: ['error', ...testCase.options],
      },
    },
  ];
}

function filenameFor(testCase, language) {
  if (testCase.parser === 'typescript') {
    return language === 'jsx' ? 'fixture.tsx' : 'fixture.ts';
  }
  return language === 'jsx' ? 'fixture.jsx' : 'fixture.js';
}

function verify(testCase, language, source = testCase.code, settings) {
  return new Linter()
    .verify(source, configFor(testCase, settings), {
      filename: filenameFor(testCase, language),
    })
    .filter((message) => message.ruleId === RULE_ID);
}

function offsetAt(source, line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line) {
    const match = /\r\n|[\n\r\u2028\u2029]/u.exec(source.slice(offset));
    if (!match) {
      throw new Error(`Cannot map ${line}:${column}`);
    }
    offset += match.index + match[0].length;
    currentLine += 1;
  }
  return offset + column - 1;
}

function actualDiagnostic(source, message) {
  const dataMatch = /^Expected indentation of (.+) but found (.+)\.$/u.exec(message.message);
  if (!dataMatch) {
    throw new Error(`Unexpected indent message: ${message.message}`);
  }
  return {
    messageId: message.messageId,
    message: message.message,
    data: {
      expected: dataMatch[1],
      actual: /^\d+$/u.test(dataMatch[2]) ? Number(dataMatch[2]) : dataMatch[2],
    },
    range: [
      offsetAt(source, message.line, message.column),
      offsetAt(source, message.endLine, message.endColumn),
    ],
    loc: {
      line: message.line,
      column: message.column,
      endLine: message.endLine,
      endColumn: message.endColumn,
    },
    fix: message.fix ? { range: message.fix.range, text: message.fix.text } : null,
  };
}

function applyFirstPass(source, messages) {
  const fixes = messages.flatMap((message) => (message.fix ? [message.fix] : []));
  if (fixes.length === 0) {
    return null;
  }
  fixes.sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  let output = '';
  let lastPosition = Number.NEGATIVE_INFINITY;
  for (const fix of fixes) {
    if (lastPosition >= fix.range[0]) {
      continue;
    }
    output += source.slice(Math.max(0, lastPosition), fix.range[0]) + fix.text;
    lastPosition = fix.range[1];
  }
  return output + source.slice(Math.max(0, lastPosition));
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

describe('@stylistic/indent v5.10.0 exhaustive upstream replay', () => {
  it('pins every stable authored suite and its deterministic inventory', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      license: 'MIT',
      parserMatrix: 'ESLint 10: Babel disabled; JSX authored cases expand to Espree and TypeScript',
      tool: 'tools/tasks/sync-stylistic-indent-tests.ts',
      inventory: {
        suites: [
          { name: 'indent', language: 'js', valid: 706, invalid: 387 },
          { name: 'jsx-indent', language: 'jsx', valid: 150, invalid: 101 },
          { name: 'indent', language: 'ts', valid: 76, invalid: 83 },
          { name: 'indent', language: 'css', valid: 1, invalid: 0 },
        ],
        valid: 933,
        invalid: 571,
        diagnostics: 1298,
        fixableInvalid: 571,
      },
    });
    expect(fixture.__generated.sourceFiles).toHaveLength(4);
    expect(Object.keys(fixture.__generated.sourceHashes)).toHaveLength(7);
    expect(
      Object.values(fixture.__generated.sourceHashes).every((hash) => /^[\da-f]{64}$/u.test(hash)),
    ).toBe(true);
  });

  it.each(validCases)('accepts $suite valid case $index', ({ language, testCase }) => {
    expect(verify(testCase, language)).toEqual([]);
  });

  it.each(invalidCases)(
    'matches $suite invalid case $index diagnostics, ranges, data, and fixes',
    ({ language, testCase }) => {
      const messages = verify(testCase, language);
      expect(messages.map((message) => actualDiagnostic(testCase.code, message))).toEqual(
        testCase.expectedDiagnostics,
      );
      expect(applyFirstPass(testCase.code, messages)).toBe(testCase.output);

      const fixed = new Linter().verifyAndFix(testCase.code, configFor(testCase), {
        filename: filenameFor(testCase, language),
      });
      expect(fixed.fixed ? fixed.output : null).toBe(testCase.recursiveOutput);
      expect(
        verify(testCase, language, fixed.output).map((message) =>
          actualDiagnostic(fixed.output, message),
        ),
      ).toEqual(testCase.recursiveDiagnostics);
    },
  );
});

describe('indent integration surfaces', () => {
  it('preserves the complete upstream metadata and option schema', () => {
    expect(plugin.rules.indent.meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce consistent indentation',
        recommended: false,
        requiresTypeChecking: false,
        url: expect.stringMatching(/github\.com\/ubugeeei-prod\/oxlint-plugins/u),
      },
      fixable: 'whitespace',
      hasSuggestions: false,
      messages: {
        wrongIndentation: 'Expected indentation of {{expected}} but found {{actual}}.',
      },
      defaultOptions: [
        4,
        {
          SwitchCase: 1,
          flatTernaryExpressions: false,
          ignoredNodes: [],
        },
      ],
    });
    expect(plugin.rules.indent.meta.schema).toHaveLength(2);
  });

  it('uses shared settings while direct rule options keep precedence', () => {
    const testCase = { code: 'if (ready) {\nvalue();\n}\n', options: [], parser: 'espree' };
    const settings = { corsaStylistic: { rules: { indent: [2] } } };
    const shared = verify(testCase, 'js', testCase.code, settings);
    expect(shared).toHaveLength(1);
    expect(shared[0].message).toBe('Expected indentation of 2 spaces but found 0.');

    const direct = { ...testCase, options: [8] };
    const overridden = verify(direct, 'js', direct.code, settings);
    expect(overridden[0].message).toBe('Expected indentation of 8 spaces but found 0.');
  });

  it('keeps Unicode, CRLF, comments, tabs, selectors, and malformed input deterministic', () => {
    const cases = [
      {
        code: 'if (日本語) {\r\nvalue();\r\n}\r\n',
        options: [2],
        parser: 'espree',
        expected: ['Expected indentation of 2 spaces but found 0.'],
      },
      {
        code: 'if (ready) {\n// keep\n\tvalue();\n}\n',
        options: ['tab', { ignoreComments: true }],
        parser: 'espree',
        expected: [],
      },
      {
        code: 'if (ready) {\nvalue();\n}\n',
        options: [2, { ignoredNodes: ['ExpressionStatement'] }],
        parser: 'espree',
        expected: [],
      },
      {
        code: 'if (ready) {\nvalue(\n}\n',
        options: [2],
        parser: 'espree',
        expected: [],
      },
    ];
    for (const testCase of cases) {
      expect(verify(testCase, 'js').map((message) => message.message)).toEqual(testCase.expected);
    }
  });

  it('exposes native API metadata, byte ranges, data, and fixes', () => {
    expect(plugin.nativeStylisticRuleMetas().find((meta) => meta.name === RULE)).toMatchObject({
      name: RULE,
      docsDescription: 'Enforce consistent indentation.',
      hasSuggestions: true,
      messages: {
        wrongIndentation: 'Expected indentation of {{expected}} but found {{actual}}.',
      },
    });
    const source = 'function 日本語() {\r\nreturn 1;\r\n}\r\n';
    expect(
      plugin.runNativeStylisticLint(source, {
        filename: 'fixture.ts',
        rules: [{ name: RULE, options: [2] }],
      }),
    ).toEqual([
      {
        ruleName: RULE,
        messageId: 'wrongIndentation',
        message: 'Expected indentation of 2 spaces but found 0.',
        data: { actual: '0', expected: '2 spaces' },
        range: { start: 24, end: 24 },
        suggestions: [
          {
            messageId: 'wrongIndentation',
            message: 'Expected indentation of 2 spaces but found 0.',
            fixes: [{ range: { start: 24, end: 24 }, replacementText: '  ' }],
          },
        ],
      },
    ]);
  });

  it('runs and fixes JavaScript, TypeScript, and TSX through real Oxlint', () => {
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-indent-plugin-'));
    try {
      const jsPath = join(temp, 'sample.js');
      const tsPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(jsPath, 'export function value() {\nreturn 1;\n}\n');
      writeFileSync(tsPath, 'export interface Value {\nanswer: number;\n}\n');
      writeFileSync(tsxPath, 'export const view = (\n<App\nvalue={{\nanswer: 42,\n}}\n/>\n);\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: { [RULE_ID]: ['error', 2] },
        }),
      );

      const oxlint = findOxlintCli();
      const linted = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', jsPath, tsPath, tsxPath],
        { encoding: 'utf8' },
      );
      expect(linted.status).toBe(1);
      expect(linted.stderr).toBe('');
      const diagnostics = JSON.parse(linted.stdout).diagnostics;
      expect(diagnostics).toHaveLength(7);
      expect(diagnostics.every((diagnostic) => diagnostic.code === 'stylistic(indent)')).toBe(true);

      const fixed = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--fix', jsPath, tsPath, tsxPath],
        { encoding: 'utf8' },
      );
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(jsPath, 'utf8')).toBe('export function value() {\n  return 1;\n}\n');
      expect(readFileSync(tsPath, 'utf8')).toBe('export interface Value {\n  answer: number;\n}\n');
      expect(readFileSync(tsxPath, 'utf8')).toBe(
        'export const view = (\n  <App\n    value={{\n      answer: 42,\n    }}\n  />\n);\n',
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
});
