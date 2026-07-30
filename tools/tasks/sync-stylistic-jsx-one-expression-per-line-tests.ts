// Captures every authored stable @stylistic/jsx-one-expression-per-line
// v5.10.0 RuleTester case from the exact pinned upstream commit. The upstream
// parser matrix repeats authored JSX cases across compatible parsers; this
// fixture stores every semantic case once and native tests replay JSX and TSX.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };

const ROOT = process.cwd();
const RULE = 'jsx-one-expression-per-line';
const VERSION = 'v5.10.0';
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const RULE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.ts`;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-${VERSION}.json`);
const CAPTURE_KEY = '__stylisticJsxOneExpressionPerLineCapture__';
const MESSAGE = '`{{descriptor}}` must be placed on a new line';

if (!existsSync(UPSTREAM)) {
  throw new Error(`Missing ${UPSTREAM}; initialize upstream/eslint-stylistic.`);
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== COMMIT) {
  throw new Error(`Expected eslint-stylistic ${COMMIT}, received ${actualCommit}.`);
}

const ruleSource = upstreamFile(RULE_FILE);
for (const expected of [
  "description: 'Require one JSX element per line'",
  "fixable: 'whitespace'",
  "defaultOptions: [{ allow: 'none' }]",
  `moveToNewLine: '${MESSAGE}'`,
]) {
  if (!ruleSource.includes(expected)) {
    throw new Error(`Pinned ${RULE_FILE} no longer contains ${JSON.stringify(expected)}.`);
  }
}

registerCaptureHooks();
const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-one-expression-per-line-sync-'));
const sourcePath = join(temp, `${RULE}.test.ts`);
writeFileSync(sourcePath, upstreamFile(SOURCE_FILE));
(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(sourcePath).href}?commit=${COMMIT}`);
} finally {
  rmSync(temp, { recursive: true, force: true });
}

const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 1 || runs[0].name !== RULE) {
  throw new Error(`Expected one ${RULE} suite, received ${runs.length}.`);
}

const valid = runs[0].valid.map((value, index) => normalizeCase(value, false, index));
const invalid = runs[0].invalid.map((value, index) => normalizeCase(value, true, index));
const diagnostics = invalid.reduce(
  (count, testCase) => count + (testCase.errors as unknown[]).length,
  0,
);
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: VERSION,
    commit: COMMIT,
    sourceFile: SOURCE_FILE,
    ruleFile: RULE_FILE,
    license: 'MIT',
    parserMatrix: 'authored semantic cases; replayed with Oxc JSX and TSX',
    tool: 'tools/tasks/sync-stylistic-jsx-one-expression-per-line-tests.ts',
    inventory: {
      valid: valid.length,
      invalid: invalid.length,
      diagnostics,
      fixableInvalid: invalid.filter((testCase) => typeof testCase.output === 'string').length,
      unfixableInvalid: invalid.filter((testCase) => testCase.output === null).length,
      total: valid.length + invalid.length,
    },
  },
  valid,
  invalid,
};

mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE], { stdio: 'inherit' });
console.log(
  `Synced ${RULE} ${VERSION}: ${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function upstreamFile(path: string): string {
  return execFileSync('git', ['-C', UPSTREAM, 'show', `${COMMIT}:${path}`], {
    encoding: 'utf8',
  });
}

function normalizeCase(raw: RawCase, invalid: boolean, index: number): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`${invalid ? 'invalid' : 'valid'} case ${index} has no code.`);
  }
  const allowed = new Set(
    invalid
      ? [
          'code',
          'output',
          'errors',
          'options',
          'features',
          'parserOptions',
          'settings',
          'languageOptions',
        ]
      : ['code', 'options', 'features', 'parserOptions', 'settings', 'languageOptions'],
  );
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported case ${index} keys: ${unsupported.join(', ')}`);
  }
  if (
    invalid &&
    ((typeof value.output !== 'string' && value.output !== null) || !Array.isArray(value.errors))
  ) {
    throw new Error(`Invalid case ${index} is missing output/errors.`);
  }

  const normalized = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  if (invalid) {
    normalized.errors = (normalized.errors as Array<Record<string, unknown>>).map(
      (error, errorIndex) => {
        if (error.messageId !== 'moveToNewLine') {
          throw new Error(
            `Invalid case ${index} error ${errorIndex} has unsupported message ${String(error.messageId)}.`,
          );
        }
        const descriptor = (error.data as { descriptor?: unknown } | undefined)?.descriptor;
        return {
          ...error,
          ...(typeof descriptor === 'string'
            ? { message: MESSAGE.replace('{{descriptor}}', descriptor) }
            : {}),
        };
      },
    );
  }
  return normalized;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({ name: options.name, valid: options.valid || [], invalid: options.invalid || [] });',
    '}',
    'export function $(strings, ...substitutions) {',
    '  const text = typeof strings === "string"',
    '    ? strings',
    '    : strings.reduce((result, part, index) => result + part + (index < substitutions.length ? substitutions[index] : ""), "");',
    '  const lines = text.split("\\n");',
    '  const nonblank = lines.filter((line) => !/^\\s*$/.test(line));',
    '  const indent = nonblank.reduce((minimum, line) => Math.min(minimum, /^\\s*/.exec(line)[0].length), Infinity);',
    '  while (lines.length && /^\\s*$/.test(lines[0])) lines.shift();',
    '  while (lines.length && /^\\s*$/.test(lines.at(-1))) lines.pop();',
    '  return lines.map((line) => line.slice(indent)).join("\\n");',
    '}',
  ].join('\n');
  const parserStub = [
    'function authored(tests) { return tests.flat(Infinity).filter(Boolean); }',
    'export function valids(...tests) { return authored(tests); }',
    'export function invalids(...tests) { return authored(tests); }',
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
        return { format: 'module', source: parserStub, shortCircuit: true };
      }
      if (url === 'stub:///stylistic-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
