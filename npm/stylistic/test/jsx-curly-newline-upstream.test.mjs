import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(
  readFileSync(join(here, 'fixtures', 'jsx-curly-newline-v5.10.0.json'), 'utf8'),
);
const rule = plugin.rules['jsx-curly-newline'];

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
  throw new Error(`jsx-curly-newline fixes did not converge:\n${output}`);
}

describe('@stylistic/jsx-curly-newline v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned stable parser-expanded inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-curly-newline/jsx-curly-newline.test.ts',
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
      },
      tool: 'tools/tasks/sync-stylistic-jsx-curly-newline-tests.ts',
      inventory: {
        valid: 24,
        invalid: 24,
        diagnostics: 30,
        unfixableInvalid: 4,
        total: 48,
        fixableInvalid: 20,
      },
    });

    const parserCounts = {};
    for (const testCase of [...fixture.valid, ...fixture.invalid]) {
      parserCounts[testCase.parser] = (parserCounts[testCase.parser] ?? 0) + 1;
    }
    expect(parserCounts).toEqual({ espree: 24, typescript: 24 });

    const messageCounts = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messageCounts[diagnostic.messageId] = (messageCounts[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messageCounts).toEqual({
      unexpectedBefore: 16,
      expectedBefore: 4,
      expectedAfter: 4,
      unexpectedAfter: 6,
    });
    expect(
      new Set(
        [...fixture.valid, ...fixture.invalid].map((testCase) => JSON.stringify(testCase.options)),
      ).size,
    ).toBe(3);
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

  it('maps Unicode, CRLF, and Unicode separators to exact UTF-16 ranges and fixes', () => {
    const source = [
      'const 日本語 = <div>{\r\n値\r\n}</div>;\r\n',
      'const café = <Comp value={\u2028élément\u2028} />;\u2028',
      'const τέλος = <>{\u2029κόσμος\u2029}</>;\u2029',
      "const emoji = <span>{\n'😀'\n}</span>;\n",
    ].join('');
    const reports = runRule(source, ['never'], 'fixture.tsx');

    expect(reports.map((report) => report.messageId)).toEqual([
      'unexpectedAfter',
      'unexpectedBefore',
      'unexpectedAfter',
      'unexpectedBefore',
      'unexpectedAfter',
      'unexpectedBefore',
      'unexpectedAfter',
      'unexpectedBefore',
    ]);
    expect(reports.map((report) => source.slice(...report.node.range))).toEqual([
      '{',
      '}',
      '{',
      '}',
      '{',
      '}',
      '{',
      '}',
    ]);
    expect(fixedOutput(source, reports)).toBe(
      [
        'const 日本語 = <div>{値}</div>;\r\n',
        'const café = <Comp value={élément} />;\u2028',
        'const τέλος = <>{κόσμος}</>;\u2029',
        "const emoji = <span>{'😀'}</span>;\n",
      ].join(''),
    );
  });

  it('covers TSX attributes, nested JSX, comments, empty containers, and invalid syntax', () => {
    const tsx = [
      'type Props = { value: string };',
      'const view: JSX.Element = <Outer value={foo &&\nbar}>',
      '{condition ? <Inner data={baz} /> : null}',
      '</Outer>;',
    ].join('\n');
    expect(
      runRule(tsx, [{ singleline: 'forbid', multiline: 'require' }], 'fixture.tsx').map(
        (report) => report.messageId,
      ),
    ).toEqual(['expectedAfter', 'expectedBefore']);

    const commented = '<div>{ /* keep */\nfoo }</div>';
    expect(runRule(commented, ['never'])[0].suggest).toBeUndefined();
    expect(runRule('<div>{/* only */}</div>', ['never'])).toEqual([]);
    expect(runRule('<div>{foo</div>', ['never'], 'fixture.tsx')).toEqual([]);
    expect(runRule('const object = { value: 1 };', ['never'], 'fixture.js')).toEqual([]);
  });

  it('falls back safely for malformed option payloads', () => {
    const source = '<div>{\nfoo\n}</div>';
    for (const options of [['sideways'], [42], [{ singleline: 12, multiline: false }], null]) {
      expect(runRule(source, options), JSON.stringify(options)).toEqual([]);
    }
  });
});
