import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-max-props-per-line-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options) {
  const reports = [];
  const visitor = plugin.rules['jsx-max-props-per-line'].createOnce({
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
  let message = plugin.rules['jsx-max-props-per-line'].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data ?? {})) {
    message = message.replaceAll(`{{${key}}}`, value);
  }
  return message;
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
  throw new Error('jsx-max-props-per-line fixes did not converge after ten passes');
}

describe('@stylistic/jsx-max-props-per-line v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored inventory complete and reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-max-props-per-line/jsx-max-props-per-line.test.ts',
      license: 'MIT',
      parserMatrix: 'authored semantic cases; replayed with Oxc JSX/TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-max-props-per-line-tests.ts',
      inventory: {
        valid: 19,
        invalid: 22,
        diagnostics: 22,
        fixableInvalid: 22,
        unfixableInvalid: 0,
        total: 41,
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
    'replays invalid case %i with exact messages, data, attribute ranges, and first-pass output',
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
      for (const [report, error] of reports.map((report, index) => [
        report,
        testCase.errors[index],
      ])) {
        const reported = testCase.code.slice(...report.node.range);
        expect(
          reported.startsWith(error.data.prop) || reported.startsWith('{'),
          testCase.code,
        ).toBe(true);
      }
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      const converged = fixToConvergence(testCase.code, testCase.options);
      expect(runRule(converged, testCase.options), converged).toEqual([]);
    },
  );

  it('maps Unicode prefixes to UTF-16 ranges and preserves exact spread prop data', () => {
    const source =
      'const marker = "😀"; const view = <部品 xml:lang="日本語" {...props.値} final />;';
    const reports = runRule(source, [{ maximum: 1 }]);
    const spreadStart = source.indexOf('{...props.値}');
    expect(reports).toHaveLength(1);
    expect(reports[0].node.range).toEqual([spreadStart, spreadStart + '{...props.値}'.length]);
    expect(reports[0].data).toEqual({ prop: 'props.値' });
    expect(fixedOutput(source, reports)).toBe(
      'const marker = "😀"; const view = <部品 xml:lang="日本語"\n{...props.値}\nfinal />;',
    );
  });

  it('handles every ECMAScript line terminator and reports one excess prop per line', () => {
    for (const newline of ['\r\n', '\r', '\n', '\u2028', '\u2029']) {
      const source = `<Panel one two${newline}three four />`;
      const reports = runRule(source, [{ maximum: 1 }]);
      expect(
        reports.map((report) => report.data.prop),
        JSON.stringify(newline),
      ).toEqual(['two', 'four']);
      expect(fixedOutput(source, reports), JSON.stringify(newline)).toBe(
        `<Panel one\ntwo${newline}three\nfour />`,
      );
    }
  });

  it('handles nested TSX generics, malformed source, and unknown options safely', () => {
    const source =
      '<Outer one two><DataTable<Items> fullscreen keyField="id" items={items} /></Outer>';
    expect(runRule(source, [{ maximum: 1 }]).map((report) => report.data.prop)).toEqual([
      'two',
      'keyField',
    ]);
    expect(runRule('const broken = <Panel one two', [{ maximum: 1 }])).toEqual([]);
    expect(
      runRule('<Panel one two />', [{ maximum: 'many' }]).map((report) => report.data.prop),
    ).toEqual(['two']);
  });
});
