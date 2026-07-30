import { describe, expect, it } from 'vitest';

import { nativeStylisticRuleMetas, runNativeStylisticLint } from '../api.js';

describe('stylistic native API', () => {
  it('exposes native stylistic rule metadata', () => {
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quotes');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-trailing-spaces');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quote-props');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('line-comment-position');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'multiline-comment-style',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'one-var-declaration-per-line',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'lines-between-class-members',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('lines-around-comment');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-equals-spacing');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'jsx-closing-tag-location',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-curly-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-curly-spacing');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'jsx-first-prop-new-line',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('jsx-quotes');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'jsx-child-element-spacing',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-confusing-arrow');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'type-annotation-spacing',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('type-generic-spacing');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'type-named-tuple-spacing',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'function-call-argument-newline',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('function-paren-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'padding-line-between-statements',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('member-delimiter-style');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('indent-binary-ops');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-mixed-operators');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('array-element-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('object-curly-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'object-property-newline',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('array-bracket-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('brace-style');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('curly-newline');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'nonblock-statement-body-position',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-extra-parens');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('semi');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain(
      'newline-per-chained-call',
    );
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('multiline-ternary');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('wrap-iife');
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

  it('runs object-curly-newline with exact UTF-8 byte ranges and fixes', () => {
    const source = 'const 日本語 = {値: 1};';
    const open = Buffer.byteLength(source.slice(0, source.indexOf('{')));
    const close = Buffer.byteLength(source.slice(0, source.indexOf('}')));
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'object-curly-newline', options: ['always'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'object-curly-newline',
        messageId: 'expectedLinebreakAfterOpeningBrace',
        message: 'Expected a line break after this opening brace.',
        range: { start: open, end: open + 1 },
        suggestions: [
          {
            messageId: 'expectedLinebreakAfterOpeningBrace',
            message: 'Expected a line break after this opening brace.',
            fixes: [{ range: { start: open + 1, end: open + 1 }, replacementText: '\n' }],
          },
        ],
      },
      {
        ruleName: 'object-curly-newline',
        messageId: 'expectedLinebreakBeforeClosingBrace',
        message: 'Expected a line break before this closing brace.',
        range: { start: close, end: close + 1 },
        suggestions: [
          {
            messageId: 'expectedLinebreakBeforeClosingBrace',
            message: 'Expected a line break before this closing brace.',
            fixes: [{ range: { start: close, end: close }, replacementText: '\n' }],
          },
        ],
      },
    ]);
  });

  it('runs object-property-newline with exact native messages, ranges, and fixes', () => {
    const source = 'const value = { first: 1, second: 2 };';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.js',
      rules: [{ name: 'object-property-newline', options: [] }],
    });
    const start = source.indexOf('second');

    expect(diagnostics).toEqual([
      {
        ruleName: 'object-property-newline',
        messageId: 'propertiesOnNewline',
        message: 'Object properties must go on a new line.',
        range: { start, end: start + 'second'.length },
        suggestions: [
          {
            messageId: 'propertiesOnNewline',
            message: 'Object properties must go on a new line.',
            fixes: [
              {
                range: { start: source.indexOf(',') + 1, end: start },
                replacementText: '\n',
              },
            ],
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

  it('runs jsx-closing-tag-location with exact UTF-8 byte ranges and fixes', () => {
    const source = 'const 日本語 = <App>\n  child</App>;';
    const closingStart = Buffer.byteLength(source.slice(0, source.indexOf('</App>')));
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [{ name: 'jsx-closing-tag-location', options: [] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-closing-tag-location',
        messageId: 'onOwnLine',
        message: 'Closing tag of a multiline JSX expression must be on its own line.',
        range: { start: closingStart, end: closingStart + Buffer.byteLength('</App>') },
        suggestions: [
          {
            messageId: 'onOwnLine',
            message: 'Closing tag of a multiline JSX expression must be on its own line.',
            fixes: [
              {
                range: { start: closingStart, end: closingStart },
                replacementText: `\n${' '.repeat('const 日本語 = '.length)}`,
              },
            ],
          },
        ],
      },
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

  it('runs jsx-child-element-spacing with exact UTF-8 points, data, and no fixes', () => {
    const source = '<App>日本語\r\n<a>リンク</a>\r\n後続</App>';
    const elementStart = Buffer.byteLength(source.slice(0, source.indexOf('<a>')));
    const elementEnd = Buffer.byteLength(source.slice(0, source.indexOf('</a>') + '</a>'.length));
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [{ name: 'jsx-child-element-spacing', options: ['ignored'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-child-element-spacing',
        messageId: 'spacingBeforeNext',
        message: 'Ambiguous spacing before next element a',
        data: { element: 'a' },
        range: { start: elementStart, end: elementStart },
      },
      {
        ruleName: 'jsx-child-element-spacing',
        messageId: 'spacingAfterPrev',
        message: 'Ambiguous spacing after previous element a',
        data: { element: 'a' },
        range: { start: elementEnd, end: elementEnd },
      },
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

  it('runs type-generic-spacing with exact UTF-8 byte ranges and fixes', () => {
    const source = 'type 日本語< 値=string > = 値;\r\n';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'type-generic-spacing', options: [{ ignored: true }] }],
    });
    const openGap = Buffer.byteLength('type 日本語<');
    const defaultGap = Buffer.byteLength('type 日本語< 値');
    const closeGap = Buffer.byteLength('type 日本語< 値=string');

    expect(diagnostics).toHaveLength(3);
    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'genericSpacingMismatch',
      'genericSpacingMismatch',
      'genericSpacingMismatch',
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.range)).toEqual([
      { start: openGap, end: openGap + 1 },
      { start: closeGap, end: closeGap + 1 },
      { start: defaultGap, end: defaultGap + 1 },
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.suggestions[0].fixes[0])).toEqual([
      {
        range: { start: openGap, end: openGap + 1 },
        replacementText: '',
      },
      {
        range: { start: closeGap, end: closeGap + 1 },
        replacementText: '',
      },
      {
        range: { start: defaultGap, end: defaultGap + 1 },
        replacementText: ' = ',
      },
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

  it('runs curly-newline with exact native UTF-8 byte ranges and fixes', () => {
    const source = 'const 日本語 = true; if (日本語) {}';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'sample.ts',
      rules: [{ name: 'curly-newline', options: ['always'] }],
    });
    const opening = Buffer.byteLength('const 日本語 = true; if (日本語) ');
    const closing = opening + 1;

    expect(diagnostics).toEqual([
      {
        ruleName: 'curly-newline',
        messageId: 'expectedLinebreakAfterOpeningBrace',
        message: 'Expected a line break after this opening brace.',
        range: { start: opening, end: opening + 1 },
        suggestions: [
          {
            messageId: 'expectedLinebreakAfterOpeningBrace',
            message: 'Expected a line break after this opening brace.',
            fixes: [{ range: { start: opening + 1, end: opening + 1 }, replacementText: '\n' }],
          },
        ],
      },
      {
        ruleName: 'curly-newline',
        messageId: 'expectedLinebreakBeforeClosingBrace',
        message: 'Expected a line break before this closing brace.',
        range: { start: closing, end: closing + 1 },
        suggestions: [
          {
            messageId: 'expectedLinebreakBeforeClosingBrace',
            message: 'Expected a line break before this closing brace.',
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

  it('runs multiline-comment-style with stable upstream options and exact fixes', () => {
    const source = '  // first\n  // second\n';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'multiline-comment-style', options: ['starred-block'] }],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'multiline-comment-style',
        messageId: 'expectedBlock',
        range: { start: 2, end: 22 },
        suggestions: [
          {
            messageId: 'fixStyle',
            fixes: [
              {
                range: { start: 2, end: 22 },
                replacementText: '/*\n   * first\n   * second\n   */',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('runs indent-binary-ops with exact UTF-8 byte ranges, data, and fixes', () => {
    const source = 'const 日本語 = first\n    + second';
    const lineStart = Buffer.byteLength('const 日本語 = first\n');
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.js',
      rules: [{ name: 'indent-binary-ops', options: [] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'indent-binary-ops',
        messageId: 'wrongIndentation',
        message: 'Expected indentation of 2 spaces',
        data: { expected: '2 spaces' },
        range: { start: lineStart, end: lineStart + 4 },
        suggestions: [
          {
            messageId: 'wrongIndentation',
            message: 'Expected indentation of 2 spaces',
            fixes: [
              {
                range: { start: lineStart, end: lineStart + 4 },
                replacementText: '  ',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('supports indent-binary-ops tab and zero-width options through the native API', () => {
    const source = 'const total = first\n  + second';
    const tabbed = runNativeStylisticLint(source, {
      filename: 'fixture.js',
      rules: [{ name: 'indent-binary-ops', options: ['tab'] }],
    });
    const zero = runNativeStylisticLint(source, {
      filename: 'fixture.js',
      rules: [{ name: 'indent-binary-ops', options: [0] }],
    });

    expect(tabbed).toMatchObject([
      {
        message: 'Expected indentation of 1 tab',
        data: { expected: '1 tab' },
        suggestions: [{ fixes: [{ replacementText: '\t' }] }],
      },
    ]);
    expect(zero).toMatchObject([
      {
        message: 'Expected indentation of 0 spaces',
        data: { expected: '0 spaces' },
        suggestions: [{ fixes: [{ replacementText: '' }] }],
      },
    ]);
  });

  it('runs multiline-ternary with exact native UTF-8 byte ranges and fixes', () => {
    const source = 'const 日本語 = 条件 ? はい : いいえ;';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'multiline-ternary', options: ['always'] }],
    });
    const testStart = Buffer.byteLength('const 日本語 = ');
    const testEnd = Buffer.byteLength('const 日本語 = 条件');
    const consequentStart = Buffer.byteLength('const 日本語 = 条件 ? ');
    const consequentEnd = Buffer.byteLength('const 日本語 = 条件 ? はい');
    const question = Buffer.byteLength('const 日本語 = 条件 ');
    const colon = Buffer.byteLength('const 日本語 = 条件 ? はい ');

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'multiline-ternary',
        messageId: 'expectedTestCons',
        message: 'Expected newline between test and consequent of ternary expression.',
        range: { start: testStart, end: testEnd },
        suggestions: [
          {
            messageId: 'expectedTestCons',
            fixes: [
              {
                range: { start: testEnd, end: question },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'multiline-ternary',
        messageId: 'expectedConsAlt',
        message: 'Expected newline between consequent and alternate of ternary expression.',
        range: { start: consequentStart, end: consequentEnd },
        suggestions: [
          {
            messageId: 'expectedConsAlt',
            fixes: [
              {
                range: { start: consequentEnd, end: colon },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('preserves multiline-ternary multi-edit removals and comment fix suppression', () => {
    const source = 'condition\n?\nconsequent : alternate';
    const diagnostics = runNativeStylisticLint(source, {
      rules: [{ name: 'multiline-ternary', options: ['never'] }],
    });

    expect(diagnostics).toMatchObject([
      {
        messageId: 'unexpectedTestCons',
        range: { start: 0, end: 'condition'.length },
        suggestions: [
          {
            fixes: [
              {
                range: { start: 'condition'.length, end: 'condition\n'.length },
                replacementText: '',
              },
              {
                range: { start: 'condition\n?'.length, end: 'condition\n?\n'.length },
                replacementText: '',
              },
            ],
          },
        ],
      },
    ]);

    const commented = runNativeStylisticLint('condition ? // keep\nconsequent : alternate', {
      rules: [{ name: 'multiline-ternary', options: ['always'] }],
    });
    expect(commented).toMatchObject([
      {
        messageId: 'expectedConsAlt',
      },
    ]);
    expect(commented[0].suggestions).toBeUndefined();
  });

  it('runs brace-style with exact native UTF-8 byte ranges, messages, and fixes', () => {
    const source = 'const 名 = 1;\r\nif (名)\r\n{\r\nrun(); }\r\n';
    const opening = Buffer.byteLength('const 名 = 1;\r\nif (名)\r\n');
    const closing = Buffer.byteLength('const 名 = 1;\r\nif (名)\r\n{\r\nrun(); ');
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'brace-style', options: [] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'brace-style',
        messageId: 'nextLineOpen',
        message: 'Opening curly brace does not appear on the same line as controlling statement.',
        range: { start: opening, end: opening + 1 },
        suggestions: [
          {
            messageId: 'nextLineOpen',
            message:
              'Opening curly brace does not appear on the same line as controlling statement.',
            fixes: [
              {
                range: {
                  start: Buffer.byteLength('const 名 = 1;\r\nif (名)'),
                  end: opening,
                },
                replacementText: ' ',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'brace-style',
        messageId: 'singleLineClose',
        message:
          'Closing curly brace should be on the same line as opening curly brace or on the line after the previous block.',
        range: { start: closing, end: closing + 1 },
        suggestions: [
          {
            messageId: 'singleLineClose',
            message:
              'Closing curly brace should be on the same line as opening curly brace or on the line after the previous block.',
            fixes: [
              {
                range: { start: closing, end: closing },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('runs semi with exact native ranges and FixTracker-compatible fixes', () => {
    expect(
      runNativeStylisticLint('const value = 1\n', {
        filename: 'fixture.js',
        rules: [{ name: 'semi', options: [] }],
      }),
    ).toEqual([
      {
        ruleName: 'semi',
        messageId: 'missingSemi',
        message: 'Missing semicolon.',
        range: { start: 15, end: 16 },
        suggestions: [
          {
            messageId: 'missingSemi',
            message: 'Missing semicolon.',
            fixes: [{ range: { start: 15, end: 15 }, replacementText: ';' }],
          },
        ],
      },
    ]);

    expect(
      runNativeStylisticLint('const value = 1;', {
        filename: 'fixture.js',
        rules: [{ name: 'semi', options: ['never'] }],
      }),
    ).toEqual([
      {
        ruleName: 'semi',
        messageId: 'extraSemi',
        message: 'Extra semicolon.',
        range: { start: 15, end: 16 },
        suggestions: [
          {
            messageId: 'extraSemi',
            message: 'Extra semicolon.',
            fixes: [{ range: { start: 14, end: 16 }, replacementText: '1' }],
          },
        ],
      },
    ]);
  });

  it('runs no-extra-parens with exact UTF-8 byte ranges and paired code fixes', () => {
    const source = 'const 名 = ((value));\nconst f = (x => x);\n';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'no-extra-parens', options: [] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'no-extra-parens',
        messageId: 'unexpected',
        message: 'Unnecessary parentheses around expression.',
        range: { start: 13, end: 14 },
        suggestions: [
          {
            messageId: 'unexpected',
            message: 'Unnecessary parentheses around expression.',
            fixes: [
              { range: { start: 13, end: 14 }, replacementText: '' },
              { range: { start: 19, end: 20 }, replacementText: '' },
            ],
          },
        ],
      },
      {
        ruleName: 'no-extra-parens',
        messageId: 'unexpected',
        message: 'Unnecessary parentheses around expression.',
        range: { start: 33, end: 34 },
        suggestions: [
          {
            messageId: 'unexpected',
            message: 'Unnecessary parentheses around expression.',
            fixes: [
              { range: { start: 33, end: 34 }, replacementText: '' },
              { range: { start: 40, end: 41 }, replacementText: '' },
            ],
          },
        ],
      },
    ]);
  });

  it('does not offer an unsafe no-extra-parens directive fix', () => {
    expect(
      runNativeStylisticLint("('directive');\n", {
        filename: 'fixture.js',
        rules: [{ name: 'no-extra-parens', options: [] }],
      }),
    ).toEqual([
      {
        ruleName: 'no-extra-parens',
        messageId: 'unexpected',
        message: 'Unnecessary parentheses around expression.',
        range: { start: 0, end: 1 },
      },
    ]);
  });

  it('supports allman and allowSingleLine across TypeScript modules and TSX', () => {
    const source = [
      'namespace 名前 { value(); }',
      'const View = () => <Panel value={{ nested: true }} />;',
      'if (ok) { render(<View />); }',
    ].join('\n');
    const strict = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [{ name: 'brace-style', options: ['allman'] }],
    });
    expect(strict.map((diagnostic) => diagnostic.messageId)).toEqual([
      'sameLineOpen',
      'blockSameLine',
      'singleLineClose',
      'sameLineOpen',
      'blockSameLine',
      'singleLineClose',
    ]);
    expect(
      runNativeStylisticLint(source, {
        filename: 'fixture.tsx',
        rules: [{ name: 'brace-style', options: ['allman', { allowSingleLine: true }] }],
      }),
    ).toEqual([]);
  });

  it('reports brace-style comment-separated violations without an unsafe fix', () => {
    const diagnostics = runNativeStylisticLint('if (ok) // preserve\n{\nwork();\n}', {
      filename: 'fixture.js',
      rules: [{ name: 'brace-style', options: [] }],
    });
    expect(diagnostics).toEqual([
      {
        ruleName: 'brace-style',
        messageId: 'nextLineOpen',
        message: 'Opening curly brace does not appear on the same line as controlling statement.',
        range: { start: 20, end: 21 },
      },
    ]);
  });

  it('runs nonblock-statement-body-position with exact UTF-8 ranges and fixes', () => {
    const source = 'if (準備) 実行();';
    const start = Buffer.byteLength(source.slice(0, source.indexOf('実行')));
    const end = Buffer.byteLength(source);
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'nonblock-statement-body-position', options: ['below'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'nonblock-statement-body-position',
        messageId: 'expectLinebreak',
        message: 'Expected a linebreak before this statement.',
        range: { start, end },
        suggestions: [
          {
            messageId: 'expectLinebreak',
            message: 'Expected a linebreak before this statement.',
            fixes: [{ range: { start, end: start }, replacementText: '\n' }],
          },
        ],
      },
    ]);
  });

  it('suppresses unsafe nonblock-statement-body-position comment fixes', () => {
    const source = 'while (ready)\n/* preserve */\nrun();';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.js',
      rules: [{ name: 'nonblock-statement-body-position', options: ['beside'] }],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'nonblock-statement-body-position',
        messageId: 'expectNoLinebreak',
        message: 'Expected no linebreak before this statement.',
      },
    ]);
    expect(diagnostics[0].suggestions).toBeUndefined();
  });

  it('runs jsx-curly-newline with exact native UTF-8 byte ranges and fixes', () => {
    const source = 'const 日本語 = <div>{\r\n値\r\n}</div>;';
    const opening = Buffer.byteLength('const 日本語 = <div>');
    const expressionStart = Buffer.byteLength('const 日本語 = <div>{\r\n');
    const expressionEnd = Buffer.byteLength('const 日本語 = <div>{\r\n値');
    const closing = Buffer.byteLength('const 日本語 = <div>{\r\n値\r\n');
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [{ name: 'jsx-curly-newline', options: ['never'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-curly-newline',
        messageId: 'unexpectedAfter',
        message: "Unexpected newline after '{'.",
        range: { start: opening, end: opening + 1 },
        suggestions: [
          {
            messageId: 'unexpectedAfter',
            message: "Unexpected newline after '{'.",
            fixes: [
              {
                range: { start: opening + 1, end: expressionStart },
                replacementText: '',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'jsx-curly-newline',
        messageId: 'unexpectedBefore',
        message: "Unexpected newline before '}'.",
        range: { start: closing, end: closing + 1 },
        suggestions: [
          {
            messageId: 'unexpectedBefore',
            message: "Unexpected newline before '}'.",
            fixes: [
              {
                range: { start: expressionEnd, end: closing },
                replacementText: '',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('keeps jsx-curly-newline comment-separated removals unfixable', () => {
    const diagnostics = runNativeStylisticLint('<div>{ /* keep */\nfoo }</div>', {
      filename: 'fixture.tsx',
      rules: [{ name: 'jsx-curly-newline', options: ['never'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-curly-newline',
        messageId: 'unexpectedAfter',
        message: "Unexpected newline after '{'.",
        range: { start: 5, end: 6 },
      },
    ]);
  });

  it('runs type-named-tuple-spacing with exact UTF-8 ranges and replacement fixes', () => {
    const source = 'type 日本語 = [value :  number];';
    const start = Buffer.byteLength(source.slice(0, source.indexOf('value')));
    const end = Buffer.byteLength(source.slice(0, source.indexOf(']')));
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'type-named-tuple-spacing', options: [] }],
    });
    const suggestion = (messageId, message) => ({
      messageId,
      message,
      fixes: [{ range: { start, end }, replacementText: 'value: number' }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'type-named-tuple-spacing',
        messageId: 'unexpectedSpaceBefore',
        message: "Unexpected space before the ':'.",
        range: { start, end },
        suggestions: [suggestion('unexpectedSpaceBefore', "Unexpected space before the ':'.")],
      },
      {
        ruleName: 'type-named-tuple-spacing',
        messageId: 'expectedSpaceAfter',
        message: "Expected a space after the ':'.",
        range: { start, end },
        suggestions: [suggestion('expectedSpaceAfter', "Expected a space after the ':'.")],
      },
    ]);
  });

  it('runs jsx-closing-bracket-location with exact UTF-8 ranges, data, and fixes', () => {
    const source = 'const marker = "😀"; const view = <Panel\n  prop />;';
    const slash = Buffer.byteLength(source.slice(0, source.indexOf('/>')));
    const propEnd = Buffer.byteLength(source.slice(0, source.indexOf('prop') + 'prop'.length));
    const elementEnd = Buffer.byteLength(source.slice(0, source.indexOf('/>') + 2));
    const message =
      'The closing bracket must be aligned with the opening tag (expected column 35 on the next line)';
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [
        {
          name: 'jsx-closing-bracket-location',
          options: [{ location: 'tag-aligned' }],
        },
      ],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-closing-bracket-location',
        messageId: 'bracketLocation',
        message,
        data: {
          details: ' (expected column 35 on the next line)',
          location: 'aligned with the opening tag',
        },
        range: { start: slash, end: slash + 1 },
        suggestions: [
          {
            messageId: 'bracketLocation',
            message,
            fixes: [
              {
                range: { start: propEnd, end: elementEnd },
                replacementText: `\n${' '.repeat(34)}/>`,
              },
            ],
          },
        ],
      },
    ]);
  });
  it('runs jsx-curly-spacing with exact bytes, ordering, data, and fixes', () => {
    const source = 'const marker = "😀"; const view = <App attr={value}>{ child }</App>;';
    const attributeOpen = Buffer.byteLength(source.slice(0, source.indexOf('{value}')));
    const attributeClose = attributeOpen + '{value'.length;
    const childOpen = Buffer.byteLength(source.slice(0, source.indexOf('{ child }')));
    const childClose = childOpen + '{ child '.length;
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [
        {
          name: 'jsx-curly-spacing',
          options: [
            {
              attributes: { when: 'always' },
              children: { when: 'never' },
            },
          ],
        },
      ],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-curly-spacing',
        messageId: 'spaceNeededAfter',
        message: "A space is required after '{'",
        data: { token: '{' },
        range: { start: attributeOpen, end: attributeOpen + 1 },
        suggestions: [
          {
            messageId: 'spaceNeededAfter',
            message: "A space is required after '{'",
            fixes: [
              {
                range: { start: attributeOpen + 1, end: attributeOpen + 1 },
                replacementText: ' ',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'jsx-curly-spacing',
        messageId: 'spaceNeededBefore',
        message: "A space is required before '}'",
        data: { token: '}' },
        range: { start: attributeClose, end: attributeClose + 1 },
        suggestions: [
          {
            messageId: 'spaceNeededBefore',
            message: "A space is required before '}'",
            fixes: [
              {
                range: { start: attributeClose, end: attributeClose },
                replacementText: ' ',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'jsx-curly-spacing',
        messageId: 'noSpaceAfter',
        message: "There should be no space after '{'",
        data: { token: '{' },
        range: { start: childOpen, end: childOpen + 1 },
        suggestions: [
          {
            messageId: 'noSpaceAfter',
            message: "There should be no space after '{'",
            fixes: [
              {
                range: { start: childOpen + 1, end: childOpen + 2 },
                replacementText: '',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'jsx-curly-spacing',
        messageId: 'noSpaceBefore',
        message: "There should be no space before '}'",
        data: { token: '}' },
        range: { start: childClose, end: childClose + 1 },
        suggestions: [
          {
            messageId: 'noSpaceBefore',
            message: "There should be no space before '}'",
            fixes: [
              {
                range: { start: childClose - 1, end: childClose },
                replacementText: '',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('runs jsx-first-prop-new-line with exact UTF-8 attribute ranges and fixes', () => {
    const source = [
      'const 日本語 = <UI.Root first="one" second />;',
      'const view = <svg:path',
      'xml:lang="ja" />;',
    ].join('\n');
    const firstStart = Buffer.byteLength(source.slice(0, source.indexOf('first="one"')));
    const firstEnd = firstStart + Buffer.byteLength('first="one"');
    const memberNameEnd = Buffer.byteLength(source.slice(0, source.indexOf(' first="one"')));
    const namespacedStart = Buffer.byteLength(source.slice(0, source.indexOf('xml:lang')));
    const namespacedEnd = namespacedStart + Buffer.byteLength('xml:lang="ja"');
    const namespacedNameEnd = Buffer.byteLength(
      source.slice(0, source.indexOf('\nxml:lang', source.indexOf('<svg:path'))),
    );
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.tsx',
      rules: [{ name: 'jsx-first-prop-new-line', options: ['multiprop'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'jsx-first-prop-new-line',
        messageId: 'propOnNewLine',
        message: 'Property should be placed on a new line',
        range: { start: firstStart, end: firstEnd },
        suggestions: [
          {
            messageId: 'propOnNewLine',
            message: 'Property should be placed on a new line',
            fixes: [
              {
                range: { start: memberNameEnd, end: firstStart },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
      {
        ruleName: 'jsx-first-prop-new-line',
        messageId: 'propOnSameLine',
        message: 'Property should be placed on the same line as the component declaration',
        range: { start: namespacedStart, end: namespacedEnd },
        suggestions: [
          {
            messageId: 'propOnSameLine',
            message: 'Property should be placed on the same line as the component declaration',
            fixes: [
              {
                range: { start: namespacedNameEnd, end: namespacedStart },
                replacementText: ' ',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('runs padding-line-between-statements with exact UTF-8 ranges and insertion fixes', () => {
    const source = 'const 日本語 = 1;\nuse();';
    const statementStart = Buffer.byteLength('const 日本語 = 1;\n');
    const statementEnd = Buffer.byteLength(source);
    const insertAt = Buffer.byteLength('const 日本語 = 1;');
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [
        {
          name: 'padding-line-between-statements',
          options: [{ blankLine: 'always', prev: 'const', next: '*' }],
        },
      ],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'padding-line-between-statements',
        messageId: 'expectedBlankLine',
        message: 'Expected blank line before this statement.',
        range: { start: statementStart, end: statementEnd },
        suggestions: [
          {
            messageId: 'expectedBlankLine',
            message: 'Expected blank line before this statement.',
            fixes: [
              {
                range: { start: insertAt, end: insertAt },
                replacementText: '\n',
              },
            ],
          },
        ],
      },
    ]);
  });

  it('does not offer an unsafe padding removal across two comment-separated sequences', () => {
    const diagnostics = runNativeStylisticLint('foo();\n\n// preserve\n\nbar();', {
      filename: 'fixture.js',
      rules: [
        {
          name: 'padding-line-between-statements',
          options: [{ blankLine: 'never', prev: '*', next: '*' }],
        },
      ],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'padding-line-between-statements',
        messageId: 'unexpectedBlankLine',
        message: 'Unexpected blank line before this statement.',
      },
    ]);
    expect(diagnostics[0].suggestions).toBeUndefined();
  });

  it('runs wrap-iife with exact UTF-8 byte ranges and code fixes', () => {
    const source = "const 日本語 = function () { return '😀'; }();";
    const callStart = Buffer.byteLength(source.slice(0, source.indexOf('function')));
    const functionEnd = Buffer.byteLength(source.slice(0, source.indexOf('}') + 1));
    const callEnd = Buffer.byteLength(source.slice(0, source.lastIndexOf(';')));
    const functionText = source.slice(source.indexOf('function'), source.indexOf('}') + 1);
    const diagnostics = runNativeStylisticLint(source, {
      filename: 'fixture.ts',
      rules: [{ name: 'wrap-iife', options: ['inside'] }],
    });

    expect(diagnostics).toEqual([
      {
        ruleName: 'wrap-iife',
        messageId: 'wrapInvocation',
        message: 'Wrap an immediate function invocation in parentheses.',
        range: { start: callStart, end: callEnd },
        suggestions: [
          {
            messageId: 'wrapInvocation',
            message: 'Wrap an immediate function invocation in parentheses.',
            fixes: [
              {
                range: { start: callStart, end: functionEnd },
                replacementText: `(${functionText})`,
              },
            ],
          },
        ],
      },
    ]);
  });
});
