import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'type-annotation-spacing.json'), 'utf8'),
);

function runRule(sourceText, options) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['type-annotation-spacing'].createOnce({
    options: options ?? [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function locationAt(source, offset) {
  const prefix = source.slice(0, offset);
  const line = prefix.split('\n').length;
  const lineStart = prefix.lastIndexOf('\n') + 1;
  return {
    line,
    column: source.slice(lineStart, offset).length + 1,
  };
}

function applySuggestions(source, reports) {
  const edits = reports
    .flatMap((report) =>
      report.suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    )
    .sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);

  let output = source;
  for (const edit of edits) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

describe('type-annotation-spacing upstream v5.10.0 parity', () => {
  it('keeps the stable upstream case inventory complete', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
    });
    expect(fixture.valid).toHaveLength(255);
    expect(fixture.invalid).toHaveLength(223);
    expect(fixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(408);
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'matches upstream invalid case %i',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(reports.map((report) => report.messageId)).toEqual(
        testCase.errors.map((error) => error.messageId),
      );
      expect(reports.map((report) => locationAt(testCase.code, report.node.range[0]))).toEqual(
        testCase.errors.map(({ line, column }) => ({ line, column })),
      );
      expect(applySuggestions(testCase.code, reports)).toBe(testCase.output);
    },
  );
});
