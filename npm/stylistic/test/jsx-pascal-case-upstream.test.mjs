import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-pascal-case-v5.10.0.json', import.meta.url), 'utf8'),
);
const rule = plugin.rules['jsx-pascal-case'];

function runRule(sourceText, options = [], filename = 'fixture.tsx') {
  const reports = [];
  const visitor = rule.createOnce({
    options,
    filename,
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

function renderedMessage(report) {
  return rule.meta.messages[report.messageId].replace('{{name}}', report.data.name);
}

function actualDiagnostic(sourceText, report) {
  const [start, end] = report.node.range;
  const startPosition = positionAt(sourceText, start);
  const endPosition = positionAt(sourceText, end);
  return {
    messageId: report.messageId,
    message: renderedMessage(report),
    data: report.data ?? {},
    range: [start, end],
    loc: {
      line: startPosition.line,
      column: startPosition.column,
      endLine: endPosition.line,
      endColumn: endPosition.column,
    },
    fix: null,
  };
}

describe('@stylistic/jsx-pascal-case v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned stable parser-expanded inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-pascal-case/jsx-pascal-case.test.ts',
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
        typescript: '5.9.3',
      },
      parserMatrix: 'ESLint 10 parsers-jsx expansion; Babel disabled by the stable runner',
      tool: 'tools/tasks/sync-stylistic-jsx-pascal-case-tests.ts',
      inventory: {
        logicalValid: 29,
        logicalInvalid: 14,
        valid: 57,
        invalid: 28,
        diagnostics: 28,
        fixableInvalid: 0,
        unfixableInvalid: 28,
        total: 85,
      },
    });

    const parserCounts = {};
    for (const testCase of [...fixture.valid, ...fixture.invalid]) {
      parserCounts[testCase.parser] = (parserCounts[testCase.parser] ?? 0) + 1;
    }
    expect(parserCounts).toEqual({ espree: 43, typescript: 42 });

    const messageCounts = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messageCounts[diagnostic.messageId] = (messageCounts[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messageCounts).toEqual({
      usePascalCase: 18,
      usePascalOrSnakeCase: 10,
    });
    expect(
      new Set(
        [...fixture.valid, ...fixture.invalid].map((testCase) => JSON.stringify(testCase.options)),
      ).size,
    ).toBe(9);
  });

  it.each(fixture.valid)('accepts every upstream valid case %#', (testCase) => {
    const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
    expect(runRule(testCase.code, testCase.options, filename), testCase.code).toEqual([]);
  });

  it.each(fixture.invalid)(
    'replays every upstream invalid message, data, range, and non-fixable output %# exactly',
    (testCase) => {
      const filename = testCase.parser === 'typescript' ? 'fixture.tsx' : 'fixture.jsx';
      const reports = runRule(testCase.code, testCase.options, filename);
      expect(
        reports.map((report) => actualDiagnostic(testCase.code, report)),
        testCase.code,
      ).toEqual(testCase.expectedDiagnostics);
      expect(reports.every((report) => report.suggest === undefined)).toBe(true);
      expect(testCase.output).toBeNull();
      expect(testCase.recursiveOutput).toBeNull();
    },
  );

  it('maps Unicode prefixes and component names to exact UTF-16 ranges', () => {
    const source = 'const emoji = "😀"; const view = <É_bad title="日本語" />;';
    const reports = runRule(source);
    expect(reports).toHaveLength(1);
    expect(reports[0].data).toEqual({ name: 'É_bad' });
    expect(source.slice(...reports[0].node.range)).toBe('<É_bad title="日本語" />');
    expect(reports[0].node.range).toEqual([source.indexOf('<É_bad'), source.indexOf('/>') + 2]);
    expect(renderedMessage(reports[0])).toBe('Imported JSX component É_bad must be in PascalCase');
  });

  it('preserves CRLF locations, nested source order, fragments, and TSX generics', () => {
    const source = [
      'type Item = { id: string };\r\n',
      'const view = <>\r\n',
      '  <Outer_bad>\r\n',
      '    <Inner_bad<Item> />\r\n',
      '  </Outer_bad>\r\n',
      '</>;\r\n',
    ].join('');
    const reports = runRule(source, [], 'fixture.tsx');
    expect(reports.map((report) => report.data.name)).toEqual(['Outer_bad', 'Inner_bad']);
    expect(reports.map((report) => positionAt(source, report.node.range[0]))).toEqual([
      { line: 3, column: 3 },
      { line: 4, column: 5 },
    ]);
    expect(source.slice(...reports[1].node.range)).toBe('<Inner_bad<Item> />');
  });

  it('covers namespace/member short-circuiting and all option switches', () => {
    expect(runRule('<Styled.h1 />').map((report) => report.data.name)).toEqual(['h1']);
    expect(runRule('<Styled.h1 />', [{ allowNamespace: true }])).toEqual([]);
    expect(runRule('<STYLED.h1 />', [{ allowNamespace: true }])[0].data.name).toBe('STYLED');
    expect(runRule('<T.bad />')).toEqual([]);
    expect(runRule('<qualification.bad />')).toEqual([]);
    expect(runRule('<_TestComponent />', [{ allowLeadingUnderscore: true }])).toEqual([]);
    expect(runRule('<TEST_COMPONENT />', [{ allowAllCaps: true }])).toEqual([]);
    expect(runRule('<_TEST_COMPONENT />', [{ allowAllCaps: true }])[0].messageId).toBe(
      'usePascalOrSnakeCase',
    );
    expect(runRule('<Modal:Header />', [], 'fixture.jsx')).toEqual([]);
  });

  it('matches exact, wildcard, extglob, brace, and character-class ignore patterns', () => {
    for (const pattern of [
      'Foo_DEPRECATED',
      '*_D*D',
      '*_+(DEPRECATED|IGNORED)',
      'Foo_{DEPRECATED,IGNORED}',
      'Foo_[A-Z]*',
    ]) {
      expect(runRule('<Foo_DEPRECATED />', [{ ignore: [pattern] }]), pattern).toEqual([]);
    }
    for (const pattern of ['*_+(DEPRECATED|IGNORED)', 'Foo_{DEPRECATED,IGNORED}']) {
      expect(runRule('<Foo_IGNORED />', [{ ignore: [pattern] }]), pattern).toEqual([]);
    }
    expect(runRule('<Foo_DEPRECATED />', [{ ignore: ['*_FOO'] }])).toHaveLength(1);
  });

  it('safely defaults malformed options and ignores invalid/non-JSX sources', () => {
    for (const options of [null, [42], [{ allowAllCaps: 'yes' }], [{ ignore: [42, null] }]]) {
      expect(runRule('<TEST_COMPONENT />', options), JSON.stringify(options)).toHaveLength(1);
    }
    for (const [source, filename] of [
      ['const value = { Bad_name: 1 };', 'fixture.js'],
      ['type Bad_name = string;', 'fixture.ts'],
      ['const view = <Bad_name>;', 'fixture.tsx'],
      ['const view = <Bad_name></Other>;', 'fixture.jsx'],
    ]) {
      expect(runRule(source, [], filename), source).toEqual([]);
    }
  });
});
