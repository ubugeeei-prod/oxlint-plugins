import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(join(here, 'fixtures', 'wrap-iife-v5.10.0.json'), 'utf8'));
const rule = plugin.rules['wrap-iife'];

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

function positionAt(sourceText, offset) {
  let line = 1;
  let column = 1;
  for (let index = 0; index < offset; index += 1) {
    const character = sourceText[index];
    if (character === '\r') {
      if (sourceText[index + 1] === '\n' && index + 1 < offset) {
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
  for (let pass = 0; pass < 10; pass += 1) {
    const reports = runRule(output, options, filename);
    const next = fixedOutput(output, reports);
    if (next === null) {
      return changed ? output : null;
    }
    expect(next, output).not.toBe(output);
    output = next;
    changed = true;
  }
  throw new Error(`wrap-iife fixes did not converge:\n${output}`);
}

describe('@stylistic/wrap-iife v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned stable inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/wrap-iife/wrap-iife.test.ts',
      license: 'MIT',
      eslintVersion: '10.4.1',
      tool: 'tools/tasks/sync-stylistic-wrap-iife-tests.ts',
      inventory: {
        valid: 86,
        invalid: 42,
        diagnostics: 42,
        unfixableInvalid: 0,
        total: 128,
        fixableInvalid: 42,
      },
    });

    const messages = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messages[diagnostic.messageId] = (messages[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messages).toEqual({
      wrapInvocation: 12,
      moveInvocation: 3,
      wrapExpression: 27,
    });
    expect(
      new Set(
        [...fixture.valid, ...fixture.invalid].map((testCase) => JSON.stringify(testCase.options)),
      ).size,
    ).toBe(9);
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
      expect(recursiveOutput(testCase.code, testCase.options, 'fixture.js'), testCase.code).toBe(
        testCase.recursiveOutput,
      );
    },
  );

  it('maps Unicode, CRLF, and Unicode separators to exact UTF-16 ranges and fixes', () => {
    const source = [
      "const 日本語 = function () { return '😀'; }();\r\n",
      'const café = function () {}();\u2028',
      'const τέλος = function () {}();\u2029',
    ].join('');
    const reports = runRule(source, ['inside'], 'fixture.ts');

    expect(reports.map((report) => report.messageId)).toEqual([
      'wrapInvocation',
      'wrapInvocation',
      'wrapInvocation',
    ]);
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual([
      "function () { return '😀'; }()",
      'function () {}()',
      'function () {}()',
    ]);
    expect(fixedOutput(source, reports)).toBe(
      [
        "const 日本語 = (function () { return '😀'; })();\r\n",
        'const café = (function () {})();\u2028',
        'const τέλος = (function () {})();\u2029',
      ].join(''),
    );
  });

  it('covers TypeScript, TSX, comments, optional chains, and invalid input', () => {
    const ts = 'const value: number = function (): number { return 1 }();';
    const tsx = 'const view = <div>{function (): JSX.Element { return <span /> }()}</div>;';
    const optional = 'const value = function () {}?.call?.(null);';
    const comments = '(function () {} /* function */ () /* invocation */)';

    expect(runRule(ts, ['outside'], 'fixture.ts')).toHaveLength(1);
    expect(runRule(tsx, ['inside'], 'fixture.tsx')).toHaveLength(1);
    expect(
      runRule(optional, ['inside', { functionPrototypeMethods: true }], 'fixture.js'),
    ).toHaveLength(1);
    expect(fixedOutput(comments, runRule(comments, ['inside']))).toBe(
      '(function () {}) /* function */ () /* invocation */',
    );
    expect(runRule('const = function () {}()', [], 'fixture.js')).toEqual([]);
    expect(runRule('<div>{function () {}(}</div>', [], 'fixture.tsx')).toEqual([]);
  });

  it('falls back safely for invalid option payloads', () => {
    const source = 'const value = function () {}();';
    for (const options of [
      ['sideways'],
      [42, { functionPrototypeMethods: 'yes' }],
      { functionPrototypeMethods: true },
      null,
    ]) {
      const reports = runRule(source, options);
      expect(reports.map((report) => report.messageId)).toEqual(['wrapInvocation']);
      expect(fixedOutput(source, reports)).toBe('const value = (function () {}());');
    }
  });
});
