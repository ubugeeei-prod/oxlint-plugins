import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-wrap-multilines-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options, settings) {
  const reports = [];
  const visitor = plugin.rules['jsx-wrap-multilines'].createOnce({
    filename: 'fixture.tsx',
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
  return plugin.rules['jsx-wrap-multilines'].meta.messages[report.messageId];
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

function fixToConvergence(sourceText, options) {
  let output = sourceText;
  for (let pass = 0; pass < 10; pass += 1) {
    const next = fixedOutput(output, runRule(output, options));
    if (next === null) {
      return output;
    }
    expect(next, `fix pass ${pass + 1} must make progress`).not.toBe(output);
    output = next;
  }
  throw new Error('jsx-wrap-multilines fixes did not converge after ten passes');
}

describe('@stylistic/jsx-wrap-multilines v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored inventory complete and reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-wrap-multilines/jsx-wrap-multilines.test.ts',
      license: 'MIT',
      parserMatrix: 'authored semantic cases; replayed with Oxc JSX/TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-wrap-multilines-tests.ts',
      inventory: {
        valid: 71,
        invalid: 75,
        diagnostics: 93,
        fixableInvalid: 75,
        unfixableInvalid: 0,
        total: 146,
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
    'replays invalid case %i with exact messages, JSX ranges, fixes, and recursive output',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(reports.map(renderedMessage), testCase.code).toEqual(
        testCase.errors.map((error) => error.message),
      );
      for (const report of reports) {
        const reported = testCase.code.slice(...report.node.range);
        expect(reported.startsWith('<'), testCase.code).toBe(true);
        expect(reported.endsWith('>'), testCase.code).toBe(true);
      }
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      const converged = fixToConvergence(testCase.code, testCase.options);
      expect(runRule(converged, testCase.options), converged).toEqual([]);
    },
  );

  it('maps Unicode prefixes to exact UTF-16 JSX and operator fix ranges', () => {
    const source =
      'const marker = "😀"; const 印 = ready &&\n  <部品<Item>>日本語\n    <子 />\n  </部品>;';
    const reports = runRule(source, [{ logical: 'parens-new-line' }]);
    const jsxStart = source.indexOf('<部品');
    const jsxEnd = source.indexOf('</部品>') + '</部品>'.length;
    const operatorStart = source.indexOf('&&');
    expect(reports).toHaveLength(1);
    expect(reports[0]).toMatchObject({
      messageId: 'missingParens',
      node: { range: [jsxStart, jsxEnd] },
    });
    expect(
      reports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [operatorStart, jsxEnd],
        replacementText: '&& (\n  <部品<Item>>日本語\n    <子 />\n  </部品>\n)',
      },
    ]);
  });

  it('covers all owner modes, booleans, ignores, line terminators, comments, and TSX', () => {
    const options = [
      {
        declaration: 'parens-new-line',
        assignment: 'parens-new-line',
        return: 'parens-new-line',
        arrow: 'parens-new-line',
        condition: 'parens-new-line',
        logical: 'parens-new-line',
        prop: 'parens-new-line',
        propertyValue: 'parens-new-line',
      },
    ];
    for (const newline of ['\r\n', '\r', '\n', '\u2028', '\u2029']) {
      const source = `const view = <Panel<Item>>one${newline}<Child /></Panel>;`;
      expect(runRule(source, options).map((report) => report.messageId)).toEqual(['missingParens']);
    }

    const commentSource =
      'const Component = () =>\n  // 説明 😀\n  <Panel<Item>>\n    <Child />\n  </Panel>;';
    expect(fixToConvergence(commentSource, options)).toBe(
      'const Component = () => (\n  // 説明 😀\n  <Panel<Item>>\n    <Child />\n  </Panel>\n);',
    );
    expect(
      runRule('const value = <Panel>\n  <Child />\n</Panel>;', [{ declaration: false }]),
    ).toEqual([]);
    expect(
      runRule('const value = <Panel>\n  <Child />\n</Panel>;', [{ declaration: 'ignore' }]),
    ).toEqual([]);
  });

  it('honors the same configuration through shared stylistic settings', () => {
    const source = 'const value = <Panel>\n  <Child />\n</Panel>;';
    const reports = runRule(source, [], {
      corsaStylistic: {
        rules: {
          'jsx-wrap-multilines': [{ declaration: 'parens-new-line' }],
        },
      },
    });
    expect(reports.map((report) => report.messageId)).toEqual(['missingParens']);
    expect(fixedOutput(source, reports)).toBe(
      'const value = (\n<Panel>\n  <Child />\n</Panel>\n);',
    );
  });

  it('defaults malformed options and invalid source without throwing', () => {
    expect(() =>
      runRule('const value = <Panel>\n  <Child />\n</Panel>;', [
        {
          declaration: 'unknown',
          assignment: 1,
          return: null,
          arrow: [],
          condition: {},
          logical: 'unknown',
          prop: 1,
          propertyValue: null,
        },
      ]),
    ).not.toThrow();
    expect(runRule('const broken = <Panel>', [])).toEqual([]);
  });
});
