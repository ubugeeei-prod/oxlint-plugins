import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const RULE = 'jsx-props-no-multi-spaces';
const fixture = JSON.parse(
  readFileSync(
    new URL('./fixtures/jsx-props-no-multi-spaces-v5.10.0.json', import.meta.url),
    'utf8',
  ),
);

function runRule(sourceText, options, overrides = {}) {
  const reports = [];
  const sourceCode = overrides.sourceCode ?? {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[RULE].createOnce({
    filename: overrides.filename ?? 'fixture.tsx',
    options: options ?? [],
    settings: overrides.settings,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });
  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderedMessage(report) {
  let message = plugin.rules[RULE].meta.messages[report.messageId];
  for (const [key, value] of Object.entries(report.data ?? {})) {
    message = message.replaceAll(`{{${key}}}`, value);
  }
  return message;
}

function locationAt(source, offset) {
  const prefix = source.slice(0, offset);
  const terminators = [...prefix.matchAll(/\r\n|[\n\r\u2028\u2029]/gu)];
  const lastTerminator = terminators.at(-1);
  const lineStart = lastTerminator ? lastTerminator.index + lastTerminator[0].length : 0;
  return {
    line: terminators.length + 1,
    column: source.slice(lineStart, offset).length + 1,
  };
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

function normalizedReport(source, report) {
  const start = locationAt(source, report.node.range[0]);
  const end = locationAt(source, report.node.range[1]);
  return {
    messageId: report.messageId,
    message: renderedMessage(report),
    data: report.data ?? {},
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
    range: report.node.range,
    fix: reportFix(report),
  };
}

function fixedOutput(sourceText, reports) {
  const edits = reports
    .map((report, index) => ({ index, fix: reportFix(report) }))
    .filter(({ fix }) => fix !== null)
    .sort(
      (left, right) =>
        left.fix.range[0] - right.fix.range[0] ||
        left.fix.range[1] - right.fix.range[1] ||
        left.index - right.index,
    );
  if (edits.length === 0) {
    return null;
  }

  const accepted = [];
  let lastEnd = -1;
  for (const { fix } of edits) {
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

function recursiveOutput(sourceText, options) {
  let output = sourceText;
  let changed = false;
  for (let pass = 0; pass < 10; pass += 1) {
    const next = fixedOutput(output, runRule(output, options));
    if (next === null) {
      return changed ? output : null;
    }
    expect(next, `fix pass ${pass + 1} must progress`).not.toBe(output);
    output = next;
    changed = true;
  }
  throw new Error(`${RULE} fixes did not converge`);
}

describe('@stylistic/jsx-props-no-multi-spaces v5.10.0 upstream parity', () => {
  it('keeps the exact pinned authored inventory complete and reproducible', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: 'v5.10.0',
      commit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-props-no-multi-spaces/jsx-props-no-multi-spaces.test.ts',
      ruleFile:
        'packages/eslint-plugin/rules/jsx-props-no-multi-spaces/jsx-props-no-multi-spaces.ts',
      parserMatrixFile: 'shared/test-utils/parsers-jsx.ts',
      sourceSha256: '2df423ddb26f7dd88c9aa39dfe9e040e4c64d6bf53ba2e3eec8a93d1088ef0f9',
      ruleSourceSha256: 'bd0b69b6183f1278825a6089f5e1445dec7a2ee538284983ff1178c8388b07d4',
      parserMatrixSourceSha256: '64dd12d67eac1eadf8a5a93de02bbb76c1d764c0ec7ebbdaae0c45389b52435c',
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-jsx-props-no-multi-spaces-tests.ts',
      capturePolicy:
        'Each authored semantic case is captured once; exact replay uses @typescript-eslint/parser in TSX mode.',
      exactReplay: {
        eslint: '10.4.1',
        typescriptEslintParser: '8.60.0',
      },
      inventory: {
        valid: 16,
        invalid: 12,
        diagnostics: 17,
        fixableInvalid: 7,
        unfixableInvalid: 5,
        total: 28,
      },
    });
  });

  it.each(fixture.valid.map((testCase, index) => [index, testCase]))(
    'accepts authored valid case %i',
    (_index, testCase) => {
      expect(runRule(testCase.code, testCase.options), testCase.code).toEqual([]);
    },
  );

  it.each(fixture.invalid.map((testCase, index) => [index, testCase]))(
    'replays invalid case %i with exact messages, data, ranges, fixes, and output',
    (_index, testCase) => {
      const reports = runRule(testCase.code, testCase.options);
      expect(
        reports.map((report) => normalizedReport(testCase.code, report)),
        testCase.code,
      ).toEqual(testCase.diagnostics);
      expect(fixedOutput(testCase.code, reports), testCase.code).toBe(testCase.output);
      expect(recursiveOutput(testCase.code, testCase.options), testCase.code).toBe(testCase.output);
    },
  );

  it('maps Unicode and all line terminators to exact UTF-16 report and fix ranges', () => {
    const inline = 'const marker = "😀"; const view = <App<T>  foo="日本語"   {...props} />;';
    const inlineReports = runRule(inline);
    expect(inlineReports.map((report) => inline.slice(...report.node.range))).toEqual([
      'foo="日本語"',
      '{...props}',
    ]);
    expect(inlineReports.map(reportFix)).toEqual([
      {
        range: [inline.indexOf('>  foo') + 1, inline.indexOf('foo=')],
        replacementText: ' ',
      },
      {
        range: [
          inline.indexOf('foo="日本語"') + 'foo="日本語"'.length,
          inline.indexOf('{...props}'),
        ],
        replacementText: ' ',
      },
    ]);

    for (const separator of ['\n', '\r\n', '\r', '\u2028', '\u2029']) {
      const source = `<App foo${separator}${separator}bar />`;
      const reports = runRule(source);
      expect(
        reports.map((report) => report.messageId),
        JSON.stringify(separator),
      ).toEqual(['noLineGap']);
      expect(reportFix(reports[0])).toBeNull();
    }
  });

  it('batches shared settings without leaking another native rule report', () => {
    const source = `const view = <App  title='value' />;`;
    const sourceCode = {
      text: source,
      getText() {
        return this.text;
      },
    };
    const settings = {
      corsaStylistic: {
        rules: {
          [RULE]: [],
          'jsx-quotes': ['prefer-double'],
        },
      },
    };
    const reports = runRule(source, [], { sourceCode, settings });
    expect(reports.map((report) => report.messageId)).toEqual(['onlyOneSpace']);
    expect(reports.map(renderedMessage)).toEqual([
      'Expected only one space between “App” and “title”',
    ]);
  });

  it('keeps fragments inert, handles namespaced and spread props, and ignores option payloads', () => {
    const source = '<><svg:path  xml:lang="en"   {...props.value} /></>';
    const reports = runRule(source, [{ unsupported: true }], { filename: 'fixture.jsx' });
    expect(reports.map((report) => report.data)).toEqual([
      { prop1: 'svg:path', prop2: 'xml:lang' },
      { prop1: 'xml:lang', prop2: 'props.value' },
    ]);
    expect(recursiveOutput(source, [{ unsupported: true }])).toBe(
      '<><svg:path xml:lang="en" {...props.value} /></>',
    );
    expect(runRule('<></>')).toEqual([]);
  });
});
