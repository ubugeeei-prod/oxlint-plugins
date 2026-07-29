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
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('lines-around-comment');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-equals-spacing');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-quotes');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-confusing-arrow');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'type-annotation-spacing',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'function-call-argument-newline',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('function-paren-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-mixed-operators');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('array-element-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('array-bracket-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'newline-per-chained-call',
    );
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

  it('runs array-element-newline with exact native messages, ranges, and fixes', () => {
    const source = 'const list = [1, 2, 3];';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'array-element-newline', options: [] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'array-element-newline',
        messageId: 'missingLineBreak',
        message: 'There should be a linebreak after this element.',
        range: { start: 16, end: 17 },
        suggestions: [
          {
            messageId: 'missingLineBreak',
            message: 'There should be a linebreak after this element.',
            fixes: [{ range: { start: 16, end: 17 }, replacementText: '\n' }],
          },
        ],
      },
      {
        ruleName: 'array-element-newline',
        messageId: 'missingLineBreak',
        message: 'There should be a linebreak after this element.',
        range: { start: 19, end: 20 },
        suggestions: [
          {
            messageId: 'missingLineBreak',
            message: 'There should be a linebreak after this element.',
            fixes: [{ range: { start: 19, end: 20 }, replacementText: '\n' }],
          },
        ],
      },
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

  it('runs lines-around-comment with stable messages, ranges, and fixes', () => {
    const sourceText = 'before();\n/** 注釈 */\nafter();';
    const commentStart = Buffer.byteLength('before();\n');
    const commentEnd = commentStart + Buffer.byteLength('/** 注釈 */');
    const diagnostics = runNativeStylisticLint(sourceText, {
      rules: [
        {
          name: 'lines-around-comment',
          options: [{ beforeBlockComment: true, afterBlockComment: true }],
        },
      ],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'lines-around-comment',
        messageId: 'before',
        message: 'Expected line before comment.',
        range: { start: commentStart, end: commentEnd },
      },
      {
        ruleName: 'lines-around-comment',
        messageId: 'after',
        message: 'Expected line after comment.',
        range: { start: commentStart, end: commentEnd },
      },
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.suggestions[0].fixes[0])).toEqual([
      {
        range: { start: commentStart, end: commentStart },
        replacementText: '\n',
      },
      {
        range: { start: commentEnd, end: commentEnd },
        replacementText: '\n',
      },
    ]);
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

  it('runs no-confusing-arrow with stable options and exact native fixes', () => {
    const sourceText =
      "const direct = value => value ? 'yes' : 'no';\n" +
      "const parenthesized = value => (value ? 'yes' : 'no');\n" +
      "const destructured = ({ value }) => value ? 'yes' : 'no';\n";
    const diagnostics = runNativeStylisticLint(sourceText, {
      rules: [{ name: 'no-confusing-arrow', options: [{ onlyOneSimpleParam: true }] }],
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toMatchObject({
      ruleName: 'no-confusing-arrow',
      messageId: 'confusing',
      message: 'Arrow function used ambiguously with a conditional expression.',
    });
    expect(diagnostics[0].range).toEqual({
      start: sourceText.indexOf("value => value ? 'yes' : 'no'"),
      end:
        sourceText.indexOf("value => value ? 'yes' : 'no'") +
        "value => value ? 'yes' : 'no'".length,
    });
    expect(diagnostics[0].suggestions).toEqual([
      {
        messageId: 'confusing',
        message: 'Arrow function used ambiguously with a conditional expression.',
        fixes: [
          {
            range: {
              start: sourceText.indexOf("value ? 'yes' : 'no'"),
              end: sourceText.indexOf("value ? 'yes' : 'no'") + "value ? 'yes' : 'no'".length,
            },
            replacementText: "(value ? 'yes' : 'no')",
          },
        ],
      },
    ]);
  });

  it('does not expose a no-confusing-arrow fix when allowParens is false', () => {
    const diagnostics = runNativeStylisticLint('value => condition ? yes : no', {
      rules: [{ name: 'no-confusing-arrow', options: [{ allowParens: false }] }],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'no-confusing-arrow',
        messageId: 'confusing',
      },
    ]);
    expect(diagnostics[0].suggestions).toBeUndefined();
  });

  it('runs type-annotation-spacing with context overrides and exact byte fixes', () => {
    const source = 'const 日本語 :string = 1; type F = (value:string)=>number;';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [
        {
          name: 'type-annotation-spacing',
          options: [
            {
              overrides: {
                variable: { before: false, after: true },
                parameter: { before: true, after: false },
                arrow: { before: true, after: true },
              },
            },
          ],
        },
      ],
    });

    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'expectedSpaceAfter',
      'unexpectedSpaceBefore',
      'expectedSpaceBefore',
      'expectedSpaceAfter',
      'expectedSpaceBefore',
    ]);
    const variableColon = Buffer.byteLength('const 日本語 ');
    const parameterColon = Buffer.byteLength('const 日本語 :string = 1; type F = (value');
    const arrow = Buffer.byteLength('const 日本語 :string = 1; type F = (value:string)');
    expect(diagnostics.map((diagnostic) => diagnostic.range)).toEqual([
      { start: variableColon, end: variableColon + 1 },
      { start: variableColon, end: variableColon + 1 },
      { start: parameterColon, end: parameterColon + 1 },
      { start: arrow, end: arrow + 2 },
      { start: arrow, end: arrow + 2 },
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.suggestions?.[0]?.fixes[0])).toMatchObject([
      { replacementText: ' ' },
      { replacementText: '' },
      { replacementText: ' ' },
      { replacementText: ' ' },
      { replacementText: ' ' },
    ]);
  });

  it('runs function-paren-newline with exact UTF-8 byte ranges and fixes', () => {
    const source = 'const 日本語 = call(first, second);';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'function-paren-newline', options: ['always'] }],
    });
    const left = Buffer.byteLength('const 日本語 = call');
    const right = Buffer.byteLength('const 日本語 = call(first, second');

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'function-paren-newline',
        messageId: 'expectedAfter',
        message: "Expected newline after '('.",
        range: { start: left, end: left + 1 },
        suggestions: [
          {
            messageId: 'expectedAfter',
            fixes: [{ range: { start: left + 1, end: left + 1 }, replacementText: '\n' }],
          },
        ],
      },
      {
        ruleName: 'function-paren-newline',
        messageId: 'expectedBefore',
        message: "Expected newline before ')'.",
        range: { start: right, end: right + 1 },
        suggestions: [
          {
            messageId: 'expectedBefore',
            fixes: [{ range: { start: right, end: right }, replacementText: '\n' }],
          },
        ],
      },
    ]);
  });

  it('runs function-paren-newline multiline-arguments and preserves unsafe comments', () => {
    const unsafeSource = ['function value(/* retain */', 'first', '/* retain */) {}'].join('\n');
    const unsafeDiagnostics = runNativeStylisticLint(unsafeSource, {
      rules: [{ name: 'function-paren-newline', options: ['never'] }],
    });
    expect(unsafeDiagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'unexpectedAfter',
      'unexpectedBefore',
    ]);
    expect(unsafeDiagnostics.every((diagnostic) => diagnostic.suggestions === undefined)).toBe(
      true,
    );

    const source = ['function value(', 'first,', 'second, third', ') {}'].join('\n');
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'function-paren-newline', options: ['multiline-arguments'] }],
    });

    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual(['expectedBetween']);
    expect(diagnostics[0].suggestions[0].fixes[0]).toEqual({
      range: {
        start: Buffer.byteLength('function value(\nfirst,\nsecond, '),
        end: Buffer.byteLength('function value(\nfirst,\nsecond, '),
      },
      replacementText: '\n',
    });
  });

  it('runs no-mixed-operators with exact messages, template data, and ranges', () => {
    const diagnostics = runNativeStylisticLint('a && b > 0 || c', {
      rules: [
        {
          name: 'no-mixed-operators',
          options: [{ groups: [['&&', '||', '>']] }],
        },
      ],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'no-mixed-operators',
        messageId: 'unexpectedMixedOperator',
        message:
          "Unexpected mix of '&&' and '||'. Use parentheses to clarify the intended order of operations.",
        data: { leftOperator: '&&', rightOperator: '||' },
        range: { start: 2, end: 4 },
      },
      {
        ruleName: 'no-mixed-operators',
        messageId: 'unexpectedMixedOperator',
        message:
          "Unexpected mix of '&&' and '>'. Use parentheses to clarify the intended order of operations.",
        data: { leftOperator: '&&', rightOperator: '>' },
        range: { start: 2, end: 4 },
      },
      {
        ruleName: 'no-mixed-operators',
        messageId: 'unexpectedMixedOperator',
        message:
          "Unexpected mix of '&&' and '>'. Use parentheses to clarify the intended order of operations.",
        data: { leftOperator: '&&', rightOperator: '>' },
        range: { start: 7, end: 8 },
      },
      {
        ruleName: 'no-mixed-operators',
        messageId: 'unexpectedMixedOperator',
        message:
          "Unexpected mix of '&&' and '||'. Use parentheses to clarify the intended order of operations.",
        data: { leftOperator: '&&', rightOperator: '||' },
        range: { start: 11, end: 13 },
      },
    ]);
  });

  it('runs newline-per-chained-call with exact native data, byte ranges, and fixes', () => {
    const source = 'const 日本語 = service.first().second().third();';
    const diagnostic = runNativeStylisticLint(source, {
      rules: [{ name: 'newline-per-chained-call', options: [] }],
    })[0];
    const propertyStart = source.indexOf('.third');
    const byteOffset = (offset) => new TextEncoder().encode(source.slice(0, offset)).length;

    expect(diagnostic).toMatchObject({
      ruleName: 'newline-per-chained-call',
      messageId: 'expected',
      message: 'Expected line break before `.third`.',
      data: { callee: '.third' },
      range: {
        start: byteOffset(propertyStart),
        end: byteOffset(propertyStart + '.third'.length),
      },
    });
    expect(diagnostic.suggestions[0].fixes).toEqual([
      {
        range: {
          start: byteOffset(propertyStart),
          end: byteOffset(propertyStart),
        },
        replacementText: '\n',
      },
    ]);
  });

  it('runs array-bracket-newline with exact native byte ranges and fixes', () => {
    const source = 'const 日本語 = [1, 2];';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'array-bracket-newline', options: ['always'] }],
    });
    const opening = Buffer.byteLength('const 日本語 = ');
    const closing = Buffer.byteLength('const 日本語 = [1, 2');

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'array-bracket-newline',
        messageId: 'missingOpeningLinebreak',
        range: { start: opening, end: opening + 1 },
        suggestions: [
          {
            messageId: 'missingOpeningLinebreak',
            fixes: [{ range: { start: opening + 1, end: opening + 1 }, replacementText: '\n' }],
          },
        ],
      },
      {
        ruleName: 'array-bracket-newline',
        messageId: 'missingClosingLinebreak',
        range: { start: closing, end: closing + 1 },
        suggestions: [
          {
            messageId: 'missingClosingLinebreak',
            fixes: [{ range: { start: closing, end: closing }, replacementText: '\n' }],
          },
        ],
      },
    ]);
  });

  it('runs function-call-argument-newline with exact native byte ranges and fixes', () => {
    const source = "fn('日本語', value)";
    const commaEnd = Buffer.byteLength("fn('日本語',");
    const valueStart = Buffer.byteLength("fn('日本語', ");
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'function-call-argument-newline', options: [] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'function-call-argument-newline',
        messageId: 'missingLineBreak',
        message: 'There should be a line break after this argument.',
        range: { start: commaEnd, end: valueStart },
        suggestions: [
          {
            messageId: 'missingLineBreak',
            message: 'There should be a line break after this argument.',
            fixes: [
              {
                range: { start: commaEnd, end: valueStart },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('keeps function-call-argument-newline line-comment diagnostics unfixable', () => {
    const source = 'fn(first, // keep\nsecond)';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'function-call-argument-newline', options: ['never'] }],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'function-call-argument-newline',
        messageId: 'unexpectedLineBreak',
        message: 'There should be no line break here.',
      },
    ]);
    expect(diagnostics[0].suggestions).toBeUndefined();
  });
});
