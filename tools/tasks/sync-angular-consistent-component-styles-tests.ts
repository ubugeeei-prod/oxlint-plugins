// Snapshot every authored angular-eslint v22.0.0 consistent-component-styles
// RuleTester case. The upstream source uses a TypeScript-only helper, so this
// script supplies the small annotation parser locally and lets Node strip the
// remaining type syntax before evaluating the case module.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SOURCE_COMMIT = '7ee4556badebf8c140ffdefdd0b07b02820d5e96';
const sourcePath = resolve(
  ROOT,
  'upstream/angular-eslint/packages/eslint-plugin/tests/rules/consistent-component-styles/cases.ts',
);
const outputPath = resolve(
  ROOT,
  'npm/angular-eslint/test/fixtures/consistent-component-styles-v22.0.0.json',
);
const helperImport =
  "import { convertAnnotatedSourceToFailureCase } from '@angular-eslint/test-utils';";

type AuthoredError = {
  messageId: string;
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
};

type AuthoredCase = {
  name?: string;
  code: string;
  options?: unknown[];
};

type AuthoredInvalidCase = AuthoredCase & {
  errors: AuthoredError[];
  output: null | string | string[];
};

const actualCommit = execFileSync(
  'git',
  ['-C', resolve(ROOT, 'upstream/angular-eslint'), 'rev-parse', 'HEAD'],
  { encoding: 'utf8' },
).trim();
if (actualCommit !== SOURCE_COMMIT) {
  throw new Error(`Expected angular-eslint ${SOURCE_COMMIT}, received ${actualCommit}.`);
}

const source = readFileSync(sourcePath, 'utf8');
if (!source.includes(helperImport)) {
  throw new Error('The upstream consistent-component-styles case harness changed shape.');
}

const executableSource = source.replace(helperImport, () => annotatedCaseHelper());
const javascript = stripTypeScriptTypes(executableSource, { mode: 'strip' });
const cases = (await import(
  `data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`
)) as {
  valid: Array<AuthoredCase | string>;
  invalid: AuthoredInvalidCase[];
};

const fixture = {
  metadata: {
    package: '@angular-eslint/eslint-plugin',
    version: '22.0.0',
    sourceCommit: SOURCE_COMMIT,
    sourcePath: 'packages/eslint-plugin/tests/rules/consistent-component-styles/cases.ts',
    sourceSha256: createHash('sha256').update(source).digest('hex'),
    capture: 'every authored valid and invalid semantic case exactly once',
    counts: {
      valid: cases.valid.length,
      invalid: cases.invalid.length,
      diagnostics: cases.invalid.reduce((count, testCase) => count + testCase.errors.length, 0),
    },
  },
  valid: cases.valid.map((testCase, index) => normalizeValidCase(testCase, index)),
  invalid: cases.invalid.map((testCase, index) => normalizeInvalidCase(testCase, index)),
};

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', outputPath], { cwd: ROOT, stdio: 'ignore' });
console.log(
  `Captured ${fixture.metadata.counts.valid} valid and ${fixture.metadata.counts.invalid} invalid consistent-component-styles cases.`,
);

function normalizeValidCase(testCase: AuthoredCase | string, index: number) {
  if (typeof testCase === 'string') {
    return { name: `valid-${index + 1}`, code: testCase, options: [] };
  }
  return {
    name: testCase.name ?? `valid-${index + 1}`,
    code: testCase.code,
    options: testCase.options ?? [],
  };
}

function normalizeInvalidCase(testCase: AuthoredInvalidCase, index: number) {
  return {
    name: testCase.name ?? `invalid-${index + 1}`,
    code: testCase.code,
    options: testCase.options ?? [],
    errors: testCase.errors.map((error) => ({
      messageId: error.messageId,
      line: error.line,
      column: error.column,
      endLine: error.endLine,
      endColumn: error.endColumn,
    })),
    output: testCase.output,
  };
}

function annotatedCaseHelper() {
  return String.raw`
function convertAnnotatedSourceToFailureCase(errorOptions) {
  const messages =
    'messageId' in errorOptions
      ? [{ ...errorOptions, char: '~' }]
      : errorOptions.messages;
  let parsedSource = '';
  const errors = messages.map(({ char, data, messageId, suggestions }) => {
    const otherChars = messages
      .map((message) => message.char)
      .filter((candidate) => candidate !== char);
    const parsed = parseInvalidSource(errorOptions.annotatedSource, char, otherChars);
    parsedSource = parsed.source;
    return {
      data,
      messageId,
      line: parsed.start.line + 1,
      column: parsed.start.character + 1,
      endLine: parsed.end.line + 1,
      endColumn: parsed.end.character + 1,
      suggestions,
    };
  });
  return {
    name: errorOptions.description,
    code: parsedSource,
    options: errorOptions.options ?? [],
    errors,
    output: errorOptions.annotatedOutputs
      ? errorOptions.annotatedOutputs.map((output) => parseInvalidSource(output).source)
      : errorOptions.annotatedOutput
        ? parseInvalidSource(errorOptions.annotatedOutput).source
        : null,
  };
}

function parseInvalidSource(source, specialChar = '~', otherChars = []) {
  const ignored = new Set(otherChars);
  let column = 0;
  let line = 0;
  let sourceLine = 0;
  let annotationLine = false;
  let start;
  let end;
  for (const character of source) {
    if (character === '\n') {
      if (annotationLine) annotationLine = false;
      else sourceLine = line;
      column = 0;
      line += 1;
      continue;
    }
    column += 1;
    if (ignored.has(character)) annotationLine = true;
    if (character !== specialChar) continue;
    annotationLine = true;
    start ??= { character: column - 1, line: sourceLine };
    end = { character: column, line: sourceLine };
  }
  const ignoredPattern =
    otherChars.length === 0
      ? null
      : new RegExp(
          '[' + otherChars.map((value) => value.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&')).join('') + ']',
          'g',
        );
  const withoutOtherAnnotations = ignoredPattern ? source.replace(ignoredPattern, ' ') : source;
  return {
    source: withoutOtherAnnotations.replaceAll(specialChar, ''),
    start,
    end,
  };
}
`;
}
