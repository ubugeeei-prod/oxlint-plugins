import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'jsx-first-prop-new-line-v5.10.0.json'), 'utf8'),
);
const rule = plugin.rules['jsx-first-prop-new-line'];

function runRule(sourceText, options = [], filename = 'fixture.tsx') {
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
  throw new Error(`jsx-first-prop-new-line fixes did not converge:\n${output}`);
}

describe('@stylistic/jsx-first-prop-new-line v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned stable parser-expanded inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile:
        'packages/eslint-plugin/rules/jsx-first-prop-new-line/jsx-first-prop-new-line.test.ts',
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
      },
      tool: 'tools/tasks/sync-stylistic-jsx-first-prop-new-line-tests.ts',
      inventory: {
        valid: 42,
        invalid: 17,
        diagnostics: 17,
        unfixableInvalid: 0,
        total: 59,
        fixableInvalid: 17,
      },
    });

    const parserCounts = {};
    for (const testCase of [...fixture.valid, ...fixture.invalid]) {
      parserCounts[testCase.parser] = (parserCounts[testCase.parser] ?? 0) + 1;
    }
    expect(parserCounts).toEqual({ espree: 29, typescript: 30 });

    const messageCounts = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messageCounts[diagnostic.messageId] = (messageCounts[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messageCounts).toEqual({
      propOnNewLine: 11,
      propOnSameLine: 6,
    });
    expect(
      new Set(
        [...fixture.valid, ...fixture.invalid].map((testCase) => JSON.stringify(testCase.options)),
      ).size,
    ).toBe(5);
  });

  it.each(fixture.valid)('accepts every upstream valid case %#', (testCase) => {
    const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
    expect(runRule(testCase.code, testCase.options, filename), testCase.code).toEqual([]);
  });

  it.each(fixture.invalid)(
    'replays every upstream invalid diagnostic and fix %# exactly',
    (testCase) => {
      const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
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

  it('maps Unicode, CRLF, and every ECMAScript line separator to UTF-16 ranges', () => {
    const source = [
      'const 日本語 = <部品\r\n値="😀" />;\r\n',
      'const solo = <Solo\rprop />;\r',
      'const café = <Élément\u2028nom="été" />;\u2028',
      'const τέλος = <Στοιχείο\u2029τιμή="κόσμος" />;\u2029',
      'const nested = <Outer><内側\n属性 /></Outer>;\n',
    ].join('');
    const reports = runRule(source, ['never'], 'fixture.tsx');

    expect(reports.map((report) => report.messageId)).toEqual([
      'propOnSameLine',
      'propOnSameLine',
      'propOnSameLine',
      'propOnSameLine',
      'propOnSameLine',
    ]);
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual([
      '値="😀"',
      'prop',
      'nom="été"',
      'τιμή="κόσμος"',
      '属性',
    ]);
    expect(fixedOutput(source, reports)).toBe(
      [
        'const 日本語 = <部品 値="😀" />;\r\n',
        'const solo = <Solo prop />;\r',
        'const café = <Élément nom="été" />;\u2028',
        'const τέλος = <Στοιχείο τιμή="κόσμος" />;\u2029',
        'const nested = <Outer><内側 属性 /></Outer>;\n',
      ].join(''),
    );
  });

  it('uses the type-argument boundary for TSX generic component fixes', () => {
    const source = [
      'type Items = { id: string };',
      '<DataTable<Items> fullscreen items={{',
      '  value: 1',
      '}} />;',
    ].join('\n');
    const reports = runRule(source, ['multiline'], 'fixture.tsx');
    expect(reports).toHaveLength(1);
    expect(source.slice(...reports[0].node.range)).toBe('fullscreen');
    expect(fixesFor(reports[0])).toEqual([
      {
        range: [source.indexOf('> fullscreen') + 1, source.indexOf('fullscreen')],
        replacementText: '\n',
      },
    ]);
    expect(fixedOutput(source, reports)).toContain('<DataTable<Items>\nfullscreen');
  });

  it('covers namespaced and member names, spreads, booleans, and nested JSX', () => {
    const source = [
      '<UI.Root first="one" second>',
      '  <svg:path xml:lang="en" {...props} />',
      '  <Leaf flag other />',
      '</UI.Root>',
    ].join('\n');
    const reports = runRule(source, ['always']);
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual([
      'first="one"',
      'xml:lang="en"',
      'flag',
    ]);
    expect(fixedOutput(source, reports)).toBe(
      [
        '<UI.Root\nfirst="one" second>',
        '  <svg:path\nxml:lang="en" {...props} />',
        '  <Leaf\nflag other />',
        '</UI.Root>',
      ].join('\n'),
    );
  });

  it('matches upstream raw comment-deleting replacement semantics', () => {
    const newLine = '<Foo /* displaced */ first second />';
    expect(fixedOutput(newLine, runRule(newLine, ['always']))).toBe('<Foo\nfirst second />');

    const sameLine = '<Foo\n/* displaced */\nfirst />';
    expect(fixedOutput(sameLine, runRule(sameLine, ['never']))).toBe('<Foo first />');
  });

  it('ignores invalid syntax and non-JSX sources', () => {
    for (const [source, filename] of [
      ['const object = { first: 1, second: 2 };', 'fixture.js'],
      ['type Props = { first: string };', 'fixture.ts'],
      ['const view = <Foo first={value />;', 'fixture.tsx'],
      ['const view = <Foo first></Bar>;', 'fixture.jsx'],
    ]) {
      expect(runRule(source, ['always'], filename), source).toEqual([]);
    }
  });

  it('falls back safely to multiline-multiprop for malformed option payloads', () => {
    const source = '<Foo first={{\nvalue: 1\n}} second />';
    for (const options of [['sideways'], [42], [{ mode: 'always' }], null]) {
      expect(runRule(source, options), JSON.stringify(options)).toHaveLength(1);
      expect(runRule(source, options)[0].messageId).toBe('propOnNewLine');
    }
  });
});
