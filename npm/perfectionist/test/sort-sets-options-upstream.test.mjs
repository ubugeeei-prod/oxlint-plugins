import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/sort-sets-options-v5.10.0.json', import.meta.url), 'utf8'),
);
const fixtureSource = readFileSync(
  new URL('./fixtures/sort-sets-options-v5.10.0.json', import.meta.url),
);
const rule = plugin.rules['sort-sets'];

describe('perfectionist/sort-sets v5.10.0 option parity', () => {
  it('pins every non-React authored upstream case and its exact fixture inventory', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-perfectionist',
      version: '5.10.0',
      sourceCommit: '84aa039c46522f82a61ad43cf676afc92dd64704',
      sourceHashes: {
        'rules/sort-sets.ts': 'd069d550677d062a5c1d488a5a38e426bdf76ac681a3e2598e99d09016ad21fe',
        'rules/sort-sets/types.ts':
          '927bfce114499a6e224415245e952a6eaa43c052dfd78b827bd7d8d085e7a098',
        'rules/sort-arrays/types.ts':
          '9aa7faafdb1f2262aa32798623ebd6507b5a80f2ad6fa66446d66bb722bba582',
        'rules/sort-arrays/sort-array.ts':
          'c65199dd2f5b56ae5302368f3e4fda46849f98641415bd77cf8d56a36ab0d60a',
        'test/rules/sort-sets.test.ts':
          '44b4daf3d881e4a8ae80af1dc2d1ea92c405170c0cdda03c613577171b6215bd',
      },
      inventory: {
        valid: 75,
        invalid: 138,
        diagnostics: 182,
        total: 213,
      },
    });
    expect(fixture.cases).toHaveLength(fixture.__generated.inventory.total);
    expect(
      fixture.cases.reduce((total, testCase) => total + testCase.expectedDiagnostics.length, 0),
    ).toBe(fixture.__generated.inventory.diagnostics);
    expect(
      fixture.cases.some(
        (testCase) =>
          /\.(?:jsx|tsx)$/u.test(testCase.filename) ||
          /<[A-Z][A-Za-z]*(?:\s|\/?>)/u.test(testCase.code),
      ),
    ).toBe(false);
    expect(createHash('sha256').update(fixtureSource).digest('hex')).toBe(
      '7e541bfeda9369ef6d3a30681a079b84769447d43c51a4691be0742440c0d0d7',
    );
  });

  for (const [index, testCase] of fixture.cases.entries()) {
    it(`replays case ${index + 1}/${fixture.cases.length}: ${testCase.name}`, () => {
      const reports = runRule(
        testCase.code,
        testCase.options,
        testCase.filename,
        testCase.settings,
      );
      expect(
        reports.map((report) => exactDiagnostic(report)),
        testCase.name,
      ).toEqual(testCase.expectedDiagnostics);

      if (testCase.output === null) {
        expect(reports, testCase.name).toEqual([]);
        return;
      }

      const convergence = applyIteratively(
        testCase.code,
        testCase.options,
        testCase.filename,
        testCase.settings,
      );
      expect(convergence.output, testCase.name).toBe(testCase.output);
      expect(convergence.remainingReports, testCase.name).toEqual([]);
      expect(convergence.passes, testCase.name).toBeGreaterThanOrEqual(1);
      expect(convergence.passes, testCase.name).toBeLessThanOrEqual(16);
    });
  }
});

function runRule(sourceText, options, filename, settings) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    options,
    settings,
    sourceCode,
    filename,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function exactDiagnostic(report) {
  const replacement = report.fix?.({
    replaceTextRange(range, text) {
      return { range, text };
    },
  });
  return {
    messageId: report.messageId,
    message: Object.entries(report.data).reduce(
      (message, [key, value]) => message.replace(`{{${key}}}`, value),
      rule.meta.messages[report.messageId],
    ),
    data: report.data,
    loc: {
      startLine: report.loc.start.line,
      startColumn: report.loc.start.column + 1,
      endLine: report.loc.end.line,
      endColumn: report.loc.end.column + 1,
    },
    fix: replacement ?? null,
  };
}

function applyIteratively(sourceText, options, filename, settings) {
  let output = sourceText;
  let passes = 0;
  for (; passes < 16; passes += 1) {
    const reports = runRule(output, options, filename, settings);
    const next = applyFixPass(output, reports);
    if (next === null) {
      return { output, passes, remainingReports: reports };
    }
    output = next;
  }
  throw new Error(`Fixes did not converge for ${JSON.stringify(sourceText)}`);
}

function applyFixPass(sourceText, reports) {
  const fixes = reports
    .flatMap((report) => {
      const fix = report.fix?.({
        replaceTextRange(range, text) {
          return { range, text };
        },
      });
      return fix ? [fix] : [];
    })
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
    output = output.slice(0, fix.range[0]) + fix.text + output.slice(fix.range[1]);
  }
  return output;
}
