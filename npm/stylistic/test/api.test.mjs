import { describe, expect, it } from 'vitest';

import { nativeStylisticRuleMetas, runNativeStylisticLint } from '../api.js';

describe('stylistic native API', () => {
  it('exposes native stylistic rule metadata', () => {
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quotes');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-trailing-spaces');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quote-props');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('line-comment-position');
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
});
