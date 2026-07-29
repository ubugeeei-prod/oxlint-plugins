import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/nonblock-statement-body-position-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options) {
  const reports = [];
  const visitor = plugin.rules['nonblock-statement-body-position'].createOnce({
    options: options ?? [],
    filename: 'fixture.tsx',
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

function editsForReports(reports) {
  return reports.flatMap(
    (report) =>
      report.suggest?.flatMap((suggestion) =>
        suggestion.fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ) ?? [],
  );
}

function fixedOutput(sourceText, reports) {
  const edits = editsForReports(reports).sort(
    (left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1],
  );
  if (edits.length === 0) {
    return null;
  }
  let output = sourceText;
  for (const edit of edits) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

describe('@stylistic/nonblock-statement-body-position v5.10.0 upstream parity', () => {
  it('keeps the exact pinned inventory complete', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/nonblock-statement-body-position/nonblock-statement-body-position.test.ts',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-nonblock-statement-body-position-tests.ts',
      inventory: {
        valid: 31,
        invalid: 17,
        diagnostics: 19,
        fixableInvalid: 17,
        unfixableInvalid: 0,
        total: 48,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays upstream invalid case %i with exact messages and output',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(
        reports.every(
          (report) =>
            report.node.range[0] >= 0 &&
            report.node.range[0] < report.node.range[1] &&
            report.node.range[1] <= testCase.code.length,
        ),
      ).toBe(true);
      expect(runRule(testCase.output, testCase.options), testCase.output).toEqual([]);
    },
  );
});
