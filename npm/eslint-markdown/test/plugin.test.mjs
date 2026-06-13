import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const expectedRuleNames = [
  'fenced-code-language',
  'fenced-code-meta',
  'heading-increment',
  'no-bare-urls',
  'no-duplicate-definitions',
  'no-duplicate-headings',
  'no-empty-definitions',
  'no-empty-images',
  'no-empty-links',
  'no-html',
  'no-invalid-label-refs',
  'no-missing-atx-heading-space',
  'no-missing-label-refs',
  'no-missing-link-fragments',
  'no-multiple-h1',
  'no-reference-like-urls',
  'no-reversed-media-syntax',
  'no-space-in-emphasis',
  'no-unused-definitions',
  'require-alt-text',
  'table-column-count',
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
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function applyFix(sourceText, report) {
  const fix = report.fix({
    replaceTextRange(range, replacement) {
      return { range, text: replacement };
    },
  });
  return sourceText.slice(0, fix.range[0]) + fix.text + sourceText.slice(fix.range[1]);
}

describe('@eslint/markdown plugin shape', () => {
  it('exposes all ported rules and native helpers', () => {
    expect(plugin.meta?.name).toBe('@eslint/markdown');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedEslintMarkdownRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanEslintMarkdown).toBe('function');
  });

  it('ships recommended and all configs', () => {
    expect(plugin.configs.recommended.rules['markdown/fenced-code-language']).toBe('error');
    expect(plugin.configs.recommended.rules['markdown/fenced-code-meta']).toBeUndefined();
    expect(plugin.configs.all.rules['markdown/fenced-code-meta']).toBe('error');
    expect(Object.keys(plugin.configs.all.rules)).toHaveLength(expectedRuleNames.length);
  });
});

describe('@eslint/markdown rules through direct adapter harness', () => {
  it('reports and fixes missing ATX heading spaces', () => {
    const source = '###Heading';
    const reports = runRule('no-missing-atx-heading-space', source);

    expect(reports).toHaveLength(1);
    expect(reports[0]).toMatchObject({
      messageId: 'missingSpace',
      data: { position: 'after' },
    });
    expect(applyFix(source, reports[0])).toBe('### Heading');
  });

  it('maps rule options before scanning', () => {
    expect(
      runRule('no-html', '<DIV>x</DIV>', [{ allowed: ['div'], allowedIgnoreCase: true }]),
    ).toEqual([]);
    expect(runRule('fenced-code-meta', '```js title="x"\n```', ['never'])).toMatchObject([
      { messageId: 'disallowedMetadata' },
    ]);
  });

  it('reports table column counts with interpolated data', () => {
    const reports = runRule(
      'table-column-count',
      ['| a | b |', '| --- | --- |', '| 1 | 2 | 3 |'].join('\n'),
    );

    expect(reports).toMatchObject([
      {
        messageId: 'extraCells',
        data: { expectedCells: '2', actualCells: '3' },
      },
    ]);
  });
});
