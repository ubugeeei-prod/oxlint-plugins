import { describe, expect, it } from 'vitest';

import { implementedEslintJsonRuleNames, scanEslintJson } from '../api.js';

const expectedRuleNames = [
  'no-duplicate-keys',
  'no-empty-keys',
  'no-unnormalized-keys',
  'no-unsafe-values',
  'sort-keys',
  'top-level-interop',
];

function diagnosticsByRule(sourceText, options = {}) {
  return scanEslintJson(sourceText, options).reduce((acc, diagnostic) => {
    acc[diagnostic.ruleName] ||= [];
    acc[diagnostic.ruleName].push(diagnostic);
    return acc;
  }, {});
}

function applyFix(sourceText, fix) {
  return sourceText.slice(0, fix.start) + fix.replacement + sourceText.slice(fix.end);
}

describe('@eslint/json native API', () => {
  it('exposes all @eslint/json rule names', () => {
    expect(implementedEslintJsonRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans duplicate, empty, unsafe, sort, and top-level rules', () => {
    const source = '{"": 1, "b": 2, "a": 3, "b": 4, "unsafe": 2e308}';
    const grouped = diagnosticsByRule(source);

    expect(grouped['no-empty-keys']).toHaveLength(1);
    expect(grouped['no-empty-keys'][0].messageId).toBe('emptyKey');
    expect(grouped['no-duplicate-keys']).toHaveLength(1);
    expect(grouped['no-duplicate-keys'][0].data.key).toBe('b');
    expect(grouped['no-unsafe-values']).toHaveLength(1);
    expect(grouped['no-unsafe-values'][0]).toMatchObject({
      messageId: 'unsafeNumber',
      data: { value: '2e308' },
    });
    expect(grouped['sort-keys']).toHaveLength(1);

    expect(scanEslintJson('1', { ruleNames: ['top-level-interop'] })).toMatchObject([
      {
        ruleName: 'top-level-interop',
        messageId: 'topLevel',
        data: { type: 'Number' },
      },
    ]);
  });

  it('handles JSONC and JSON5-style syntax used by upstream tests', () => {
    const source = `{
      // jsonc comment
      unquoted: 1,
      'unquoted': 2,
      trailing: [1, 2,],
    }`;

    expect(
      scanEslintJson(source, { ruleNames: ['no-duplicate-keys'] }).map(
        (diagnostic) => diagnostic.data.key,
      ),
    ).toEqual(['unquoted']);
  });

  it('reports normalization issues and emits safe fixes for raw keys', () => {
    const decomposed = 'e\u0301';
    const source = `{"${decomposed}": 1}`;
    const diagnostics = scanEslintJson(source, {
      ruleNames: ['no-unnormalized-keys'],
      normalizationForm: 'NFC',
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('unnormalizedKey');
    expect(applyFix(source, diagnostics[0].fix)).toBe('{"é": 1}');
  });

  it('does not auto-fix escaped unnormalized keys', () => {
    const diagnostics = scanEslintJson('{"\\u0065\\u0301": 1}', {
      ruleNames: ['no-unnormalized-keys'],
      normalizationForm: 'NFC',
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].fix).toBeUndefined();
  });

  it('covers unsafe integers, zero underflow, subnormal numbers, and lone surrogates', () => {
    const source = '[9007199254740992, 1e-400, 2.2250738585072009e-308, "\\ud83d"]';
    const diagnostics = scanEslintJson(source, { ruleNames: ['no-unsafe-values'] });

    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'unsafeInteger',
      'unsafeZero',
      'subnormal',
      'loneSurrogate',
    ]);
    expect(diagnostics[3].data.surrogate).toBe('\\ud83d');
  });

  it('honors sort direction, case sensitivity, natural ordering, min keys, and blank groups', () => {
    expect(scanEslintJson('{"b":1,"a":2}', { ruleNames: ['sort-keys'] })).toHaveLength(1);
    expect(
      scanEslintJson('{"b":1,"a":2}', {
        ruleNames: ['sort-keys'],
        sortDirection: 'desc',
      }),
    ).toEqual([]);
    expect(
      scanEslintJson('{"A":1,"a":2}', {
        ruleNames: ['sort-keys'],
        sortCaseSensitive: false,
      }),
    ).toEqual([]);
    expect(
      scanEslintJson('{"item2":1,"item11":2}', {
        ruleNames: ['sort-keys'],
        sortNatural: true,
      }),
    ).toEqual([]);
    expect(
      scanEslintJson('{"b":1,"a":2}', {
        ruleNames: ['sort-keys'],
        sortMinKeys: 3,
      }),
    ).toEqual([]);
    expect(
      scanEslintJson('{"b":1,\n\n"a":2}', {
        ruleNames: ['sort-keys'],
        sortAllowLineSeparatedGroups: true,
      }),
    ).toEqual([]);
  });

  it('returns LSP-friendly locations', () => {
    const diagnostics = scanEslintJson('{\n  "a": 1,\n  "a": 2\n}', {
      ruleNames: ['no-duplicate-keys'],
    });

    expect(diagnostics[0].loc).toEqual({
      startLine: 3,
      startColumn: 2,
      endLine: 3,
      endColumn: 5,
    });
  });
});
