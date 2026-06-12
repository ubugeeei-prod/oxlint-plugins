import { describe, expect, it } from 'vitest';

import {
  scanDisableEnablePair,
  scanNoAggregatingEnable,
  scanNoDuplicateDisable,
  scanNoRestrictedDisable,
  scanNoUnlimitedDisable,
  scanNoUnusedDisable,
  scanNoUnusedEnable,
  scanNoUse,
  scanRequireDescription,
} from '../api.js';

function block(value, line = 1) {
  return {
    kind: 'Block',
    value,
    startLine: line,
    startColumn: 0,
    endLine: line,
    endColumn: value.length + 4,
  };
}

describe('JS API', () => {
  it('exports the NAPI-backed scans used by the plugin rules', () => {
    expect(
      [
        scanDisableEnablePair,
        scanNoAggregatingEnable,
        scanNoDuplicateDisable,
        scanNoRestrictedDisable,
        scanNoUnlimitedDisable,
        scanNoUnusedDisable,
        scanNoUnusedEnable,
        scanNoUse,
        scanRequireDescription,
      ].every((fn) => typeof fn === 'function'),
    ).toBe(true);
  });

  it('reports unlimited disables and ignores scoped ones', () => {
    const comments = [block('eslint-disable '), block('eslint-disable eqeqeq', 2)];

    expect(scanNoUnlimitedDisable(comments)).toEqual([
      {
        messageId: 'unexpected',
        data: { kind: 'eslint-disable' },
        loc: { startLine: 1, startColumn: -1, endLine: 1, endColumn: 19 },
      },
    ]);
  });

  it('rejects non-array input', () => {
    expect(() => scanNoUnlimitedDisable(null)).toThrow(TypeError);
  });

  it('reports disabled areas without enable pairs', () => {
    expect(scanDisableEnablePair([block('eslint-disable no-undef')])).toEqual([
      {
        messageId: 'missingRulePair',
        data: { ruleId: 'no-undef' },
        loc: { startLine: 1, startColumn: 17, endLine: 1, endColumn: 25 },
      },
    ]);
  });

  it('reports aggregating enables and duplicate disables', () => {
    const comments = [
      block('eslint-disable no-undef'),
      block('eslint-disable no-unused-vars', 2),
      block('eslint-enable', 3),
    ];

    expect(scanNoAggregatingEnable(comments)).toEqual([
      {
        messageId: 'aggregatingEnable',
        data: { count: 2 },
        loc: { startLine: 3, startColumn: -1, endLine: 3, endColumn: 17 },
      },
    ]);

    expect(
      scanNoDuplicateDisable([
        block('eslint-disable no-undef'),
        block('eslint-disable no-undef', 2),
      ]),
    ).toEqual([
      {
        messageId: 'duplicateRule',
        data: { ruleId: 'no-undef' },
        loc: { startLine: 2, startColumn: 17, endLine: 2, endColumn: 25 },
      },
    ]);
  });

  it('reports restricted disables with rule-id locations', () => {
    expect(
      scanNoRestrictedDisable(
        [block('eslint-disable semi, no-extra-semi, comma-style')],
        ['*semi*'],
      ),
    ).toEqual([
      {
        messageId: 'disallow',
        data: { ruleId: 'semi' },
        loc: { startLine: 1, startColumn: 17, endLine: 1, endColumn: 21 },
      },
      {
        messageId: 'disallow',
        data: { ruleId: 'no-extra-semi' },
        loc: { startLine: 1, startColumn: 23, endLine: 1, endColumn: 36 },
      },
    ]);
  });

  it('reports unused disables from synthetic lint problems', () => {
    expect(scanNoUnusedDisable([block('eslint-disable no-undef')], [])).toEqual([
      {
        messageId: 'unusedRule',
        data: { ruleId: 'no-undef' },
        loc: { startLine: 1, startColumn: 17, endLine: 1, endColumn: 25 },
      },
    ]);

    expect(
      scanNoUnusedDisable(
        [block('eslint-disable no-undef')],
        [{ ruleId: 'no-undef', loc: { start: { line: 2, column: 0 } } }],
      ),
    ).toEqual([]);
  });

  it('reports unused enables for global and rule-specific directives', () => {
    expect(
      scanNoUnusedEnable([block('eslint-enable'), block('eslint-enable no-undef', 2)]),
    ).toEqual([
      {
        messageId: 'unused',
        data: {},
        loc: { startLine: 1, startColumn: -1, endLine: 1, endColumn: 17 },
      },
      {
        messageId: 'unusedRule',
        data: { ruleId: 'no-undef' },
        loc: { startLine: 2, startColumn: 16, endLine: 2, endColumn: 24 },
      },
    ]);
  });

  it('normalizes rule option lists for no-use and require-description', () => {
    expect(scanNoUse([block('eslint-disable')], ['eslint-disable'])).toEqual([]);
    expect(scanRequireDescription([block('eslint-disable')], ['eslint-disable'])).toEqual([]);
    expect(() => scanNoRestrictedDisable([], ['semi', 1])).toThrow(TypeError);
  });
});
