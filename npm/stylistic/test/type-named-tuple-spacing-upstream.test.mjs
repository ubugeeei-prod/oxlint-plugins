import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/type-named-tuple-spacing-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText) {
  const reports = [];
  const visitor = plugin.rules['type-named-tuple-spacing'].createOnce({
    filename: 'fixture.tsx',
    options: [],
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

function fixedOutput(sourceText, reports) {
  const byRange = new Map();
  for (const report of reports) {
    for (const suggestion of report.suggest ?? []) {
      for (const edit of suggestion.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      })) {
        byRange.set(`${edit.range[0]}:${edit.range[1]}`, edit);
      }
    }
  }
  const edits = [...byRange.values()].sort(
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

describe('@stylistic/type-named-tuple-spacing v5.10.0 upstream parity', () => {
  it('keeps the exact pinned inventory complete', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/type-named-tuple-spacing/type-named-tuple-spacing.test.ts',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-type-named-tuple-spacing-tests.ts',
      inventory: {
        valid: 5,
        invalid: 11,
        diagnostics: 18,
        fixableInvalid: 11,
        unfixableInvalid: 0,
        total: 16,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays upstream invalid case %i with exact messages, ranges, and output',
    (_index, testCase) => {
      const reports = runRule(testCase.code);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(
        reports.every(
          (report) =>
            report.node.range[0] >= 0 &&
            report.node.range[0] < report.node.range[1] &&
            report.node.range[1] <= testCase.code.length,
        ),
      ).toBe(true);
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(runRule(testCase.output), testCase.output).toEqual([]);
    },
  );
});
