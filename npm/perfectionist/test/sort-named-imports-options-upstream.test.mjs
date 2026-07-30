import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/sort-named-imports-options-v5.9.1.json', import.meta.url),
    'utf8',
  ),
);
const rule = plugin.rules['sort-named-imports'];

describe('perfectionist/sort-named-imports v5.9.1 scalar option parity', () => {
  it('pins the reviewed upstream source and exact fixture inventory', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-perfectionist',
      version: '5.9.1',
      sourceCommit: 'b35e8e4caf0c8d350cf386e504241f21827dd60b',
      sourceHashes: {
        'rules/sort-named-imports.ts':
          'ef0f575cb4aca0248120e8eb831748b2fb7a170e2e2220b8cdee66f1c2740ae6',
        'rules/sort-named-imports/sort-named-import.ts':
          '3e35c4b385cab6e07acac20e2ee4f5049e3cfc1fb05403ac0158cd360e4b3415',
        'test/rules/sort-named-imports.test.ts':
          'a5fe43752e460a2d29432e01e01a21aaa92bf9bc6d5193840dc593c6767ddeee',
      },
      inventory: {
        valid: 23,
        invalid: 28,
        diagnostics: 41,
        total: 51,
      },
    });
    expect(fixture.cases).toHaveLength(fixture.__generated.inventory.total);
    expect(
      fixture.cases.reduce((total, testCase) => total + testCase.expectedDiagnostics.length, 0),
    ).toBe(fixture.__generated.inventory.diagnostics);
  });

  for (const [index, testCase] of fixture.cases.entries()) {
    it(`replays case ${index + 1}/${fixture.cases.length}: ${testCase.name}`, () => {
      const reports = runRule(testCase.code, testCase.options, testCase.filename);
      expect(
        reports.map((report) => exactDiagnostic(report)),
        testCase.name,
      ).toEqual(testCase.expectedDiagnostics);

      if (testCase.output === null) {
        expect(reports, testCase.name).toEqual([]);
        return;
      }

      const convergence = applyIteratively(testCase.code, testCase.options, testCase.filename);
      expect(convergence.output, testCase.name).toBe(testCase.output);
      expect(convergence.remainingReports, testCase.name).toEqual([]);
      expect(convergence.passes, testCase.name).toBeGreaterThanOrEqual(1);
      expect(convergence.passes, testCase.name).toBeLessThanOrEqual(8);
    });
  }
});

function runRule(sourceText, options, filename) {
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
    message: rule.meta.messages[report.messageId]
      .replace('{{right}}', report.data.right)
      .replace('{{left}}', report.data.left),
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

function applyIteratively(sourceText, options, filename) {
  let output = sourceText;
  let passes = 0;
  for (; passes < 8; passes += 1) {
    const reports = runRule(output, options, filename);
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
