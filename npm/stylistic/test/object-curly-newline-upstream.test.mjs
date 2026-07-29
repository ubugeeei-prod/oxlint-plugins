import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'object-curly-newline.json'), 'utf8'),
);
const rule = plugin.rules['object-curly-newline'];

function runRule(sourceText, options = [], language = 'typescript') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    options,
    filename: language === 'flow' || language === 'javascript' ? 'fixture.js' : 'fixture.tsx',
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function actualDiagnostic(sourceText, report) {
  const [start, end] = report.node.range;
  const startPosition = positionAt(sourceText, start);
  const endPosition = positionAt(sourceText, end);
  return {
    messageId: report.messageId,
    message: rule.meta.messages[report.messageId],
    data: report.data ?? {},
    range: [start, end],
    loc: {
      line: startPosition.line,
      column: startPosition.column,
      endLine: endPosition.line,
      endColumn: endPosition.column,
    },
  };
}

function positionAt(sourceText, offset) {
  let line = 1;
  let column = 1;
  for (let index = 0; index < offset; index++) {
    const character = sourceText[index];
    if (character === '\r') {
      if (sourceText[index + 1] === '\n' && index + 1 < offset) {
        index++;
      }
      line++;
      column = 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line++;
      column = 1;
    } else {
      column++;
    }
  }
  return { line, column };
}

function fixesForReport(report) {
  return (
    report.suggest?.flatMap((suggestion) =>
      suggestion.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ) ?? []
  );
}

function fixedOutput(sourceText, reports) {
  const fixes = reports.flatMap(fixesForReport);
  if (fixes.length === 0) {
    return null;
  }

  fixes.sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  const accepted = [];
  let lastEnd = Number.NEGATIVE_INFINITY;
  for (const fix of fixes) {
    if (lastEnd >= fix.range[0]) {
      continue;
    }
    accepted.push(fix);
    lastEnd = fix.range[1];
  }

  let output = sourceText;
  for (const fix of accepted.reverse()) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

function expectFixConvergence(sourceText, options, language) {
  let output = sourceText;
  for (let pass = 0; pass < 8; pass++) {
    const reports = runRule(output, options, language);
    if (reports.length === 0) {
      return;
    }
    const next = fixedOutput(output, reports);
    if (next === null) {
      expect(
        reports.every((report) => fixesForReport(report).length === 0),
        output,
      ).toBe(true);
      return;
    }
    expect(next, output).not.toBe(output);
    output = next;
  }
  throw new Error(`object-curly-newline fixes did not converge:\n${output}`);
}

describe('@stylistic/object-curly-newline v5.10.0 exhaustive upstream replay', () => {
  it('is generated from every audited stable suite without dropped cases', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/object-curly-newline/object-curly-newline._js_.test.ts',
        'packages/eslint-plugin/rules/object-curly-newline/object-curly-newline._js_.test.ts',
        'packages/eslint-plugin/rules/object-curly-newline/object-curly-newline._ts_.test.ts',
      ],
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-object-curly-newline-tests.ts',
      inventory: {
        valid: 256,
        invalid: 223,
        diagnostics: 387,
        unfixableInvalid: 3,
        total: 479,
        fixableInvalid: 220,
      },
    });
    expect(
      fixture.suites.map((suite) => ({
        language: suite.language,
        valid: suite.valid.length,
        invalid: suite.invalid.length,
        diagnostics: suite.invalid.reduce(
          (total, testCase) => total + testCase.expectedDiagnostics.length,
          0,
        ),
      })),
    ).toEqual([
      { language: 'javascript', valid: 66, invalid: 70, diagnostics: 122 },
      { language: 'flow', valid: 4, invalid: 4, diagnostics: 8 },
      { language: 'typescript', valid: 186, invalid: 149, diagnostics: 257 },
    ]);

    const messageDistribution = {};
    for (const suite of fixture.suites) {
      for (const testCase of suite.invalid) {
        for (const diagnostic of testCase.expectedDiagnostics) {
          messageDistribution[diagnostic.messageId] =
            (messageDistribution[diagnostic.messageId] ?? 0) + 1;
        }
      }
    }
    expect(messageDistribution).toEqual({
      unexpectedLinebreakBeforeClosingBrace: 103,
      expectedLinebreakAfterOpeningBrace: 93,
      expectedLinebreakBeforeClosingBrace: 92,
      unexpectedLinebreakAfterOpeningBrace: 99,
    });
  });

  for (const suite of fixture.suites) {
    it.each(suite.valid)(`${suite.language} accepts every upstream valid case %#`, (testCase) => {
      expect(runRule(testCase.code, testCase.options ?? [], suite.language), testCase.code).toEqual(
        [],
      );
    });

    it.each(suite.invalid)(
      `${suite.language} replays every upstream invalid case %# exactly`,
      (testCase) => {
        const reports = runRule(testCase.code, testCase.options ?? [], suite.language);
        expect(
          reports.map((report) => actualDiagnostic(testCase.code, report)),
          testCase.code,
        ).toEqual(testCase.expectedDiagnostics);
        expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);

        if (testCase.output === null) {
          expect(reports.every((report) => fixesForReport(report).length === 0)).toBe(true);
        } else {
          expectFixConvergence(testCase.output, testCase.options ?? [], suite.language);
        }
      },
    );
  }
});
