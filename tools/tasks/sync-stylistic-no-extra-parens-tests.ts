// Captures both stable @stylistic/eslint-plugin no-extra-parens RuleTester
// suites from the pinned vendored submodule as a committed JSON fixture.
//
// Re-run with `pnpm run port:tests:stylistic:no-extra-parens`.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { registerHooks } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type RawCase = string | Record<string, unknown>;
type CapturedRun = {
  lang?: string;
  name: string;
  valid: RawCase[];
  invalid: RawCase[];
};

const ROOT = process.cwd();
const UPSTREAM_REF = 'v5.10.0';
const UPSTREAM_COMMIT = 'efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712';
const RULE = 'no-extra-parens';
const UPSTREAM_DIR = join(ROOT, 'upstream', 'eslint-stylistic');
const SOURCE_FILES = [
  `packages/eslint-plugin/rules/${RULE}/${RULE}._js_.test.ts`,
  `packages/eslint-plugin/rules/${RULE}/${RULE}._ts_.test.ts`,
] as const;
const FIXTURES_DIR = join(ROOT, 'npm', 'stylistic', 'test', 'fixtures');
const FIXTURE_FILE = join(FIXTURES_DIR, `${RULE}-v5.10.0.json`);
const CAPTURE_KEY = '__stylisticNoExtraParensSyncCapture__';

if (!existsSync(UPSTREAM_DIR)) {
  throw new Error(
    `Upstream checkout not found at ${UPSTREAM_DIR}. Run \`git submodule update --init upstream/eslint-stylistic\` first.`,
  );
}

const actualCommit = execFileSync('git', ['-C', UPSTREAM_DIR, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== UPSTREAM_COMMIT) {
  throw new Error(
    `Expected upstream/eslint-stylistic at ${UPSTREAM_COMMIT}, received ${actualCommit}.`,
  );
}

registerCaptureHooks();
const tempDir = mkdtempSync(join(tmpdir(), 'stylistic-no-extra-parens-sync-'));

try {
  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  for (const [index, sourceFile] of SOURCE_FILES.entries()) {
    const tempFile = join(tempDir, `${RULE}-${index}.test.ts`);
    const source = execFileSync(
      'git',
      ['-C', UPSTREAM_DIR, 'show', `${UPSTREAM_COMMIT}:${sourceFile}`],
      { encoding: 'utf8' },
    );
    writeFileSync(tempFile, source);
    await import(`${pathToFileURL(tempFile).href}?capture=${index}-${Date.now()}`);
  }

  const runs = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
  if (runs.length !== SOURCE_FILES.length || runs.some((run) => run.name !== RULE)) {
    throw new Error(`Expected two captured ${RULE} suites, received ${runs.length}.`);
  }

  const valid = runs.flatMap((run) =>
    run.valid.map((testCase) => normalizeCase(testCase, run.lang ?? 'ts')),
  );
  const invalid = runs.flatMap((run) =>
    run.invalid.map((testCase) => normalizeCase(testCase, run.lang ?? 'ts')),
  );
  const fixture = {
    __generated: {
      source: '@stylistic/eslint-plugin',
      version: UPSTREAM_REF,
      commit: UPSTREAM_COMMIT,
      sourceFiles: SOURCE_FILES,
      license: 'MIT',
      tool: 'tools/tasks/sync-stylistic-no-extra-parens-tests.ts',
    },
    valid,
    invalid,
  };

  mkdirSync(FIXTURES_DIR, { recursive: true });
  writeFileSync(FIXTURE_FILE, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('vp', ['fmt', FIXTURE_FILE], { stdio: 'ignore' });
  console.log(
    `Synced ${RULE} from @stylistic/eslint-plugin ${UPSTREAM_REF}: ${valid.length} valid, ${invalid.length} invalid.`,
  );
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}

function normalizeCase(raw: RawCase, language: string): Record<string, unknown> {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  if (typeof clone.code !== 'string') {
    throw new TypeError(`Captured ${RULE} case is missing string code.`);
  }
  return { language, ...clone };
}

function registerCaptureHooks(): void {
  const testStub = [
    `const key = '${CAPTURE_KEY}';`,
    'export function run(options) {',
    '  globalThis[key].push({',
    '    name: options.name,',
    '    lang: options.lang,',
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
      if (specifier === `./${RULE}` || specifier === './types' || specifier === './types.d.ts') {
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
