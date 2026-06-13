import { describe, expect, it } from 'vitest';

import { implementedEslintMarkdownRuleNames, scanEslintMarkdown } from '../api.js';

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

function applyFix(sourceText, fix) {
  return sourceText.slice(0, fix.start) + fix.replacement + sourceText.slice(fix.end);
}

describe('@eslint/markdown native API', () => {
  it('exposes all ported rule names', () => {
    expect(implementedEslintMarkdownRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans representative Markdown for all implemented rules', () => {
    const source = [
      '---',
      'title: Example',
      '---',
      '',
      '```',
      'plain',
      '```',
      '',
      '```js',
      'console.log(1);',
      '```',
      '',
      '#Title',
      '# Before',
      '### Skipped',
      '# Title',
      '# Title',
      '',
      '[foo]: #',
      '[foo]: https://example.com',
      '[ref]: https://example.com',
      '[unused]: https://example.com',
      '[missing][nope]',
      '[empty]()',
      '![](#)',
      '[resource](ref)',
      '[broken](#missing)',
      '(label)[https://example.com]',
      'https://example.com',
      '* spaced *',
      '[ ][]',
      '<div><img src="x"></div>',
      '',
      '| a | b |',
      '| --- | --- |',
      '| 1 | 2 | 3 |',
    ].join('\n');

    const ruleNames = new Set(scanEslintMarkdown(source).map((diagnostic) => diagnostic.ruleName));

    expect([...ruleNames].sort()).toEqual([...expectedRuleNames].sort());
  });

  it('supports fenced-code-language required language options', () => {
    const diagnostics = scanEslintMarkdown('```ts\nlet value = 1;\n```', {
      ruleNames: ['fenced-code-language'],
      requiredCodeLanguages: ['js'],
    });

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'fenced-code-language',
        messageId: 'disallowedLanguage',
        data: { lang: 'ts' },
      },
    ]);
  });

  it('fixes bare URLs as autolinks', () => {
    const source = 'See https://example.com/path.';
    const diagnostics = scanEslintMarkdown(source, { ruleNames: ['no-bare-urls'] });

    expect(diagnostics).toHaveLength(1);
    expect(applyFix(source, diagnostics[0].fix)).toBe('See <https://example.com/path>.');
  });

  it('honors HTML allow lists and fragment allow patterns', () => {
    expect(
      scanEslintMarkdown('<div id="ok">x</div>\n[figure](#figure-1)', {
        ruleNames: ['no-html', 'no-missing-link-fragments'],
        allowedHtml: ['DIV'],
        allowedHtmlIgnoreCase: true,
        allowFragmentPattern: '^figure-',
      }),
    ).toEqual([]);
  });

  it('can disable frontmatter H1 detection through frontmatterTitle', () => {
    expect(
      scanEslintMarkdown('---\ntitle: Example\n---\n# Title', {
        ruleNames: ['no-multiple-h1'],
        frontmatterTitle: '',
      }),
    ).toEqual([]);
  });

  it('can check missing table cells when configured', () => {
    const source = ['| a | b |', '| --- | --- |', '| 1 |'].join('\n');

    expect(
      scanEslintMarkdown(source, {
        ruleNames: ['table-column-count'],
        checkMissingTableCells: true,
      }),
    ).toMatchObject([
      {
        ruleName: 'table-column-count',
        messageId: 'missingCells',
        data: { expectedCells: 2, actualCells: 1 },
      },
    ]);
  });
});
