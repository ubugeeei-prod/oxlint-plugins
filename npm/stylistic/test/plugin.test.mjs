import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const linesAroundCommentFixture = JSON.parse(
  readFileSync(new URL('./fixtures/lines-around-comment-v5.10.0.json', import.meta.url), 'utf8'),
);
const functionParenNewlineFixture = JSON.parse(
  readFileSync(new URL('./fixtures/function-paren-newline-v5.10.0.json', import.meta.url), 'utf8'),
);

const stylisticRuleFixtures = [
  ['eol-last', 'const x = 1;', [], ['missing']],
  ['linebreak-style', 'const x = 1;\r\n', ['unix'], ['expectedUnix']],
  ['no-multiple-empty-lines', 'const a = 1;\n\n\nconst b = 2;\n', [{ max: 1 }], ['tooMany']],
  ['no-mixed-spaces-and-tabs', 'function f() {\n\t return 1;\n}\n', [], ['mixedSpacesAndTabs']],
  [
    'no-mixed-operators',
    'const result = a + b * c;\n',
    [],
    ['unexpectedMixedOperator', 'unexpectedMixedOperator'],
  ],
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
  ['array-element-newline', 'const a = [1, 2];\n', [], ['missingLineBreak']],
  ['computed-property-spacing', 'a[ 0 ];\n', [], ['unexpectedSpaceAfter', 'unexpectedSpaceBefore']],
  ['block-spacing', 'function f() {g();}\n', [], ['missing', 'missing']],
  ['padded-blocks', 'if (x) {\n  y();\n}\n', [], ['missingPadBlock', 'missingPadBlock']],
  ['space-before-blocks', 'if (x){ y(); }\n', [], ['missingSpace']],
  ['function-call-argument-newline', 'fn(first, second);\n', [], ['missingLineBreak']],
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
  ['lines-around-comment', 'before();\n/** docs */\nafter();\n', [], ['before']],
  ['jsx-quotes', "<App title='value' />;\n", [], ['unexpected']],
  ['multiline-comment-style', '// first\n// second\n', [], ['expectedBlock']],
  ['lines-between-class-members', 'class C { a() {}\nb() {} }\n', [], ['always']],
  [
    'array-bracket-newline',
    'const values = [1, 2];\n',
    ['always'],
    ['missingOpeningLinebreak', 'missingClosingLinebreak'],
  ],
  ['newline-per-chained-call', 'first().second().third();\n', [], ['expected']],
  ['one-var-declaration-per-line', 'var a, b = 0;\n', [], ['expectVarOnNewline']],
  ['jsx-equals-spacing', '<App foo = {bar} />;\n', [], ['noSpaceBefore', 'noSpaceAfter']],
  ['no-confusing-arrow', 'const f = value => value ? yes : no;\n', [], ['confusing']],
  [
    'type-annotation-spacing',
    'const value :string = 1;\n',
    [],
    ['expectedSpaceAfter', 'unexpectedSpaceBefore'],
  ],
  [
    'function-paren-newline',
    'function value(first, second) {}\n',
    ['always'],
    ['expectedAfter', 'expectedBefore'],
  ],
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

function offsetAt(source, line, column) {
  const lines = source.split('\n');
  return (
    lines.slice(0, line - 1).reduce((offset, value) => offset + value.length + 1, 0) + column - 1
  );
}

function utf16OffsetForLocation(sourceText, line, column) {
  let offset = 0;
  let currentLine = 1;
  while (currentLine < line && offset < sourceText.length) {
    const character = sourceText[offset];
    if (character === '\r') {
      offset += sourceText[offset + 1] === '\n' ? 2 : 1;
      currentLine += 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      offset += 1;
      currentLine += 1;
    } else {
      offset += 1;
    }
  }
  return offset + column - 1;
}

function expectedReportRange(sourceText, reportRange) {
  const [line, column, endLine, endColumn] = reportRange;
  return [
    utf16OffsetForLocation(sourceText, line, column),
    utf16OffsetForLocation(sourceText, endLine, endColumn),
  ];
}

function reportFix(report) {
  if (!report.suggest?.[0]) {
    return null;
  }
  return report.suggest[0].fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  })[0];
}

function applyReportFixes(sourceText, reports) {
  const fixes = reports.map(reportFix).sort((left, right) => right.range[0] - left.range[0]);
  let output = sourceText;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

function iterativeFixedOutput(ruleName, source, options) {
  let output = source;
  let changed = false;

  for (let iteration = 0; iteration < 10; iteration += 1) {
    const fixes = runRule(ruleName, output, options)
      .map((report, index) => ({
        index,
        fix: report.suggest?.[0] ? reportFix(report) : null,
      }))
      .filter(({ fix }) => fix !== null)
      .sort(
        (left, right) =>
          left.fix.range[0] - right.fix.range[0] ||
          left.fix.range[1] - right.fix.range[1] ||
          left.index - right.index,
      );
    if (fixes.length === 0) {
      break;
    }

    let cursor = 0;
    let lastEnd = Number.NEGATIVE_INFINITY;
    let next = '';
    let applied = false;
    for (const { fix } of fixes) {
      if (lastEnd >= fix.range[0]) {
        continue;
      }
      next += output.slice(cursor, fix.range[0]);
      next += fix.replacementText;
      cursor = fix.range[1];
      lastEnd = fix.range[1];
      applied = true;
    }
    if (!applied) {
      break;
    }
    output = next + output.slice(cursor);
    changed = true;
  }

  return changed ? output : null;
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

  it('preserves upstream code-fix metadata for no-confusing-arrow', () => {
    expect(plugin.rules['no-confusing-arrow'].meta.fixable).toBe('code');
    expect(plugin.rules['jsx-quotes'].meta.fixable).toBe('code');
    expect(plugin.rules['arrow-spacing'].meta.fixable).toBe('whitespace');
  });

  it('exposes whitespace-fix metadata for array-element-newline', () => {
    expect(plugin.rules['array-element-newline'].meta.fixable).toBe('whitespace');
    expect(plugin.rules['array-element-newline'].meta.messages).toEqual({
      missingLineBreak: 'There should be a linebreak after this element.',
      unexpectedLineBreak: 'There should be no linebreak here.',
    });
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

  it('supports lines-between-class-members options and fixes', () => {
    const neverReports = runRule('lines-between-class-members', 'class C { a() {}\n\nb() {} }\n', [
      'never',
    ]);

    expect(neverReports).toMatchObject([{ messageId: 'never' }]);
    expect(
      neverReports[0].suggest?.[0]?.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      { range: ['class C { a() {}'.length, 'class C { a() {}\n\n'.length], replacementText: '\n' },
    ]);

    expect(
      messageIds(
        runRule('lines-between-class-members', 'class C { a() {}\nb() {} }\n', [
          'always',
          { exceptAfterSingleLine: true },
        ]),
      ),
    ).toEqual([]);
  });

  it('supports lines-between-class-members enforce pairs', () => {
    expect(
      messageIds(
        runRule('lines-between-class-members', 'class C { a() {}\nb() {}\nfield;\n\nnext; }\n', [
          {
            enforce: [
              { blankLine: 'always', prev: 'method', next: 'method' },
              { blankLine: 'never', prev: 'field', next: 'field' },
            ],
          },
        ]),
      ),
    ).toEqual(['always', 'never']);
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

  it('maps no-confusing-arrow ranges and fixes across Unicode source', () => {
    const sourceText = "const 日本語 = value => value ? 'はい' : 'いいえ';\n";
    const arrowText = "value => value ? 'はい' : 'いいえ'";
    const bodyText = "value ? 'はい' : 'いいえ'";
    const arrowStart = sourceText.indexOf(arrowText);
    const bodyStart = sourceText.indexOf(bodyText);
    const reports = runRule('no-confusing-arrow', sourceText, []);

    expect(reports).toHaveLength(1);
    expect(reports[0]).toMatchObject({
      messageId: 'confusing',
      node: { range: [arrowStart, arrowStart + arrowText.length] },
    });
    expect(
      reports[0].suggest?.[0]?.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [bodyStart, bodyStart + bodyText.length],
        replacementText: `(${bodyText})`,
      },
    ]);
  });

  it('honors no-confusing-arrow options through shared settings', () => {
    expect(
      runRule('no-confusing-arrow', '({ value }) => value ? yes : no', [], {
        corsaStylistic: {
          rules: {
            'no-confusing-arrow': [{ onlyOneSimpleParam: true }],
          },
        },
      }),
    ).toEqual([]);

    const reports = runRule('no-confusing-arrow', 'value => (value ? yes : no)', [], {
      corsaStylistic: {
        rules: {
          'no-confusing-arrow': [{ allowParens: false }],
        },
      },
    });
    expect(messageIds(reports)).toEqual(['confusing']);
    expect(reports[0].suggest).toBeUndefined();
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

  it('supports all multiline-comment-style modes and exact fixes', () => {
    const starredReports = runRule('multiline-comment-style', '  // first\n  // second\n', [
      'starred-block',
    ]);
    expect(messageIds(starredReports)).toEqual(['expectedBlock']);
    expect(starredReports[0].node.range).toEqual([2, 22]);
    expect(
      starredReports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [2, 22],
        replacementText: '/*\n   * first\n   * second\n   */',
      },
    ]);

    const bareReports = runRule('multiline-comment-style', '/*\n * first\n * second\n */', [
      'bare-block',
    ]);
    expect(messageIds(bareReports)).toEqual(['expectedBareBlock']);
    expect(
      bareReports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [0, 25],
        replacementText: '/* first\n   second */',
      },
    ]);

    const separateReports = runRule('multiline-comment-style', '/*\n * first\n * second\n */', [
      'separate-lines',
    ]);
    expect(messageIds(separateReports)).toEqual(['expectedLines']);
    expect(
      separateReports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [0, 25],
        replacementText: '// first\n// second',
      },
    ]);
  });

  it('honors multiline-comment-style JSDoc and exclamation options', () => {
    const sourceText = '/**\n * docs\n */\n/*!\n * license\n */\n';

    expect(messageIds(runRule('multiline-comment-style', sourceText, ['separate-lines']))).toEqual(
      [],
    );
    expect(
      messageIds(
        runRule('multiline-comment-style', sourceText, [
          'separate-lines',
          { checkJSDoc: true, checkExclamation: true },
        ]),
      ),
    ).toEqual(['expectedLines', 'expectedLines']);
  });

  it('maps multiline-comment-style UTF-8 ranges and fixes to UTF-16', () => {
    const sourceText = '日本語\n  // première\n  // deuxième\n';
    const reports = runRule('multiline-comment-style', sourceText, []);
    const commentStart = sourceText.indexOf('// première');
    const commentEnd = sourceText.indexOf('\n', sourceText.indexOf('// deuxième'));

    expect(reports).toHaveLength(1);
    expect(reports[0].node.range).toEqual([commentStart, commentEnd]);
    expect(
      reports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [commentStart, commentEnd],
        replacementText: '/*\n   * première\n   * deuxième\n   */',
      },
    ]);
  });

  it('avoids multiline-comment-style literal, TypeScript, JSX text, and inline false positives', () => {
    for (const sourceText of [
      "const literal = '/* first\\nsecond */';",
      'const template = `// first\\n// second`;',
      'type Box<T> = { value: T }; // first\n// second\n',
      'const view = <div>/* first\nsecond */</div>;',
      'const view = <div>{/* first\nsecond */}</div>;',
      'call(/* first\nsecond */);',
    ]) {
      expect(runRule('multiline-comment-style', sourceText, [])).toEqual([]);
    }
  });

  it('runs multiline-comment-style through shared stylistic settings', () => {
    expect(
      messageIds(
        runRule('multiline-comment-style', '/*\n * first\n * second\n */', [], {
          corsaStylistic: {
            rules: {
              'multiline-comment-style': ['bare-block'],
            },
          },
        }),
      ),
    ).toEqual(['expectedBareBlock']);
  });

  it('supports jsx-equals-spacing options and exact fixes', () => {
    const neverReports = runRule('jsx-equals-spacing', '<App foo = {bar} />;\n', ['never']);
    expect(messageIds(neverReports)).toEqual(['noSpaceBefore', 'noSpaceAfter']);
    expect(neverReports.map((report) => report.node.range)).toEqual([
      [9, 10],
      [9, 10],
    ]);
    expect(
      neverReports.flatMap((report) =>
        report.suggest[0].fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    ).toEqual([
      { range: [8, 9], replacementText: '' },
      { range: [10, 11], replacementText: '' },
    ]);

    const alwaysReports = runRule('jsx-equals-spacing', '<App foo={bar} />;\n', ['always']);
    expect(messageIds(alwaysReports)).toEqual(['needSpaceBefore', 'needSpaceAfter']);
    expect(
      alwaysReports.flatMap((report) =>
        report.suggest[0].fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    ).toEqual([
      { range: [8, 8], replacementText: ' ' },
      { range: [9, 9], replacementText: ' ' },
    ]);
  });

  it('runs jsx-equals-spacing through shared settings without expression false positives', () => {
    const reports = runRule(
      'jsx-equals-spacing',
      '<App foo={fallback = next}>text<Child bar = "value" /></App>;\n',
      [],
      {
        corsaStylistic: {
          rules: {
            'jsx-equals-spacing': ['never'],
          },
        },
      },
    );

    expect(messageIds(reports)).toEqual(['noSpaceBefore', 'noSpaceAfter']);
  });

  it('ports every upstream jsx-quotes invalid case with exact report and fix ranges', () => {
    const cases = [
      ["<foo bar='baz' />", [], [9, 14], '"baz"'],
      ['<foo bar="baz" />', ['prefer-single'], [9, 14], "'baz'"],
      ['<foo bar="&quot;" />', ['prefer-single'], [9, 17], "'&quot;'"],
      ["<foo bar='&#39;' />", [], [9, 16], '"&#39;"'],
    ];

    for (const [source, options, range, replacementText] of cases) {
      const reports = runRule('jsx-quotes', source, options);
      expect(messageIds(reports), source).toEqual(['unexpected']);
      expect(reports[0].node.range, source).toEqual(range);
      expect(reports[0].data, source).toEqual({
        description: options[0] === 'prefer-single' ? 'doublequote' : 'singlequote',
      });
      expect(
        reports[0].suggest[0].fix({
          replaceTextRange(fixRange, text) {
            return { range: fixRange, replacementText: text };
          },
        }),
        source,
      ).toEqual([{ range, replacementText }]);
    }
  });

  it('accepts every upstream jsx-quotes valid case', () => {
    const cases = [
      ['<foo bar="baz" />', []],
      ["<foo bar='\"' />", []],
      ['<foo bar="\'" />', ['prefer-single']],
      ["<foo bar='baz' />", ['prefer-single']],
      ['<foo bar="baz">"</foo>', []],
      ["<foo bar='baz'>'</foo>", ['prefer-single']],
      ["<foo bar={'baz'} />", []],
      ['<foo bar={"baz"} />', ['prefer-single']],
      ['<foo bar={baz} />', []],
      ['<foo bar />', []],
      ["<foo bar='&quot;' />", ['prefer-single']],
      ['<foo bar="&quot;" />', []],
      ["<foo bar='&#39;' />", ['prefer-single']],
      ['<foo bar="&#39;" />', []],
    ];

    for (const [source, options] of cases) {
      expect(runRule('jsx-quotes', source, options), source).toEqual([]);
    }
  });

  it('accepts every stable v5.10.0 lines-around-comment valid fixture', () => {
    expect(linesAroundCommentFixture.upstream).toBe('@stylistic/eslint-plugin@5.10.0');
    expect(linesAroundCommentFixture.suites.map((suite) => suite.valid.length)).toEqual([125, 32]);

    for (const suite of linesAroundCommentFixture.suites) {
      for (const [index, test] of suite.valid.entries()) {
        expect(
          runRule('lines-around-comment', test.code, test.options),
          `${suite.lang} valid fixture ${index}`,
        ).toEqual([]);
      }
    }
  });

  it('replays every stable v5.10.0 lines-around-comment diagnostic and fix exactly', () => {
    expect(linesAroundCommentFixture.suites.map((suite) => suite.invalid.length)).toEqual([74, 30]);

    for (const suite of linesAroundCommentFixture.suites) {
      for (const [index, test] of suite.invalid.entries()) {
        const label = `${suite.lang} invalid fixture ${index}`;
        const reports = runRule('lines-around-comment', test.code, test.options);
        expect(messageIds(reports), label).toEqual(test.errors.map((error) => error.messageId));
        expect(
          reports.map((report) => report.node.range),
          `${label} report ranges`,
        ).toEqual(test.errors.map((error) => expectedReportRange(test.code, error.reportRange)));
        expect(reports.map(reportFix), `${label} fixes`).toEqual(
          test.errors.map((error) => ({
            range: error.fix.range,
            replacementText: error.fix.text,
          })),
        );
        expect(applyReportFixes(test.code, reports), `${label} output`).toBe(test.output);
      }
    }
  });

  it('runs jsx-quotes through shared settings without non-JSX false positives', () => {
    const source = [
      "type Box<T = 'default'> = { value: 'literal' };",
      "const plain = 'value';",
      "const template = `<App title='template' />`;",
      "const node = <UI.Root expression={'value'} title='attribute' />;",
    ].join('\n');
    const reports = runRule('jsx-quotes', source, [], {
      corsaStylistic: {
        rules: {
          'jsx-quotes': ['prefer-double'],
        },
      },
    });

    expect(messageIds(reports)).toEqual(['unexpected']);
    expect(source.slice(...reports[0].node.range)).toBe("'attribute'");
  });

  it('supports type-annotation-spacing options, exact UTF-16 ranges, and fixes', () => {
    const source = 'const 日本語 :string = 1; type F = (value:string)=>number;\n';
    const reports = runRule('type-annotation-spacing', source, [
      {
        overrides: {
          variable: { before: false, after: true },
          parameter: { before: true, after: false },
          arrow: { before: true, after: true },
        },
      },
    ]);
    const variableColon = source.indexOf(':');
    const parameterColon = source.indexOf(':', variableColon + 1);
    const arrow = source.indexOf('=>');

    expect(messageIds(reports)).toEqual([
      'expectedSpaceAfter',
      'unexpectedSpaceBefore',
      'expectedSpaceBefore',
      'expectedSpaceAfter',
      'expectedSpaceBefore',
    ]);
    expect(reports.map((report) => report.node.range)).toEqual([
      [variableColon, variableColon + 1],
      [variableColon, variableColon + 1],
      [parameterColon, parameterColon + 1],
      [arrow, arrow + 2],
      [arrow, arrow + 2],
    ]);
    expect(
      reports.flatMap((report) =>
        report.suggest[0].fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    ).toEqual([
      { range: [variableColon + 1, variableColon + 1], replacementText: ' ' },
      { range: [variableColon - 1, variableColon], replacementText: '' },
      { range: [parameterColon, parameterColon], replacementText: ' ' },
      { range: [arrow + 2, arrow + 2], replacementText: ' ' },
      { range: [arrow, arrow], replacementText: ' ' },
    ]);
  });

  it('runs type-annotation-spacing through shared settings without colon false positives', () => {
    const source = [
      'const value : string = condition ? left : right;',
      'const object = { key: value };',
      'label: for (;;) break label;',
      'type F = (input : string) => number;',
    ].join('\n');
    const reports = runRule('type-annotation-spacing', source, [], {
      corsaStylistic: {
        rules: {
          'type-annotation-spacing': [
            {
              overrides: {
                variable: { before: false },
                parameter: { before: false },
              },
            },
          ],
        },
      },
    });

    expect(messageIds(reports)).toEqual(['unexpectedSpaceBefore', 'unexpectedSpaceBefore']);
  });

  it('supports every function-call-argument-newline mode with exact UTF-16 fixes', () => {
    const source = 'fn("😀", first, second)';
    const firstGap = [source.indexOf(',') + 1, source.indexOf('first')];
    const secondComma = source.indexOf(',', source.indexOf('first'));
    const secondGap = [secondComma + 1, source.indexOf('second')];
    const alwaysReports = runRule('function-call-argument-newline', source, ['always']);

    expect(alwaysReports).toMatchObject([
      { messageId: 'missingLineBreak', node: { range: firstGap } },
      { messageId: 'missingLineBreak', node: { range: secondGap } },
    ]);
    expect(
      alwaysReports.flatMap((report) =>
        report.suggest[0].fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    ).toEqual([
      { range: firstGap, replacementText: '\n' },
      { range: secondGap, replacementText: '\n' },
    ]);

    expect(
      messageIds(
        runRule('function-call-argument-newline', 'fn(first,\nsecond,\nthird)', ['never']),
      ),
    ).toEqual(['unexpectedLineBreak', 'unexpectedLineBreak']);
    expect(
      messageIds(
        runRule('function-call-argument-newline', 'fn(first,\nsecond, third)', ['consistent']),
      ),
    ).toEqual(['missingLineBreak']);
  });

  it('uses shared function-call-argument-newline settings for call, new, and import', () => {
    const source = [
      'fn(first, second);',
      'new Factory(first, second);',
      "import('data.json', { with: { type: 'json' } });",
    ].join('\n');
    const reports = runRule('function-call-argument-newline', source, [], {
      corsaStylistic: {
        rules: {
          'function-call-argument-newline': ['always'],
        },
      },
    });

    expect(messageIds(reports)).toEqual([
      'missingLineBreak',
      'missingLineBreak',
      'missingLineBreak',
    ]);
  });

  it('does not offer a function-call-argument-newline fix after a line comment', () => {
    const reports = runRule('function-call-argument-newline', 'fn(first, // preserve\nsecond)', [
      'never',
    ]);

    expect(reports).toMatchObject([
      {
        messageId: 'unexpectedLineBreak',
        node: { range: [21, 22] },
      },
    ]);
    expect(reports[0].suggest).toBeUndefined();
  });

  it('keeps the stable function-paren-newline upstream inventory complete', () => {
    expect(functionParenNewlineFixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
    });
    expect(functionParenNewlineFixture.valid).toHaveLength(112);
    expect(functionParenNewlineFixture.invalid).toHaveLength(86);
    expect(functionParenNewlineFixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(
      135,
    );
  });

  it('accepts every stable v5.10.0 function-paren-newline valid fixture', () => {
    for (const [index, testCase] of functionParenNewlineFixture.valid.entries()) {
      expect(
        runRule('function-paren-newline', testCase.code, testCase.options),
        `upstream valid fixture ${index}`,
      ).toEqual([]);
    }
  });

  it('replays every stable v5.10.0 function-paren-newline invalid fixture exactly', () => {
    const messages = plugin.rules['function-paren-newline'].meta.messages;

    for (const [index, testCase] of functionParenNewlineFixture.invalid.entries()) {
      const label = `upstream invalid fixture ${index}`;
      const reports = runRule('function-paren-newline', testCase.code, testCase.options);
      expect(messageIds(reports), label).toEqual(testCase.errors.map((error) => error.messageId));
      expect(
        reports.map((report) => report.node.range),
        `${label} report ranges`,
      ).toEqual(
        testCase.errors.map((error) => [
          offsetAt(testCase.code, error.line, error.column),
          offsetAt(testCase.code, error.endLine, error.endColumn),
        ]),
      );
      expect(
        testCase.errors.map((error) => messages[error.messageId]),
        `${label} messages`,
      ).toEqual(testCase.errors.map((error) => error.message));
      expect(reports.map(reportFix), `${label} fixes`).toEqual(
        testCase.errors.map((error) =>
          error.fix ? { range: error.fix.range, replacementText: error.fix.text } : null,
        ),
      );
      expect(
        iterativeFixedOutput('function-paren-newline', testCase.code, testCase.options),
        `${label} iterative output`,
      ).toBe(testCase.output);
    }
  });

  it('runs function-paren-newline through shared settings with UTF-16 ranges', () => {
    const source = 'const 日本語 = call(first, second);\n';
    const reports = runRule('function-paren-newline', source, [], {
      corsaStylistic: {
        rules: {
          'function-paren-newline': ['always'],
        },
      },
    });
    const left = source.indexOf('(');
    const right = source.indexOf(')');

    expect(messageIds(reports)).toEqual(['expectedAfter', 'expectedBefore']);
    expect(reports.map((report) => report.node.range)).toEqual([
      [left, left + 1],
      [right, right + 1],
    ]);
    expect(reports.map(reportFix)).toEqual([
      { range: [left + 1, left + 1], replacementText: '\n' },
      { range: [right, right], replacementText: '\n' },
    ]);
  });

  it('handles CRLF, CR, line separator, and paragraph separator without lookalike reports', () => {
    for (const linebreak of ['\r\n', '\r', '\u2028', '\u2029']) {
      expect(
        messageIds(
          runRule('function-paren-newline', `call(${linebreak}first, second);`, ['consistent']),
        ),
        JSON.stringify(linebreak),
      ).toEqual(['expectedBefore']);
    }

    expect(
      runRule(
        'function-paren-newline',
        [
          "const text = 'call(\\nvalue\\n)';",
          'if (condition) { call(); }',
          'const grouped = (value);',
          'const arrow = value => value;',
        ].join('\n'),
        ['never'],
      ),
    ).toEqual([]);
  });

  it('matches upstream no-mixed-operators defaults, custom groups, and report data', () => {
    const defaults = runRule('no-mixed-operators', 'a + b * c;\n');
    expect(defaults).toMatchObject([
      {
        messageId: 'unexpectedMixedOperator',
        data: { leftOperator: '+', rightOperator: '*' },
        node: { range: [2, 3] },
      },
      {
        messageId: 'unexpectedMixedOperator',
        data: { leftOperator: '+', rightOperator: '*' },
        node: { range: [6, 7] },
      },
    ]);
    expect(defaults.every((report) => report.suggest === undefined)).toBe(true);

    expect(
      messageIds(runRule('no-mixed-operators', 'a + b - c;\n', [{ allowSamePrecedence: false }])),
    ).toEqual(['unexpectedMixedOperator', 'unexpectedMixedOperator']);
    expect(
      messageIds(
        runRule('no-mixed-operators', 'a + b * c && d || e;\n', [{ groups: [['&&', '||']] }]),
      ),
    ).toEqual(['unexpectedMixedOperator', 'unexpectedMixedOperator']);
    expect(runRule('no-mixed-operators', '((a + b) * c) + (d * e);\n')).toEqual([]);
  });

  it('handles no-mixed-operators in TypeScript, JSX, optional chains, and multiline code', () => {
    const sourceText = `
type Union = A | B & C;
const generic = factory<A | B, C & D>();
const optional = object?.value + fallback * scale;
const view = <Panel value={left && right || fallback} />;
const multiline = alpha
  + /* keep */ beta
  * gamma;
`;
    const reports = runRule('no-mixed-operators', sourceText);

    expect(messageIds(reports)).toEqual([
      'unexpectedMixedOperator',
      'unexpectedMixedOperator',
      'unexpectedMixedOperator',
      'unexpectedMixedOperator',
      'unexpectedMixedOperator',
      'unexpectedMixedOperator',
    ]);
    expect(reports.map((report) => sourceText.slice(...report.node.range))).toEqual([
      '+',
      '*',
      '&&',
      '||',
      '+',
      '*',
    ]);
  });

  it('ignores no-mixed-operators lookalikes in comments, strings, regexes, and raw templates', () => {
    const sourceText = `
// a + b * c
const 文 = "a + b * c";
const regex = /a+b*c/;
const raw = \`a + b * c\`;
const parenthesized = (a + b) * c;
`;

    expect(runRule('no-mixed-operators', sourceText)).toEqual([]);
  });

  it('supports array-bracket-newline modes, UTF-16 ranges, shared settings, and fixes', () => {
    const source = 'const 日本語 = [1, 2];\n';
    const opening = source.indexOf('[');
    const closing = source.indexOf(']');
    const reports = runRule('array-bracket-newline', source, ['always']);

    expect(messageIds(reports)).toEqual(['missingOpeningLinebreak', 'missingClosingLinebreak']);
    expect(reports.map((report) => report.node.range)).toEqual([
      [opening, opening + 1],
      [closing, closing + 1],
    ]);
    expect(
      reports.map(
        (report) =>
          report.suggest[0].fix({
            replaceTextRange(range, replacementText) {
              return { range, replacementText };
            },
          })[0],
      ),
    ).toEqual([
      { range: [opening + 1, opening + 1], replacementText: '\n' },
      { range: [closing, closing], replacementText: '\n' },
    ]);

    expect(
      messageIds(
        runRule('array-bracket-newline', 'const values = [\n1,\n2\n];\n', [], {
          corsaStylistic: {
            rules: {
              'array-bracket-newline': ['never'],
            },
          },
        }),
      ),
    ).toEqual(['unexpectedOpeningLinebreak', 'unexpectedClosingLinebreak']);
  });

  it('handles array patterns, comments, nested arrays, TypeScript, and TSX without lookalikes', () => {
    const source = [
      'const [first, second] = values;',
      'const nested = [[1, 2]];',
      'const commented = [/* preserve */ 1];',
      'type Tuple = [string, number];',
      'const view = <Panel value={[1, 2]} />;',
      'const member = object[index];',
    ].join('\n');
    const reports = runRule('array-bracket-newline', source, ['always']);

    expect(messageIds(reports)).toEqual([
      'missingOpeningLinebreak',
      'missingClosingLinebreak',
      'missingOpeningLinebreak',
      'missingOpeningLinebreak',
      'missingClosingLinebreak',
      'missingClosingLinebreak',
      'missingOpeningLinebreak',
      'missingClosingLinebreak',
      'missingOpeningLinebreak',
      'missingClosingLinebreak',
    ]);
    expect(reports.every((report) => !source.slice(...report.node.range).includes('Tuple'))).toBe(
      true,
    );
  });

  it('reports newline-per-chained-call data and fixes with UTF-16 ranges', () => {
    const source = 'const value = "😀".trim().toString().valueOf();\n';
    const propertyStart = source.indexOf('.valueOf');
    const reports = runRule('newline-per-chained-call', source, []);

    expect(reports).toMatchObject([
      {
        messageId: 'expected',
        data: { callee: '.valueOf' },
        node: { range: [propertyStart, propertyStart + '.valueOf'.length] },
      },
    ]);
    expect(
      reports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([{ range: [propertyStart, propertyStart], replacementText: '\n' }]);
  });

  it('honors newline-per-chained-call depth options from direct and shared config', () => {
    const source = 'first().second().third();\n';

    expect(
      runRule('newline-per-chained-call', source, [{ ignoreChainWithDepth: 1 }]),
    ).toMatchObject([
      { messageId: 'expected', data: { callee: '.second' } },
      { messageId: 'expected', data: { callee: '.third' } },
    ]);
    expect(
      runRule('newline-per-chained-call', source, [], {
        corsaStylistic: {
          rules: {
            'newline-per-chained-call': [{ ignoreChainWithDepth: 3 }],
          },
        },
      }),
    ).toEqual([]);
  });

  it('keeps newline-per-chained-call comments, optional calls, and computed properties fixable', () => {
    const source = [
      'first().second() /* preserve */ .third();',
      'obj?.foo1()?.foo2()?.foo3();',
      'first()[second()]()[third()]();',
    ].join('\n');
    const reports = runRule('newline-per-chained-call', source, []);

    expect(reports.map((report) => report.data)).toEqual([
      { callee: '.third' },
      { callee: '?.foo3' },
      { callee: '[third()]' },
    ]);
    expect(
      reports.map(
        (report) =>
          report.suggest[0].fix({
            replaceTextRange(range, replacementText) {
              return { range, replacementText };
            },
          })[0],
      ),
    ).toEqual([
      { range: [source.indexOf('.third'), source.indexOf('.third')], replacementText: '\n' },
      {
        range: [source.indexOf('?.foo3'), source.indexOf('?.foo3')],
        replacementText: '\n',
      },
      {
        range: [source.indexOf('[third()]'), source.indexOf('[third()]')],
        replacementText: '\n',
      },
    ]);
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

  it('reports jsx-equals-spacing through a real oxlint JSX run', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.jsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(sourcePath, 'export const view = <App foo = {bar} />;\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/jsx-equals-spacing': 'error',
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
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(jsx-equals-spacing)',
        'stylistic(jsx-equals-spacing)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        "There should be no space before '='",
        "There should be no space after '='",
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs jsx-quotes through an oxlint jsPlugins config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-quotes-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.jsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(sourcePath, "const node = <App title='value' expression={'ignored'} />;\n");
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/jsx-quotes': 'error',
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
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(jsx-quotes)',
          labels: [{ span: { offset: 24, length: 7 } }],
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs no-confusing-arrow through oxlint jsPlugins on TSX', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-no-confusing-arrow-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(
        sourcePath,
        [
          'type Props = { value: boolean };',
          'export const View = ({ value }: Props) => (',
          '  <button onClick={() => value ? enabled : disabled}>run</button>',
          ');',
          '',
        ].join('\n'),
      );
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/no-confusing-arrow': 'error',
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
        'stylistic(no-confusing-arrow)',
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs type-annotation-spacing through oxlint on TypeScript', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-type-annotation-spacing-'));

    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(sourcePath, 'const value :string = "value";\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/type-annotation-spacing': 'error',
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
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        { code: 'stylistic(type-annotation-spacing)', message: "Expected a space after the ':'." },
        {
          code: 'stylistic(type-annotation-spacing)',
          message: "Unexpected space before the ':'.",
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs function-paren-newline through an actual oxlint JavaScript config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-function-paren-newline-'));

    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const sourceText = 'export function value(first: string, second: string) {}\n';
      writeFileSync(sourcePath, sourceText);
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/function-paren-newline': ['error', 'always'],
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
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(function-paren-newline)',
          message: "Expected newline after '('.",
          labels: [{ span: { offset: sourceText.indexOf('('), length: 1 } }],
        },
        {
          code: 'stylistic(function-paren-newline)',
          message: "Expected newline before ')'.",
          labels: [{ span: { offset: sourceText.indexOf(')'), length: 1 } }],
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('reports no-mixed-operators through real oxlint TS and TSX runs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-no-mixed-operators-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const typeScriptSourcePath = join(temp, 'assertion.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(
        sourcePath,
        'type T = A | B & C;\nexport const view = <Panel value={a + b * c} />;\n',
      );
      writeFileSync(typeScriptSourcePath, 'export const value = <number>(a + b * c);\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/no-mixed-operators': 'error',
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath, typeScriptSourcePath],
        {
          encoding: 'utf8',
        },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(no-mixed-operators)',
        'stylistic(no-mixed-operators)',
        'stylistic(no-mixed-operators)',
        'stylistic(no-mixed-operators)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
        "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
        "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
        "Unexpected mix of '+' and '*'. Use parentheses to clarify the intended order of operations.",
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs newline-per-chained-call through a real oxlint TSX config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-newline-chain-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const source =
        'declare const service: any;\nexport const view = <div>{service.first().second().third()}</div>;\n';

      writeFileSync(sourcePath, source);
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/newline-per-chained-call': 'error',
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
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics).toMatchObject([
        {
          code: 'stylistic(newline-per-chained-call)',
          message: 'Expected line break before `.third`.',
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs array-bracket-newline through real oxlint TypeScript and TSX configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-array-bracket-newline-'));

    try {
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(typeScriptPath, 'export const values: number[] = [1, 2];\n');
      writeFileSync(tsxPath, 'export const view = <Panel value={[1, 2]} />;\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/array-bracket-newline': ['error', 'always'],
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', typeScriptPath, tsxPath],
        {
          encoding: 'utf8',
        },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(array-bracket-newline)',
          message: "A linebreak is required after '['.",
        },
        {
          code: 'stylistic(array-bracket-newline)',
          message: "A linebreak is required before ']'.",
        },
        {
          code: 'stylistic(array-bracket-newline)',
          message: "A linebreak is required after '['.",
        },
        {
          code: 'stylistic(array-bracket-newline)',
          message: "A linebreak is required before ']'.",
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs lines-around-comment through an actual oxlint JavaScript config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-lines-around-comment-'));

    try {
      const sourcePath = join(temp, 'sample.js');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const sourceText = 'before();\n// note\nafter();\n';
      writeFileSync(sourcePath, sourceText);
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/lines-around-comment': [
              'error',
              {
                beforeLineComment: true,
                afterLineComment: true,
              },
            ],
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
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(lines-around-comment)',
          message: 'Expected line before comment.',
          labels: [{ span: { offset: sourceText.indexOf('// note'), length: '// note'.length } }],
        },
        {
          code: 'stylistic(lines-around-comment)',
          message: 'Expected line after comment.',
          labels: [{ span: { offset: sourceText.indexOf('// note'), length: '// note'.length } }],
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs function-call-argument-newline through oxlint on TSX', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-function-call-argument-newline-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(
        sourcePath,
        [
          'declare function render<T>(first: T, second: T): T;',
          'declare class Factory { constructor(first: unknown, second: unknown); }',
          'const node = render<JSX.Element>(<One />, <Two />);',
          'const instance = new Factory(first, second);',
          '',
        ].join('\n'),
      );
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/function-call-argument-newline': 'error',
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
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(function-call-argument-newline)',
          message: 'There should be a line break after this argument.',
        },
        {
          code: 'stylistic(function-call-argument-newline)',
          message: 'There should be a line break after this argument.',
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs array-element-newline through oxlint jsPlugins on TSX', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-array-element-newline-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(
        sourcePath,
        [
          'type Tuple = [string, number];',
          'export const View = () => <Panel values={[first, second, third]} />;',
          '',
        ].join('\n'),
      );
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/array-element-newline': 'error',
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
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(array-element-newline)',
        'stylistic(array-element-newline)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'There should be a linebreak after this element.',
        'There should be a linebreak after this element.',
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('reports multiline-comment-style through a real oxlint run', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-multiline-comment-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(sourcePath, '// first\n// second\nexport const value = 1;\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [
            {
              name: 'stylistic',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'stylistic/multiline-comment-style': 'error',
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
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(multiline-comment-style)',
      ]);
      expect(diagnostics[0].message).toBe(
        'Expected a block comment instead of consecutive line comments.',
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
});
