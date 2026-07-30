import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-indent-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options = [], filename = 'fixture.tsx', settings) {
  const reports = [];
  const visitor = plugin.rules['jsx-indent'].createOnce({
    options,
    filename,
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

function recursivelyFix(source, options, filename = 'fixture.tsx') {
  let output = source;
  for (let pass = 0; pass < 10; pass += 1) {
    const next = applyOnePass(output, runRule(output, options, filename));
    if (next === output) {
      return output;
    }
    output = next;
  }
  return output;
}

describe('@stylistic/jsx-indent v5.10.0 exhaustive upstream parity', () => {
  it('pins the exact commit, source hashes, replay toolchain, and complete authored inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-indent/jsx-indent.test.ts',
      ruleFile: 'packages/eslint-plugin/rules/jsx-indent/jsx-indent.ts',
      parserMatrixFile: 'shared/test-utils/parsers-jsx.ts',
      sourceSha256: '0469c5c8ae40e881bfb21abecaf5c08813955fefc99608526fc460b3fc16fbf2',
      ruleSourceSha256: '845ae761d471cfb80dde5745e18b41554c6e5f810d74abd129e88ba18af0885f',
      parserMatrixSourceSha256: '64dd12d67eac1eadf8a5a93de02bbb76c1d764c0ec7ebbdaae0c45389b52435c',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-indent-tests.ts',
      capturePolicy:
        'Each authored semantic case is captured once; compatible cases are exactly replayed through the published rule.',
      exactReplay: {
        eslint: '9.39.2',
        typescriptEslintParser: '8.60.0',
        babelEslintParser: '7.28.6',
      },
      inventory: {
        valid: 106,
        invalid: 65,
        diagnostics: 84,
        fixableInvalid: 65,
        unfixableInvalid: 0,
        total: 171,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts authored valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays invalid case %i with exact reports, fixes, and recursive output',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(reports, testCase.code).toHaveLength(testCase.diagnostics.length);
      for (const [diagnosticIndex, [report, diagnostic]] of reports
        .map((report, index) => [report, testCase.diagnostics[index]])
        .entries()) {
        const label = `${diagnosticIndex}\n${testCase.code}`;
        expect(report.messageId, label).toBe(diagnostic.messageId);
        expect(report.data, label).toEqual(
          Object.fromEntries(
            Object.entries(diagnostic.data).map(([key, value]) => [key, String(value)]),
          ),
        );
        expect(
          `Expected indentation of ${report.data.needed} ${report.data.type} ${report.data.characters} but found ${report.data.gotten}.`,
          label,
        ).toBe(diagnostic.message);
        expect(report.node.range, label).toEqual(diagnostic.range);
        expect(reportFix(report), label).toEqual(
          diagnostic.fix
            ? {
                range: diagnostic.fix.range,
                replacementText: diagnostic.fix.replacementText,
              }
            : null,
        );
      }
      expect(applyOnePass(testCase.code, reports), testCase.code).toBe(
        testCase.output ?? testCase.code,
      );
      expect(recursivelyFix(testCase.code, testCase.options), testCase.code).toBe(
        testCase.recursiveOutput,
      );
    },
  );

  it('maps Unicode byte ranges to UTF-16 and preserves TSX syntax', () => {
    const source =
      "const emoji: string = '😀';\nconst view = (\n  <Panel<T>>\n  <子 />\n  </Panel>\n);";
    const reports = runRule(source, [2]);
    const childStart = source.indexOf('<子');
    expect(
      reports.map((report) => ({
        messageId: report.messageId,
        data: report.data,
        range: report.node.range,
        fix: reportFix(report),
      })),
    ).toEqual([
      {
        messageId: 'wrongIndent',
        data: {
          needed: '4',
          type: 'space',
          characters: 'characters',
          gotten: '2',
        },
        range: [childStart, childStart + '<子 />'.length],
        fix: {
          range: [source.lastIndexOf('\n', childStart) + 1, childStart],
          replacementText: '    ',
        },
      },
    ]);
  });

  it('covers CRLF, CR, LF, LS, PS, tabs, comments, attributes, and shared settings', () => {
    for (const terminator of ['\r\n', '\r', '\n', '\u2028', '\u2029']) {
      const source = `<App>${terminator}<Child />${terminator}  </App>`;
      const reports = runRule(source, [0], 'fixture.jsx', {
        stylistic: { lane: 'jsx-indent' },
      });
      expect(reports, JSON.stringify(terminator)).toHaveLength(1);
      expect(reportFix(reports[0])).toEqual({
        range: [source.lastIndexOf(terminator) + terminator.length, source.indexOf('</App>')],
        replacementText: '',
      });
    }

    expect(runRule('<App>\n\t{/* comment */}\n\t<Child />\n</App>', ['tab'])).toEqual([]);
    const attributeSource =
      'const Component = () => (\n  <View\n    value={(\n      <Child />\n)}\n  />\n);';
    expect(runRule(attributeSource, [2, { checkAttributes: false }])).toEqual([]);
    expect(runRule(attributeSource, [2, { checkAttributes: true }])).not.toEqual([]);
  });

  it('fails safely for malformed input and non-JSX source types', () => {
    expect(runRule('<App><Broken></App>')).toEqual([]);
    expect(runRule('<App>\n<Child />\n</App>', [2], 'fixture.ts')).toEqual([]);
  });
});
