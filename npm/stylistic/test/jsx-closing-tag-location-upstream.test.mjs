import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE = 'jsx-closing-tag-location';
const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-closing-tag-location-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[RULE].createOnce({
    filename: 'fixture.tsx',
    options: options ?? [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function locationAt(source, offset) {
  let line = 1;
  let lineStart = 0;
  for (let index = 0; index < offset; index += 1) {
    const character = source[index];
    if (character === '\r') {
      if (source[index + 1] === '\n') {
        index += 1;
      }
      line += 1;
      lineStart = index + 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      line += 1;
      lineStart = index + 1;
    }
  }
  return { line, column: offset - lineStart + 1 };
}

function resolveMessage(report) {
  const template = plugin.rules[RULE].meta.messages[report.messageId];
  return template.replace(/\{\{([^}]+)\}\}/gu, (_match, key) => report.data?.[key] ?? '');
}

function reportFix(report) {
  const suggestion = report.suggest?.[0];
  if (!suggestion) {
    return null;
  }
  return suggestion.fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  })[0];
}

function normalizeReport(source, report) {
  const start = locationAt(source, report.node.range[0]);
  const end = locationAt(source, report.node.range[1]);
  return {
    messageId: report.messageId,
    message: resolveMessage(report),
    data: report.data ?? {},
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
    range: report.node.range,
    fix: reportFix(report),
  };
}

function applyFixesUntilStable(source, options) {
  let output = source;
  let changed = false;

  for (let pass = 0; pass < 10; pass += 1) {
    const fixes = runRule(output, options)
      .map((report, index) => ({ index, fix: reportFix(report) }))
      .filter(({ fix }) => fix !== null)
      .sort(
        (left, right) =>
          left.fix.range[0] - right.fix.range[0] ||
          left.fix.range[1] - right.fix.range[1] ||
          left.index - right.index,
      );
    if (fixes.length === 0) {
      return changed ? output : null;
    }

    const accepted = [];
    let lastEnd = -1;
    for (const { fix } of fixes) {
      if (lastEnd >= fix.range[0]) {
        continue;
      }
      accepted.push(fix);
      lastEnd = fix.range[1];
    }
    let next = output;
    for (const fix of accepted.reverse()) {
      next = next.slice(0, fix.range[0]) + fix.replacementText + next.slice(fix.range[1]);
    }
    if (next === output) {
      throw new Error(`${RULE} produced a non-progressing fix`);
    }
    output = next;
    changed = true;
  }

  throw new Error(`${RULE} fixes did not converge within 10 passes`);
}

describe('jsx-closing-tag-location upstream v5.10.0 parity', () => {
  it('keeps the deterministic pinned parser-matrix inventory complete', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-closing-tag-location/jsx-closing-tag-location.test.ts',
      ruleFile: 'packages/eslint-plugin/rules/jsx-closing-tag-location/jsx-closing-tag-location.ts',
      parserMatrixFile: 'shared/test-utils/parsers-jsx.ts',
      sourceSha256: '969057cf48a7ebe4ad00e1f08e410ca64f81e0cce6161b55d32e0a6b2550aa9d',
      ruleSourceSha256: '712d13d53f8c069fbc31ff896a8a2d32fe510ceaa93e98285e76dcd3ee56a7ac',
      parserMatrixSourceSha256: '64dd12d67eac1eadf8a5a93de02bbb76c1d764c0ec7ebbdaae0c45389b52435c',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-closing-tag-location-tests.ts',
      inventory: {
        valid: 28,
        invalid: 16,
        diagnostics: 16,
        fixableInvalid: 16,
        unfixableInvalid: 0,
        total: 44,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid parser-matrix case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays exact diagnostics and converged fix for upstream invalid case %i',
    (_index, testCase) => {
      expect(
        runRule(testCase.code, testCase.options).map((report) =>
          normalizeReport(testCase.code, report),
        ),
      ).toEqual(testCase.diagnostics);
      expect(applyFixesUntilStable(testCase.code, testCase.options)).toBe(testCase.output);
    },
  );
});
