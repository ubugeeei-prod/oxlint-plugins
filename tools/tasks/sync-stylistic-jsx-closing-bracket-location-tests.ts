// Captures every authored stable @stylistic/jsx-closing-bracket-location
// v5.10.0 RuleTester case from the exact pinned upstream commit. The upstream
// parser matrix repeats each authored JSX case across compatible parsers; this
// fixture stores each semantic case once and the native replay runs it through
// Oxc's JSX/TSX parser.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };

const ROOT = process.cwd();
const RULE = 'jsx-closing-bracket-location';
const VERSION = 'v5.10.0';
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v5.10.0.json`);
const CAPTURE_KEY = '__stylisticJsxClosingBracketLocationCapture__';

if (!existsSync(UPSTREAM)) {
  throw new Error(`Missing ${UPSTREAM}; initialize upstream/eslint-stylistic.`);
}
const actualCommit = execFileSync('git', ['-C', UPSTREAM, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== COMMIT) {
  throw new Error(`Expected eslint-stylistic ${COMMIT}, received ${actualCommit}.`);
}

registerCaptureHooks();
const temp = mkdtempSync(join(tmpdir(), 'stylistic-jsx-closing-bracket-sync-'));
const sourcePath = join(temp, `${RULE}.test.ts`);
writeFileSync(
  sourcePath,
  execFileSync('git', ['-C', UPSTREAM, 'show', `${COMMIT}:${SOURCE_FILE}`], {
    encoding: 'utf8',
  }),
);
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
    license: 'MIT',
    parserMatrix: 'authored semantic cases; replayed with Oxc JSX/TSX',
    tool: 'tools/tasks/sync-stylistic-jsx-closing-bracket-location-tests.ts',
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

function normalizeCase(raw: RawCase, invalid: boolean, index: number): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new Error(`${invalid ? 'invalid' : 'valid'} case ${index} has no code.`);
  }
  const allowed = new Set(
    invalid
      ? ['code', 'output', 'errors', 'options', 'parserOptions', 'settings', 'languageOptions']
      : ['code', 'options', 'parserOptions', 'settings', 'languageOptions'],
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
    normalized.errors = (normalized.errors as Array<Record<string, unknown>>).map((error) => {
      const data = error.data as { location?: unknown; details?: unknown } | undefined;
      if (
        error.messageId !== 'bracketLocation' ||
        typeof data?.location !== 'string' ||
        typeof data.details !== 'string'
      ) {
        throw new Error(`Invalid case ${index} has an unsupported error contract.`);
      }
      return {
        ...error,
        message: `The closing bracket must be ${data.location}${data.details}`,
      };
    });
  }
  return normalized;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({ name: options.name, valid: options.valid || [], invalid: options.invalid || [] });',
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
