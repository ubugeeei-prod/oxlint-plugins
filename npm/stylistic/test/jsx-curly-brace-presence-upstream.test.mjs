import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-curly-brace-presence-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function filename(parser) {
  return parser === 'tsx' ? 'fixture.tsx' : 'fixture.jsx';
}

function runRule(sourceText, options = [], sourceFilename = 'fixture.tsx') {
  const reports = [];
  const visitor = plugin.rules['jsx-curly-brace-presence'].createOnce({
    filename: sourceFilename,
    options,
    sourceCode: {
      text: sourceText,
      getText() {
        return this.text;
      },
    },
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderedMessage(report) {
  let message = plugin.rules['jsx-curly-brace-presence'].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data ?? {})) {
    message = message.replaceAll(`{{${key}}}`, value);
  }
  return message;
}

function reportFix(report) {
  const suggestion = report.suggest?.[0];
  if (!suggestion) {
    return null;
  }
  const fixes = suggestion.fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  });
  expect(fixes).toHaveLength(1);
  return fixes[0];
}

function fixedPass(source, reports) {
  const fixes = reports
    .map((report, index) => ({ index, fix: reportFix(report) }))
    .filter(({ fix }) => fix !== null)
    .sort(
      (left, right) =>
        left.fix.range[0] - right.fix.range[0] ||
        left.fix.range[1] - right.fix.range[1] ||
        left.index - right.index,
    );
  if (fixes.length === 0) {
    return null;
  }
  const accepted = [];
  let lastEnd = Number.NEGATIVE_INFINITY;
  for (const { fix } of fixes) {
    if (lastEnd >= fix.range[0]) {
      continue;
    }
    accepted.push(fix);
    lastEnd = fix.range[1];
  }
  let output = source;
  for (const fix of accepted.reverse()) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

function recursivelyFixed(source, options, sourceFilename) {
  let output = source;
  let fixed = false;
  const seen = new Set([source]);
  for (let iteration = 0; iteration < 10; iteration += 1) {
    const next = fixedPass(output, runRule(output, options, sourceFilename));
    if (next === null) {
      return fixed ? output : null;
    }
    fixed = true;
    if (seen.has(next)) {
      return next;
    }
    seen.add(next);
    output = next;
  }
  return output;
}

describe('@stylistic/jsx-curly-brace-presence v5.10.0 exhaustive upstream parity', () => {
  it('pins the exact source and complete authored/parser-expanded inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-curly-brace-presence/jsx-curly-brace-presence.test.ts',
      ruleFile: 'packages/eslint-plugin/rules/jsx-curly-brace-presence/jsx-curly-brace-presence.ts',
      license: 'MIT',
      eslintVersion: '9.39.2',
      typescriptEslintParserVersion: '8.60.0',
      babelEslintParserVersion: '7.28.6',
      parserMatrix: 'authored semantic cases replayed with Oxc-compatible JSX and TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-curly-brace-presence-tests.ts',
      inventory: {
        authoredValid: 89,
        authoredInvalid: 46,
        authoredDiagnostics: 62,
        exactDiagnostics: 64,
        fixableInvalid: 46,
        unfixableInvalid: 0,
        authoredTotal: 135,
        parserExpandedValid: 175,
        parserExpandedInvalid: 88,
        parserExpandedDiagnostics: 118,
        parserExpandedTotal: 263,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts authored valid case %i in every compatible parser mode',
    (_index, testCase) => {
      for (const parser of testCase.parsers) {
        expect(
          runRule(testCase.code, testCase.options, filename(parser)),
          `${parser}\n${testCase.code}`,
        ).toEqual([]);
      }
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays invalid case %i with exact reports, first-pass fixes, and convergence',
    (_index, testCase) => {
      for (const parser of testCase.parsers) {
        const sourceFilename = filename(parser);
        const reports = runRule(testCase.code, testCase.options, sourceFilename);
        expect(reports, `${parser}\n${testCase.code}`).toHaveLength(
          testCase.expectedDiagnostics.length,
        );
        expect(
          reports.map((report) => ({
            messageId: report.messageId,
            message: renderedMessage(report),
            data: report.data ?? {},
            range: report.node.range,
            fix: reportFix(report)
              ? {
                  range: reportFix(report).range,
                  text: reportFix(report).replacementText,
                }
              : null,
          })),
          `${parser}\n${testCase.code}`,
        ).toEqual(testCase.expectedDiagnostics.map(({ loc: _loc, ...diagnostic }) => diagnostic));
        expect(fixedPass(testCase.code, reports), `${parser}\n${testCase.code}`).toBe(
          testCase.firstPassOutput,
        );
        expect(
          recursivelyFixed(testCase.code, testCase.options, sourceFilename),
          `${parser}\n${testCase.code}`,
        ).toBe(testCase.recursiveOutput);
      }
    },
  );

  it('maps UTF-8 native ranges to exact UTF-16 ranges and preserves source order', () => {
    const source =
      "const marker = \"😀日本語\"; const view = <App title={'plain'}>{'outer'}<B>{'inner'}</B>{'tail'}</App>;";
    const reports = runRule(source, ['never']);
    const ranges = [...source.matchAll(/\{(?:'plain'|'outer'|'inner'|'tail')\}/gu)].map((match) => [
      match.index,
      match.index + match[0].length,
    ]);
    expect(reports.map((report) => report.node.range)).toEqual(ranges);
    expect(reports.map((report) => report.messageId)).toEqual(
      Array.from({ length: 4 }, () => 'unnecessaryCurly'),
    );
    expect(fixedPass(source, reports)).toBe(
      'const marker = "😀日本語"; const view = <App title="plain">outer<B>inner</B>tail</App>;',
    );
  });

  it('covers all line terminators, comments, entities, escapes, fragments, and invalid syntax', () => {
    const lineTerminatorOutputs = new Map([
      ['\r\n', '<App>{"before\\r"}\n{"after"}</App>'],
      ['\r', '<App>{"before\\rafter"}</App>'],
      ['\n', '<App>{"before"}\n{"after"}</App>'],
      ['\u2028', '<App>{"before\u2028after"}</App>'],
      ['\u2029', '<App>{"before\u2029after"}</App>'],
    ]);
    for (const [terminator, expectedOutput] of lineTerminatorOutputs) {
      const source = `<App>before${terminator}after</App>`;
      const reports = runRule(source, [{ children: 'always' }]);
      expect(reports, JSON.stringify(terminator)).toHaveLength(1);
      expect(reports[0].messageId).toBe('missingCurly');
      expect(fixedPass(source, reports), JSON.stringify(terminator)).toBe(expectedOutput);
    }
    for (const source of [
      "<App>{/* retain */ 'text'}</App>",
      "<App>{'left'}{'right'}</App>",
      "<App>{' '}<B /></App>",
      '<App>&nbsp;</App>',
      "<App>{'Hello \\\\n world'}</App>",
      "<App>{'Hello &middot; world'}</App>",
      "<App>{'<Component />'}</App>",
    ]) {
      expect(runRule(source, ['never']), source).toEqual([]);
    }
    expect(runRule('<App>{<>text</>}</App>', [{ children: 'never' }])).toHaveLength(1);
    expect(runRule('<App>{broken</App>', ['never'])).toEqual([]);
    expect(runRule('const text = "plain";', ['always'], 'fixture.ts')).toEqual([]);
  });
});
