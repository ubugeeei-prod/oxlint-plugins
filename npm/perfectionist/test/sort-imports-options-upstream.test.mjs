import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/sort-imports-options-v5.9.1.json', import.meta.url), 'utf8'),
);
const rule = plugin.rules['sort-imports'];

describe('perfectionist/sort-imports v5.9.1 option parity', () => {
  it('pins every authored upstream case and its exact fixture inventory', () => {
    expect(fixture.__generated).toMatchObject({
      source: 'eslint-plugin-perfectionist',
      version: '5.9.1',
      sourceCommit: 'b35e8e4caf0c8d350cf386e504241f21827dd60b',
      sourceHashes: {
        'rules/sort-imports.ts': 'c5102c424e0364b0e9ce7681b41d9d3543d3a76b9227b3fea70371a4e83efa05',
        'rules/sort-imports/types.ts':
          '81aa65e9d8f085fa7e8479ea9a5e98c1c2e180450632e484494a1d64395ebffe',
        'test/rules/sort-imports.test.ts':
          '8065552b1ccf4d8110524ee48cdc5ee4ab701302d5316a0e2eef9c55ada249ab',
      },
      inventory: {
        valid: 151,
        invalid: 262,
        diagnostics: 468,
        total: 413,
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
