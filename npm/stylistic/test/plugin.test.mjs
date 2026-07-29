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
const multilineTernaryFixture = JSON.parse(
  readFileSync(new URL('./fixtures/multiline-ternary-v5.10.0.json', import.meta.url), 'utf8'),
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
  ['indent-binary-ops', 'const value = first\n+ second;\n', [], ['wrongIndentation']],
  ['no-tabs', 'const\tlabel = 1;\n', [], ['unexpectedTab']],
  ['no-trailing-spaces', 'const x = 1;  \n', [], ['trailingSpace']],
  ['quotes', 'const label = "value";\n', ['single'], ['wrongQuote']],
  ['unicode-bom', '\u{feff}const x = 1;\n', ['never'], ['unexpected']],
  ['arrow-spacing', 'const f = ()=>1;\n', [], ['expectedBefore', 'expectedAfter']],
  ['comma-spacing', '[1 ,2]\n', [], ['unexpected', 'missing']],
  ['semi-spacing', 'a ;b\n', [], ['unexpected', 'missing']],
  ['semi', 'const value = 1\n', [], ['missingSemi']],
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
    'object-property-newline',
    'const value = { first: 1, second: 2 };\n',
    [],
    ['propertiesOnNewline'],
  ],
  [
    'array-bracket-spacing',
    'const a = [ 1, 2 ];\n',
    [],
    ['unexpectedSpaceAfter', 'unexpectedSpaceBefore'],
  ],
  ['array-element-newline', 'const a = [1, 2];\n', [], ['missingLineBreak']],
  [
    'object-curly-newline',
    'const value = {first, second};\n',
    ['always'],
    ['expectedLinebreakAfterOpeningBrace', 'expectedLinebreakBeforeClosingBrace'],
  ],
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
  ['wrap-iife', 'const value = function () {}();\n', ['inside'], ['wrapInvocation']],
  ['implicit-arrow-linebreak', 'const f = (a) =>\n  a;\n', [], ['unexpectedLinebreak']],
  ['operator-linebreak', 'const x = 1\n  + 2;\n', [], ['operatorAtBeginning']],
  ['keyword-spacing', 'if(foo) {}\n', [], ['missingAfter']],
  ['line-comment-position', 'value; // inline\n// above\n', [], ['above']],
  ['lines-around-comment', 'before();\n/** docs */\nafter();\n', [], ['before']],
  ['jsx-child-element-spacing', '<App>word\n<a>link</a></App>;\n', [], ['spacingBeforeNext']],
  [
    'jsx-curly-spacing',
    '<App attr={ value }>{child}</App>;\n',
    [],
    ['noSpaceAfter', 'noSpaceBefore'],
  ],
  [
    'jsx-closing-bracket-location',
    '<App\n  prop />;\n',
    [{ location: 'tag-aligned' }],
    ['bracketLocation'],
  ],
  ['jsx-closing-tag-location', '<App>\n  content</App>;\n', [], ['onOwnLine']],
  [
    'jsx-curly-newline',
    '<App value={\nfoo\n} />;\n',
    ['never'],
    ['unexpectedAfter', 'unexpectedBefore'],
  ],
  ['jsx-first-prop-new-line', '<App first={{\n  value: 1\n}} second />;\n', [], ['propOnNewLine']],
  ['jsx-quotes', "<App title='value' />;\n", [], ['unexpected']],
  ['multiline-comment-style', '// first\n// second\n', [], ['expectedBlock']],
  ['lines-between-class-members', 'class C { a() {}\nb() {} }\n', [], ['always']],
  [
    'array-bracket-newline',
    'const values = [1, 2];\n',
    ['always'],
    ['missingOpeningLinebreak', 'missingClosingLinebreak'],
  ],
  [
    'multiline-ternary',
    'const value = condition ? yes : no;\n',
    [],
    ['expectedTestCons', 'expectedConsAlt'],
  ],
  ['brace-style', 'if (value)\n{\n  work(); }\n', [], ['nextLineOpen', 'singleLineClose']],
  ['nonblock-statement-body-position', 'if (value) work();\n', ['below'], ['expectLinebreak']],
  [
    'curly-newline',
    'if (ready) {}\n',
    ['always'],
    ['expectedLinebreakAfterOpeningBrace', 'expectedLinebreakBeforeClosingBrace'],
  ],
  ['no-extra-parens', 'const value = (answer);\n', [], ['unexpected']],
  ['newline-per-chained-call', 'first().second().third();\n', [], ['expected']],
  ['one-var-declaration-per-line', 'var a, b = 0;\n', [], ['expectVarOnNewline']],
  ['jsx-equals-spacing', '<App foo = {bar} />;\n', [], ['noSpaceBefore', 'noSpaceAfter']],
  [
    'member-delimiter-style',
    'interface Value {\n  first: string,\n  second: number,\n}\n',
    [],
    ['expectedSemi', 'expectedSemi'],
  ],
  ['no-confusing-arrow', 'const f = value => value ? yes : no;\n', [], ['confusing']],
  [
    'type-annotation-spacing',
    'const value :string = 1;\n',
    [],
    ['expectedSpaceAfter', 'unexpectedSpaceBefore'],
  ],
  ['type-named-tuple-spacing', 'type Tuple = [value:number];\n', [], ['expectedSpaceAfter']],
  [
    'type-generic-spacing',
    'type Box< T=string > = T;\n',
    [],
    ['genericSpacingMismatch', 'genericSpacingMismatch', 'genericSpacingMismatch'],
  ],
  [
    'function-paren-newline',
    'function value(first, second) {}\n',
    ['always'],
    ['expectedAfter', 'expectedBefore'],
  ],
  [
    'padding-line-between-statements',
    'const value = 1;\nuse(value);\n',
    [{ blankLine: 'always', prev: 'const', next: '*' }],
    ['expectedBlankLine'],
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

function reportFixes(report) {
  if (!report.suggest?.[0]) {
    return [];
  }
  return report.suggest[0].fix({
    replaceTextRange(range, replacementText) {
      return { range, text: replacementText };
    },
  });
}

function mergedReportFix(report, source) {
  const fixes = reportFixes(report);
  if (fixes.length === 0) {
    return null;
  }
  let cursor = fixes[0].range[0];
  let text = '';
  for (const fix of fixes) {
    text += source.slice(cursor, fix.range[0]);
    text += fix.text;
    cursor = fix.range[1];
  }
  return {
    range: [fixes[0].range[0], fixes.at(-1).range[1]],
    text,
  };
}

function iterativeFixedOutput(ruleName, source, options) {
  let output = source;
  let changed = false;

  for (let iteration = 0; iteration < 10; iteration += 1) {
    const fixes = runRule(ruleName, output, options)
      .map((report, index) => ({ index, fix: mergedReportFix(report, output) }))
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
      next += fix.text;
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

  it('preserves upstream code-fix metadata', () => {
    expect(plugin.rules['no-confusing-arrow'].meta.fixable).toBe('code');
    expect(plugin.rules['jsx-first-prop-new-line'].meta.fixable).toBe('code');
    expect(plugin.rules['jsx-quotes'].meta.fixable).toBe('code');
    expect(plugin.rules['no-extra-parens'].meta.fixable).toBe('code');
    expect(plugin.rules['arrow-spacing'].meta.fixable).toBe('whitespace');
  });

  it('exposes whitespace-fix metadata for array-element-newline', () => {
    expect(plugin.rules['array-element-newline'].meta.fixable).toBe('whitespace');
    expect(plugin.rules['array-element-newline'].meta.messages).toEqual({
      missingLineBreak: 'There should be a linebreak after this element.',
      unexpectedLineBreak: 'There should be no linebreak here.',
    });
  });

  it('exposes the complete object-curly-newline message catalog', () => {
    expect(plugin.rules['object-curly-newline'].meta.fixable).toBe('whitespace');
    expect(plugin.rules['object-curly-newline'].meta.messages).toEqual({
      unexpectedLinebreakBeforeClosingBrace: 'Unexpected line break before this closing brace.',
      unexpectedLinebreakAfterOpeningBrace: 'Unexpected line break after this opening brace.',
      expectedLinebreakBeforeClosingBrace: 'Expected a line break before this closing brace.',
      expectedLinebreakAfterOpeningBrace: 'Expected a line break after this opening brace.',
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

  it('supports semi modes and ASI continuation options through shared settings', () => {
    const source = 'import value from "value"\r\n(value => value)()\r\n';
    const alwaysReports = runRule('semi', source, [], {
      corsaStylistic: {
        rules: {
          semi: ['never', { beforeStatementContinuationChars: 'always' }],
        },
      },
    });
    expect(messageIds(alwaysReports)).toEqual(['missingSemi']);
    expect(alwaysReports[0].node.range).toEqual([
      'import value from "value"'.length,
      'import value from "value"\r\n'.length,
    ]);
    expect(reportFix(alwaysReports[0])).toEqual({
      range: ['import value from "value"'.length, 'import value from "value"'.length],
      replacementText: ';',
    });

    expect(
      runRule('semi', source.replace('\r\n(', ';\r\n('), [], {
        corsaStylistic: {
          rules: {
            semi: ['never', { beforeStatementContinuationChars: 'any' }],
          },
        },
      }),
    ).toEqual([]);
  });

  it('keeps semi option normalization total for malformed API inputs', () => {
    expect(() =>
      runRule('semi', 'const value = 1\n', [
        'invalid-mode',
        {
          omitLastInOneLineBlock: 'yes',
          omitLastInOneLineClassBody: 1,
          beforeStatementContinuationChars: false,
        },
      ]),
    ).not.toThrow();
    expect(messageIds(runRule('semi', 'const value = 1\n', ['invalid-mode']))).toEqual([
      'missingSemi',
    ]);
    expect(runRule('semi', 'const broken =', [])).toEqual([]);
  });

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

  it('maps no-extra-parens ranges and paired fixes across Unicode source', () => {
    const sourceText = 'const 名 = ((value));\n';
    const reports = runRule('no-extra-parens', sourceText, []);

    expect(reports).toHaveLength(1);
    expect(reports[0]).toMatchObject({
      messageId: 'unexpected',
      node: { range: [11, 12] },
    });
    expect(reportFixes(reports[0])).toEqual([
      { range: [11, 12], text: '' },
      { range: [17, 18], text: '' },
    ]);
    expect(mergedReportFix(reports[0], sourceText)).toEqual({
      range: [11, 18],
      text: 'value',
    });
  });

  it('honors no-extra-parens functions and JSX options through shared settings', () => {
    const settings = (options) => ({
      corsaStylistic: {
        rules: {
          'no-extra-parens': options,
        },
      },
    });

    expect(runRule('no-extra-parens', '(value)', [], settings(['functions']))).toEqual([]);
    expect(
      messageIds(
        runRule('no-extra-parens', 'const value = (function () {});', [], settings(['functions'])),
      ),
    ).toEqual(['unexpected']);
    expect(
      runRule(
        'no-extra-parens',
        'const view = (<Panel />);',
        [],
        settings(['all', { ignoreJSX: 'all' }]),
      ),
    ).toEqual([]);
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

  it('supports jsx-closing-tag-location options, UTF-16 ranges, and exact fixes', () => {
    const source = 'const 日本語 = <App>\n  child</App>;';
    const closingStart = source.indexOf('</App>');
    const tagAligned = runRule('jsx-closing-tag-location', source, []);

    expect(tagAligned).toMatchObject([
      {
        messageId: 'onOwnLine',
        node: { range: [closingStart, closingStart + '</App>'.length] },
      },
    ]);
    expect(reportFix(tagAligned[0])).toEqual({
      range: [closingStart, closingStart],
      replacementText: `\n${' '.repeat('const 日本語 = '.length)}`,
    });
    expect(applyReportFixes(source, tagAligned)).toBe(
      `const 日本語 = <App>\n  child\n${' '.repeat('const 日本語 = '.length)}</App>;`,
    );

    const lineAligned = runRule(
      'jsx-closing-tag-location',
      '  const view = <App>\n    child\n        </App>;',
      ['line-aligned'],
    );
    expect(messageIds(lineAligned)).toEqual(['alignWithOpening']);
    expect(reportFix(lineAligned[0])).toEqual({
      range: [31, 39],
      replacementText: '  ',
    });
  });

  it('runs jsx-closing-tag-location through shared settings for fragments', () => {
    const source = 'const view = <>\n  child</>;';
    const reports = runRule('jsx-closing-tag-location', source, [], {
      corsaStylistic: {
        rules: {
          'jsx-closing-tag-location': ['line-aligned'],
        },
      },
    });
    expect(messageIds(reports)).toEqual(['onOwnLine']);
    expect(iterativeFixedOutput('jsx-closing-tag-location', source, ['line-aligned'])).toBe(
      'const view = <>\n  child\n</>;',
    );
  });

  it('exposes the complete jsx-curly-newline message catalog and exact fixes', () => {
    expect(plugin.rules['jsx-curly-newline'].meta.fixable).toBe('whitespace');
    expect(plugin.rules['jsx-curly-newline'].meta.messages).toEqual({
      expectedBefore: "Expected newline before '}'.",
      expectedAfter: "Expected newline after '{'.",
      unexpectedBefore: "Unexpected newline before '}'.",
      unexpectedAfter: "Unexpected newline after '{'.",
    });

    const source = '<App value={foo &&\nbar} />;';
    const reports = runRule('jsx-curly-newline', source, [
      { singleline: 'forbid', multiline: 'require' },
    ]);
    expect(messageIds(reports)).toEqual(['expectedAfter', 'expectedBefore']);
    expect(reports.map(reportFix)).toEqual([
      { range: [12, 12], replacementText: '\n' },
      { range: [22, 22], replacementText: '\n' },
    ]);
  });

  it('keeps jsx-curly-newline comment removals diagnostic-only', () => {
    const reports = runRule('jsx-curly-newline', '<App value={ /* preserve */\nfoo } />;', [
      'never',
    ]);
    expect(messageIds(reports)).toEqual(['unexpectedAfter']);
    expect(reports[0].suggest).toBeUndefined();
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

  it('reports indent-binary-ops data and exact UTF-16 fixes for numeric and tab options', () => {
    const source = 'const 日本語 = first\n    + second';
    const lineStart = source.indexOf('    +');
    const reports = runRule('indent-binary-ops', source, [2]);

    expect(reports).toMatchObject([
      {
        messageId: 'wrongIndentation',
        data: { expected: '2 spaces' },
        node: { range: [lineStart, lineStart + 4] },
      },
    ]);
    expect(
      reports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([{ range: [lineStart, lineStart + 4], replacementText: '  ' }]);

    expect(
      runRule('indent-binary-ops', source, ['tab']).map((report) => ({
        data: report.data,
        fix: report.suggest[0].fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      })),
    ).toEqual([
      {
        data: { expected: '1 tab' },
        fix: [{ range: [lineStart, lineStart + 4], replacementText: '\t' }],
      },
    ]);
  });

  it('runs indent-binary-ops TypeScript unions through shared settings', () => {
    const source = 'type Value =\n| A\n    | B';
    const reports = runRule('indent-binary-ops', source, [], {
      corsaStylistic: {
        rules: {
          'indent-binary-ops': ['tab'],
        },
      },
    });

    expect(messageIds(reports)).toEqual(['wrongIndentation', 'wrongIndentation']);
    expect(reports.map((report) => report.data)).toEqual([
      { expected: '1 tab' },
      { expected: '1 tab' },
    ]);
    expect(
      reports.map((report) =>
        report.suggest[0].fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ),
    ).toEqual([
      [{ range: [13, 13], replacementText: '\t' }],
      [{ range: [17, 21], replacementText: '\t' }],
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

  it('keeps the stable multiline-ternary upstream inventory complete', () => {
    expect(multilineTernaryFixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
    });
    expect(multilineTernaryFixture.valid).toHaveLength(84);
    expect(multilineTernaryFixture.invalid).toHaveLength(63);
    expect(multilineTernaryFixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(
      104,
    );
  });

  it('accepts every stable v5.10.0 multiline-ternary valid fixture individually', () => {
    for (const [index, testCase] of multilineTernaryFixture.valid.entries()) {
      expect(
        runRule('multiline-ternary', testCase.code, testCase.options),
        `upstream valid fixture ${index}`,
      ).toEqual([]);
    }
  });

  it('replays every stable v5.10.0 multiline-ternary invalid fixture exactly', () => {
    const messages = plugin.rules['multiline-ternary'].meta.messages;

    for (const [index, testCase] of multilineTernaryFixture.invalid.entries()) {
      const label = `upstream invalid fixture ${index}`;
      const reports = runRule('multiline-ternary', testCase.code, testCase.options);
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
      expect(
        reports.map((report) => mergedReportFix(report, testCase.code)),
        `${label} fixes`,
      ).toEqual(
        testCase.errors.map((error) =>
          error.fix ? { range: error.fix.range, text: error.fix.text } : null,
        ),
      );
      expect(
        iterativeFixedOutput('multiline-ternary', testCase.code, testCase.options),
        `${label} iterative output`,
      ).toBe(testCase.output);
    }
  });

  it('supports multiline-ternary modes, shared settings, UTF-16 ranges, and multi-edit fixes', () => {
    const unicodeSource = 'const 日本語 = 条件 ? はい : いいえ;\n';
    const reports = runRule('multiline-ternary', unicodeSource, [], {
      corsaStylistic: {
        rules: {
          'multiline-ternary': ['always'],
        },
      },
    });

    expect(messageIds(reports)).toEqual(['expectedTestCons', 'expectedConsAlt']);
    expect(reports.map((report) => unicodeSource.slice(...report.node.range))).toEqual([
      '条件',
      'はい',
    ]);
    expect(reports.map(reportFixes)).toEqual([
      [
        {
          range: [unicodeSource.indexOf('条件') + '条件'.length, unicodeSource.indexOf('?')],
          text: '\n',
        },
      ],
      [
        {
          range: [unicodeSource.indexOf('はい') + 'はい'.length, unicodeSource.indexOf(':')],
          text: '\n',
        },
      ],
    ]);

    const neverSource = 'condition\n?\nconsequent : alternate';
    const neverReports = runRule('multiline-ternary', neverSource, ['never']);
    expect(messageIds(neverReports)).toEqual(['unexpectedTestCons']);
    expect(reportFixes(neverReports[0])).toEqual([
      { range: ['condition'.length, 'condition\n'.length], text: '' },
      {
        range: ['condition\n?'.length, 'condition\n?\n'.length],
        text: '',
      },
    ]);
    expect(mergedReportFix(neverReports[0], neverSource)).toEqual({
      range: ['condition'.length, 'condition\n?\n'.length],
      text: '?',
    });

    expect(runRule('multiline-ternary', 'condition ? yes : no', ['always-multiline'])).toEqual([]);
  });

  it('handles all line terminators, comments, TS syntax, TSX, and ignoreJSX boundaries', () => {
    for (const linebreak of ['\n', '\r', '\r\n', '\u2028', '\u2029']) {
      const source = `condition${linebreak}? yes${linebreak}: no`;
      expect(runRule('multiline-ternary', source, ['always']), JSON.stringify(linebreak)).toEqual(
        [],
      );
      expect(
        messageIds(runRule('multiline-ternary', source, ['never'])),
        JSON.stringify(linebreak),
      ).toEqual(['unexpectedTestCons', 'unexpectedConsAlt']);
    }

    const commentReports = runRule(
      'multiline-ternary',
      'condition ? // keep\nconsequent : alternate',
      ['always'],
    );
    expect(messageIds(commentReports)).toEqual(['expectedConsAlt']);
    expect(commentReports[0].suggest).toBeUndefined();

    const source = [
      'const typed: string = condition ? (yes as string) : (no satisfies string);',
      'const ignored = <Panel>{condition ? <Yes /> : <No />}</Panel>;',
      'const ignoredParentheses = <>{(condition ? <Yes /> : <No />)}</>;',
      'const checked = <>{flag && (condition ? <Yes /> : <No />)}</>;',
      'const attribute = <Panel value={condition ? yes : no} />;',
      "const text = 'condition ? yes : no';",
      'type Conditional<T> = T extends true ? Yes : No;',
    ].join('\n');
    expect(
      messageIds(runRule('multiline-ternary', source, ['always', { ignoreJSX: true }])),
    ).toEqual(['expectedTestCons', 'expectedConsAlt', 'expectedTestCons', 'expectedConsAlt']);
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

  it('maps brace-style byte offsets to exact UTF-16 token ranges and fixes', () => {
    const source = 'const emoji = "😀";\nif (ok)\n{\nwork(); }\n';
    const opening = source.indexOf('{');
    const closing = source.lastIndexOf('}');
    const reports = runRule('brace-style', source);

    expect(reports).toMatchObject([
      { messageId: 'nextLineOpen', node: { range: [opening, opening + 1] } },
      { messageId: 'singleLineClose', node: { range: [closing, closing + 1] } },
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
      {
        range: [source.indexOf('\n', source.indexOf('if (ok)')), opening],
        replacementText: ' ',
      },
      { range: [closing, closing], replacementText: '\n' },
    ]);
  });

  it('supports brace-style styles and allowSingleLine through shared settings', () => {
    const source = 'namespace Foo { value(); }\nif (ok) { render(); }\n';
    const settings = (allowSingleLine) => ({
      corsaStylistic: {
        rules: {
          'brace-style': ['allman', { allowSingleLine }],
        },
      },
    });
    expect(messageIds(runRule('brace-style', source, [], settings(false)))).toEqual([
      'sameLineOpen',
      'blockSameLine',
      'singleLineClose',
      'sameLineOpen',
      'blockSameLine',
      'singleLineClose',
    ]);
    expect(runRule('brace-style', source, [], settings(true))).toEqual([]);
  });

  it('preserves brace-style comment safety and ignores JSX/object brace lookalikes', () => {
    const source = [
      'const View = () => <Panel value={{ nested: true }} />;',
      'if (ok) // preserve',
      '{',
      'render(<View />);',
      '}',
    ].join('\n');
    const reports = runRule('brace-style', source);

    expect(reports).toMatchObject([{ messageId: 'nextLineOpen' }]);
    expect(reports[0].suggest).toBeUndefined();
    expect(source.slice(...reports[0].node.range)).toBe('{');
  });

  it('supports curly-newline modes, UTF-16 ranges, shared settings, and exact fixes', () => {
    const source = 'const 日本語 = true; if (日本語) {}\n';
    const opening = source.lastIndexOf('{');
    const closing = source.lastIndexOf('}');
    const reports = runRule('curly-newline', source, ['always']);

    expect(messageIds(reports)).toEqual([
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
    ]);
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
        runRule('curly-newline', 'if (ready) {\r\nwork();\r\n}\r\n', [], {
          corsaStylistic: {
            rules: {
              'curly-newline': ['never'],
            },
          },
        }),
      ),
    ).toEqual(['unexpectedLinebreakAfterOpeningBrace', 'unexpectedLinebreakBeforeClosingBrace']);
  });

  it('honors curly-newline specialization overrides across JavaScript, TypeScript, and TSX', () => {
    const options = [
      {
        IfStatementConsequent: 'always',
        ArrowFunctionExpression: 'always',
        ClassBody: 'always',
        StaticBlock: 'always',
        TSModuleBlock: 'always',
      },
    ];
    const source = [
      'if (ready) {}',
      'const callback = () => {};',
      'class Example { static {} }',
      'namespace 日本語 {}',
      'const view = <Panel render={() => {}} />;',
    ].join('\n');
    const reports = runRule('curly-newline', source, options);

    expect(messageIds(reports)).toEqual([
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
      'expectedLinebreakBeforeClosingBrace',
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
    ]);
  });

  it('keeps curly-newline comments unfixable and ignores object/type-like braces', () => {
    const commented = runRule('curly-newline', 'if (ready) {/* first */work();/* last */}', [
      'always',
    ]);
    expect(messageIds(commented)).toEqual([
      'expectedLinebreakAfterOpeningBrace',
      'expectedLinebreakBeforeClosingBrace',
    ]);
    expect(commented.every((report) => report.suggest === undefined)).toBe(true);

    for (const source of [
      'const object = {\nvalue: true\n};',
      'type Shape = {\nvalue: boolean\n};',
      'interface Shape {\nvalue: boolean\n}',
      'enum Shape {\nValue\n}',
      'const view = <Panel value={{\nnested: true\n}} />;',
      'const literal = "{\\n}";',
      'const template = `{\\n}`;',
      'const regex = /\\{\\n\\}/;',
    ]) {
      expect(runRule('curly-newline', source, ['never'])).toEqual([]);
    }
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

  it('reports jsx-closing-tag-location through real Oxlint JSX and TSX runs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-closing-tag-location-'));

    try {
      const jsxPath = join(temp, 'sample.jsx');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(jsxPath, 'export const jsx = <App>\n  child</App>;\n');
      writeFileSync(tsxPath, 'export const tsx: JSX.Element = <>\n  child\n    </>;\n');
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
            'stylistic/jsx-closing-tag-location': 'error',
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', jsxPath, tsxPath],
        {
          encoding: 'utf8',
        },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(jsx-closing-tag-location)',
        'stylistic(jsx-closing-tag-location)',
      ]);
      expect(
        diagnostics
          .map((diagnostic) => ({
            message: diagnostic.message,
            length: diagnostic.labels[0].span.length,
          }))
          .sort((left, right) => left.message.localeCompare(right.message)),
      ).toEqual([
        {
          message: 'Closing tag of a multiline JSX expression must be on its own line.',
          length: 6,
        },
        {
          message: 'Expected closing tag to match indentation of opening.',
          length: 3,
        },
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

  it('runs unfixable jsx-child-element-spacing through a real oxlint TSX config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-child-spacing-plugin-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const source =
        'export const view: JSX.Element = <App>日本語\r\n<a>リンク</a>\r\n後続</App>;\n';

      writeFileSync(sourcePath, source);
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'stylistic/jsx-child-element-spacing': 'error',
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(jsx-child-element-spacing)',
        'stylistic(jsx-child-element-spacing)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Ambiguous spacing before next element a',
        'Ambiguous spacing after previous element a',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.labels[0].span.length)).toEqual([0, 0]);

      const fixResult = spawnSync(oxlint, ['-c', configPath, '--quiet', '--fix', sourcePath], {
        encoding: 'utf8',
      });
      expect(fixResult.status).toBe(1);
      expect(readFileSync(sourcePath, 'utf8')).toBe(source);
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

  it('runs no-extra-parens through real oxlint JavaScript, TypeScript, and TSX configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-no-extra-parens-'));

    try {
      const javaScriptPath = join(temp, 'sample.js');
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(javaScriptPath, 'export const answer = (42);\n');
      writeFileSync(typeScriptPath, 'export const answer = (42 as number);\n');
      writeFileSync(tsxPath, 'export const view = (<Panel />);\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: { 'stylistic/no-extra-parens': 'error' },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', javaScriptPath, typeScriptPath, tsxPath],
        { encoding: 'utf8' },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics).toHaveLength(3);
      expect(new Set(diagnostics.map((diagnostic) => diagnostic.code))).toEqual(
        new Set(['stylistic(no-extra-parens)']),
      );
      expect(
        diagnostics.every(
          (diagnostic) => diagnostic.message === 'Unnecessary parentheses around expression.',
        ),
      ).toBe(true);
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

  it('runs type-named-tuple-spacing through real oxlint TypeScript configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-type-named-tuple-spacing-'));

    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(sourcePath, 'type Events = [change:string, update?:  number];\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'stylistic/type-named-tuple-spacing': 'error',
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );
      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(type-named-tuple-spacing)',
          message: "Expected a space after the ':'.",
        },
        {
          code: 'stylistic(type-named-tuple-spacing)',
          message: "Expected a space after the ':'.",
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

  it('runs padding-line-between-statements through an actual oxlint TypeScript config', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-padding-line-between-statements-'));

    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const sourceText = 'import value from "value";\nconst result = value;\nconsume(result);\n';
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
            'stylistic/padding-line-between-statements': [
              'error',
              { blankLine: 'always', prev: 'import', next: '*' },
              { blankLine: 'always', prev: 'const', next: '*' },
            ],
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(padding-line-between-statements)',
          message: 'Expected blank line before this statement.',
        },
        {
          code: 'stylistic(padding-line-between-statements)',
          message: 'Expected blank line before this statement.',
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

  it('runs multiline-ternary through real oxlint TypeScript and TSX configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-multiline-ternary-'));

    try {
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(
        typeScriptPath,
        'export const value: string = condition ? (yes as string) : no;\n',
      );
      writeFileSync(
        tsxPath,
        'export const view = <Panel>{condition ? <Yes /> : <No />}</Panel>;\n',
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
            'stylistic/multiline-ternary': ['error', 'always'],
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
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(multiline-ternary)',
        'stylistic(multiline-ternary)',
        'stylistic(multiline-ternary)',
        'stylistic(multiline-ternary)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Expected newline between test and consequent of ternary expression.',
        'Expected newline between consequent and alternate of ternary expression.',
        'Expected newline between test and consequent of ternary expression.',
        'Expected newline between consequent and alternate of ternary expression.',
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

  it('runs brace-style through real oxlint JavaScript, TypeScript, and TSX configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-brace-style-'));

    try {
      const javaScriptPath = join(temp, 'sample.js');
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(javaScriptPath, 'export function value() { return 1; }\n');
      writeFileSync(typeScriptPath, 'export namespace Foo { export const value = 1; }\n');
      writeFileSync(tsxPath, 'export function View() { return <div />; }\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: { 'stylistic/brace-style': ['error', 'allman'] },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', javaScriptPath, typeScriptPath, tsxPath],
        { encoding: 'utf8' },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics).toHaveLength(9);
      expect(new Set(diagnostics.map((diagnostic) => diagnostic.code))).toEqual(
        new Set(['stylistic(brace-style)']),
      );
      const grouped = Object.groupBy(diagnostics, (diagnostic) => diagnostic.message);
      expect(
        grouped['Opening curly brace appears on the same line as controlling statement.'],
      ).toHaveLength(3);
      expect(grouped['Statement inside of curly braces should be on next line.']).toHaveLength(3);
      expect(
        grouped[
          'Closing curly brace should be on the same line as opening curly brace or on the line after the previous block.'
        ],
      ).toHaveLength(3);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs nonblock-statement-body-position through real oxlint TSX', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-nonblock-position-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const source = 'if (ready) render(); else <View />;\n';
      writeFileSync(sourcePath, source);
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'stylistic/nonblock-statement-body-position': ['error', 'below'],
          },
        }),
      );

      const lintResult = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );
      expect(lintResult.status).toBe(1);
      expect(lintResult.stderr).toBe('');
      expect(JSON.parse(lintResult.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(nonblock-statement-body-position)',
          message: 'Expected a linebreak before this statement.',
        },
        {
          code: 'stylistic(nonblock-statement-body-position)',
          message: 'Expected a linebreak before this statement.',
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs curly-newline specializations through real oxlint TypeScript and TSX configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-curly-newline-'));

    try {
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(typeScriptPath, 'export namespace 日本語 {}\n');
      writeFileSync(tsxPath, 'export const view = <Panel render={() => {}} />;\n');
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
            'stylistic/curly-newline': [
              'error',
              {
                TSModuleBlock: 'always',
                ArrowFunctionExpression: 'always',
              },
            ],
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
          code: 'stylistic(curly-newline)',
          message: 'Expected a line break after this opening brace.',
        },
        {
          code: 'stylistic(curly-newline)',
          message: 'Expected a line break before this closing brace.',
        },
        {
          code: 'stylistic(curly-newline)',
          message: 'Expected a line break after this opening brace.',
        },
        {
          code: 'stylistic(curly-newline)',
          message: 'Expected a line break before this closing brace.',
        },
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('supports jsx-closing-bracket-location shared settings, data, UTF-16 ranges, and fixes', () => {
    const source = 'const marker = "😀"; const view = <Panel\n  prop />;\n';
    const slash = source.indexOf('/>');
    const reports = runRule('jsx-closing-bracket-location', source, [], {
      corsaStylistic: {
        rules: {
          'jsx-closing-bracket-location': [{ location: 'tag-aligned' }],
        },
      },
    });

    expect(messageIds(reports)).toEqual(['bracketLocation']);
    expect(reports[0].data).toEqual({
      location: 'aligned with the opening tag',
      details: ' (expected column 35 on the next line)',
    });
    expect(reports[0].node.range).toEqual([slash, slash + 1]);
    expect(
      reports[0].suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ).toEqual([
      {
        range: [source.indexOf('prop') + 'prop'.length, slash + 2],
        replacementText: `\n${' '.repeat(34)}/>`,
      },
    ]);
  });

  it('runs jsx-closing-bracket-location through real oxlint TSX lint and fixes', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-closing-bracket-location-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(sourcePath, 'export const view = <Panel\n  title="日本語" />;\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'stylistic/jsx-closing-bracket-location': ['error', { location: 'tag-aligned' }],
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );
      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(jsx-closing-bracket-location)',
          message:
            'The closing bracket must be aligned with the opening tag (expected column 21 on the next line)',
        },
      ]);

      const fixed = spawnSync(oxlint, ['-c', configPath, '--fix-suggestions', sourcePath], {
        encoding: 'utf8',
      });
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(sourcePath, 'utf8')).toBe(
        `export const view = <Panel\n  title="日本語"\n${' '.repeat(20)}/>;\n`,
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('supports jsx-curly-spacing shared settings, UTF-16 ranges, data, and fixes', () => {
    const source = 'const marker = "😀日本語"; const view = <App attr={value}>{ child }</App>;\n';
    const reports = runRule('jsx-curly-spacing', source, [], {
      corsaStylistic: {
        rules: {
          'jsx-curly-spacing': [
            {
              attributes: { when: 'always' },
              children: { when: 'never' },
            },
          ],
        },
      },
    });
    const ranges = [...source.matchAll(/[{}]/gu)].map((match) => [
      match.index,
      match.index + match[0].length,
    ]);

    expect(messageIds(reports)).toEqual([
      'spaceNeededAfter',
      'spaceNeededBefore',
      'noSpaceAfter',
      'noSpaceBefore',
    ]);
    expect(reports.map((report) => report.data)).toEqual([
      { token: '{' },
      { token: '}' },
      { token: '{' },
      { token: '}' },
    ]);
    expect(reports.map((report) => report.node.range)).toEqual(ranges);
    expect(applyReportFixes(source, reports)).toBe(
      'const marker = "😀日本語"; const view = <App attr={ value }>{child}</App>;\n',
    );
  });

  it('runs jsx-curly-spacing through real oxlint TSX lint and recursive fixes', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-curly-spacing-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(sourcePath, 'export const view = <App title={value}>{ child }</App>;\n');
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'stylistic/jsx-curly-spacing': [
              'error',
              {
                attributes: { when: 'always' },
                children: { when: 'never' },
              },
            ],
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
        { encoding: 'utf8' },
      );
      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(jsx-curly-spacing)',
          message: "A space is required after '{'",
        },
        {
          code: 'stylistic(jsx-curly-spacing)',
          message: "A space is required before '}'",
        },
        {
          code: 'stylistic(jsx-curly-spacing)',
          message: "There should be no space after '{'",
        },
        {
          code: 'stylistic(jsx-curly-spacing)',
          message: "There should be no space before '}'",
        },
      ]);

      const fixed = spawnSync(oxlint, ['-c', configPath, '--fix-suggestions', sourcePath], {
        encoding: 'utf8',
      });
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(sourcePath, 'utf8')).toBe(
        'export const view = <App title={ value }>{child}</App>;\n',
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('supports jsx-first-prop-new-line shared settings, UTF-16 ranges, and exact fixes', () => {
    const source = 'const marker = "😀"; const view = <DataTable<Items> first second />;\n';
    const firstStart = source.indexOf('first');
    const reports = runRule('jsx-first-prop-new-line', source, [], {
      corsaStylistic: {
        rules: {
          'jsx-first-prop-new-line': ['always'],
        },
      },
    });

    expect(messageIds(reports)).toEqual(['propOnNewLine']);
    expect(reports[0].data).toBeUndefined();
    expect(reports[0].node.range).toEqual([firstStart, firstStart + 'first'.length]);
    expect(reportFix(reports[0])).toEqual({
      range: [source.indexOf('> first') + 1, firstStart],
      replacementText: '\n',
    });
    expect(applyReportFixes(source, reports)).toBe(
      'const marker = "😀"; const view = <DataTable<Items>\nfirst second />;\n',
    );
  });

  it('runs jsx-first-prop-new-line through real Oxlint JSX and TSX lint and fixes', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-first-prop-new-line-'));

    try {
      const jsxPath = join(temp, 'sample.jsx');
      const tsxPath = join(temp, 'generic.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(jsxPath, 'export const view = <Panel first second />;\n');
      writeFileSync(
        tsxPath,
        'type Items = { id: string };\nexport const table = <DataTable<Items> first second />;\n',
      );
      writeFileSync(
        configPath,
        JSON.stringify({
          jsPlugins: [{ name: 'stylistic', specifier: join(packageRoot, 'index.js') }],
          rules: {
            'stylistic/jsx-first-prop-new-line': ['error', 'always'],
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', jsxPath, tsxPath],
        { encoding: 'utf8' },
      );
      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(JSON.parse(result.stdout).diagnostics).toMatchObject([
        {
          code: 'stylistic(jsx-first-prop-new-line)',
          message: 'Property should be placed on a new line',
        },
        {
          code: 'stylistic(jsx-first-prop-new-line)',
          message: 'Property should be placed on a new line',
        },
      ]);

      const fixed = spawnSync(oxlint, ['-c', configPath, '--fix-suggestions', jsxPath, tsxPath], {
        encoding: 'utf8',
      });
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(jsxPath, 'utf8')).toBe('export const view = <Panel\nfirst second />;\n');
      expect(readFileSync(tsxPath, 'utf8')).toBe(
        'type Items = { id: string };\nexport const table = <DataTable<Items>\nfirst second />;\n',
      );
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

  it('runs object-curly-newline through oxlint jsPlugins on TypeScript and TSX', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-object-curly-newline-'));

    try {
      const sourcePath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(
        sourcePath,
        [
          'type Shape = { first: string; second: number };',
          'export const View = () => <Panel value={{ first, second }} />;',
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
            'stylistic/object-curly-newline': [
              'error',
              {
                ObjectExpression: 'always',
                TSTypeLiteral: 'always',
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
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(object-curly-newline)',
        'stylistic(object-curly-newline)',
        'stylistic(object-curly-newline)',
        'stylistic(object-curly-newline)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Expected a line break after this opening brace.',
        'Expected a line break before this closing brace.',
        'Expected a line break after this opening brace.',
        'Expected a line break before this closing brace.',
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

  it('reports indent-binary-ops through real oxlint JS, TS, and TSX runs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-indent-binary-ops-plugin-'));

    try {
      const javaScriptPath = join(temp, 'sample.js');
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(javaScriptPath, 'export const total = first\n+ second;\n');
      writeFileSync(typeScriptPath, 'export type Value =\n| A\n    | B;\n');
      writeFileSync(tsxPath, 'export const view = <Box value={first\n+ second} />;\n');
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
            'stylistic/indent-binary-ops': 'error',
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', javaScriptPath, typeScriptPath, tsxPath],
        {
          encoding: 'utf8',
        },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(indent-binary-ops)',
        'stylistic(indent-binary-ops)',
        'stylistic(indent-binary-ops)',
        'stylistic(indent-binary-ops)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Expected indentation of 2 spaces',
        'Expected indentation of 2 spaces',
        'Expected indentation of 2 spaces',
        'Expected indentation of 2 spaces',
      ]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs member-delimiter-style through real oxlint TypeScript', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-member-delimiter-style-'));

    try {
      const sourcePath = join(temp, 'sample.ts');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(
        sourcePath,
        ['interface 日本語 {', '  first: string,', '  second(): number,', '}', ''].join('\n'),
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
            'stylistic/member-delimiter-style': 'error',
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
        'stylistic(member-delimiter-style)',
        'stylistic(member-delimiter-style)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Expected a semicolon.',
        'Expected a semicolon.',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.labels[0].span.length)).toEqual([0, 0]);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs semi diagnostics and fixes through real oxlint JS, TS, and TSX configs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-semi-plugin-'));

    try {
      const javaScriptPath = join(temp, 'sample.js');
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');

      writeFileSync(javaScriptPath, 'export const value = 1;\n');
      writeFileSync(typeScriptPath, 'export type Name = string;\ndeclare function run(): void;\n');
      writeFileSync(tsxPath, 'export const view = <div>😀</div>;\n');
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
            'stylistic/semi': ['error', 'never'],
          },
        }),
      );

      const result = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', javaScriptPath, typeScriptPath, tsxPath],
        { encoding: 'utf8' },
      );

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      const diagnostics = JSON.parse(result.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(semi)',
        'stylistic(semi)',
        'stylistic(semi)',
        'stylistic(semi)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Extra semicolon.',
        'Extra semicolon.',
        'Extra semicolon.',
        'Extra semicolon.',
      ]);

      const fixed = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--fix-suggestions', javaScriptPath, typeScriptPath, tsxPath],
        { encoding: 'utf8' },
      );
      expect(fixed.status).toBe(0);
      expect(readFileSync(javaScriptPath, 'utf8')).toBe('export const value = 1\n');
      expect(readFileSync(typeScriptPath, 'utf8')).toBe(
        'export type Name = string\ndeclare function run(): void\n',
      );
      expect(readFileSync(tsxPath, 'utf8')).toBe('export const view = <div>😀</div>\n');
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('runs and fixes jsx-curly-newline through real oxlint JSX and TSX', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-curly-newline-plugin-'));

    try {
      const jsxPath = join(temp, 'sample.jsx');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      writeFileSync(jsxPath, 'export const view = <App value={\nfoo\n} />;\n');
      writeFileSync(
        tsxPath,
        [
          'type Props = { value: string };',
          'export const view: JSX.Element = <div>{',
          'foo',
          '}</div>;',
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
            'stylistic/jsx-curly-newline': ['error', 'never'],
          },
        }),
      );

      const lint = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', jsxPath, tsxPath],
        {
          encoding: 'utf8',
        },
      );
      expect(lint.status).toBe(1);
      expect(lint.stderr).toBe('');
      expect(JSON.parse(lint.stdout).diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(jsx-curly-newline)',
        'stylistic(jsx-curly-newline)',
        'stylistic(jsx-curly-newline)',
        'stylistic(jsx-curly-newline)',
      ]);

      const fixed = spawnSync(oxlint, ['-c', configPath, '--fix-suggestions', jsxPath, tsxPath], {
        encoding: 'utf8',
      });
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(jsxPath, 'utf8')).toBe('export const view = <App value={foo} />;\n');
      expect(readFileSync(tsxPath, 'utf8')).toBe(
        [
          'type Props = { value: string };',
          'export const view: JSX.Element = <div>{foo}</div>;',
          '',
        ].join('\n'),
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('supports wrap-iife styles, prototype methods, shared settings, and exact fixes', () => {
    const source = 'const 日本語 = function () {}();';
    const reports = runRule('wrap-iife', source, ['inside']);

    expect(messageIds(reports)).toEqual(['wrapInvocation']);
    expect(reports[0].node.range).toEqual([source.indexOf('function'), source.lastIndexOf(';')]);
    expect(applyReportFixes(source, reports)).toBe('const 日本語 = (function () {})();');

    const prototypeSource = 'const value = function () {}.call(null);';
    const sharedReports = runRule('wrap-iife', prototypeSource, [], {
      corsaStylistic: {
        rules: {
          'wrap-iife': ['outside', { functionPrototypeMethods: true }],
        },
      },
    });
    expect(messageIds(sharedReports)).toEqual(['wrapInvocation']);
    expect(applyReportFixes(prototypeSource, sharedReports)).toBe(
      'const value = (function () {}.call(null));',
    );

    const commented = '(function () {} /* function */ () /* invocation */)';
    const commentReports = runRule('wrap-iife', commented, ['inside']);
    expect(applyReportFixes(commented, commentReports)).toBe(
      '(function () {}) /* function */ () /* invocation */',
    );
  });

  it('runs wrap-iife and applies suggestions through real oxlint JS, TS, and TSX runs', () => {
    const oxlint = findOxlintCli();
    const temp = mkdtempSync(join(tmpdir(), 'stylistic-wrap-iife-plugin-'));

    try {
      const javaScriptPath = join(temp, 'sample.js');
      const typeScriptPath = join(temp, 'sample.ts');
      const tsxPath = join(temp, 'sample.tsx');
      const configPath = join(temp, 'oxlint.config.jsonc');
      const javaScript = 'export const js = function () {}();\n';
      const typeScript = 'export const ts: number = function (): number { return 1 }();\n';
      const tsx =
        'export const view = <div>{function (): JSX.Element { return <span /> }()}</div>;\n';

      writeFileSync(javaScriptPath, javaScript);
      writeFileSync(typeScriptPath, typeScript);
      writeFileSync(tsxPath, tsx);
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
            'stylistic/wrap-iife': ['error', 'inside'],
          },
        }),
      );

      const lintResult = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--format', 'json', javaScriptPath, typeScriptPath, tsxPath],
        { encoding: 'utf8' },
      );
      expect(lintResult.status).toBe(1);
      expect(lintResult.stderr).toBe('');
      const diagnostics = JSON.parse(lintResult.stdout).diagnostics;
      expect(diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
        'stylistic(wrap-iife)',
        'stylistic(wrap-iife)',
        'stylistic(wrap-iife)',
      ]);
      expect(diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Wrap an immediate function invocation in parentheses.',
        'Wrap an immediate function invocation in parentheses.',
        'Wrap an immediate function invocation in parentheses.',
      ]);

      const fixResult = spawnSync(
        oxlint,
        ['-c', configPath, '--quiet', '--fix-suggestions', javaScriptPath, typeScriptPath, tsxPath],
        { encoding: 'utf8' },
      );
      expect(fixResult.status).toBe(0);
      expect(fixResult.stderr).toBe('');
      expect(readFileSync(javaScriptPath, 'utf8')).toBe('export const js = (function () {})();\n');
      expect(readFileSync(typeScriptPath, 'utf8')).toBe(
        'export const ts: number = (function (): number { return 1 })();\n',
      );
      expect(readFileSync(tsxPath, 'utf8')).toBe(
        'export const view = <div>{(function (): JSX.Element { return <span /> })()}</div>;\n',
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
});
