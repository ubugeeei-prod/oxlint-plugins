import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'member-delimiter-style-v5.10.0.json'), 'utf8'),
);
const rule = plugin.rules['member-delimiter-style'];

function runRule(sourceText, options = [], filename = 'fixture.ts') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    options,
    filename,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function positionAt(sourceText, offset) {
  let line = 1;
  let column = 1;
  for (let index = 0; index < offset; index++) {
    const character = sourceText[index];
    if (character === '\r') {
      if (sourceText[index + 1] === '\n' && index + 1 < offset) {
        index++;
      }
      line++;
      column = 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line++;
      column = 1;
    } else {
      column++;
    }
  }
  return { line, column };
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

function actualDiagnostic(sourceText, report) {
  const [start, end] = report.node.range;
  const startPosition = positionAt(sourceText, start);
  const endPosition = positionAt(sourceText, end);
  const fixes = fixesFor(report);
  return {
    messageId: report.messageId,
    message: rule.meta.messages[report.messageId],
    data: report.data ?? {},
    range: [start, end],
    loc: {
      line: startPosition.line,
      column: startPosition.column,
      endLine: endPosition.line,
      endColumn: endPosition.column,
    },
    fix:
      fixes.length === 0
        ? null
        : {
            range: fixes[0].range,
            text: fixes[0].replacementText,
          },
  };
}

function fixedOutput(sourceText, reports) {
  const fixes = reports.flatMap(fixesFor);
  if (fixes.length === 0) {
    return null;
  }
  fixes.sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);
  let output = sourceText;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

function recursiveOutput(sourceText, options) {
  let output = sourceText;
  let changed = false;
  for (let pass = 0; pass < 10; pass++) {
    const reports = runRule(output, options);
    const next = fixedOutput(output, reports);
    if (next === null) {
      return changed ? output : null;
    }
    expect(next, output).not.toBe(output);
    output = next;
    changed = true;
  }
  throw new Error(`member-delimiter-style fixes did not converge:\n${output}`);
}

describe('@stylistic/member-delimiter-style v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned stable inventory and message distribution', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/member-delimiter-style/member-delimiter-style.test.ts',
      license: 'MIT',
      eslintVersion: '10.4.1',
      typescriptEslintParserVersion: '8.60.0',
      tool: 'tools/tasks/sync-stylistic-member-delimiter-style-tests.ts',
      inventory: {
        valid: 61,
        invalid: 99,
        diagnostics: 153,
        unfixableInvalid: 1,
        total: 160,
        fixableInvalid: 98,
      },
    });

    const messages = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messages[diagnostic.messageId] = (messages[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messages).toEqual({
      expectedComma: 46,
      expectedSemi: 42,
      unexpectedComma: 25,
      unexpectedSemi: 40,
    });
    expect(new Set(fixture.invalid.map((testCase) => JSON.stringify(testCase.options))).size).toBe(
      41,
    );
  });

  it.each(fixture.valid)('accepts every upstream valid case %#', (testCase) => {
    expect(runRule(testCase.code, testCase.options), testCase.code).toEqual([]);
  });

  it.each(fixture.invalid)(
    'replays every upstream invalid diagnostic and fix %# exactly',
    (testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => actualDiagnostic(testCase.code, report)),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics);
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(recursiveOutput(testCase.code, testCase.options), testCase.code).toBe(
        testCase.recursiveOutput,
      );
    },
  );

  it('maps UTF-8 diagnostics and fixes to UTF-16 in TSX with CRLF and Unicode separators', () => {
    const source = [
      'type 日本語 = {\r\n  propriété: string,\r\n  絵文字: "😀",\r\n};',
      'export const view = <div />;\u2028',
    ].join('\n');
    const reports = runRule(source, [], 'fixture.tsx');
    const delimiters = [...source.matchAll(/,/gu)].map((match) => match.index);

    expect(reports.map((report) => report.messageId)).toEqual(['expectedSemi', 'expectedSemi']);
    expect(reports.map((report) => report.node.range)).toEqual(
      delimiters.map((offset) => [offset + 1, offset + 1]),
    );
    expect(reports.flatMap(fixesFor)).toEqual(
      delimiters.map((offset) => ({
        range: [offset, offset + 1],
        replacementText: ';',
      })),
    );
  });

  it('keeps unsafe inline removals diagnostic-only while fixing end-of-line comments', () => {
    const source = 'type T = {\n  first: string; second: number;\n  third: boolean; // safe\n}\n';
    const reports = runRule(source, [{ multiline: { delimiter: 'none', requireLast: true } }]);

    expect(reports.map((report) => report.messageId)).toEqual([
      'unexpectedSemi',
      'unexpectedSemi',
      'unexpectedSemi',
    ]);
    expect(reports.map((report) => fixesFor(report).length)).toEqual([0, 1, 1]);
  });
});
