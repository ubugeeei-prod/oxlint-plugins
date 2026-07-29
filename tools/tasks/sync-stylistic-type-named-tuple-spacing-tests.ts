// Captures every stable @stylistic/type-named-tuple-spacing v5.10.0
// RuleTester case from the exact pinned upstream commit.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };

const ROOT = process.cwd();
const RULE = 'type-named-tuple-spacing';
const VERSION = 'v5.10.0';
const COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v5.10.0.json`);
const CAPTURE_KEY = '__stylisticTypeNamedTupleSpacingCapture__';

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
const temp = mkdtempSync(join(tmpdir(), 'stylistic-named-tuple-sync-'));
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
    tool: 'tools/tasks/sync-stylistic-type-named-tuple-spacing-tests.ts',
    inventory: {
      valid: valid.length,
      invalid: invalid.length,
      diagnostics,
      fixableInvalid: invalid.length,
      unfixableInvalid: 0,
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
  const allowed = new Set(invalid ? ['code', 'output', 'errors'] : ['code']);
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported case ${index} keys: ${unsupported.join(', ')}`);
  }
  if (invalid && (typeof value.output !== 'string' || !Array.isArray(value.errors))) {
    throw new Error(`Invalid case ${index} is missing output/errors.`);
  }
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = ${JSON.stringify(CAPTURE_KEY)};`,
    'export function run(options) {',
    '  globalThis[key].push({ name: options.name, valid: options.valid || [], invalid: options.invalid || [] });',
    '}',
    `export const $ = ${unindent.toString()};`,
  ].join('\n');
  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///stylistic-test', shortCircuit: true };
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
      if (url === 'stub:///stylistic-rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

function unindent(value: string | TemplateStringsArray) {
  const lines = (typeof value === 'string' ? value : value[0]).split('\n');
  const blank = lines.map((line) => /^\s*$/.test(line));
  const indent = lines.reduce(
    (minimum, line, index) =>
      blank[index] ? minimum : Math.min(minimum, line.match(/^\s*/)?.[0].length ?? minimum),
    Number.POSITIVE_INFINITY,
  );
  let head = 0;
  while (head < lines.length && blank[head]) head++;
  let tail = 0;
  while (tail < lines.length && blank[lines.length - tail - 1]) tail++;
  return lines
    .slice(head, lines.length - tail)
    .map((line) => line.slice(indent))
    .join('\n');
}
