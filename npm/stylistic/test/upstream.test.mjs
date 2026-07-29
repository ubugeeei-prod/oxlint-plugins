import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'type-annotation-spacing.json'), 'utf8'),
);
const braceStyleFixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'brace-style.json'), 'utf8'),
);
const functionCallArgumentNewlineFixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'function-call-argument-newline.json'), 'utf8'),
);
const indentBinaryOpsFixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'indent-binary-ops.json'), 'utf8'),
);

function runRule(sourceText, options) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['type-annotation-spacing'].createOnce({
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
  const prefix = source.slice(0, offset);
  const line = prefix.split('\n').length;
  const lineStart = prefix.lastIndexOf('\n') + 1;
  return {
    line,
    column: source.slice(lineStart, offset).length + 1,
  };
}

function applySuggestions(source, reports) {
  const edits = reports
    .flatMap((report) =>
      report.suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    )
    .sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);

  let output = source;
  for (const edit of edits) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

function runIndentBinaryOps(sourceText, options) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['indent-binary-ops'].createOnce({
    options: options ?? [],
    filename: 'fixture.ts',
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function applyIndentBinaryOpsRecursively(source, options) {
  let output = source;
  for (let iteration = 0; iteration < indentBinaryOpsFixture.__generated.recursive; iteration++) {
    const reports = runIndentBinaryOps(output, options);
    if (reports.length === 0) {
      return output;
    }
    output = applySuggestions(output, reports);
  }
  return output;
}

function runFunctionCallArgumentNewline(sourceText, options) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['function-call-argument-newline'].createOnce({
    options: options ?? [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

describe('type-annotation-spacing upstream v5.10.0 parity', () => {
  it('keeps the stable upstream case inventory complete', () => {
    expect(fixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
    });
    expect(fixture.valid).toHaveLength(255);
    expect(fixture.invalid).toHaveLength(223);
    expect(fixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(408);
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'matches upstream invalid case %i',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(reports.map((report) => report.messageId)).toEqual(
        testCase.errors.map((error) => error.messageId),
      );
      expect(reports.map((report) => locationAt(testCase.code, report.node.range[0]))).toEqual(
        testCase.errors.map(({ line, column }) => ({ line, column })),
      );
      expect(applySuggestions(testCase.code, reports)).toBe(testCase.output);
    },
  );
});

describe('function-call-argument-newline upstream v5.10.0 parity', () => {
  it('keeps the pinned stable upstream inventory complete', () => {
    expect(functionCallArgumentNewlineFixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/function-call-argument-newline/function-call-argument-newline.test.ts',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-function-call-argument-newline-tests.ts',
    });
    expect(functionCallArgumentNewlineFixture.valid).toHaveLength(32);
    expect(functionCallArgumentNewlineFixture.invalid).toHaveLength(32);
    expect(
      functionCallArgumentNewlineFixture.invalid.flatMap((testCase) => testCase.errors),
    ).toHaveLength(42);
    expect(
      functionCallArgumentNewlineFixture.invalid.filter(
        (testCase) => typeof testCase.output === 'string',
      ),
    ).toHaveLength(29);
    expect(
      functionCallArgumentNewlineFixture.invalid.filter((testCase) => testCase.output === null),
    ).toHaveLength(3);
  });

  it.each(functionCallArgumentNewlineFixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runFunctionCallArgumentNewline(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(functionCallArgumentNewlineFixture.invalid.map((testCase, index) => [index, testCase]))(
    'matches upstream invalid case %i',
    (_index, testCase) => {
      const reports = runFunctionCallArgumentNewline(testCase.code, testCase.options);
      expect(reports.map((report) => report.messageId)).toEqual(
        testCase.errors.map((error) => error.messageId),
      );
      expect(
        reports.map((report) => ({
          ...locationAt(testCase.code, report.node.range[0]),
          endLine: locationAt(testCase.code, report.node.range[1]).line,
          endColumn: locationAt(testCase.code, report.node.range[1]).column,
        })),
      ).toEqual(
        testCase.errors.map(({ line, column, endLine, endColumn }) => ({
          line,
          column,
          endLine,
          endColumn,
        })),
      );

      if (testCase.output === null) {
        expect(reports.every((report) => report.suggest === undefined)).toBe(true);
      } else {
        expect(applySuggestions(testCase.code, reports)).toBe(testCase.output);
      }
    },
  );
});

describe('indent-binary-ops upstream v5.10.0 parity', () => {
  it('keeps the pinned stable upstream inventory complete', () => {
    expect(indentBinaryOpsFixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/indent-binary-ops/indent-binary-ops.test.ts',
      license: 'MIT',
      recursive: 10,
      tool: 'tools/tasks/sync-stylistic-indent-binary-ops-tests.ts',
    });
    expect(indentBinaryOpsFixture.valid).toHaveLength(19);
    expect(indentBinaryOpsFixture.invalid).toHaveLength(29);
    expect(
      indentBinaryOpsFixture.invalid.every((testCase) => typeof testCase.output === 'string'),
    ).toBe(true);
  });

  it.each(indentBinaryOpsFixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i',
    (_index, testCase) => {
      expect(runIndentBinaryOps(testCase.code, testCase.options)).toEqual([]);
    },
  );

  it.each(indentBinaryOpsFixture.invalid.map((testCase, index) => [index, testCase]))(
    'matches upstream invalid case %i including recursive output',
    (_index, testCase) => {
      const reports = runIndentBinaryOps(testCase.code, testCase.options);
      expect(reports.length).toBeGreaterThan(0);
      expect(reports.every((report) => report.messageId === 'wrongIndentation')).toBe(true);
      expect(
        reports.every(
          (report) =>
            typeof report.data?.expected === 'string' &&
            /^\d+ (?:spaces?|tabs?)$/.test(report.data.expected),
        ),
      ).toBe(true);
      expect(
        reports.every((report) => {
          const [start, end] = report.node.range;
          const lineStart = testCase.code.lastIndexOf('\n', start - 1) + 1;
          return start === lineStart && /^\s*$/.test(testCase.code.slice(start, end));
        }),
      ).toBe(true);
      expect(applyIndentBinaryOpsRecursively(testCase.code, testCase.options)).toBe(
        testCase.output,
      );
    },
  );
});

function runBraceStyle(testCase) {
  const reports = [];
  const sourceCode = {
    text: testCase.code,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules['brace-style'].createOnce({
    filename: testCase.language === 'js' ? 'fixture.js' : 'fixture.ts',
    options: testCase.options ?? [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, testCase.code.length] });
  return reports;
}

function applyBraceStyleSuggestions(source, reports) {
  const edits = [];
  for (const report of reports) {
    if (!report.suggest?.[0]) {
      return null;
    }
    edits.push(
      ...report.suggest[0].fix({
        replaceTextRange(range, replacementText) {
          return { range, replacementText };
        },
      }),
    );
  }
  edits.sort((left, right) => right.range[0] - left.range[0] || right.range[1] - left.range[1]);

  let output = source;
  for (const edit of edits) {
    output = output.slice(0, edit.range[0]) + edit.replacementText + output.slice(edit.range[1]);
  }
  return output;
}

describe('brace-style upstream v5.10.0 parity', () => {
  it('keeps both JavaScript and TypeScript suite inventories complete', () => {
    expect(braceStyleFixture.__generated).toMatchObject({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFiles: [
        'packages/eslint-plugin/rules/brace-style/brace-style._js_.test.ts',
        'packages/eslint-plugin/rules/brace-style/brace-style._ts_.test.ts',
      ],
    });
    expect(braceStyleFixture.valid).toHaveLength(89);
    expect(braceStyleFixture.invalid).toHaveLength(91);
    expect(braceStyleFixture.invalid.flatMap((testCase) => testCase.errors)).toHaveLength(130);
    expect(braceStyleFixture.valid.filter((testCase) => testCase.language === 'js')).toHaveLength(
      81,
    );
    expect(braceStyleFixture.invalid.filter((testCase) => testCase.language === 'ts')).toHaveLength(
      8,
    );
  });

  it.each(braceStyleFixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream brace-style valid case %i',
    (_index, testCase) => {
      expect(runBraceStyle(testCase)).toEqual([]);
    },
  );

  it.each(braceStyleFixture.invalid.map((testCase, index) => [index, testCase]))(
    'matches upstream brace-style invalid case %i',
    (_index, testCase) => {
      const reports = runBraceStyle(testCase);
      expect(reports.map((report) => report.messageId)).toEqual(
        testCase.errors.map((error) => error.messageId),
      );
      for (const [reportIndex, expected] of testCase.errors.entries()) {
        if (expected.line !== undefined) {
          expect(locationAt(testCase.code, reports[reportIndex].node.range[0]).line).toBe(
            expected.line,
          );
        }
      }
      expect(reports.map((report) => testCase.code.slice(...report.node.range))).toEqual(
        testCase.errors.map((error) =>
          error.messageId.endsWith('Open') || error.messageId === 'blockSameLine' ? '{' : '}',
        ),
      );
      expect(applyBraceStyleSuggestions(testCase.code, reports)).toBe(testCase.output);
    },
  );
});
