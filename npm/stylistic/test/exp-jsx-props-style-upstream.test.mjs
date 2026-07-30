import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/exp-jsx-props-style-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options, filename = 'fixture.tsx') {
  const reports = [];
  const visitor = plugin.rules['exp-jsx-props-style'].createOnce({
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
  let message = plugin.rules['exp-jsx-props-style'].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data ?? {})) {
    message = message.replaceAll(`{{${key}}}`, value);
  }
  return message;
}

function fixesFor(reports) {
  return reports
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
}

function fixedOutput(sourceText, reports) {
  const edits = fixesFor(reports);
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
    const reports = runRule(output, options);
    const next = fixedOutput(output, reports);
    if (next === null) {
      return { output, reports };
    }
    expect(next, `fix pass ${pass + 1} must make progress`).not.toBe(output);
    output = next;
  }
  throw new Error('exp-jsx-props-style fixes did not converge after ten passes');
}

describe('@stylistic/exp-jsx-props-style v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored inventory and source hashes reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      rule: 'exp-jsx-props-style',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-props-style/jsx-props-style.test.ts',
      ruleFile: 'packages/eslint-plugin/rules/jsx-props-style/jsx-props-style.ts',
      sourceSha256: '926f8805c068941c3ae2d959c180aae14da09e55d2c1eefabef45081ea0f602a',
      ruleSourceSha256: 'c3dbc6b2026f5ec5c25677d27ae9be0bd78752d691722049fd0602fd9c12a063',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-exp-jsx-props-style-tests.ts',
      capturePolicy:
        'Each authored semantic case is captured once; exact replay uses the published rule with ESLint Espree in JSX mode.',
      exactReplay: {
        eslint: '10.4.1',
        parser: 'espree bundled with ESLint',
      },
      inventory: {
        valid: 17,
        invalid: 11,
        diagnostics: 17,
        fixableDiagnostics: 15,
        unfixableDiagnostics: 2,
        fixableInvalid: 11,
        unfixableInvalid: 0,
        total: 28,
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
    'replays authored invalid case %i with exact diagnostics and outputs',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.diagnostics.map((diagnostic) => diagnostic.messageId));
      expect(
        reports.map((report) => report.data),
        testCase.code,
      ).toEqual(testCase.diagnostics.map((diagnostic) => diagnostic.data));
      expect(reports.map(renderedMessage), testCase.code).toEqual(
        testCase.diagnostics.map((diagnostic) => diagnostic.message),
      );
      expect(
        reports.map((report) => report.node.range),
        testCase.code,
      ).toEqual(testCase.diagnostics.map((diagnostic) => diagnostic.range));
      expect(
        reports.map((report) => {
          const fixes = fixesFor([report]);
          return fixes.length === 0
            ? null
            : { range: fixes[0].range, replacementText: fixes[0].replacementText };
        }),
        testCase.code,
      ).toEqual(testCase.diagnostics.map((diagnostic) => diagnostic.fix));
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);

      const recursive = fixToConvergence(testCase.code, testCase.options);
      expect(recursive.output, testCase.code).toBe(testCase.recursiveOutput);
      expect(
        recursive.reports.map((report) => ({
          messageId: report.messageId,
          message: renderedMessage(report),
          range: report.node.range,
        })),
        testCase.code,
      ).toEqual(
        testCase.recursiveDiagnostics.map((diagnostic) => ({
          messageId: diagnostic.messageId,
          message: diagnostic.message,
          range: diagnostic.range,
        })),
      );
    },
  );

  it('covers every nested option and first-prop decision', () => {
    expect(runRule('<App one two />')).toEqual([]);
    expect(
      runRule('<App one two />', [{ singleLine: { maxItems: 1 } }]).map(
        (report) => report.data.prop,
      ),
    ).toEqual(['one', 'two']);
    const grouped = '<App one two three four />';
    expect(
      fixedOutput(
        grouped,
        runRule(grouped, [{ singleLine: { maxItems: 3 }, multiLine: { maxItemsPerLine: 2 } }]),
      ),
    ).toBe('<App\none two\nthree four />');
    const collapsed = '<App\n one\n two />';
    expect(
      fixedOutput(
        collapsed,
        runRule(collapsed, [{ multiLine: { minItems: 3, maxItemsPerLine: 2 } }]),
      ),
    ).toBe('<App one two />');
  });

  it('maps Unicode to UTF-16 ranges and preserves namespaced and spread prop data', () => {
    const source =
      'const marker = "😀"; const view = <部品 xml:lang="日本語" {...props.値} final />;';
    const reports = runRule(source, [{ singleLine: { maxItems: 1 } }]);
    const spreadStart = source.indexOf('{...props.値}');
    expect(reports.map((report) => report.data.prop)).toEqual(['xml:lang', 'props.値', 'final']);
    expect(reports[1].node.range).toEqual([spreadStart, spreadStart + '{...props.値}'.length]);
    expect(fixedOutput(source, reports)).toBe(
      'const marker = "😀"; const view = <部品\nxml:lang="日本語"\n{...props.値}\nfinal />;',
    );
  });

  it('handles CRLF, CR, LF, U+2028, and U+2029 exactly', () => {
    for (const newline of ['\r\n', '\r', '\n', '\u2028', '\u2029']) {
      const wrapped = `<Panel${newline}one two />`;
      expect(fixedOutput(wrapped, runRule(wrapped))).toBe(`<Panel${newline}one\ntwo />`);
      const collapsed = `<Panel${newline}one${newline}two />`;
      expect(fixedOutput(collapsed, runRule(collapsed, [{ multiLine: { minItems: 3 } }]))).toBe(
        '<Panel one two />',
      );
    }
  });

  it('preserves comments, parses nested TSX generics, sorts reports, and rejects malformed input', () => {
    const commented = '<App foo /* keep */ bar baz />';
    const commentReports = runRule(commented, [{ singleLine: { maxItems: 1 } }]);
    expect(commentReports.map((report) => report.suggest?.length ?? 0)).toEqual([1, 0, 1]);
    expect(fixedOutput(commented, commentReports)).toBe('<App\nfoo /* keep */ bar\nbaz />');

    const nested = '<Outer first child={<DataTable<Row> one two />} third\nfourth />';
    const nestedReports = runRule(nested, [{ singleLine: { maxItems: 1 } }]);
    expect(nestedReports.map((report) => report.data.prop)).toEqual(['one', 'two', 'fourth']);
    expect(nestedReports.map((report) => report.node.range[0])).toEqual(
      [...nestedReports].map((report) => report.node.range[0]).sort((left, right) => left - right),
    );

    expect(runRule('const broken = <Panel one two')).toEqual([]);
    expect(runRule('const comparison = left < right > value;', [], 'fixture.js')).toEqual([]);
  });
});
