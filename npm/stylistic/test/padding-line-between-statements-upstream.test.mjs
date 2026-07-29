import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/padding-line-between-statements-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function filename(testCase) {
  if (testCase.language === 'typescript') {
    return 'fixture.ts';
  }
  return testCase.parserOptions?.ecmaFeatures?.jsx ? 'fixture.jsx' : 'fixture.js';
}

function runRule(testCase, sourceText = testCase.code) {
  const reports = [];
  const visitor = plugin.rules['padding-line-between-statements'].createOnce({
    filename: filename(testCase),
    options: testCase.options,
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

function offsetForLocation(sourceText, line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line && offset < sourceText.length) {
    if (sourceText[offset] === '\r') {
      offset += sourceText[offset + 1] === '\n' ? 2 : 1;
      currentLine += 1;
    } else if (
      sourceText[offset] === '\n' ||
      sourceText[offset] === '\u2028' ||
      sourceText[offset] === '\u2029'
    ) {
      offset += 1;
      currentLine += 1;
    } else {
      offset += 1;
    }
  }
  return offset + column - 1;
}

function expectedRange(sourceText, error) {
  return [
    offsetForLocation(sourceText, error.line, error.column),
    offsetForLocation(sourceText, error.endLine, error.endColumn),
  ];
}

function editsForReports(reports) {
  return reports.flatMap(
    (report) =>
      report.suggest?.flatMap((suggestion) =>
        suggestion.fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ) ?? [],
  );
}

function applyReports(sourceText, reports) {
  const edits = editsForReports(reports).sort(
    (left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1],
  );
  if (edits.length === 0) {
    return null;
  }
  let output = sourceText;
  for (const edit of edits) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

function converge(testCase) {
  let output = testCase.code;
  let changed = false;
  for (let pass = 0; pass < 10; pass += 1) {
    const next = applyReports(output, runRule(testCase, output));
    if (next === null || next === output) {
      break;
    }
    output = next;
    changed = true;
  }
  return changed ? output : null;
}

describe('@stylistic/padding-line-between-statements v5.10.0 upstream parity', () => {
  it('pins the complete JavaScript and TypeScript inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/padding-line-between-statements/padding-line-between-statements._js_.test.ts',
        'packages/eslint-plugin/rules/padding-line-between-statements/padding-line-between-statements._ts_.test.ts',
      ],
      license: 'MIT',
      eslintVersion: '10.4.1',
      typescriptEslintParserVersion: '8.60.0',
      tool: 'tools/tasks/sync-stylistic-padding-line-between-statements-tests.ts',
      inventory: {
        valid: 419,
        invalid: 323,
        diagnostics: 339,
        fixableInvalid: 323,
        unfixableInvalid: 0,
        total: 742,
      },
    });
    expect(fixture.valid.filter((testCase) => testCase.language === 'javascript')).toHaveLength(
      391,
    );
    expect(fixture.valid.filter((testCase) => testCase.language === 'typescript')).toHaveLength(28);
    expect(fixture.invalid.filter((testCase) => testCase.language === 'javascript')).toHaveLength(
      300,
    );
    expect(fixture.invalid.filter((testCase) => testCase.language === 'typescript')).toHaveLength(
      23,
    );
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays invalid case %i with exact order, locations, fixes, and convergence',
    (_index, testCase) => {
      const reports = runRule(testCase);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(
        reports.map((report) => report.node.range),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => expectedRange(testCase.code, error)));
      expect(
        reports.map((report) => {
          const edits = editsForReports([report]);
          return edits.length === 0
            ? null
            : { range: edits[0].range, text: edits[0].replacementText };
        }),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.fix));
      expect(converge(testCase), testCase.code).toBe(testCase.output);
      expect(runRule(testCase, testCase.output), testCase.output).toEqual([]);
    },
  );

  it('keeps comment-separated multiple padding sequences deliberately unfixable', () => {
    const testCase = {
      code: 'foo();\n\n// one\n\nbar();',
      language: 'javascript',
      options: [{ blankLine: 'never', prev: '*', next: '*' }],
    };
    const reports = runRule(testCase);
    expect(reports.map((report) => report.messageId)).toEqual(['unexpectedBlankLine']);
    expect(reports[0].suggest).toBeUndefined();
  });
});
