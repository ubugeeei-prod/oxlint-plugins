import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE = 'jsx-tag-spacing';
const rule = plugin.rules[RULE];
const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-tag-spacing-v5.10.0.json', import.meta.url), 'utf8'),
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

function actualDiagnostic(report) {
  const [start, end] = report.node.range;
  const fixes = fixesFor(report);
  return {
    messageId: report.messageId,
    message: rule.meta.messages[report.messageId],
    data: report.data ?? {},
    range: [start, end],
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
  throw new Error(`jsx-tag-spacing fixes did not converge:\n${output}`);
}

describe('@stylistic/jsx-tag-spacing v5.10.0 exhaustive upstream replay', () => {
  it('pins the complete parser-expanded stable inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-tag-spacing/jsx-tag-spacing.test.ts',
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
        typescript: '5.9.3',
      },
      parserMatrix: 'ESLint 10 parsers-jsx expansion; Babel disabled by the stable runner',
      tool: 'tools/tasks/sync-stylistic-jsx-tag-spacing-tests.ts',
      inventory: {
        logicalValid: 38,
        logicalInvalid: 36,
        valid: 74,
        invalid: 69,
        diagnostics: 73,
        unfixableInvalid: 0,
        total: 143,
        fixableInvalid: 69,
      },
    });

    const parserCounts = {};
    for (const testCase of [...fixture.valid, ...fixture.invalid]) {
      parserCounts[testCase.parser] = (parserCounts[testCase.parser] ?? 0) + 1;
    }
    expect(parserCounts).toEqual({ espree: 74, typescript: 69 });

    const messageCounts = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messageCounts[diagnostic.messageId] = (messageCounts[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messageCounts).toEqual({
      afterOpenNeedSpace: 10,
      afterOpenNoSpace: 14,
      beforeCloseNeedNewline: 4,
      beforeCloseNeedSpace: 6,
      beforeCloseNoSpace: 6,
      beforeSelfCloseNeedNewline: 4,
      beforeSelfCloseNeedSpace: 12,
      beforeSelfCloseNoSpace: 8,
      closeSlashNeedSpace: 1,
      closeSlashNoSpace: 2,
      selfCloseSlashNeedSpace: 2,
      selfCloseSlashNoSpace: 4,
    });
  });

  it.each(fixture.valid)('accepts every upstream valid case %#', (testCase) => {
    const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
    expect(runRule(testCase.code, testCase.options, { filename }), testCase.code).toEqual([]);
  });

  it.each(fixture.invalid)(
    'replays every diagnostic, point/token range, fix, and recursive output %#',
    (testCase) => {
      const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
      const reports = runRule(testCase.code, testCase.options, { filename });
      expect(
        reports.map((report) => actualDiagnostic(report)),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics.map(({ loc: _loc, ...diagnostic }) => diagnostic));
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(recursiveOutput(testCase.code, testCase.options, filename), testCase.code).toBe(
        testCase.recursiveOutput,
      );
    },
  );

  it('maps Unicode native byte offsets back to exact UTF-16 ranges', () => {
    const source = 'const emoji = "😀"; const view = <外.部 値={1}/ >;';
    const options = [
      {
        closingSlash: 'never',
        beforeSelfClosing: 'allow',
        afterOpening: 'allow',
        beforeClosing: 'allow',
      },
    ];
    const reports = runRule(source, options);
    const slash = source.indexOf('/ >');
    expect(reports).toHaveLength(1);
    expect(reports[0].node.range).toEqual([slash, slash + 3]);
    expect(fixesFor(reports[0])).toEqual([
      {
        range: [slash + 1, slash + 2],
        replacementText: '',
      },
    ]);
  });

  it('honors shared settings and all ECMAScript line terminators', () => {
    for (const separator of ['\n', '\r\n', '\r', '\u2028', '\u2029']) {
      const source = `<App${separator}value={1}/>`;
      const reports = runRule(source, [], {
        settings: {
          corsaStylistic: {
            rules: {
              [RULE]: [
                {
                  closingSlash: 'allow',
                  beforeSelfClosing: 'proportional-always',
                  afterOpening: 'allow',
                  beforeClosing: 'allow',
                },
              ],
            },
          },
        },
      });
      expect(
        reports.map((report) => report.messageId),
        JSON.stringify(separator),
      ).toEqual(['beforeSelfCloseNeedNewline']);
    }
  });

  it('preserves fragments, comments, namespaces, members, and TSX generics', () => {
    for (const source of [
      '<></>',
      '<svg:path />',
      '<UI.Button />',
      '<App value={/* keep */ 1} />',
      'const view: JSX.Element = <List<Item> />;',
    ]) {
      expect(runRule(source), source).toEqual([]);
    }
  });
});
