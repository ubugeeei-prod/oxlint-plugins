import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-curly-spacing-v5.10.0.json', import.meta.url), 'utf8'),
);

function runRule(sourceText, options = [], filename = 'fixture.tsx') {
  const reports = [];
  const visitor = plugin.rules['jsx-curly-spacing'].createOnce({
    filename,
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
  let message = plugin.rules['jsx-curly-spacing'].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data ?? {})) {
    message = message.replaceAll(`{{${key}}}`, value);
  }
  return message;
}

function reportFix(report) {
  return report.suggest?.[0]?.fix({
    replaceTextRange(range, replacementText) {
      return { range, replacementText };
    },
  })?.[0];
}

function fixedPass(sourceText, reports) {
  const fixes = reports
    .map(reportFix)
    .filter(Boolean)
    .sort((left, right) => left.range[0] - right.range[0] || left.range[1] - right.range[1]);
  const accepted = [];
  let lastEnd = -1;
  for (const fix of fixes) {
    if (lastEnd >= fix.range[0]) {
      continue;
    }
    accepted.push(fix);
    lastEnd = fix.range[1];
  }
  let output = sourceText;
  for (const fix of accepted.reverse()) {
    output = output.slice(0, fix.range[0]) + fix.replacementText + output.slice(fix.range[1]);
  }
  return output;
}

function recursivelyFixed(sourceText, options, filename = 'fixture.tsx') {
  let output = sourceText;
  for (let iteration = 0; iteration < 10; iteration++) {
    const reports = runRule(output, options, filename);
    if (reports.length === 0) {
      return output;
    }
    const next = fixedPass(output, reports);
    expect(next, `fix pass ${iteration} must progress`).not.toBe(output);
    output = next;
  }
  return output;
}

describe('@stylistic/jsx-curly-spacing v5.10.0 exhaustive upstream parity', () => {
  it('pins the exact source, parser semantics, and full authored inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-curly-spacing/jsx-curly-spacing.test.ts',
      ruleFile: 'packages/eslint-plugin/rules/jsx-curly-spacing/jsx-curly-spacing.ts',
      license: 'MIT',
      parserMatrix: ['default', '@babel/eslint-parser', '@typescript-eslint/parser'],
      parserExpansion: 'authored semantic cases replayed with Oxc JSX/TSX',
      tool: 'tools/tasks/sync-stylistic-jsx-curly-spacing-tests.ts',
      inventory: {
        valid: 142,
        invalid: 154,
        diagnostics: 320,
        fixableInvalid: 154,
        unfixableInvalid: 0,
        total: 296,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts upstream valid case %i in JSX and TSX',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options, 'fixture.jsx'), testCase.code).toEqual([]);
      expect(runRule(testCase.code, testCase.options, 'fixture.tsx'), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays invalid case %i with exact order, IDs, messages, data, braces, and convergence',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => report.messageId),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.messageId));
      expect(
        reports.map((report) => report.data),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.data));
      expect(reports.map(renderedMessage), testCase.code).toEqual(
        testCase.errors.map((error) => error.message),
      );
      expect(
        reports.map((report) => testCase.code.slice(...report.node.range)),
        testCase.code,
      ).toEqual(testCase.errors.map((error) => error.data.token));
      expect(
        reports.every((report) => report.suggest?.length === 1),
        testCase.code,
      ).toBe(true);
      expect(recursivelyFixed(testCase.code, testCase.options), testCase.code).toBe(
        testCase.output,
      );
      expect(runRule(testCase.output, testCase.options), testCase.output).toEqual([]);
    },
  );

  it('keeps UTF-16 report and fix ranges exact after astral and non-ASCII prefixes', () => {
    const source = 'const marker = "😀日本語"; const view = <App attr={value}>{child}</App>;';
    const reports = runRule(source, [
      { attributes: { when: 'always' }, children: { when: 'always' } },
    ]);
    const ranges = [...source.matchAll(/[{}]/gu)].map((match) => [
      match.index,
      match.index + match[0].length,
    ]);

    expect(reports.map((report) => report.node.range)).toEqual(ranges);
    expect(reports.map((report) => report.data.token)).toEqual(['{', '}', '{', '}']);
    expect(
      recursivelyFixed(source, [{ attributes: { when: 'always' }, children: { when: 'always' } }]),
    ).toBe('const marker = "😀日本語"; const view = <App attr={ value }>{ child }</App>;');
  });

  it('preserves comments and matches JavaScript CRLF, CR, LF, LS, and PS trimming', () => {
    for (const newline of ['\r\n', '\r', '\n', '\u2028', '\u2029']) {
      const source = `<App attr={${newline}/* lead */${newline}value${newline}/* trail */${newline}} />`;
      const options = [{ attributes: { when: 'always', allowMultiline: false } }];
      const reports = runRule(source, options);
      expect(
        reports.map((report) => report.messageId),
        JSON.stringify(newline),
      ).toEqual(['noNewlineAfter', 'noNewlineBefore']);
      const output = recursivelyFixed(source, options);
      expect(output).toContain('/* lead */');
      expect(output).toContain('/* trail */');
      expect(runRule(output, options), JSON.stringify(newline)).toEqual([]);
    }
  });

  it('covers object overrides, spread attributes, fragments, disable switches, and invalid syntax', () => {
    const source = '<><App object={{value: 1}} {...props}>{child}{{nested: 1}}</App></>';
    const options = [
      {
        when: 'never',
        spacing: { objectLiterals: 'always' },
        attributes: true,
        children: true,
      },
    ];
    expect(recursivelyFixed(source, options)).toBe(
      '<><App object={ {value: 1} } {...props}>{child}{ {nested: 1} }</App></>',
    );
    expect(
      runRule('<App attr={ spaced }>{ spaced }</App>', [{ attributes: false, children: false }]),
    ).toEqual([]);
    expect(runRule('<App attr={value>', options)).toEqual([]);
    expect(runRule('const value = {plain: true};', options, 'fixture.ts')).toEqual([]);
  });
});
