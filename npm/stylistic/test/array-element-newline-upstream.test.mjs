import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'array-element-newline.json'), 'utf8'),
);
const rule = plugin.rules['array-element-newline'];

function runRule(sourceText, options = []) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = rule.createOnce({
    options,
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
    if (sourceText[index] === '\r') {
      if (sourceText[index + 1] === '\n') {
        index++;
      }
      line++;
      column = 1;
    } else if (sourceText[index] === '\n') {
      line++;
      column = 1;
    } else {
      column++;
    }
  }
  return { line, column };
}

function actualError(sourceText, report) {
  const start = positionAt(sourceText, report.node.range[0]);
  const end = positionAt(sourceText, report.node.range[1]);
  return {
    messageId: report.messageId,
    message: rule.meta.messages[report.messageId],
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}

function fixedOutput(sourceText, reports) {
  const fixes = reports.flatMap(
    (report) =>
      report.suggest?.flatMap((suggestion) =>
        suggestion.fix({
          replaceTextRange(range, replacementText) {
            return { range, replacementText };
          },
        }),
      ) ?? [],
  );
  if (fixes.length === 0) {
    return null;
  }

  fixes.sort((left, right) => right.range[0] - left.range[0]);
  let output = sourceText;
  for (const fix of fixes) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

describe('@stylistic/array-element-newline v5.10.0 upstream replay', () => {
  it('is generated from the audited stable commit without dropped cases', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/array-element-newline/array-element-newline.test.ts',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-tests.ts',
    });
    expect(fixture.valid).toHaveLength(109);
    expect(fixture.invalid).toHaveLength(58);
  });

  it.each(fixture.valid)('accepts upstream valid fixture %#', (testCase) => {
    expect(runRule(testCase.code, testCase.options ?? []), testCase.code).toEqual([]);
  });

  it.each(fixture.invalid)('replays upstream invalid fixture %# exactly', (testCase) => {
    const reports = runRule(testCase.code, testCase.options ?? []);
    expect(reports, testCase.code).toHaveLength(testCase.errors.length);
    for (const [index, expected] of testCase.errors.entries()) {
      expect(actualError(testCase.code, reports[index]), testCase.code).toMatchObject(expected);
    }
    expect(
      reports.map((report) => report.messageId),
      testCase.code,
    ).toEqual(testCase.errors.map((error) => error.messageId));
    expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
  });
});
