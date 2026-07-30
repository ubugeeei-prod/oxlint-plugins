import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const fixture = JSON.parse(
  readFileSync(new URL('./fixtures/jsx-newline-v5.10.0.json', import.meta.url), 'utf8'),
);
const rule = plugin.rules['jsx-newline'];

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
    const next = fixedOutput(output, runRule(output, options, filename));
    if (next === null || next === output) {
      return changed ? output : next;
    }
    output = next;
    changed = true;
  }
  throw new Error(`jsx-newline fixes did not converge:\n${output}`);
}

describe('@stylistic/jsx-newline v5.10.0 exhaustive upstream replay', () => {
  it('keeps the complete pinned stable parser-expanded inventory', () => {
    expect(fixture.__generated).toEqual({
      source: '@stylistic/eslint-plugin',
      version: '5.10.0',
      sourceCommit: 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712',
      sourceFile: 'packages/eslint-plugin/rules/jsx-newline/jsx-newline.test.ts',
      license: 'MIT',
      eslintVersion: '10.0.0',
      parserVersions: {
        espree: '10.4.0',
        typescriptEslint: '8.56.0',
        typescript: '5.9.3',
      },
      parserMatrix: 'ESLint 10 parsers-jsx expansion; Babel disabled by the stable runner',
      tool: 'tools/tasks/sync-stylistic-jsx-newline-tests.ts',
      inventory: {
        logicalValid: 12,
        logicalInvalid: 19,
        valid: 20,
        invalid: 34,
        diagnostics: 48,
        unfixableInvalid: 0,
        total: 54,
        fixableInvalid: 34,
      },
    });

    const parserCounts = {};
    for (const testCase of [...fixture.valid, ...fixture.invalid]) {
      parserCounts[testCase.parser] = (parserCounts[testCase.parser] ?? 0) + 1;
    }
    expect(parserCounts).toEqual({ espree: 23, typescript: 31 });

    const messageCounts = {};
    for (const diagnostic of fixture.invalid.flatMap((testCase) => testCase.expectedDiagnostics)) {
      messageCounts[diagnostic.messageId] = (messageCounts[diagnostic.messageId] ?? 0) + 1;
    }
    expect(messageCounts).toEqual({
      require: 17,
      allowMultilines: 11,
      prevent: 20,
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
    'replays every upstream invalid diagnostic, range, and fix %# exactly',
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

  it('maps Unicode prefixes to UTF-16 report and fix ranges', () => {
    const source = 'const emoji = "😀";\nconst view: JSX.Element = <外><内 />\n<次 /></外>;';
    const reports = runRule(source, [], 'fixture.tsx');
    expect(reports).toHaveLength(1);
    expect(source.slice(...reports[0].node.range)).toBe('<次 />');
    expect(reports[0].node.range).toEqual([
      source.indexOf('<次 />'),
      source.indexOf('<次 />') + '<次 />'.length,
    ]);
    expect(fixesFor(reports[0])).toEqual([
      {
        range: [source.indexOf('\n<次'), source.indexOf('<次')],
        replacementText: '\n\n',
      },
    ]);
  });

  it('preserves upstream mixed CRLF, CR, LS, and PS semantics', () => {
    const crlf = '<A><B />\r\n<C /></A>';
    expect(fixedOutput(crlf, runRule(crlf))).toBe('<A><B />\r\n\n<C /></A>');

    for (const separator of ['\r', '\u2028', '\u2029']) {
      const source = `<A><B />${separator}<C /></A>`;
      const reports = runRule(source);
      expect(reports.map((report) => report.messageId)).toEqual(['require']);
      expect(fixedOutput(source, reports)).toBe(source);
    }

    const lfBoundaries = '<A><B />\n\u2028\n<C /></A>';
    expect(runRule(lfBoundaries)).toEqual([]);
  });

  it('handles fragments, expressions, block comments, and multiline lookahead', () => {
    const source = [
      '<>',
      '  {/* ignored as a current child */}',
      '  <One />',
      '  {/* reported as the following child */}',
      '  {condition && (',
      '    <ManyLines />',
      '  )}',
      '</>',
    ].join('\n');
    const reports = runRule(source, [{ prevent: true, allowMultilines: true }]);
    expect(reports.map((report) => report.messageId)).toEqual(['allowMultilines']);
    expect(source.slice(...reports[0].node.range)).toBe('{/* reported as the following child */}');
    expect(fixedOutput(source, reports)).toContain(
      '<One />\n\n  {/* reported as the following child */}',
    );
  });

  it('supports TSX types and safely defaults malformed option fields', () => {
    const source = 'const view: JSX.Element = <List<Item>><Row<Item> />\n<Row<Item> /></List>;';
    expect(runRule(source, [], 'fixture.tsx')).toHaveLength(1);
    expect(runRule(source, [42], 'fixture.tsx')).toHaveLength(1);
    expect(
      runRule(source, [{ prevent: 'yes', allowMultilines: 'yes' }], 'fixture.tsx'),
    ).toHaveLength(1);
    expect(runRule(source, [{ prevent: true }], 'fixture.tsx')).toEqual([]);
  });

  it('ignores non-JSX and invalid JSX/TSX syntax', () => {
    for (const [source, filename] of [
      ['const value = { first: 1, second: 2 };', 'fixture.js'],
      ['type Props = { first: string };', 'fixture.ts'],
      ['const view = <A><B /></C>;', 'fixture.tsx'],
      ['const view = <A>{broken</A>;', 'fixture.jsx'],
    ]) {
      expect(runRule(source, [], filename), source).toEqual([]);
    }
  });
});
