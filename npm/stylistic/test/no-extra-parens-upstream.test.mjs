import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/no-extra-parens-v5.10.0.json', import.meta.url), 'utf8'),
);

function run(testCase, sourceText = testCase.code) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['no-extra-parens'].createOnce({
    filename: testCase.language === 'js' ? 'fixture.jsx' : 'fixture.ts',
    options: testCase.options ?? [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function fixesFor(report) {
  if (!report.suggest?.[0]) {
    return [];
  }
  return report.suggest[0].fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  });
}

function applyOnePass(sourceText, reports) {
  const edits = reports
    .flatMap(fixesFor)
    .sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  if (edits.length === 0) {
    return null;
  }

  const accepted = [];
  let lastEnd = -1;
  for (const edit of edits) {
    if (edit.range[0] < lastEnd) {
      continue;
    }
    accepted.push(edit);
    lastEnd = edit.range[1];
  }

  let output = sourceText;
  for (const edit of accepted.reverse()) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

function applyToExpectedOutput(testCase) {
  if (testCase.output == null) {
    return applyOnePass(testCase.code, run(testCase));
  }
  let output = testCase.code;
  for (let pass = 0; pass < 20; pass += 1) {
    const next = applyOnePass(output, run(testCase, output));
    if (next === null) {
      return null;
    }
    expect(next, `fix pass ${pass + 1} must make progress for ${testCase.code}`).not.toBe(output);
    output = next;
    if (output === testCase.output) {
      return output;
    }
  }
  throw new Error(`no-extra-parens fixes did not reach upstream output: ${testCase.code}`);
}

describe('no-extra-parens upstream v5.10.0 parity', () => {
  it('pins both complete stable upstream suites and their diagnostic inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/no-extra-parens/no-extra-parens._js_.test.ts',
        'packages/eslint-plugin/rules/no-extra-parens/no-extra-parens._ts_.test.ts',
      ],
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-no-extra-parens-tests.ts',
    });
    expect(fixture.valid).toHaveLength(595);
    expect(fixture.invalid).toHaveLength(624);
    expect(
      fixture.invalid.reduce(
        (total, testCase) => total + (Array.isArray(testCase.errors) ? testCase.errors.length : 1),
        0,
      ),
    ).toBe(681);
    expect(fixture.invalid.filter((testCase) => typeof testCase.output === 'string')).toHaveLength(
      617,
    );
    expect(fixture.invalid.filter((testCase) => testCase.output == null)).toHaveLength(7);
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(run(testCase)).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'matches upstream invalid case %i reports and recursive fixes',
    (_index, testCase) => {
      const reports = run(testCase);
      const expectedCount = Array.isArray(testCase.errors) ? testCase.errors.length : 1;
      expect(reports).toHaveLength(expectedCount);
      expect(reports.map((report) => report.messageId)).toEqual(
        Array.from({ length: expectedCount }, () => 'unexpected'),
      );
      expect(reports.map((report) => testCase.code.slice(...report.node.range))).toEqual(
        Array.from({ length: expectedCount }, () => '('),
      );
      expect(reports.every((report) => report.message === undefined)).toBe(true);
      expect(applyToExpectedOutput(testCase)).toBe(testCase.output ?? null);
    },
  );
});
