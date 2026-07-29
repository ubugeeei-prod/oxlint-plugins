import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(join(here, 'fixtures', 'semi-v5.10.0.json'), 'utf8'));
const rule = plugin.rules.semi;

function runRule(sourceText, options = [], filename = 'fixture.js') {
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

function filenameFor(testCase) {
  return testCase.language === 'ts' ? 'fixture.ts' : 'fixture.js';
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
      ...(start === end && start === sourceText.length
        ? {}
        : {
            endLine: endPosition.line,
            endColumn: endPosition.column,
          }),
    },
    fix: {
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

function recursiveOutput(sourceText, options, filename) {
  let output = sourceText;
  let changed = false;
  for (let pass = 0; pass < 10; pass++) {
    const reports = runRule(output, options, filename);
    const next = fixedOutput(output, reports);
    if (next === null) {
      return changed ? output : null;
    }
    expect(next, output).not.toBe(output);
    output = next;
    changed = true;
  }
  throw new Error(`semi fixes did not converge:\n${output}`);
}

describe('@stylistic/semi v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned JS and TypeScript inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/semi/semi._js_.test.ts',
        'packages/eslint-plugin/rules/semi/semi._ts_.test.ts',
      ],
      license: 'MIT',
      eslintVersion: '10.4.1',
      typescriptEslintParserVersion: '8.60.0',
      tool: 'tools/tasks/sync-stylistic-semi-tests.ts',
      inventory: {
        valid: 199,
        invalid: 152,
        diagnostics: 158,
        unfixableInvalid: 0,
        total: 351,
        fixableInvalid: 152,
        javascript: { valid: 179, invalid: 130 },
        typescript: { valid: 20, invalid: 22 },
      },
    });

    const messages = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messages[diagnostic.messageId] = (messages[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messages).toEqual({
      extraSemi: 78,
      missingSemi: 80,
    });
    expect(
      new Set(
        [...fixture.valid, ...fixture.invalid].map((testCase) => JSON.stringify(testCase.options)),
      ).size,
    ).toBe(11);
  });

  it.each(fixture.valid)('accepts every upstream valid case %#', (testCase) => {
    expect(runRule(testCase.code, testCase.options, filenameFor(testCase)), testCase.code).toEqual(
      [],
    );
  });

  it.each(fixture.invalid)(
    'replays every upstream invalid diagnostic and fix %# exactly',
    (testCase) => {
      const filename = filenameFor(testCase);
      const reports = runRule(testCase.code, testCase.options, filename);
      expect(
        reports.map((report) => actualDiagnostic(testCase.code, report)),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics);
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(recursiveOutput(testCase.code, testCase.options, filename), testCase.code).toBe(
        testCase.recursiveOutput,
      );
    },
  );

  it('maps Unicode TSX, CRLF, and Unicode separators to exact UTF-16 ranges', () => {
    const source = [
      'type 日本語 = { value: string };\r\n',
      'declare function café(): void;\u2028',
      'const view = <div>😀</div>;\u2029',
    ].join('');
    const reports = runRule(source, ['never'], 'fixture.tsx');
    const semicolons = [...source.matchAll(/;/gu)].map((match) => match.index);

    expect(reports.map((report) => report.messageId)).toEqual([
      'extraSemi',
      'extraSemi',
      'extraSemi',
    ]);
    expect(reports.map((report) => report.node.range)).toEqual(
      semicolons.map((offset) => [offset, offset + 1]),
    );
    expect(reports.flatMap(fixesFor).map((fix) => source.slice(...fix.range))).toEqual([
      '};\r\ndeclare',
      'void;\u2028const',
      '>;',
    ]);
  });

  it('preserves comments while reproducing FixTracker surrounding-token ranges', () => {
    const source = 'first(); /* between */\nsecond(); // trailing\nthird();';
    const reports = runRule(source, ['never']);
    const output = fixedOutput(source, reports);

    expect(reports.map((report) => report.messageId)).toEqual([
      'extraSemi',
      'extraSemi',
      'extraSemi',
    ]);
    expect(output).toBe('first() /* between */\nsecond() // trailing\nthird()');
    expect(reports.flatMap(fixesFor).every((fix) => fix.range[0] < fix.range[1])).toBe(true);
  });
});
