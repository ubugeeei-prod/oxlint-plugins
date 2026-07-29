// Captures every stable @stylistic/nonblock-statement-body-position v5.10.0
// RuleTester case from the exact pinned upstream commit.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = {
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};

const ROOT = process.cwd();
const RULE = 'nonblock-statement-body-position';
const UPSTREAM_VERSION = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURE_FILE = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures', `${RULE}-v5.10.0.json`);
const CAPTURE_KEY = '__stylisticNonblockStatementBodyPositionCapture__';

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

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-nonblock-position-sync-'));
const tempFile = join(tempDir, `${RULE}.test.ts`);
const source = execFileSync(
  'git',
  ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${SOURCE_FILE}`],
  { encoding: 'utf8' },
);
writeFileSync(tempFile, source);

(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
try {
  await import(`${pathToFileURL(tempFile).href}?commit=${UPSTREAM_COMMIT}`);
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
if (runs.length !== 1 || runs[0].name !== RULE) {
  throw new Error(`Expected one captured ${RULE} suite, received ${runs.length}.`);
}

const valid = runs[0].valid.map((testCase, index) =>
  normalizeCase(testCase, false, `valid ${index}`),
);
const invalid = runs[0].invalid.map((testCase, index) =>
  normalizeCase(testCase, true, `invalid ${index}`),
);
const diagnostics = invalid.reduce(
  (count, testCase) => count + (testCase.errors as unknown[]).length,
  0,
);
const unfixableInvalid = invalid.filter((testCase) => testCase.output === null).length;
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: UPSTREAM_VERSION,
    commit: UPSTREAM_COMMIT,
    sourceFile: SOURCE_FILE,
    license: 'MIT',
    tool: 'tools/tasks/sync-stylistic-nonblock-statement-body-position-tests.ts',
    inventory: {
      valid: valid.length,
      invalid: invalid.length,
      diagnostics,
      fixableInvalid: invalid.length - unfixableInvalid,
      unfixableInvalid,
      total: valid.length + invalid.length,
    },
  },
  valid,
  invalid,
};

mkdirSync(join(ROOT, 'npm', 'stylistic', 'test', 'fixtures'), { recursive: true });
writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'inherit' });
console.log(
  `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_VERSION}: ` +
    `${valid.length} valid, ${invalid.length} invalid, ${diagnostics} diagnostics.`,
);

function normalizeCase(raw: RawCase, invalid: boolean, label: string): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (!value || typeof value !== 'object' || typeof value.code !== 'string') {
    throw new TypeError(`Captured ${RULE} ${label} is missing string code.`);
  }
  const allowed = new Set(invalid ? ['code', 'options', 'output', 'errors'] : ['code', 'options']);
  const unsupported = Object.keys(value).filter((key) => !allowed.has(key));
  if (unsupported.length > 0) {
    throw new Error(`Unsupported ${RULE} ${label} keys: ${unsupported.join(', ')}`);
  }
  if (invalid) {
    if (!Array.isArray(value.errors)) {
      throw new Error(`Captured ${RULE} ${label} is missing errors.`);
    }
    if (typeof value.output !== 'string' && value.output !== null) {
      throw new Error(`Captured ${RULE} ${label} is missing output.`);
    }
  }
  return JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
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
  const whitespace = lines.map((line) => /^\s*$/.test(line));
  const indent = lines.reduce(
    (minimum, line, index) =>
      whitespace[index] ? minimum : Math.min(minimum, line.match(/^\s*/)?.[0].length ?? minimum),
    Number.POSITIVE_INFINITY,
  );
  let head = 0;
  while (head < lines.length && whitespace[head]) head++;
  let tail = 0;
  while (tail < lines.length && whitespace[lines.length - tail - 1]) tail++;
  return lines
    .slice(head, lines.length - tail)
    .map((line) => line.slice(indent))
    .join('\n');
}
