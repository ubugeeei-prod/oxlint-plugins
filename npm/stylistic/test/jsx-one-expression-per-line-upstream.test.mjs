import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-one-expression-per-line-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options = [], filename = 'fixture.tsx') {
  const reports = [];
  const visitor = plugin.rules['jsx-one-expression-per-line'].createOnce({
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

function reportFix(report) {
  const suggestion = report.suggest?.[0];
  if (!suggestion) {
    return null;
  }
  const fixes = suggestion.fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  });
  expect(fixes).toHaveLength(1);
  return fixes[0];
}

function applyOnePass(source, reports) {
  const fixes = reports
    .map((report, index) => ({ index, fix: reportFix(report) }))
    .filter(({ fix }) => fix)
    .sort(
      (left, right) =>
        left.fix.range[0] - right.fix.range[0] ||
        left.fix.range[1] - right.fix.range[1] ||
        left.index - right.index,
    );
  const accepted = [];
  let lastEnd = null;
  for (const { fix } of fixes) {
    if (lastEnd !== null && fix.range[0] <= lastEnd) {
      continue;
    }
    lastEnd = fix.range[1];
    accepted.push(fix);
  }
  let output = source;
  for (const fix of accepted.reverse()) {
    output = `${output.slice(0, fix.range[0])}${fix.replacementText}${output.slice(fix.range[1])}`;
  }
  return output;
}

describe('@stylistic/jsx-one-expression-per-line v5.10.0 exhaustive upstream parity', () => {
  it('pins the exact commit and complete authored inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-one-expression-per-line/jsx-one-expression-per-line.test.ts',
      ruleFile:
        'packages/eslint-plugin/rules/jsx-one-expression-per-line/jsx-one-expression-per-line.ts',
      license: 'MIT',
      parserMatrix: 'authored semantic cases; replayed with Oxc JSX and TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-one-expression-per-line-tests.ts',
      inventory: {
        valid: 47,
        invalid: 69,
        diagnostics: 84,
        fixableInvalid: 69,
        unfixableInvalid: 0,
        total: 116,
      },
    });
    expect(fixture.valid).toHaveLength(47);
    expect(fixture.invalid).toHaveLength(69);
    expect(fixture.invalid.reduce((count, testCase) => count + testCase.errors.length, 0)).toBe(84);
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts authored upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays authored upstream invalid case %i with exact reports and one-pass fixes',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(reports, testCase.code).toHaveLength(testCase.errors.length);
      for (const [errorIndex, [report, error]] of reports
        .map((report, index) => [report, testCase.errors[index]])
        .entries()) {
        expect(report.messageId, `${errorIndex}\n${testCase.code}`).toBe(error.messageId);
        expect(report.data?.descriptor, `${errorIndex}\n${testCase.code}`).toEqual(
          expect.any(String),
        );
        if (error.data) {
          expect(report.data, `${errorIndex}\n${testCase.code}`).toEqual(error.data);
        }
        if (error.message) {
          expect(
            `\`${report.data.descriptor}\` must be placed on a new line`,
            `${errorIndex}\n${testCase.code}`,
          ).toBe(error.message);
        }
        expect(report.node.range[0]).toBeLessThanOrEqual(report.node.range[1]);
        expect(report.node.range[1]).toBeLessThanOrEqual(testCase.code.length);
        expect(report.suggest).toHaveLength(1);
        expect(reportFix(report)).toMatchObject({
          range: expect.any(Array),
          replacementText: expect.any(String),
        });
      }
      expect(applyOnePass(testCase.code, reports), testCase.code).toBe(testCase.output);
    },
  );

  it('maps Unicode byte ranges and fixes to exact UTF-16 offsets', () => {
    const source = "const marker = '😀'; const view = <App>日本語<Foo />後</App>;";
    const reports = runRule(source);
    const textStart = source.indexOf('日本語');
    const elementStart = source.indexOf('<Foo />');
    const tailStart = source.indexOf('後');

    expect(
      reports.map((report) => ({
        messageId: report.messageId,
        data: report.data,
        range: report.node.range,
        fix: reportFix(report),
      })),
    ).toEqual([
      {
        messageId: 'moveToNewLine',
        data: { descriptor: '日本語' },
        range: [textStart, elementStart],
        fix: { range: [textStart, elementStart], replacementText: '\n日本語' },
      },
      {
        messageId: 'moveToNewLine',
        data: { descriptor: 'Foo' },
        range: [elementStart, elementStart + '<Foo />'.length],
        fix: {
          range: [elementStart, elementStart + '<Foo />'.length],
          replacementText: '\n<Foo />',
        },
      },
      {
        messageId: 'moveToNewLine',
        data: { descriptor: '後' },
        range: [tailStart, tailStart + 1],
        fix: { range: [tailStart, tailStart + 1], replacementText: '\n後\n' },
      },
    ]);
  });

  it('covers CRLF, every allow mode, invalid options, fragments, and TSX', () => {
    expect(runRule('<App>text</App>', [{ allow: 'literal' }])).toEqual([]);
    expect(runRule('<App><Foo /></App>', [{ allow: 'single-child' }])).toEqual([]);
    expect(runRule('<App>text<Foo /></App>', [{ allow: 'single-line' }])).toEqual([]);
    expect(runRule('<App>text {value}</App>', [{ allow: 'non-jsx' }])).toEqual([]);
    expect(runRule('<App>{<Foo />}</App>', [{ allow: 'non-jsx' }])).toEqual([]);
    expect(runRule('<><Foo/><Bar/></>', [{ allow: 'single-line' }])).toEqual([]);

    for (const options of [[], [{ allow: 'unknown' }], [{ allow: 42 }], [null], 'invalid']) {
      expect(runRule('<App>text</App>', options)).toHaveLength(1);
    }

    const crlf = '<App>\r\n  日本語<Foo />\r\n</App>';
    const crlfReports = runRule(crlf);
    const crlfElementStart = crlf.indexOf('<Foo />');
    expect(
      crlfReports.map((report) => ({
        descriptor: report.data.descriptor,
        range: report.node.range,
        fix: reportFix(report),
      })),
    ).toEqual([
      {
        descriptor: 'Foo',
        range: [crlfElementStart, crlfElementStart + '<Foo />'.length],
        fix: {
          range: [crlfElementStart, crlfElementStart + '<Foo />'.length],
          replacementText: '\n<Foo />',
        },
      },
    ]);
    expect(runRule('<App><Foo/><Bar/></App>', [], 'fixture.tsx')).toHaveLength(2);
  });

  it('silently ignores invalid syntax and a non-JSX source type', () => {
    expect(runRule('<App><Broken></App>')).toEqual([]);
    expect(runRule('<App>text</App>', [], 'fixture.ts')).toEqual([]);
  });
});
