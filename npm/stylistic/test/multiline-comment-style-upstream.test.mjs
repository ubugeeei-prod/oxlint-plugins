import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const fixture = JSON.parse(
  readFileSync(join(packageRoot, 'test/fixtures/multiline-comment-style.json'), 'utf8'),
);
const rule = plugin.rules['multiline-comment-style'];

describe('@stylistic/multiline-comment-style v5.10.0 upstream compatibility', () => {
  it('pins the complete upstream inventory and exact diagnostic totals', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      inventory: {
        valid: 55,
        invalid: 69,
        total: 124,
        diagnostics: 107,
        fixableInvalid: 66,
        unfixableInvalid: 3,
      },
    });
    expect(fixture.valid).toHaveLength(fixture.__generated.inventory.valid);
    expect(fixture.invalid).toHaveLength(fixture.__generated.inventory.invalid);
    expect(
      fixture.invalid.reduce((count, testCase) => count + testCase.expectedDiagnostics.length, 0),
    ).toBe(fixture.__generated.inventory.diagnostics);
  });

  for (const [index, testCase] of fixture.valid.entries()) {
    it(`accepts upstream valid case ${index + 1}/${fixture.valid.length}`, () => {
      expect(runRule(testCase.code, testCase.options ?? [])).toEqual([]);
    });
  }

  for (const [index, testCase] of fixture.invalid.entries()) {
    it(`replays upstream invalid case ${index + 1}/${fixture.invalid.length}`, () => {
      const reports = runRule(testCase.code, testCase.options ?? []);
      const actualDiagnostics = reports.map((report) => normalizeReport(report, testCase.code));

      expect(actualDiagnostics).toEqual(testCase.expectedDiagnostics);
      expect(actualDiagnostics.map(({ messageId }) => messageId)).toEqual(
        testCase.errors.map(({ messageId }) => messageId),
      );

      const firstPass = applyFixPass(testCase.code, reports);
      if (testCase.output === null) {
        expect(firstPass).toBeNull();
        expect(reports.every((report) => report.suggest === undefined)).toBe(true);
        return;
      }

      expect(firstPass).toBe(testCase.output);
      const convergence = applyIteratively(testCase.code, testCase.options ?? []);
      expect(convergence.output).toBe(testCase.output);
      expect(convergence.remainingReports).toEqual([]);
      expect(convergence.passes).toBeGreaterThanOrEqual(1);
      expect(convergence.passes).toBeLessThanOrEqual(8);
    });
  }
});

function runRule(sourceText, options) {
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
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function normalizeReport(report, sourceText) {
  const [start, end] = report.node.range;
  return {
    messageId: report.messageId,
    message: rule.meta.messages[report.messageId],
    data: report.data ?? {},
    range: [start, end],
    loc: rangeLocation(sourceText, start, end),
  };
}

function rangeLocation(sourceText, start, end) {
  const startPosition = positionAt(sourceText, start);
  const endPosition = positionAt(sourceText, end);
  return {
    line: startPosition.line,
    column: startPosition.column,
    endLine: endPosition.line,
    endColumn: endPosition.column,
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
