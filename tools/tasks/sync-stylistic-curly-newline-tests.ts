// Captures the stable @stylistic/eslint-plugin RuleTester suite as a committed
// JSON fixture. Node's type stripping executes a temporary copy while module
// hooks replace the upstream test runner and rule imports.

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
const UPSTREAM_REF = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const RULE = 'curly-newline';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURES_DIR = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
const FIXTURE_FILE = join(FIXTURES_DIR, `${RULE}-v5.10.0.json`);
const CAPTURE_KEY = '__stylisticCurlyNewlineCapture__';

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
  );
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-sync-'));
const tempFile = join(tempDir, `${RULE}.test.ts`);
const source = execFileSync(
  'git',
  ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${SOURCE_FILE}`],
  {
    encoding: 'utf8',
  },
);
writeFileSync(tempFile, source);

(globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
await import(`${pathToFileURL(tempFile).href}?capture=${Date.now()}`);
const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
rmSync(tempDir, { recursive: true, force: true });

if (runs.length !== 1 || runs[0].name !== RULE) {
  throw new Error(`Expected one captured ${RULE} suite, received ${runs.length}.`);
}

const valid = runs[0].valid.map(normalizeCase);
const invalid = runs[0].invalid.map(normalizeCase);
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: UPSTREAM_REF,
    commit: UPSTREAM_COMMIT,
    sourceFile: SOURCE_FILE,
    license: 'MIT',
    tool: 'tools/tasks/sync-stylistic-curly-newline-tests.ts',
  },
  valid,
  invalid,
};

mkdirSync(FIXTURES_DIR, { recursive: true });
writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'inherit' });
console.log(
  `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_REF}: ${valid.length} valid, ${invalid.length} invalid.`,
);

function normalizeCase(raw: RawCase): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  if (typeof clone.code !== 'string') {
    throw new TypeError(`Captured ${RULE} case is missing string code.`);
  }
  return clone;
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = '${CAPTURE_KEY}';`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    valid: options.valid || [],',
    '    invalid: options.invalid || [],',
    '  });',
    '}',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///test', shortCircuit: true };
      }
      if (
        specifier === './curly-newline' ||
        specifier === './types' ||
        specifier === './types.d.ts'
      ) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///test') {
        return { format: 'module', source: testStub, shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'module', source: 'export default {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}
