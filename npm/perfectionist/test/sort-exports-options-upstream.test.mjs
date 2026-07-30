import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/sort-exports-options-v5.9.1.json', import.meta.url), 'utf8'),
);
const rule = plugin.rules['sort-exports'];

describe('perfectionist/sort-exports v5.9.1 option parity', () => {
  it('pins every authored upstream case and its exact fixture inventory', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-perfectionist',
      version: '5.9.1',
      sourceCommit: 'b35e8e4caf0c8d350cf386e504241f21827dd60b',
      sourceHashes: {
        'rules/sort-exports.ts': 'dd6ee1cd385e1f8cca77357a781f5bd82176d67d7e33695ca177624095077baf',
        'rules/sort-exports/types.ts':
          '5f09e2870357909e40e8a33acce0bf2fb796350246e970f77432e2a3c8a309df',
        'test/rules/sort-exports.test.ts':
          '1590b05abda2fa82c9e9579514b04f3c03137c26834711fb94fce6af399d85e4',
      },
      inventory: {
        valid: 70,
        invalid: 133,
        diagnostics: 184,
        total: 203,
      },
    });
    expect(fixture.cases).toHaveLength(fixture.__generated.inventory.total);
    expect(
      fixture.cases.reduce((total, testCase) => total + testCase.expectedDiagnostics.length, 0),
    ).toBe(fixture.__generated.inventory.diagnostics);
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
      expect(convergence.passes, testCase.name).toBeLessThanOrEqual(12);
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
  for (; passes < 12; passes += 1) {
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
