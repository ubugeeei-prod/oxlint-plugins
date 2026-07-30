import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-sort-props-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options, filename = 'fixture.tsx', settings) {
  const reports = [];
  const visitor = plugin.rules['jsx-sort-props'].createOnce({
    filename,
    options: options ?? [],
    settings,
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
  return plugin.rules['jsx-sort-props'].meta.messages[report.messageId];
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
  const fixes = reports.flatMap((report) => reportFix(report) ?? []);
  if (fixes.length === 0) {
    return null;
  }
  const fix = fixes[0];
  return sourceText.slice(0, fix.range[0]) + fix.replacementText + sourceText.slice(fix.range[1]);
}

const validParserCases = fixture.valid.flatMap((testCase, caseIndex) =>
  testCase.parsers.map((parser) => ({ caseIndex, parser, testCase })),
);
const invalidParserCases = fixture.invalid.flatMap((testCase, caseIndex) =>
  testCase.parsers.map((parser) => ({ caseIndex, parser, testCase })),
);

describe('@stylistic/jsx-sort-props v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored and parser-expanded inventory reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-sort-props/jsx-sort-props.test.ts',
      license: 'MIT',
      eslintVersion: '10.4.1',
      typescriptEslintParserVersion: '8.60.0',
      parserMatrix: 'authored semantic cases replayed with Oxc JSX and TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-sort-props-tests.ts',
      inventory: {
        authoredValid: 43,
        authoredInvalid: 54,
        authoredDiagnostics: 120,
        fixableInvalid: 53,
        unfixableInvalid: 1,
        authoredTotal: 97,
        parserExpandedValid: 86,
        parserExpandedInvalid: 107,
        parserExpandedDiagnostics: 239,
        parserExpandedTotal: 193,
      },
    });
  });

  it.each(validParserCases)(
    'accepts authored valid case $caseIndex with Oxc $parser',
    ({ parser, testCase }) => {
      expect(runRule(testCase.code, testCase.options, `fixture.${parser}`), testCase.code).toEqual(
        [],
      );
    },
  );

  it.each(invalidParserCases)(
    'replays invalid case $caseIndex with exact $parser diagnostics and fixes',
    ({ parser, testCase }) => {
      const reports = runRule(testCase.code, testCase.options, `fixture.${parser}`);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics.map((diagnostic) => diagnostic.messageId));
      expect(reports.map(renderedMessage), testCase.code).toEqual(
        testCase.expectedDiagnostics.map((diagnostic) => diagnostic.message),
      );
      expect(
        reports.map((report) => report.node.range),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics.map((diagnostic) => diagnostic.range));

      for (const [index, report] of reports.entries()) {
        const expectedFix = testCase.expectedDiagnostics[index].fix;
        const fixes = reportFix(report);
        if (expectedFix === null) {
          expect(fixes, testCase.code).toBeNull();
        } else {
          expect(fixes, testCase.code).toEqual([
            {
              range: expectedFix.range,
              replacementText: expectedFix.text,
            },
          ]);
        }
      }

      const firstPassOutput = fixedOutput(testCase.code, reports);
      expect(firstPassOutput, testCase.code).toBe(testCase.firstPassOutput);
      if (firstPassOutput === testCase.code) {
        expect(reportFix(reports[0]), testCase.code).toEqual([
          { range: [0, 0], replacementText: '' },
        ]);
      }
      expect(firstPassOutput, testCase.code).toBe(testCase.recursiveOutput);
    },
  );

  it('maps Unicode prefixes to UTF-16 ranges and preserves CRLF TSX fixes', () => {
    const source = [
      'const marker = "😀";',
      'declare const 値: unknown;',
      '<外側<T> ζeta={値} value alpha />;',
    ].join('\r\n');
    const reports = runRule(source, [], 'fixture.tsx');
    const fixStart = source.indexOf('ζeta');
    const fixEnd = source.indexOf('alpha') + 'alpha'.length;

    expect(reports.map((report) => source.slice(...report.node.range))).toEqual(['value', 'alpha']);
    expect(reportFix(reports[0])).toEqual([
      {
        range: [fixStart, fixEnd],
        replacementText: 'alpha value ζeta={値}',
      },
    ]);
    expect(fixedOutput(source, reports)).toBe(
      [
        'const marker = "😀";',
        'declare const 値: unknown;',
        '<外側<T> alpha value ζeta={値} />;',
      ].join('\r\n'),
    );
  });

  it('uses shared settings for reserved, callback, shorthand, and multiline precedence', () => {
    const source = [
      '<Panel',
      '  onChange={onChange}',
      '  shorthand',
      '  value={{',
      '    nested: true,',
      '  }}',
      '  key="key"',
      '/>',
    ].join('\n');
    const options = {
      callbacksLast: true,
      multiline: 'last',
      reservedFirst: true,
      shorthandLast: true,
    };
    const settings = {
      corsaStylistic: {
        rules: {
          'jsx-sort-props': [options],
        },
      },
    };
    const reports = runRule(source, [], 'fixture.tsx', settings);

    expect(reports.map((report) => report.messageId)).toEqual([
      'listCallbacksLast',
      'listReservedPropsFirst',
    ]);
    expect(fixedOutput(source, reports)).toBe(
      [
        '<Panel',
        '  key="key"',
        '  value={{',
        '    nested: true,',
        '  }}',
        '  shorthand',
        '  onChange={onChange}',
        '/>',
      ].join('\n'),
    );
  });

  it('recognizes CRLF, CR, LF, LS, and PS as multiline attribute boundaries', () => {
    for (const terminator of ['\n', '\r\n', '\r', '\u2028', '\u2029']) {
      const source = `<App a b={() => (${terminator}1)} />`;
      const reports = runRule(source, [{ multiline: 'first' }], 'fixture.jsx');
      expect(
        reports.map((report) => report.messageId),
        JSON.stringify(terminator),
      ).toEqual(['listMultilineFirst']);
      expect(fixedOutput(source, reports)).toBe(`<App b={() => (${terminator}1)} a />`);
    }
  });
});
