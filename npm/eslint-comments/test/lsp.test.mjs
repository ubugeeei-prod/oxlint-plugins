// Editor-facing fixture: turns the rule's reports into LSP diagnostics. LSP
// positions are 0-indexed and non-negative, so the upstream `column: -1`
// "whole line" sentinel is clamped to column 0. These rules are not fixable, so
// there are no quick fixes to assert.

import { describe, expect, it } from 'vitest';

import { runRule } from './harness.mjs';

function toLspDiagnostics(reports) {
  return reports.map((report) => ({
    range: {
      start: { line: report.line - 1, character: Math.max(0, report.column - 1) },
      end: { line: report.endLine - 1, character: Math.max(0, report.endColumn - 1) },
    },
    severity: 1,
    source: 'oxlint-plugins',
    code: `eslint-comments/no-unlimited-disable`,
    message: report.message,
  }));
}

describe('LSP diagnostics fixture', () => {
  it('emits a diagnostic for an unlimited disable', () => {
    const reports = runRule('no-unlimited-disable', { code: '\n/*eslint-disable*/\n' });

    expect(toLspDiagnostics(reports)).toMatchInlineSnapshot(`
      [
        {
          "code": "eslint-comments/no-unlimited-disable",
          "message": "Unexpected unlimited 'eslint-disable' comment. Specify some rule names to disable.",
          "range": {
            "end": {
              "character": 18,
              "line": 1,
            },
            "start": {
              "character": 0,
              "line": 1,
            },
          },
          "severity": 1,
          "source": "oxlint-plugins",
        },
      ]
    `);
  });
});
