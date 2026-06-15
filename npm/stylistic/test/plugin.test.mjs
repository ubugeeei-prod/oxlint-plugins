import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const stylisticRuleFixtures = [
  ['eol-last', 'const x = 1;', [], ['missing']],
  ['linebreak-style', 'const x = 1;\r\n', ['unix'], ['expectedUnix']],
  ['no-multiple-empty-lines', 'const a = 1;\n\n\nconst b = 2;\n', [{ max: 1 }], ['tooMany']],
  ['no-mixed-spaces-and-tabs', 'function f() {\n\t return 1;\n}\n', [], ['mixedSpacesAndTabs']],
  ['no-tabs', 'const\tlabel = 1;\n', [], ['unexpectedTab']],
  ['no-trailing-spaces', 'const x = 1;  \n', [], ['trailingSpace']],
  ['quotes', 'const label = "value";\n', ['single'], ['wrongQuote']],
  ['unicode-bom', '\u{feff}const x = 1;\n', ['never'], ['unexpected']],
  ['arrow-spacing', 'const f = ()=>1;\n', [], ['expectedBefore', 'expectedAfter']],
  ['comma-spacing', '[1 ,2]\n', [], ['unexpected', 'missing']],
  ['semi-spacing', 'a ;b\n', [], ['unexpected', 'missing']],
  ['space-in-parens', 'f( a )\n', [], ['rejectedOpeningSpace', 'rejectedClosingSpace']],
  ['template-curly-spacing', '`${ x }`\n', [], ['unexpectedAfter', 'unexpectedBefore']],
  ['rest-spread-spacing', 'f(... args)\n', [], ['unexpectedWhitespace']],
  ['no-multi-spaces', 'a  =  b\n', [], ['multipleSpaces', 'multipleSpaces']],
  ['no-whitespace-before-property', 'foo .bar\n', [], ['unexpectedWhitespace']],
  ['dot-location', 'foo\n.bar\n', [], ['expectedDotAfterObject']],
  ['spaced-comment', '//x\n', [], ['expectedSpaceAfter']],
  [
    'object-curly-spacing',
    'const o = { a: 1 };\n',
    [],
    ['unexpectedSpaceAfter', 'unexpectedSpaceBefore'],
  ],
  [
    'array-bracket-spacing',
    'const a = [ 1, 2 ];\n',
    [],
    ['unexpectedSpaceAfter', 'unexpectedSpaceBefore'],
  ],
  ['computed-property-spacing', 'a[ 0 ];\n', [], ['unexpectedSpaceAfter', 'unexpectedSpaceBefore']],
  ['block-spacing', 'function f() {g();}\n', [], ['missing', 'missing']],
  ['padded-blocks', 'if (x) {\n  y();\n}\n', [], ['missingPadBlock', 'missingPadBlock']],
  ['space-before-blocks', 'if (x){ y(); }\n', [], ['missingSpace']],
  ['function-call-spacing', 'foo ();\n', [], ['unexpectedWhitespace']],
  ['space-before-function-paren', 'function f() {}\n', [], ['missingSpace']],
  ['no-floating-decimal', 'const x = .5;\n', [], ['leading']],
  ['template-tag-spacing', 'tag `hello`;\n', [], ['unexpectedSpace']],
  [
    'yield-star-spacing',
    'function* g() { yield *h(); }\n',
    [],
    ['unexpectedBefore', 'missingAfter'],
  ],
  ['generator-star-spacing', 'function* g() {}\n', [], ['missingBefore', 'unexpectedAfter']],
  ['comma-dangle', 'const a = [1, 2,];\n', [], ['unexpected']],
  ['space-infix-ops', 'const x = a+b;\n', [], ['missingSpace']],
  ['max-len', 'const abcdefghij = 1;\n', [{ code: 10 }], ['tooLong']],
  ['max-statements-per-line', 'const a = 1; const b = 2;\n', [], ['exceed']],
  ['semi-style', 'foo()\n;[1].forEach(bar)\n', [], ['expectedSemiColon']],
  ['comma-style', 'const a = [\n  1\n  , 2\n];\n', [], ['expectedCommaLast']],
  ['arrow-parens', 'const f = a => a;\n', [], ['expectedParens']],
  [
    'switch-colon-spacing',
    'switch (x) { case 0 :foo(); }\n',
    [],
    ['unexpectedSpaceBefore', 'expectedSpaceAfter'],
  ],
  ['key-spacing', 'const o = {foo :1};\n', [], ['extraKey', 'missingValue']],
  ['quote-props', 'const o = {foo: 1};\n', [], ['unquotedPropertyFound']],
  ['no-extra-semi', 'var x = 5;;\n', [], ['unexpected']],
  ['new-parens', 'var x = new Person;\n', [], ['missing']],
  ['space-unary-ops', '++ foo\n', [], ['nonwordOperatorAfter']],
  ['wrap-regex', '/foo/.test(bar);\n', [], ['requireParens']],
  ['implicit-arrow-linebreak', 'const f = (a) =>\n  a;\n', [], ['unexpectedLinebreak']],
  ['operator-linebreak', 'const x = 1\n  + 2;\n', [], ['operatorAtBeginning']],
  ['keyword-spacing', 'if(foo) {}\n', [], ['missingAfter']],
  ['line-comment-position', 'value; // inline\n// above\n', [], ['above']],
  ['one-var-declaration-per-line', 'var a, b = 0;\n', [], ['expectVarOnNewline']],
];

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

function messageIds(reports) {
  return reports.map((report) => report.messageId);
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((a, b) => a.localeCompare(b));

  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }

  return candidates[candidates.length - 1];
}

describe('stylistic plugin', () => {
  it('exports the stylistic plugin surface', () => {
    expect(plugin.corsaStylisticPlugin).toBe(plugin);
    expect(plugin.implementedStylisticRuleNames).toContain('quotes');
    expect(Object.keys(plugin.rules)).toContain('no-trailing-spaces');
  });

  it('has a fixture for every native stylistic rule', () => {
    expect(
      stylisticRuleFixtures.map(([ruleName]) => ruleName).sort((a, b) => a.localeCompare(b)),
    ).toEqual([...plugin.implementedStylisticRuleNames].sort((a, b) => a.localeCompare(b)));
  });

  it.each(stylisticRuleFixtures)(
    'reports %s through direct rule options',
    (ruleName, sourceText, options, expectedMessageIds) => {
      expect(messageIds(runRule(ruleName, sourceText, options))).toEqual(expectedMessageIds);
    },
  );

  it.each(stylisticRuleFixtures)(
    'reports %s through shared stylistic settings',
    (ruleName, sourceText, options, expectedMessageIds) => {
      expect(
        messageIds(
          runRule(ruleName, sourceText, [], {
            corsaStylistic: {
              rules: {
                [ruleName]: options,
              },
            },
          }),
        ),
      ).toEqual(expectedMessageIds);
    },
  );

  it('reports direct rule options', () => {
    expect(runRule('quotes', 'const label = "value";\n', ['single'])).toMatchObject([
      {
        messageId: 'wrongQuote',
        node: { range: [14, 21] },
      },
    ]);
  });

  it('reports no-mixed-spaces-and-tabs with upstream-style ranges', () => {
    const sourceText = '\t return x;\n   \tfoo\n';
    const line2Start = sourceText.indexOf('   \tfoo');

    expect(runRule('no-mixed-spaces-and-tabs', sourceText, [])).toMatchObject([
      {
        messageId: 'mixedSpacesAndTabs',
        node: { range: [0, 2] },
      },
      {
        messageId: 'mixedSpacesAndTabs',
        node: { range: [line2Start + 2, line2Start + 4] },
      },
    ]);
  });

  it('honors no-mixed-spaces-and-tabs smart-tabs from shared settings', () => {
    const sourceText = '\t    aligned\n\t\t\t   \tbad\n';
    const badLineStart = sourceText.indexOf('\t\t\t   \tbad');

    expect(
      runRule('no-mixed-spaces-and-tabs', sourceText, [], {
        corsaStylistic: {
          rules: {
            'no-mixed-spaces-and-tabs': ['smart-tabs'],
          },
        },
      }),
    ).toMatchObject([
      {
        messageId: 'mixedSpacesAndTabs',
        node: { range: [badLineStart + 5, badLineStart + 7] },
      },
    ]);
  });

  it('skips no-mixed-spaces-and-tabs inside comment continuations and literals', () => {
    const sourceText = "/*\n \t ignored\n*/\n'\\\n \t literal';\n`\n \t template\n`;\n";

    expect(runRule('no-mixed-spaces-and-tabs', sourceText, [])).toEqual([]);
  });

  it('supports one-var-declaration-per-line modes and fixes', () => {
    expect(
      messageIds(runRule('one-var-declaration-per-line', 'var a, b;\n', ['initializations'])),
    ).toEqual([]);

    const reports = runRule('one-var-declaration-per-line', 'var a, b;\n', ['always']);
    expect(reports).toMatchObject([{ messageId: 'expectVarOnNewline' }]);
    expect(
      reports[0].suggest?.[0]?.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([{ range: [7, 7], replacementText: '\n' }]);
  });

  it('ignores one-var-declaration-per-line inside for headers', () => {
    expect(
      messageIds(
        runRule('one-var-declaration-per-line', 'for (let a = 0, b = 0;;) {}\n', ['always']),
      ),
    ).toEqual([]);
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

  it('honors line-comment-position options and ignore patterns', () => {
    expect(
      runRule('line-comment-position', '// jscs: disable\nvalue;\n', [
        { position: 'beside', applyDefaultIgnorePatterns: false },
      ]),
    ).toMatchObject([
      {
        messageId: 'beside',
        node: { range: [0, 16] },
      },
    ]);

    expect(
      messageIds(
        runRule('line-comment-position', 'value; // linter\nvalue; // invalid\n', [
          { position: 'above', ignorePattern: 'linter|pragma' },
        ]),
      ),
    ).toEqual(['above']);

    expect(
      runRule(
        'line-comment-position',
        'value; // eslint-disable-line\nvalue; // global NAME\n',
        [],
      ),
    ).toEqual([]);
  });

  it('runs line-comment-position through shared stylistic settings', () => {
    expect(
      messageIds(
        runRule('line-comment-position', '// above\nvalue; // beside\n', [], {
          corsaStylistic: {
            rules: {
              'line-comment-position': ['beside'],
            },
          },
        }),
      ),
    ).toEqual(['beside']);
  });

  it('works through oxlint jsPlugins config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.js');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(sourcePath, '"value";  \n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          settings: {
            corsaStylistic: {
              rules: {
                quotes: ['single'],
                'no-trailing-spaces': [],
              },
            },
          },
          rules: {
            'stylistic/quotes': 'error',
            'stylistic/no-trailing-spaces': 'error',
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        {
          encoding: 'utf8',
        },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(no-trailing-spaces)',
        'stylistic(quotes)',
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
});
