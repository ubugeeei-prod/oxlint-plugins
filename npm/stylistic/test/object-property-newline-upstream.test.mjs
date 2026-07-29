import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = JSON.parse(
  readFileSync(join(packageRoot, 'test/fixtures/object-property-newline-v5.10.0.json'), 'utf8'),
);
const rule = plugin.rules['object-property-newline'];

describe('@stylistic/object-property-newline v5.10.0 upstream compatibility', () => {
  it('pins both complete upstream suites and their exact inventory', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/object-property-newline/object-property-newline._js_.test.ts',
        'packages/eslint-plugin/rules/object-property-newline/object-property-newline._ts_.test.ts',
      ],
      inventory: {
        valid: 49,
        invalid: 52,
        diagnostics: 62,
        total: 101,
        fixableInvalid: 51,
        unfixableInvalid: 1,
      },
    });
    expect(fixture.suites.map((suite) => suite.language)).toEqual(['javascript', 'typescript']);
    expect(fixture.suites.reduce((total, suite) => total + suite.valid.length, 0)).toBe(
      fixture.__generated.inventory.valid,
    );
    expect(fixture.suites.reduce((total, suite) => total + suite.invalid.length, 0)).toBe(
      fixture.__generated.inventory.invalid,
    );
    expect(
      fixture.suites.reduce(
        (total, suite) =>
          total +
          suite.invalid.reduce(
            (suiteTotal, testCase) => suiteTotal + testCase.expectedDiagnostics.length,
            0,
          ),
        0,
      ),
    ).toBe(fixture.__generated.inventory.diagnostics);
  });

  for (const suite of fixture.suites) {
    for (const [index, testCase] of suite.valid.entries()) {
      it(`accepts ${suite.language} valid case ${index + 1}/${suite.valid.length}`, () => {
        expect(runRule(testCase.code, testCase.options), testCase.code).toEqual([]);
      });
    }

    for (const [index, testCase] of suite.invalid.entries()) {
      it(`replays ${suite.language} invalid case ${index + 1}/${suite.invalid.length}`, () => {
        const reports = runRule(testCase.code, testCase.options);
        const actualDiagnostics = reports.map((report) => exactDiagnostic(report, testCase.code));

        expect(actualDiagnostics, testCase.code).toEqual(testCase.expectedDiagnostics);
        expect(
          actualDiagnostics.map(({ messageId }) => messageId),
          testCase.code,
        ).toEqual(testCase.upstreamErrors.map(({ messageId }) => messageId));

        const firstPass = applyFixPass(testCase.code, reports);
        if (testCase.output === null) {
          expect(firstPass, testCase.code).toBeNull();
          expect(
            reports.every((report) => report.suggest === undefined),
            testCase.code,
          ).toBe(true);
          return;
        }

        expect(firstPass, testCase.code).toBe(testCase.output);
        const convergence = applyIteratively(testCase.code, testCase.options);
        expect(convergence.output, testCase.code).toBe(testCase.output);
        expect(convergence.remainingReports, testCase.code).toEqual([]);
        expect(convergence.passes, testCase.code).toBeGreaterThanOrEqual(1);
        expect(convergence.passes, testCase.code).toBeLessThanOrEqual(8);
      });
    }
  }
});

function runRule(sourceText, options = []) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    options,
    sourceCode,
    filename: 'fixture.tsx',
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function exactDiagnostic(report, sourceText) {
  const [start, end] = report.node.range;
  const fix = report.suggest?.[0]
    ? report.suggest[0].fix({
        replaceTextRange(range, text) {
          return { range, text };
        },
      })[0]
    : null;
  return {
    messageId: report.messageId,
    message: rule.meta.messages[report.messageId],
    data: report.data ?? {},
    range: [start, end],
    loc: {
      ...positionAt(sourceText, start),
      endLine: positionAt(sourceText, end).line,
      endColumn: positionAt(sourceText, end).column,
    },
    fix,
  };
}

function positionAt(sourceText, offset) {
  let line = 1;
  let column = 1;
  for (let index = 0; index < offset; index += 1) {
    const character = sourceText[index];
    if (character === '\r') {
      if (sourceText[index + 1] === '\n') {
        index += 1;
      }
      line += 1;
      column = 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
  }
  return { line, column };
}

function applyIteratively(sourceText, options) {
  let output = sourceText;
  let passes = 0;
  for (; passes < 8; passes += 1) {
    const reports = runRule(output, options);
    const next = applyFixPass(output, reports);
    if (next === null) {
      return { output, passes, remainingReports: reports };
    }
    output = next;
  }
  throw new Error(`Fixes did not converge after 8 passes for ${JSON.stringify(sourceText)}`);
}

function applyFixPass(sourceText, reports) {
  const fixes = reports
    .flatMap((report) =>
      (report.suggest ?? []).flatMap((suggestion) =>
        suggestion.fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    )
    .sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  if (fixes.length === 0) {
    return null;
  }

  const accepted = [];
  let lastEnd = -1;
  for (const fix of fixes) {
    if (fix.range[0] < lastEnd) {
      continue;
    }
    accepted.push(fix);
    lastEnd = fix.range[1];
  }

  let output = sourceText;
  for (const fix of accepted.toReversed()) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}
