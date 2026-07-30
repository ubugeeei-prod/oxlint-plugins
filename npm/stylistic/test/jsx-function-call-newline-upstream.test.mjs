import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-function-call-newline-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options, filename = 'fixture.tsx') {
  const reports = [];
  const visitor = plugin.rules['jsx-function-call-newline'].createOnce({
    filename,
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
  return plugin.rules['jsx-function-call-newline'].meta.messages[report.messageId];
}

function reportFix(report) {
  if (!report.suggest?.[0]) {
    return null;
  }
  return report.suggest[0].fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  });
}

function fixedOutput(sourceText, reports) {
  const edits = reports
    .flatMap((report) => reportFix(report) ?? [])
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

describe('@stylistic/jsx-function-call-newline v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored inventory complete and reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-function-call-newline/jsx-function-call-newline.test.ts',
      license: 'MIT',
      parserMatrix: 'authored semantic cases; replayed with Oxc JSX and TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-function-call-newline-tests.ts',
      inventory: {
        valid: 19,
        invalid: 8,
        diagnostics: 13,
        fixableInvalid: 8,
        unfixableInvalid: 0,
        total: 27,
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
    'replays invalid case %i with exact diagnostics, JSX ranges, fixes, and convergence',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(reports.map(renderedMessage), testCase.code).toEqual(
        testCase.errors.map((error) => error.message),
      );
      expect(
        reports
          .map((report) => testCase.code.slice(...report.node.range))
          .every(
            (text) =>
              (text.startsWith('<') && text.endsWith('>')) ||
              (text.startsWith('<>') && text.endsWith('</>')),
          ),
        testCase.code,
      ).toBe(true);
      for (const report of reports) {
        const fixes = reportFix(report);
        expect(fixes).toHaveLength(1);
        expect(fixes[0].range).toEqual(report.node.range);
      }
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(runRule(testCase.output, testCase.options), testCase.output).toEqual([]);
    },
  );

  it('maps Unicode prefixes to exact UTF-16 ranges and preserves CRLF inside TSX', () => {
    const source = 'const marker = "😀";\r\nrender(<外側\r\n  label="日本語" />);';
    const start = source.indexOf('<外側');
    const end = source.indexOf('/>);') + 2;
    const reports = runRule(source, []);
    expect(reports).toHaveLength(1);
    expect(reports[0].node.range).toEqual([start, end]);
    expect(reportFix(reports[0])).toEqual([
      {
        range: [start, end],
        replacementText: `\n${source.slice(start, end)}\n`,
      },
    ]);
    expect(fixedOutput(source, reports)).toBe(
      'const marker = "😀";\r\nrender(\n<外側\r\n  label="日本語" />\n);',
    );
  });

  it('treats invalid option values as multiline without false positives', () => {
    for (const options of [[], ['unknown'], [false], [{}], 'invalid']) {
      expect(runRule('render(<App />);', options)).toEqual([]);
      expect(runRule('render(<App\n  />);', options).map((report) => report.messageId)).toEqual([
        'missingLineBreak',
      ]);
    }
  });

  it('supports fragments, new expressions, comments, and all ECMAScript line terminators', () => {
    const source = 'new Wrapper(/* before */(<>日本語</>)/* after */)';
    const reports = runRule(source, ['always']);
    expect(reports).toHaveLength(1);
    expect(source.slice(...reports[0].node.range)).toBe('<>日本語</>');
    expect(fixedOutput(source, reports)).toBe(
      'new Wrapper(/* before */(\n<>日本語</>\n)/* after */)',
    );

    for (const terminator of ['\n', '\r\n', '\r', '\u2028', '\u2029']) {
      expect(runRule(`render(${terminator}<App />${terminator});`, ['always'])).toEqual([]);
    }
  });
});
