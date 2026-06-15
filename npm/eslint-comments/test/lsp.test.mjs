// Editor-facing fixture: turns the rule's reports into LSP diagnostics. LSP
// positions are 0-indexed and non-negative, so the upstream `column: -1`
// "whole line" sentinel is clamped to column 0. These rules are not fixable, so
// there are no quick fixes to assert.

import { describe, expect, it } from 'vitest';

import { runRule } from './harness.mjs';

function toLspDiagnostics(ruleName, reports) {
  return reports.map((report) => ({
    range: {
      start: { line: report.line - 1, character: Math.max(0, report.column - 1) },
      end: { line: report.endLine - 1, character: Math.max(0, report.endColumn - 1) },
    },
    severity: 1,
    source: 'oxlint-plugins',
    code: `eslint-comments/${ruleName}`,
    message: report.message,
  }));
}

describe('LSP diagnostics fixture', () => {
  it('emits a diagnostic for an unlimited disable', () => {
    const ruleName = 'no-unlimited-disable';
    const reports = runRule(ruleName, { code: '\n/*eslint-disable*/\n' });

    expect(toLspDiagnostics(ruleName, reports)).toMatchInlineSnapshot(`
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

  it('emits diagnostics for restricted disables', () => {
    const ruleName = 'no-restricted-disable';
    const reports = runRule(ruleName, {
      code: '\n/*eslint-disable semi, no-extra-semi, comma-style*/\n',
      options: ['*semi*'],
    });

    expect(toLspDiagnostics(ruleName, reports)).toMatchInlineSnapshot(`
      [
        {
          "code": "eslint-comments/no-restricted-disable",
          "message": "Disabling 'semi' is not allowed.",
          "range": {
            "end": {
              "character": 21,
              "line": 1,
            },
            "start": {
              "character": 17,
              "line": 1,
            },
          },
          "severity": 1,
          "source": "oxlint-plugins",
        },
        {
          "code": "eslint-comments/no-restricted-disable",
          "message": "Disabling 'no-extra-semi' is not allowed.",
          "range": {
            "end": {
              "character": 36,
              "line": 1,
            },
            "start": {
              "character": 23,
              "line": 1,
            },
          },
          "severity": 1,
          "source": "oxlint-plugins",
        },
      ]
    `);
  });

  it('emits diagnostics for unused enables', () => {
    const ruleName = 'no-unused-enable';
    const reports = runRule(ruleName, { code: '\n/*eslint-enable no-undef*/\n' });

    expect(toLspDiagnostics(ruleName, reports)).toMatchInlineSnapshot(`
      [
        {
          "code": "eslint-comments/no-unused-enable",
          "message": "'no-undef' rule is re-enabled but it has not been disabled.",
          "range": {
            "end": {
              "character": 24,
              "line": 1,
            },
            "start": {
              "character": 16,
              "line": 1,
            },
          },
          "severity": 1,
          "source": "oxlint-plugins",
        },
      ]
    `);
  });

  it('emits diagnostics for unused disables with synthetic lint problems', () => {
    const ruleName = 'no-unused-disable';
    const reports = runRule(ruleName, {
      code: '\n/*eslint-disable no-alert*/\nalert("ok");\n',
      disableDirectiveProblems: [],
    });

    expect(toLspDiagnostics(ruleName, reports)).toMatchInlineSnapshot(`
      [
        {
          "code": "eslint-comments/no-unused-disable",
          "message": "Unused eslint-disable directive (no problems were reported from 'no-alert').",
          "range": {
            "end": {
              "character": 25,
              "line": 1,
            },
            "start": {
              "character": 17,
              "line": 1,
            },
          },
          "severity": 1,
          "source": "oxlint-plugins",
        },
      ]
    `);
  });

  it('suppresses no-unused-disable diagnostics when a matching problem exists', () => {
    const reports = runRule('no-unused-disable', {
      code: '\n/*eslint-disable no-alert*/\nalert("ok");\n',
      disableDirectiveProblems: [
        {
          ruleId: 'no-alert',
          loc: { start: { line: 3, column: 0 } },
        },
      ],
    });

    expect(reports).toEqual([]);
  });
});
