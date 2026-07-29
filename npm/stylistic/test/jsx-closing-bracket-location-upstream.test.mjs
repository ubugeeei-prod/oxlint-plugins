import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-closing-bracket-location-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options) {
  const reports = [];
  const visitor = plugin.rules['jsx-closing-bracket-location'].createOnce({
    filename: 'fixture.tsx',
    options: options ?? [],
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

function renderedMessage(report) {
  let message = plugin.rules['jsx-closing-bracket-location'].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data ?? {})) {
    message = message.replaceAll(`{{${key}}}`, value);
  }
  return message;
}

function locationAt(source, offset) {
  const prefix = source.slice(0, offset);
  const terminators = [...prefix.matchAll(/\r\n|[\n\r\u2028\u2029]/gu)];
  const lastTerminator = terminators.at(-1);
  const lineStart = lastTerminator ? lastTerminator.index + lastTerminator[0].length : 0;
  return {
    line: terminators.length + 1,
    column: source.slice(lineStart, offset).length + 1,
  };
}

function fixedOutput(sourceText, reports) {
  const edits = reports
    .flatMap((report) =>
      (report.suggest ?? []).flatMap((suggestion) =>
        suggestion.fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    )
    .sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  if (edits.length === 0) {
    return null;
  }
  let output = sourceText;
  for (const edit of edits) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

describe('@stylistic/jsx-closing-bracket-location v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored inventory complete and reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-closing-bracket-location/jsx-closing-bracket-location.test.ts',
      license: 'MIT',
      parserMatrix: 'authored semantic cases; replayed with Oxc JSX/TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-closing-bracket-location-tests.ts',
      inventory: {
        valid: 44,
        invalid: 65,
        diagnostics: 65,
        fixableInvalid: 65,
        unfixableInvalid: 0,
        total: 109,
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
    'replays invalid case %i with exact messages, data, bracket ranges, and output',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(
        reports.map((report) => report.data),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.data));
      expect(reports.map(renderedMessage), testCase.code).toEqual(
        testCase.errors.map((error) => error.message),
      );
      expect(
        reports
          .map((report) => testCase.code.slice(...report.node.range))
          .every((token) => token === '/' || token === '>'),
        testCase.code,
      ).toBe(true);
      for (const [report, error] of reports.map((report, index) => [
        report,
        testCase.errors[index],
      ])) {
        if (error.line !== undefined && error.column !== undefined) {
          expect(locationAt(testCase.code, report.node.range[0])).toEqual({
            line: error.line,
            column: error.column,
          });
        }
      }
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(runRule(testCase.output, testCase.options), testCase.output).toEqual([]);
    },
  );

  it('maps Unicode prefixes to UTF-16 report/fix ranges and keeps nested exit order', () => {
    const source = [
      'const prefix = "😀";',
      'const view = <外側',
      '  child={<内側',
      '    value />}',
      '  />;',
    ].join('\n');
    const reports = runRule(source, [{ location: 'line-aligned' }]);
    expect(reports).toHaveLength(2);
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual(['/', '/']);
    expect(reports[0].node.range[0]).toBe(source.indexOf('/>'));
    expect(reports[1].node.range[0]).toBe(source.lastIndexOf('/>'));
    const output = fixedOutput(source, reports);
    expect(output).toContain('"😀"');
    expect(runRule(output, [{ location: 'line-aligned' }])).toEqual([]);
  });
});
