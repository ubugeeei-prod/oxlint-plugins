import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE = 'jsx-indent-props';
const rule = plugin.rules[RULE];
const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-indent-props-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options = [], { filename = 'fixture.tsx', settings } = {}) {
  const reports = [];
  const visitor = rule.createOnce({
    options,
    filename,
    settings,
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

function fixesFor(report) {
  return (
    report.suggest?.flatMap((suggestion) =>
      suggestion.fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    ) ?? []
  );
}

function renderedMessage(report) {
  return rule.meta.messages[report.messageId].replace(
    /\{\{\s*([^{}\s]+)\s*\}\}/gu,
    (_, key) => report.data[key],
  );
}

function actualDiagnostic(report) {
  const fixes = fixesFor(report);
  return {
    messageId: report.messageId,
    message: renderedMessage(report),
    data: report.data ?? {},
    range: report.node.range,
    fix:
      fixes.length === 0
        ? null
        : {
            range: fixes[0].range,
            text: fixes[0].replacementText,
          },
  };
}

function fixedOutput(source, reports) {
  const fixes = reports.flatMap(fixesFor);
  if (fixes.length === 0) {
    return null;
  }
  fixes.sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  let output = source;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

function recursiveOutput(source, options, filename) {
  let output = source;
  let changed = false;
  for (let pass = 0; pass < 10; pass += 1) {
    const next = fixedOutput(output, runRule(output, options, { filename }));
    if (next === null || next === output) {
      return changed ? output : next;
    }
    output = next;
    changed = true;
  }
  throw new Error(`jsx-indent-props fixes did not converge:\n${output}`);
}

function rangeFromLoc(source, loc) {
  function offsetAt(line, column) {
    let offset = 0;
    let currentLine = 1;
    while (currentLine < line) {
      const match = /\r\n|[\n\r\u2028\u2029]/u.exec(source.slice(offset));
      if (!match) {
        throw new Error(`Cannot map ${line}:${column}`);
      }
      offset += match.index + match[0].length;
      currentLine += 1;
    }
    return offset + column - 1;
  }
  return [offsetAt(loc.line, loc.column), offsetAt(loc.endLine, loc.endColumn)];
}

describe('@stylistic/jsx-indent-props v5.10.0 exhaustive upstream replay', () => {
  it('pins the complete parser-expanded stable inventory and all options', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-indent-props/jsx-indent-props.test.ts',
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
        typescript: '5.9.3',
      },
      parserMatrix: 'ESLint 10 parsers-jsx expansion; Babel disabled by the stable runner',
      tool: 'tools/tasks/sync-stylistic-jsx-indent-props-tests.ts',
      inventory: {
        logicalValid: 23,
        logicalInvalid: 17,
        valid: 46,
        invalid: 34,
        diagnostics: 42,
        unfixableInvalid: 0,
        total: 80,
        fixableInvalid: 34,
      },
    });

    const parserCounts = {};
    for (const testCase of [...fixture.valid, ...fixture.invalid]) {
      parserCounts[testCase.parser] = (parserCounts[testCase.parser] ?? 0) + 1;
    }
    expect(parserCounts).toEqual({ espree: 40, typescript: 40 });

    const options = new Set(
      [...fixture.valid, ...fixture.invalid].map((testCase) => JSON.stringify(testCase.options)),
    );
    expect(options).toEqual(
      new Set([
        '[]',
        '[2]',
        '[0]',
        '[-2]',
        '["tab"]',
        '["first"]',
        '[{"indentMode":2,"ignoreTernaryOperator":false}]',
        '[{"indentMode":2,"ignoreTernaryOperator":true}]',
        '[{"indentMode":"tab","ignoreTernaryOperator":false}]',
        '[{"indentMode":"tab","ignoreTernaryOperator":true}]',
        '[{"indentMode":2}]',
      ]),
    );
    expect(fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)).toHaveLength(42);
  });

  it.each(fixture.valid)('accepts every upstream valid case %#', (testCase) => {
    const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
    expect(runRule(testCase.code, testCase.options, { filename }), testCase.code).toEqual([]);
  });

  it.each(fixture.invalid)(
    'replays every message, data field, location, range, fix, and recursive output %#',
    (testCase) => {
      const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
      const reports = runRule(testCase.code, testCase.options, { filename });
      expect(reports.map(actualDiagnostic), testCase.code).toEqual(
        testCase.expectedDiagnostics.map(({ loc: _loc, ...diagnostic }) => diagnostic),
      );
      for (const diagnostic of testCase.expectedDiagnostics) {
        expect(rangeFromLoc(testCase.code, diagnostic.loc), testCase.code).toEqual(
          diagnostic.range,
        );
      }
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(recursiveOutput(testCase.code, testCase.options, filename), testCase.code).toBe(
        testCase.recursiveOutput,
      );
    },
  );

  it('maps Unicode native byte offsets to UTF-16 and preserves first-mode columns', () => {
    const source = 'const emoji = "😀"; const view = <App<型> first\n  second />;';
    const reports = runRule(source, ['first']);
    expect(reports).toHaveLength(1);
    expect(reports[0]).toMatchObject({
      messageId: 'wrongIndent',
      data: {
        needed: 41,
        type: 'space',
        characters: 'characters',
        gotten: 2,
      },
      node: { range: [source.indexOf('second'), source.indexOf('second') + 6] },
    });
    expect(fixesFor(reports[0])).toEqual([
      {
        range: [source.indexOf('\n') + 1, source.indexOf('second')],
        replacementText: ' '.repeat(41),
      },
    ]);
  });

  it('honors shared settings for every ECMAScript line terminator', () => {
    for (const separator of ['\n', '\r\n', '\r', '\u2028', '\u2029']) {
      const source = `<App${separator}prop />`;
      const reports = runRule(source, [], {
        settings: {
          corsaStylistic: {
            rules: {
              [RULE]: [{ indentMode: 2, ignoreTernaryOperator: true }],
            },
          },
        },
      });
      expect(
        reports.map((report) => report.data),
        JSON.stringify(separator),
      ).toEqual([
        {
          needed: 2,
          type: 'space',
          characters: 'characters',
          gotten: 0,
        },
      ]);
      expect(fixedOutput(source, reports)).toBe(`<App${separator}  prop />`);
    }
  });

  it('preserves comments, spreads, namespaces, members, nested order, and malformed input', () => {
    const source = '<svg:path\n/* keep */\nfoo\n{...props} />;\n<UI.Button\nbar />;';
    const reports = runRule(source, [2]);
    expect(reports).toHaveLength(3);
    expect(fixedOutput(source, reports)).toContain('/* keep */');

    const nested = '<Outer\nbad={<Inner\nx />}\nlater />;';
    expect(runRule(nested, [2]).map((report) => report.node.range[0])).toEqual([
      nested.indexOf('bad='),
      nested.indexOf('later'),
      nested.indexOf('x />'),
    ]);
    expect(runRule('<></>')).toEqual([]);
    expect(runRule('<App>')).toEqual([]);
  });
});
