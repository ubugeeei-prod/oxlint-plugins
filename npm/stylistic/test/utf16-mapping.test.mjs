// Regression tests for byte-offset to UTF-16 offset mapping inside the
// stylistic plugin. The native Rust layer returns byte offsets, but Oxlint
// reports source ranges in UTF-16 code units. The wrapper bridges the two via
// `createByteToUtf16Mapper`, exercised here indirectly through the rule API
// using sources that contain multi-byte and surrogate-pair characters.

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

function runRule(ruleName, sourceText, options = []) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const rule = plugin.rules[ruleName];
  const visitor = rule.createOnce({
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

describe('stylistic UTF-16 byte mapping', () => {
  it('maps an ASCII-only source 1:1 (fast path)', () => {
    const sourceText = 'const label = "value";\n';
    const reports = runRule('quotes', sourceText, ['single']);
    expect(reports).toHaveLength(1);
    expect(reports[0].node.range).toEqual([
      sourceText.indexOf('"value"'),
      sourceText.indexOf('"value"') + '"value"'.length,
    ]);
  });

  it('maps positions that follow multi-byte text correctly', () => {
    const sourceText = '// 日本語\nconst label = "value";\n';
    const reports = runRule('quotes', sourceText, ['single']);

    expect(reports).toHaveLength(1);
    const expectedStart = sourceText.indexOf('"value"');
    const expectedEnd = expectedStart + '"value"'.length;
    expect(reports[0].node.range).toEqual([expectedStart, expectedEnd]);
  });

  it('handles multi-byte characters that appear before AND after the report', () => {
    const sourceText = '// 日本語\nconst label = "value";\n// 한국어\n';
    const reports = runRule('quotes', sourceText, ['single']);

    expect(reports).toHaveLength(1);
    const expectedStart = sourceText.indexOf('"value"');
    const expectedEnd = expectedStart + '"value"'.length;
    expect(reports[0].node.range).toEqual([expectedStart, expectedEnd]);
  });

  it('maps surrogate-pair (astral) characters correctly', () => {
    // 🦀 is a 4-byte UTF-8 character that takes 2 UTF-16 code units.
    const sourceText = '// 🦀\nconst label = "value";\n';
    const reports = runRule('quotes', sourceText, ['single']);

    expect(reports).toHaveLength(1);
    const expectedStart = sourceText.indexOf('"value"');
    const expectedEnd = expectedStart + '"value"'.length;
    expect(reports[0].node.range).toEqual([expectedStart, expectedEnd]);
  });

  it('maps reports correctly when the multi-byte character is immediately adjacent', () => {
    // The non-ASCII char is right next to the violation.
    const sourceText = '/*🦀*/"value";\n';
    const reports = runRule('quotes', sourceText, ['single']);

    expect(reports).toHaveLength(1);
    const expectedStart = sourceText.indexOf('"value"');
    const expectedEnd = expectedStart + '"value"'.length;
    expect(reports[0].node.range).toEqual([expectedStart, expectedEnd]);
  });

  it('maps suggestion fix ranges through the same UTF-16 mapping', () => {
    const sourceText = '// 日本語\nconst a = [\n  1\n]\n';
    const reports = runRule('comma-dangle', sourceText, ['always']);

    expect(reports).toHaveLength(1);
    const insertAt = sourceText.indexOf('1') + 1;
    expect(reports[0].node.range).toEqual([insertAt, insertAt]);

    const fixes = reports[0].suggest[0].fix({
      replaceTextRange(range, replacementText) {
        return { range, replacementText };
      },
    });
    expect(fixes).toEqual([{ range: [insertAt, insertAt], replacementText: ',' }]);
  });

  it('maps jsx-quotes reports and fixes around BMP and astral JSX text', () => {
    const sourceText = "const node = <日本語 emoji='😀' title='値' />;\n";
    const reports = runRule('jsx-quotes', sourceText);

    expect(reports).toHaveLength(2);
    for (const [report, literal, replacementText] of [
      [reports[0], "'😀'", '"😀"'],
      [reports[1], "'値'", '"値"'],
    ]) {
      const start = sourceText.indexOf(literal);
      const range = [start, start + literal.length];
      expect(report.node.range).toEqual(range);
      expect(
        report.suggest[0].fix({
          replaceTextRange(fixRange, text) {
            return { range: fixRange, replacementText: text };
          },
        }),
      ).toEqual([{ range, replacementText }]);
    }
  });

  it('maps no-mixed-operators ranges after BMP and astral Unicode', () => {
    const sourceText = 'const 日本語 = value; // 🦀\nconst result = 日本語 + left * right;\n';
    const reports = runRule('no-mixed-operators', sourceText);
    const plus = sourceText.indexOf('+');
    const star = sourceText.indexOf('*');

    expect(reports.map((report) => report.node.range)).toEqual([
      [plus, plus + 1],
      [star, star + 1],
    ]);
    expect(reports.map((report) => report.data)).toEqual([
      { leftOperator: '+', rightOperator: '*' },
      { leftOperator: '+', rightOperator: '*' },
    ]);
  });

  it('maps array-element-newline reports and fixes after BMP and astral text', () => {
    const sourceText = 'const 日本語 = "🦀";\nconst list = [日本語, 次];\n';
    const reports = runRule('array-element-newline', sourceText);
    const start = sourceText.indexOf('次') - 1;

    expect(reports).toHaveLength(1);
    expect(reports[0].node.range).toEqual([start, start + 1]);
    expect(
      reports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([{ range: [start, start + 1], replacementText: '\n' }]);
  });

  it('clamps offsets that fall past the end of the source', () => {
    // An empty source should not crash even though the mapper has nothing to walk.
    const sourceText = '';
    expect(() => runRule('no-trailing-spaces', sourceText)).not.toThrow();
  });

  it('returns identical reports on repeated invocations with the same sourceCode and options', () => {
    const sourceText = 'const a = "value";\n';
    const sourceCode = {
      text: sourceText,
      getText() {
        return this.text;
      },
    };

    const reports = [];
    const rule = plugin.rules.quotes;
    const visitor = rule.createOnce({
      options: ['single'],
      sourceCode,
      report: (descriptor) => reports.push(descriptor),
    });

    visitor.Program({ type: 'Program', range: [0, sourceText.length] });
    const firstRunLength = reports.length;
    visitor.Program({ type: 'Program', range: [0, sourceText.length] });

    expect(firstRunLength).toBe(1);
    expect(reports).toHaveLength(2);
    // Both runs must produce the same byte->UTF16 mapped range.
    expect(reports[0].node.range).toEqual(reports[1].node.range);
  });

  it('preserves byte mapping consistency when the source has many multi-byte runs', () => {
    const segments = ['日', '本', '語', '🦀', 'a', 'b', '한', '국', '어'];
    let sourceText = '';
    for (let i = 0; i < 20; i += 1) {
      sourceText += `// ${segments[i % segments.length]}\n`;
    }
    sourceText += 'const label = "value";\n';

    const reports = runRule('quotes', sourceText, ['single']);
    expect(reports).toHaveLength(1);

    const expectedStart = sourceText.indexOf('"value"');
    const expectedEnd = expectedStart + '"value"'.length;
    expect(reports[0].node.range).toEqual([expectedStart, expectedEnd]);
  });
});
