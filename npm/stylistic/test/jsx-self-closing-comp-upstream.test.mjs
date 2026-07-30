import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE = 'jsx-self-closing-comp';
const MESSAGE = 'Empty components are self-closing';
const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-self-closing-comp-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options, { filename = 'fixture.tsx', settings } = {}) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[RULE].createOnce({
    filename,
    options: options ?? [],
    settings,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function resolveMessage(report) {
  const template = plugin.rules[RULE].meta.messages[report.messageId];
  return template.replace(/\{\{([^}]+)\}\}/gu, (_match, key) => report.data?.[key] ?? '');
}

function locationAt(source, offset) {
  let line = 1;
  let lineStart = 0;
  for (let index = 0; index < offset; index += 1) {
    const character = source[index];
    if (character === '\r') {
      if (source[index + 1] === '\n') {
        index += 1;
      }
      line += 1;
      lineStart = index + 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line += 1;
      lineStart = index + 1;
    }
  }
  return { line, column: offset - lineStart + 1 };
}

function reportFix(report) {
  const suggestion = report.suggest?.[0];
  if (!suggestion) {
    return null;
  }
  return suggestion.fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  })[0];
}

function normalizeReport(source, report) {
  const start = locationAt(source, report.node.range[0]);
  const end = locationAt(source, report.node.range[1]);
  return {
    messageId: report.messageId,
    message: resolveMessage(report),
    data: report.data ?? {},
    range: report.node.range,
    location: {
      line: start.line,
      column: start.column,
      endLine: end.line,
      endColumn: end.column,
    },
    fix: reportFix(report),
  };
}

function applyReportFixes(source, reports) {
  const fixes = reports
    .map(reportFix)
    .filter(Boolean)
    .sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  if (fixes.length === 0) {
    return null;
  }

  let output = source;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

describe('jsx-self-closing-comp upstream v5.10.0 parity', () => {
  it('pins the complete authored parser matrix and upstream source metadata', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-self-closing-comp/jsx-self-closing-comp.test.ts',
      ruleFile: 'packages/eslint-plugin/rules/jsx-self-closing-comp/jsx-self-closing-comp.ts',
      typesFile: 'packages/eslint-plugin/rules/jsx-self-closing-comp/types.d.ts',
      parserMatrixFile: 'shared/test-utils/parsers-jsx.ts',
      sourceSha256: '405bea51f11b694a1a4b0ca39af48d5f40c05deeec99aacde232bebc58f6fdef',
      ruleSourceSha256: '8831d20a9dd6f5f61fabe5c0af76c367d31877df2ef8cd807a5e0cd3e10732cf',
      typesSourceSha256: '749358dd39e345c9d80c51671de01ce0e238779731419c9ed579a6e0d21c374b',
      parserMatrixSourceSha256: '64dd12d67eac1eadf8a5a93de02bbb76c1d764c0ec7ebbdaae0c45389b52435c',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-self-closing-comp-tests.ts',
      parserMatrix: ['default', '@babel/eslint-parser', '@typescript-eslint/parser'],
      inventory: {
        logicalValid: 35,
        logicalInvalid: 12,
        valid: 105,
        invalid: 36,
        diagnostics: 36,
        fixableInvalid: 36,
        unfixableInvalid: 0,
        total: 141,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts expanded upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays exact diagnostic, range, fix, and output for invalid case %i',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(reports.map((report) => normalizeReport(testCase.code, report))).toEqual(
        testCase.diagnostics,
      );
      expect(applyReportFixes(testCase.code, reports)).toBe(testCase.output);
    },
  );

  it('deep-merges component and html defaults for direct and shared settings', () => {
    const source = '<Widget></Widget>;<div></div>;';
    expect(runRule(source).map((report) => report.node.range)).toEqual([
      [0, 8],
      [18, 23],
    ]);
    expect(runRule(source, [{ component: false }]).map((report) => report.node.range)).toEqual([
      [18, 23],
    ]);
    expect(runRule(source, [{ html: false }]).map((report) => report.node.range)).toEqual([[0, 8]]);
    expect(runRule(source, [{ component: false, html: false }])).toEqual([]);
    expect(
      runRule(source, [], {
        settings: {
          corsaStylistic: {
            rules: {
              [RULE]: [{ component: false }],
            },
          },
        },
      }).map((report) => report.node.range),
    ).toEqual([[18, 23]]);
  });

  it('maps Unicode native byte ranges to UTF-16 and applies the exact code fix', () => {
    const source = 'const marker = "😀"; const 日本語 = <Widget<string> data="値"></Widget>;';
    const reports = runRule(source);
    const openingStart = source.indexOf('<Widget');
    const openingEnd = source.indexOf('></Widget>') + 1;
    const closingEnd = source.indexOf('</Widget>') + '</Widget>'.length;

    expect(reports).toHaveLength(1);
    expect(reports[0]).toMatchObject({
      messageId: 'notSelfClosing',
      node: { range: [openingStart, openingEnd] },
    });
    expect(resolveMessage(reports[0])).toBe(MESSAGE);
    expect(reportFix(reports[0])).toEqual({
      range: [openingEnd - 1, closingEnd],
      replacementText: ' />',
    });
    expect(applyReportFixes(source, reports)).toBe(
      'const marker = "😀"; const 日本語 = <Widget<string> data="値" />;',
    );
  });

  it('matches LF, CRLF, CR, LS, PS, NBSP, comment, and same-line-space boundaries', () => {
    for (const source of [
      '<Widget>\n</Widget>',
      '<Widget>\r\n</Widget>',
      '<Widget>\t\u1680\u2007\u202f\n\u3000</Widget>',
    ]) {
      expect(runRule(source), source).toHaveLength(1);
    }
    for (const source of [
      '<Widget> </Widget>',
      '<Widget>\r</Widget>',
      '<Widget>\u2028</Widget>',
      '<Widget>\u2029</Widget>',
      '<Widget>\n\u00a0</Widget>',
      '<Widget>&nbsp;</Widget>',
      '<Widget>{/* keep */}</Widget>',
    ]) {
      expect(runRule(source), source).toEqual([]);
    }
  });

  it('handles fragments, namespaces, member names, nested traversal, TSX, and invalid syntax', () => {
    expect(runRule('<></>', [], { filename: 'fixture.jsx' })).toEqual([]);
    expect(runRule('<><Widget></Widget></>', [], { filename: 'fixture.jsx' })).toHaveLength(1);
    expect(runRule('<foo:bar></foo:bar>', [], { filename: 'fixture.jsx' })).toHaveLength(1);
    expect(runRule('<Foo:bar></Foo:bar>', [], { filename: 'fixture.jsx' })).toEqual([]);
    expect(runRule('<foo.Part></foo.Part>', [], { filename: 'fixture.jsx' })).toHaveLength(1);
    expect(runRule('<Foo.Part></Foo.Part>', [], { filename: 'fixture.jsx' })).toHaveLength(1);
    expect(runRule('<this.Part></this.Part>', [], { filename: 'fixture.jsx' })).toHaveLength(1);
    expect(
      runRule('const value = <Widget<Props>></Widget>;', [], { filename: 'fixture.tsx' }),
    ).toHaveLength(1);
    expect(runRule('<Widget>', [], { filename: 'fixture.tsx' })).toEqual([]);
  });

  it('keeps option normalization total for malformed native-facing payloads', () => {
    for (const options of [null, [], [null], [{ component: 'bad', html: 1 }]]) {
      expect(runRule('<Widget></Widget>', options)).toHaveLength(1);
    }
    for (const options of [['bad'], [0], [{ component: null, html: false }]]) {
      expect(runRule('<Widget></Widget>', options)).toEqual([]);
    }
  });
});
