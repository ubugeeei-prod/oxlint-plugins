import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-child-element-spacing-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options = [], filename = 'fixture.tsx') {
  const reports = [];
  const visitor = plugin.rules['jsx-child-element-spacing'].createOnce({
    options,
    filename,
    sourceCode: {
      text: sourceText,
      getText() {
        return this.text;
      },
    },
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

describe('@stylistic/jsx-child-element-spacing v5.10.0 exhaustive upstream parity', () => {
  it('pins the exact commit, parser expansion, and complete inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-child-element-spacing/jsx-child-element-spacing.test.ts',
      ruleFile:
        'packages/eslint-plugin/rules/jsx-child-element-spacing/jsx-child-element-spacing.ts',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-child-element-spacing-tests.ts',
      parserMatrix: ['default', '@babel/eslint-parser', '@typescript-eslint/parser'],
      inventory: {
        logicalValid: 21,
        logicalInvalid: 7,
        valid: 62,
        invalid: 20,
        diagnostics: 23,
        fixableInvalid: 0,
        unfixableInvalid: 20,
        total: 82,
      },
    });
    expect(fixture.valid).toHaveLength(62);
    expect(fixture.invalid).toHaveLength(20);
    expect(
      fixture.invalid.reduce((count, testCase) => count + testCase.expectedDiagnostics.length, 0),
    ).toBe(23);
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts expanded upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code), `${testCase.parser}\n${testCase.code}`).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays expanded upstream invalid case %i with exact reports',
    (_index, testCase) => {
      const reports = runRule(testCase.code);
      expect(
        reports.map((report) => ({
          messageId: report.messageId,
          data: report.data,
          range: report.node.range,
          hasSuggestions: report.suggest !== undefined,
        })),
        `${testCase.parser}\n${testCase.code}`,
      ).toEqual(
        testCase.expectedDiagnostics.map((diagnostic) => ({
          messageId: diagnostic.messageId,
          data: diagnostic.data,
          range: [diagnostic.range.start, diagnostic.range.end],
          hasSuggestions: false,
        })),
      );
      expect(testCase.output).toBeNull();
    },
  );

  it('maps Unicode byte positions to exact UTF-16 zero-width report ranges', () => {
    const source = '<App>日本語\r\n<a>リンク</a>\r\n後続</App>';
    const reports = runRule(source);
    const start = source.indexOf('<a>');
    const end = source.indexOf('</a>') + '</a>'.length;

    expect(reports).toMatchObject([
      {
        messageId: 'spacingBeforeNext',
        data: { element: 'a' },
        node: { range: [start, start] },
      },
      {
        messageId: 'spacingAfterPrev',
        data: { element: 'a' },
        node: { range: [end, end] },
      },
    ]);
    expect(reports.every((report) => report.suggest === undefined)).toBe(true);
  });

  it('keeps comments, explicit spaces, non-inline tags, and options out of scope', () => {
    const valid = [
      "<App>word\n{' '}<a /></App>",
      '<App>word\n{/* preserve */}<a /></App>',
      '<App>word\n<br /></App>',
      '<App>word\n<p /></App>',
      '<App>word\n<Component /></App>',
      '<App>word\n<UI.a /></App>',
      '<App>word\n<svg:a /></App>',
      '<App>word\n<></></App>',
    ];

    for (const source of valid) {
      expect(runRule(source, ['ignored', { ignored: true }]), source).toEqual([]);
    }
  });

  it('silently ignores invalid syntax and a non-JSX filename', () => {
    expect(runRule('<App>word\n<a></App>')).toEqual([]);
    expect(runRule('<App>word\n<a /></App>', [], 'fixture.ts')).toEqual([]);
  });
});
