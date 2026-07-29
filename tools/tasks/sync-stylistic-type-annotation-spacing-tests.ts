// Captures the stable @stylistic/eslint-plugin RuleTester suites from the
// vendored submodule as committed JSON fixtures. The upstream tests are
// TypeScript modules, so Node 24's type stripping executes a temporary copy
// while synchronous module hooks replace the test runner and rule imports.
//
// Re-run with `pnpm run port:tests:stylistic:type-annotation-spacing`.

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
const RULE = 'type-annotation-spacing';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILE = `packages/eslint-plugin/rules/${RULE}/${RULE}.test.ts`;
const FIXTURES_DIR = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
const FIXTURE_FILE = join(FIXTURES_DIR, `${RULE}.json`);
const CAPTURE_KEY = '__stylisticSyncCapture__';

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

if (runs.length !== 3 || runs.some((run) => run.name !== RULE)) {
  throw new Error(`Expected three captured ${RULE} suites, received ${runs.length}.`);
}

const valid = runs.flatMap((run) => run.valid).map(normalizeCase);
const invalid = runs.flatMap((run) => run.invalid).map(normalizeCase);
const fixture = {
  __generated: {
    source: '@stylistic/eslint-plugin',
    version: UPSTREAM_REF,
    commit: UPSTREAM_COMMIT,
    sourceFile:
      'packages/eslint-plugin/rules/type-annotation-spacing/type-annotation-spacing.test.ts',
    license: 'MIT',
    tool: 'tools/tasks/sync-stylistic-type-annotation-spacing-tests.ts',
  },
  valid,
  invalid,
};

mkdirSync(FIXTURES_DIR, { recursive: true });
writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
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
    'const fullWhitespace = /^\\s*$/;',
    'export function $(value) {',
    '  const source = typeof value === "string" ? value : value[0];',
    '  const lines = source.split("\\n");',
    '  const whitespaceLines = lines.map((line) => fullWhitespace.test(line));',
    '  const commonIndent = lines.reduce((min, line, index) => {',
    '    if (whitespaceLines[index]) return min;',
    '    const indent = line.match(/^\\s*/)?.[0].length;',
    '    return indent === undefined ? min : Math.min(min, indent);',
    '  }, Number.POSITIVE_INFINITY);',
    '  let head = 0;',
    '  while (head < lines.length && whitespaceLines[head]) head += 1;',
    '  let tail = 0;',
    '  while (tail < lines.length && whitespaceLines[lines.length - tail - 1]) tail += 1;',
    '  return lines.slice(head, lines.length - tail)',
    '    .map((line) => line.slice(commonIndent))',
    '    .join("\\n");',
    '}',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === '#test') {
        return { url: 'stub:///test', shortCircuit: true };
      }
      if (
        specifier === './type-annotation-spacing' ||
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
