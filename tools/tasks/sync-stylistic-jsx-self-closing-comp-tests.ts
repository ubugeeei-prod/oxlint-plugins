// Captures every authored @stylistic/jsx-self-closing-comp v5.10.0
// RuleTester case after the pinned JSX parser-matrix expansion, then records
// the exact report range and first-pass fix produced by the stable rule.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type ExpandedCase = {
  code: string;
  output?: string;
  options?: unknown[];
  parser: string;
  errors?: Array<{ messageId: string }>;
};
type CapturedRun = {
  name: string;
  valid: ExpandedCase[];
  invalid: ExpandedCase[];
};

const ROOT = process.cwd();
const RULE = 'jsx-self-closing-comp';
const SUITE_NAME = 'self-closing-comp';
const UPSTREAM_VERSION = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const TYPES_FILE = `packages/eslint-plugin/rules/${RULE}/types.d.ts`;
const PARSER_MATRIX_FILE = 'shared/test-utils/parsers-jsx.ts';
const FIXTURE_FILE = join(
  ROOT,
  'npm',
  'stylistic',
  'test',
  'fixtures',
  `${RULE}-${UPSTREAM_VERSION}.json`,
);
const CAPTURE_KEY = '__stylisticJsxSelfClosingCompCapture__';
const MESSAGE_ID = 'notSelfClosing';
const MESSAGE = 'Empty components are self-closing';
const PARSERS = ['default', '@babel/eslint-parser', '@typescript-eslint/parser'] as const;

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
  );
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM_DIR, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== UPSTREAM_COMMIT) {
  throw new Error(`Expected eslint-stylistic at ${UPSTREAM_COMMIT}, received ${actualCommit}.`);
}

const source = upstreamFile(SOURCE_FILE);
const ruleSource = upstreamFile(RULE_FILE);
const typesSource = upstreamFile(TYPES_FILE);
const parserMatrixSource = upstreamFile(PARSER_MATRIX_FILE);
for (const expected of [
  `defaultOptions: [{ component: true, html: true }]`,
  `${MESSAGE_ID}: '${MESSAGE}'`,
  `fixer.replaceTextRange(range, ' />')`,
]) {
  if (!ruleSource.includes(expected)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains ${JSON.stringify(expected)}.`);
  }
}
for (const expected of ['component?: boolean', 'html?: boolean']) {
  if (!typesSource.includes(expected)) {
    throw new Error(`Pinned ${TYPES_FILE} no longer contains ${JSON.stringify(expected)}.`);
  }
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-jsx-self-closing-comp-sync-'));
const tempFile = join(tempDir, `${RULE}.test.ts`);
writeFileSync(tempFile, source);

(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(tempFile).href}?commit=${UPSTREAM_COMMIT}`);
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}

const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 1 || runs[0].name !== SUITE_NAME) {
  throw new Error(`Expected one captured ${SUITE_NAME} suite, received ${runs.length}.`);
}

const valid = runs[0].valid.map((testCase, index) => normalizeValid(testCase, index));
const invalid = runs[0].invalid.map((testCase, index) => normalizeInvalid(testCase, index));
const diagnostics = invalid.reduce((count, testCase) => count + testCase.diagnostics.length, 0);
const logicalValid = valid.length / PARSERS.length;
const logicalInvalid = invalid.length / PARSERS.length;
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: UPSTREAM_VERSION,
    commit: UPSTREAM_COMMIT,
    sourceFile: SOURCE_FILE,
    ruleFile: RULE_FILE,
    typesFile: TYPES_FILE,
    parserMatrixFile: PARSER_MATRIX_FILE,
    sourceSha256: sha256(source),
    ruleSourceSha256: sha256(ruleSource),
    typesSourceSha256: sha256(typesSource),
    parserMatrixSourceSha256: sha256(parserMatrixSource),
    license: 'MIT',
    tool: 'tools/tasks/sync-stylistic-jsx-self-closing-comp-tests.ts',
    parserMatrix: [...PARSERS],
    inventory: {
      logicalValid,
      logicalInvalid,
      valid: valid.length,
      invalid: invalid.length,
      diagnostics,
      fixableInvalid: invalid.filter((testCase) => testCase.output !== null).length,
      unfixableInvalid: invalid.filter((testCase) => testCase.output === null).length,
      total: valid.length + invalid.length,
    },
  },
  valid,
  invalid,
};

if (
  logicalValid !== 35 ||
  logicalInvalid !== 12 ||
  valid.length !== 105 ||
  invalid.length !== 36 ||
  diagnostics !== 36
) {
  throw new Error(
    `Unexpected inventory: ${logicalValid} logical valid / ${logicalInvalid} logical invalid; ` +
      `${valid.length} expanded valid / ${invalid.length} expanded invalid / ${diagnostics} diagnostics.`,
  );
}

mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'inherit' });
console.log(
  `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_VERSION}: ` +
    `${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeValid(testCase: ExpandedCase, index: number) {
  assertExpandedCase(testCase, `valid ${index}`);
  return {
    code: testCase.code,
    parser: testCase.parser,
    ...(testCase.options ? { options: testCase.options } : {}),
  };
}

function normalizeInvalid(testCase: ExpandedCase, index: number) {
  assertExpandedCase(testCase, `invalid ${index}`);
  if (
    !Array.isArray(testCase.errors) ||
    testCase.errors.length !== 1 ||
    testCase.errors[0].messageId !== MESSAGE_ID
  ) {
    throw new Error(`Captured ${RULE} invalid ${index} has unexpected errors.`);
  }
  if (typeof testCase.output !== 'string') {
    throw new Error(`Captured ${RULE} invalid ${index} is missing its output.`);
  }

  const openingStart = testCase.code.indexOf('<');
  const openingEnd = testCase.code.indexOf('>', openingStart) + 1;
  const closingStart = testCase.code.indexOf('</', openingEnd);
  const closingEnd = testCase.code.indexOf('>', closingStart) + 1;
  if (
    openingStart < 0 ||
    openingEnd <= openingStart ||
    closingStart < 0 ||
    closingEnd <= closingStart
  ) {
    throw new Error(`Unable to locate the JSX boundaries in captured invalid ${index}.`);
  }

  const expectedOutput =
    testCase.code.slice(0, openingEnd - 1) + ' />' + testCase.code.slice(closingEnd);
  if (testCase.output !== expectedOutput) {
    throw new Error(`Captured output does not match the pinned fixer for invalid ${index}.`);
  }

  return {
    code: testCase.code,
    parser: testCase.parser,
    ...(testCase.options ? { options: testCase.options } : {}),
    diagnostics: [
      {
        messageId: MESSAGE_ID,
        message: MESSAGE,
        data: {},
        range: [utf8Offset(testCase.code, openingStart), utf8Offset(testCase.code, openingEnd)],
        location: locationRange(testCase.code, openingStart, openingEnd),
        fix: {
          range: [utf8Offset(testCase.code, openingEnd - 1), utf8Offset(testCase.code, closingEnd)],
          replacementText: ' />',
        },
      },
    ],
    output: testCase.output,
  };
}

function assertExpandedCase(testCase: ExpandedCase, label: string): void {
  if (
    !testCase ||
    typeof testCase.code !== 'string' ||
    !PARSERS.includes(testCase.parser as never)
  ) {
    throw new TypeError(`Captured ${RULE} ${label} is malformed.`);
  }
}

function locationRange(sourceText: string, start: number, end: number) {
  const startLocation = locationAt(sourceText, start);
  const endLocation = locationAt(sourceText, end);
  return {
    line: startLocation.line,
    column: startLocation.column,
    endLine: endLocation.line,
    endColumn: endLocation.column,
  };
}

function locationAt(sourceText: string, offset: number) {
  let line = 1;
  let lineStart = 0;
  for (let index = 0; index < offset; index += 1) {
    const character = sourceText[index];
    if (character === '\r') {
      if (sourceText[index + 1] === '\n') {
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

function utf8Offset(sourceText: string, utf16Offset: number): number {
  return Buffer.byteLength(sourceText.slice(0, utf16Offset));
}

function sha256(value: string): string {
  return createHash('sha256').update(value).digest('hex');
}

function expandCases(...rawCases: RawCase[]): ExpandedCase[] {
  return rawCases.flatMap((rawCase) => {
    const testCase: Record<string, unknown> =
      typeof rawCase === 'string' ? { code: rawCase } : { ...rawCase };
    const features = new Set((testCase.features as string[] | undefined) ?? []);
    delete testCase.features;

    return PARSERS.map((parser) => {
      const extras = [
        `features: [${[...features].join(',')}]`,
        `parser: ${parser}`,
        testCase.parserOptions ? `parserOptions: ${JSON.stringify(testCase.parserOptions)}` : '',
        testCase.options ? `options: ${JSON.stringify(testCase.options)}` : '',
        testCase.settings ? `settings: ${JSON.stringify(testCase.settings)}` : '',
      ];
      const extraComment = `\n// ${extras.join(', ')}`;
      return {
        ...JSON.parse(JSON.stringify(testCase)),
        code: `${String(testCase.code)}${extraComment}`,
        ...(typeof testCase.output === 'string'
          ? { output: `${testCase.output}${extraComment}` }
          : {}),
        parser,
      } as ExpandedCase;
    });
  });
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    valid: options.valid || [],',
    '    invalid: options.invalid || [],',
    '  });',
    '}',
  ].join('\n');
  const parsersStub = [
    `const PARSERS = ${JSON.stringify(PARSERS)};`,
    `const expandCases = ${expandCases.toString()};`,
    'export function valids(...tests) { return expandCases(...tests.flat().filter(Boolean)); }',
    'export function invalids(...tests) { return expandCases(...tests.flat().filter(Boolean)); }',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-test', shortCircuit: true };
      }
      if (specifier === '#test/parsers-jsx') {
        return { url: 'stub:///stylistic-parsers-jsx', shortCircuit: true };
      }
      if (specifier === `./${RULE}` || specifier === './types' || specifier === './types.d.ts') {
        return { url: 'stub:///stylistic-rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///stylistic-test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-parsers-jsx') {
        return { format: 'module', source: parsersStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
