import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const expectedRuleNames = [
  'no-duplicate-keys',
  'no-empty-keys',
  'no-unnormalized-keys',
  'no-unsafe-values',
  'sort-keys',
  'top-level-interop',
];

const invalidCases = [
  ['no-duplicate-keys', '{"foo": 1, "foo": 2}', [], ['Duplicate key "foo" found.']],
  ['no-empty-keys', '{"": 1}', [], ['Empty key found.']],
  [
    'no-unnormalized-keys',
    '{"e\u0301": 1}',
    [{ form: 'NFC' }],
    ["Unnormalized key 'e\u0301' found."],
  ],
  [
    'no-unsafe-values',
    '[2e308, 9007199254740992, "\\ud83d"]',
    [],
    [
      "The number '2e308' will evaluate to Infinity.",
      "The integer '9007199254740992' is outside the safe integer range.",
      "Lone surrogate '\\ud83d' found.",
    ],
  ],
  [
    'sort-keys',
    '{"b": 1, "a": 2}',
    [],
    [
      "Expected object keys to be in alphanumeric case-sensitive ascending order. 'a' should be before 'b'.",
    ],
  ],
  ['top-level-interop', 'true', [], ["Top level item should be array or object, got 'Boolean'."]],
];

function runRule(ruleName, sourceText, options = []) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename: 'fixture.json',
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderMessage(ruleName, report) {
  let message = plugin.rules[ruleName].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data || {})) {
    message = message.replaceAll(`{{${key}}}`, value);
    message = message.replaceAll(`{{ ${key} }}`, value);
  }
  return message;
}

function applyFix(sourceText, report) {
  const fix = report.fix({
    replaceTextRange(range, replacement) {
      return { range, text: replacement };
    },
  });
  return sourceText.slice(0, fix.range[0]) + fix.text + sourceText.slice(fix.range[1]);
}

describe('@eslint/json plugin shape', () => {
  it('exposes all ported rules and native API helpers', () => {
    expect(plugin.meta?.name).toBe('@eslint/json');
    expect(plugin.meta?.namespace).toBe('json');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedEslintJsonRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanEslintJson).toBe('function');
  });

  it('ships upstream-compatible configs', () => {
    expect(plugin.configs.recommended.rules).toEqual({
      'json/no-duplicate-keys': 'error',
      'json/no-empty-keys': 'error',
      'json/no-unnormalized-keys': 'error',
      'json/no-unsafe-values': 'error',
    });
    expect(plugin.configs.all.rules['json/sort-keys']).toBe('error');
    expect(plugin.configs.all.rules['json/top-level-interop']).toBe('error');
  });
});

describe('@eslint/json rules through direct adapter harness', () => {
  it.each(invalidCases)('reports %s', (ruleName, sourceText, options, messages) => {
    const reports = runRule(ruleName, sourceText, options);

    expect(reports.map((report) => renderMessage(ruleName, report))).toEqual(messages);
  });

  it('honors sort options from the rule config', () => {
    expect(runRule('sort-keys', '{"b": 1, "a": 2}', ['desc'])).toEqual([]);
    expect(runRule('sort-keys', '{"b": 1, "a": 2}', ['asc', { minKeys: 3 }])).toEqual([]);
    expect(
      runRule('sort-keys', '{"b": 1,\n\n"a": 2}', ['asc', { allowLineSeparatedGroups: true }]),
    ).toEqual([]);
  });

  it('forwards normalization fixes', () => {
    const code = '{"e\u0301": 1}';
    const reports = runRule('no-unnormalized-keys', code, [{ form: 'NFC' }]);

    expect(reports).toHaveLength(1);
    expect(applyFix(code, reports[0])).toBe('{"é": 1}');
  });

  it('keeps diagnostics cache separated by rule options', () => {
    const sourceCode = {
      text: '{"b": 1, "a": 2}',
      getText() {
        return this.text;
      },
    };
    const ascReports = [];
    const descReports = [];

    plugin.rules['sort-keys']
      .createOnce({
        options: ['asc'],
        sourceCode,
        report(descriptor) {
          ascReports.push(descriptor);
        },
      })
      .Program({ type: 'Program', range: [0, sourceCode.text.length] });

    plugin.rules['sort-keys']
      .createOnce({
        options: ['desc'],
        sourceCode,
        report(descriptor) {
          descReports.push(descriptor);
        },
      })
      .Program({ type: 'Program', range: [0, sourceCode.text.length] });

    expect(ascReports).toHaveLength(1);
    expect(descReports).toEqual([]);
  });
});
