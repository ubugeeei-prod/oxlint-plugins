import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'array-bracket-newline.json'), 'utf8'),
);

function runRule(sourceText, options) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['array-bracket-newline'].createOnce({
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
  const line = prefix.split(/\r\n|[\n\r\u2028\u2029]/u).length;
  const lineStart = Math.max(
    prefix.lastIndexOf('\n'),
    prefix.lastIndexOf('\r'),
    prefix.lastIndexOf('\u2028'),
    prefix.lastIndexOf('\u2029'),
  );
  return {
    line,
    column: source.slice(lineStart + 1, offset).length + 1,
  };
}

function suggestionEdit(report) {
  const suggestion = report.suggest?.[0];
  if (!suggestion) {
    return undefined;
  }
  return suggestion.fix({
    insertTextAfterRange(range, replacementText) {
      return { range: [range[1], range[1]], replacementText };
    },
    insertTextBeforeRange(range, replacementText) {
      return { range: [range[0], range[0]], replacementText };
    },
    removeRange(range) {
      return { range, replacementText: '' };
    },
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  })[0];
}

function applyFixesUntilStable(source, options) {
  let output = source;
  let changed = false;

  for (let pass = 0; pass < 100; pass += 1) {
    const edits = runRule(output, options)
      .map(suggestionEdit)
      .filter(Boolean)
      .sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
    if (edits.length === 0) {
      return changed ? output : null;
    }

    const accepted = [];
    let lastEnd = -1;
    for (const edit of edits) {
      if (lastEnd >= edit.range[0]) {
        continue;
      }
      accepted.push(edit);
      lastEnd = edit.range[1];
    }

    let next = output;
    for (const edit of accepted.reverse()) {
      next = next.slice(0, edit.range[0]) + edit.replacementText + next.slice(edit.range[1]);
    }
    if (next === output) {
      throw new Error('array-bracket-newline produced a non-progressing fix');
    }
    output = next;
    changed = true;
  }

  throw new Error('array-bracket-newline fixes did not converge within 100 passes');
}

describe('array-bracket-newline upstream v5.10.0 parity', () => {
  it('keeps the stable upstream case inventory complete', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
    });
    expect(fixture.valid).toHaveLength(128);
    expect(fixture.invalid).toHaveLength(82);
    expect(fixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(149);
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
      for (const [reportIndex, error] of testCase.errors.entries()) {
        if (error.endLine !== undefined || error.endColumn !== undefined) {
          expect(locationAt(testCase.code, reports[reportIndex].node.range[1])).toEqual({
            line: error.endLine,
            column: error.endColumn,
          });
        }
      }
      expect(applyFixesUntilStable(testCase.code, testCase.options)).toBe(testCase.output);
    },
  );
});
