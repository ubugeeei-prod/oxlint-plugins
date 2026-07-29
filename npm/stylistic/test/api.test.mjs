import { describe, expect, it } from 'vitest';

import { nativeStylisticRuleMetas, runNativeStylisticLint } from '../api.js';

describe('stylistic native API', () => {
  it('exposes native stylistic rule metadata', () => {
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quotes');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-trailing-spaces');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quote-props');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('line-comment-position');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'one-var-declaration-per-line',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'lines-between-class-members',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-equals-spacing');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-quotes');
  });

  it('runs multiple stylistic rules through one native call', () => {
    const diagnostics = runNativeStylisticLint(
      '\u{feff}const label = "value";  \r\n\t label;\n\n\n',
      {
        rules: [
          { name: 'unicode-bom', options: ['never'] },
          { name: 'quotes', options: ['single'] },
          { name: 'no-trailing-spaces', options: [] },
          { name: 'no-mixed-spaces-and-tabs', options: [] },
          { name: 'no-tabs', options: [] },
          { name: 'linebreak-style', options: ['unix'] },
          { name: 'no-multiple-empty-lines', options: [{ max: 1 }] },
        ],
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'unicode-bom',
      'quotes',
      'no-trailing-spaces',
      'no-mixed-spaces-and-tabs',
      'no-tabs',
      'linebreak-style',
      'no-multiple-empty-lines',
    ]);
    expect(diagnostics[1].suggestions?.[0]?.fixes[0]?.replacementText).toBe("'value'");
  });

  it('runs additional context-backed rules through one native call', () => {
    const diagnostics = runNativeStylisticLint(
      'const o = {foo :1}; const a = 1; const b = 2;\nif (x) {\n  y();\n}\n',
      {
        rules: [
          { name: 'key-spacing', options: [] },
          { name: 'quote-props', options: [] },
          { name: 'max-statements-per-line', options: [] },
          { name: 'padded-blocks', options: [] },
        ],
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'key-spacing',
      'key-spacing',
      'quote-props',
      'max-statements-per-line',
      'padded-blocks',
      'padded-blocks',
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'extraKey',
      'missingValue',
      'unquotedPropertyFound',
      'exceed',
      'missingPadBlock',
      'missingPadBlock',
    ]);
  });

  it('runs line-comment-position with upstream default ignores', () => {
    const diagnostics = runNativeStylisticLint(
      'value; // inline\nvalue; // eslint-disable-line\n// above\n',
      {
        rules: [{ name: 'line-comment-position', options: [] }],
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual(['above']);
    expect(diagnostics[0].range).toEqual({ start: 7, end: 16 });
  });

  it('runs jsx-equals-spacing with upstream never and always options', () => {
    const source = '<App foo = {bar} baz={value} />';
    const neverDiagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'jsx-equals-spacing', options: ['never'] }],
    });
    const alwaysDiagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'jsx-equals-spacing', options: ['always'] }],
    });

    expect(neverDiagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'noSpaceBefore',
      'noSpaceAfter',
    ]);
    expect(alwaysDiagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'needSpaceBefore',
      'needSpaceAfter',
    ]);
    expect(neverDiagnostics.map((diagnostic) => diagnostic.range)).toEqual([
      { start: 9, end: 10 },
      { start: 9, end: 10 },
    ]);
  });

  it('runs jsx-quotes with both upstream options and exact native fixes', () => {
    const source = '<App single=\'one\' double="two" />';
    const preferDouble = runNativeStylisticLint(source, {
      rules: [{ name: 'jsx-quotes', options: ['prefer-double'] }],
    });
    const preferSingle = runNativeStylisticLint(source, {
      rules: [{ name: 'jsx-quotes', options: ['prefer-single'] }],
    });

    expect(preferDouble).toMatchObject([
      {
        ruleName: 'jsx-quotes',
        messageId: 'unexpected',
        message: 'Unexpected usage of singlequote.',
        range: { start: 12, end: 17 },
      },
    ]);
    expect(preferDouble[0].suggestions[0].fixes).toEqual([
      { range: { start: 12, end: 17 }, replacementText: '"one"' },
    ]);
    expect(preferSingle).toMatchObject([
      {
        ruleName: 'jsx-quotes',
        messageId: 'unexpected',
        message: 'Unexpected usage of doublequote.',
        range: { start: 25, end: 30 },
      },
    ]);
    expect(preferSingle[0].suggestions[0].fixes).toEqual([
      { range: { start: 25, end: 30 }, replacementText: "'two'" },
    ]);
  });

  it('keeps non-attribute strings out of jsx-quotes native diagnostics', () => {
    const source = [
      "import value from 'module';",
      "const plain = 'value';",
      "const node = <App expression={'value'} title='attribute'>text 'child'</App>;",
    ].join('\n');
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'jsx-quotes', options: [] }],
    });

    expect(diagnostics).toHaveLength(1);
    expect(source.slice(diagnostics[0].range.start, diagnostics[0].range.end)).toBe("'attribute'");
  });
});
