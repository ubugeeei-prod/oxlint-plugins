import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

function runRule(ruleName, sourceText, options, settings) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const rule = plugin.rules[ruleName];
  const visitor = rule.createOnce({
    options: options ?? [],
    sourceCode,
    settings,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

describe('stylistic plugin', () => {
  it('exports the stylistic plugin surface', () => {
    expect(plugin.corsaStylisticPlugin).toBe(plugin);
    expect(plugin.implementedStylisticRuleNames).toContain('quotes');
    expect(Object.keys(plugin.rules)).toContain('no-trailing-spaces');
  });

  it('reports direct rule options', () => {
    expect(runRule('quotes', 'const label = "value";', ['single'])).toMatchObject([
      {
        messageId: 'wrongQuote',
        node: { range: [14, 21] },
      },
    ]);
  });

  it('shares configured stylistic settings across enabled rules', () => {
    const reports = runRule('no-trailing-spaces', "const label = 'value';  \n", [], {
      corsaStylistic: {
        rules: {
          quotes: ['single'],
          'no-trailing-spaces': [],
        },
      },
    });

    expect(reports).toMatchObject([
      {
        messageId: 'trailingSpace',
      },
    ]);
  });

  it('does not reuse native diagnostics across files sharing a sourceCode object', () => {
    const sourceCode = {
      text: 'const source = { foo: 1 };\n',
      getText() {
        return this.text;
      },
    };
    const reports = [];
    const context = {
      sourceCode,
      settings: {
        corsaStylistic: {
          rules: {
            'object-curly-spacing': ['always'],
          },
        },
      },
      options: [],
      report(descriptor) {
        reports.push(descriptor);
      },
    };
    const rule = plugin.rules['object-curly-spacing'];

    rule.createOnce(context).Program({ type: 'Program', range: [0, sourceCode.text.length] });
    expect(reports).toEqual([]);

    sourceCode.text = 'const {foo} = source;\n';
    rule.createOnce(context).Program({ type: 'Program', range: [0, sourceCode.text.length] });

    expect(reports.map((report) => report.messageId)).toEqual([
      'requireSpaceAfter',
      'requireSpaceBefore',
    ]);
  });

  it('maps native byte ranges to Oxlint UTF-16 source ranges', () => {
    const sourceText = '// 日本語\nconst a = [\n  1\n]\n';
    const reports = runRule('comma-dangle', sourceText, ['always']);
    const insertAt = sourceText.indexOf('1') + 1;

    expect(reports).toHaveLength(1);
    expect(reports[0].node?.range).toEqual([insertAt, insertAt]);
    expect(
      reports[0].suggest?.[0]?.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([{ range: [insertAt, insertAt], replacementText: ',' }]);
  });
});
