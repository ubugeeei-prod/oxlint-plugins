import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/exp-list-style-v5.10.0.json', import.meta.url), 'utf8'),
);
const rule = plugin.rules['exp-list-style'];

function filenameFor(testCase) {
  return testCase.language === 'json' ? 'fixture.json' : 'fixture.ts';
}

function runRule(sourceText, options, filename) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    options: options ?? [],
    filename,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderMessage(report) {
  return rule.meta.messages[report.messageId].replace(
    /\{\{(\w+)\}\}/gu,
    (_match, key) => report.data?.[key] ?? '',
  );
}

function positionAt(source, offset) {
  let line = 1;
  let column = 1;
  for (let index = 0; index < offset; index += 1) {
    const character = source[index];
    if (character === '\r') {
      if (source[index + 1] === '\n' && index + 1 < offset) {
        index += 1;
      }
      line += 1;
      column = 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
  }
  return { line, column };
}

function fixForReport(report) {
  const suggestion = report.suggest?.[0];
  if (!suggestion) {
    return null;
  }
  const edits = suggestion.fix({
    replaceTextRange(range, replacementText) {
      return { range, text: replacementText };
    },
  });
  expect(edits).toHaveLength(1);
  return edits[0];
}

function actualDiagnostic(source, report) {
  const start = positionAt(source, report.node.range[0]);
  const end = positionAt(source, report.node.range[1]);
  return {
    messageId: report.messageId,
    message: renderMessage(report),
    data: report.data,
    range: report.node.range,
    loc: {
      line: start.line,
      column: start.column,
      endLine: end.line,
      endColumn: end.column,
    },
    fix: fixForReport(report),
  };
}

function applySinglePass(source, reports) {
  const fixes = reports
    .map(fixForReport)
    .filter(Boolean)
    .sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  if (fixes.length === 0) {
    return null;
  }

  let output = '';
  let last = 0;
  for (const fix of fixes) {
    if (last > fix.range[0]) {
      continue;
    }
    output += source.slice(last, fix.range[0]) + fix.text;
    last = fix.range[1];
  }
  return output + source.slice(last);
}

function recursiveOutput(testCase) {
  let output = testCase.code;
  let changed = false;
  for (let pass = 0; pass < 100; pass += 1) {
    const reports = runRule(output, testCase.options, filenameFor(testCase));
    const next = applySinglePass(output, reports);
    if (next === null) {
      return changed ? output : null;
    }
    expect(next, testCase.code).not.toBe(output);
    output = next;
    changed = true;
  }
  throw new Error(`exp-list-style fixes did not converge:\n${output}`);
}

describe('@stylistic/exp-list-style v5.10.0 exhaustive authored parity', () => {
  it('pins every TypeScript and JSON authored case with exact provenance', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/list-style/list-style.test.ts',
        'packages/eslint-plugin/rules/list-style/list-style._json_.test.ts',
      ],
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        typescriptEslint: '8.56.0',
        typescript: '5.9.3',
        jsoncEslintParser: '2.4.2',
      },
      tool: 'tools/tasks/sync-stylistic-exp-list-style-tests.ts',
      inventory: {
        logicalValid: 56,
        logicalInvalid: 56,
        valid: 56,
        invalid: 56,
        diagnostics: 107,
        unfixableInvalid: 0,
        total: 112,
        fixableInvalid: 56,
        languages: {
          typescript: { valid: 52, invalid: 51 },
          json: { valid: 4, invalid: 5 },
        },
      },
    });
    expect(
      fixture.invalid.filter((testCase) => testCase.authoredOutput !== testCase.output),
    ).toHaveLength(11);
    expect(
      fixture.invalid
        .flatMap((testCase) => testCase.expectedDiagnostics)
        .filter((diagnostic) => diagnostic.fix === null),
    ).toHaveLength(4);
  });

  it.each(fixture.valid)('$language accepts every authored valid case %#', (testCase) => {
    expect(runRule(testCase.code, testCase.options, filenameFor(testCase)), testCase.code).toEqual(
      [],
    );
  });

  it.each(fixture.invalid)(
    '$language replays every authored invalid case %# exactly',
    (testCase) => {
      const reports = runRule(testCase.code, testCase.options, filenameFor(testCase));
      expect(
        reports.map((report) => actualDiagnostic(testCase.code, report)),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics);
      expect(applySinglePass(testCase.code, reports), testCase.code).toBe(testCase.authoredOutput);
      expect(recursiveOutput(testCase), testCase.code).toBe(testCase.output);
    },
  );

  it('covers JavaScript, JSX, TSX, Unicode, comments, and all ECMAScript line endings', () => {
    for (const [source, filename] of [
      ['const 日本語 = [ 1 ];', 'fixture.js'],
      ['const view = <Comp value={[ 1 ]} />;', 'fixture.jsx'],
      ['const view: JSX.Element = <Comp value={[ 1 ]} />;', 'fixture.tsx'],
    ]) {
      expect(
        runRule(source, [], filename).map((report) => report.messageId),
        `${filename}: ${source}`,
      ).toEqual(['shouldNotSpacing', 'shouldNotSpacing']);
    }

    for (const lineEnding of ['\n', '\r\n', '\r', '\u2028', '\u2029']) {
      const source = `const 日本語 = [1,${lineEnding}2];`;
      expect(
        runRule(source, [], 'fixture.js').map((report) => report.messageId),
        JSON.stringify(source),
      ).toEqual(['shouldNotWrap']);
    }

    const commented = 'foo(a,\n// preserve\nb)';
    const reports = runRule(commented, [], 'fixture.js');
    expect(reports.map((report) => report.messageId)).toEqual(['shouldNotWrap']);
    expect(fixForReport(reports[0])).toBeNull();
  });
});
