// Captures the upstream eslint-plugin-simple-import-sort test suite straight
// from the vendored submodule and writes it to committed JSON fixtures, so our
// Vitest suite replays the real upstream cases (including the exact autofix
// output) and tracks behavior as the submodule is bumped (oxc-style test
// syncing).
//
// Unlike the other ported plugins, simple-import-sort is an autofix rule: the
// upstream tests assert the fixed source, not just a diagnostic count. The
// expected output lives inside `output: (actual) => expect(actual)
// .toMatchInlineSnapshot(`...`)` callbacks. We capture it by stubbing `vitest`
// with a capturing `expect`, then invoking each callback with a sentinel and
// reading back the snapshot argument. Error counts, options and the parser of
// each case are captured too.
//
// The upstream tests are ESM and drive cases through ESLint's `RuleTester`. We
// register synchronous module hooks (`module.registerHooks`) that stub
// `eslint` (capturing every `RuleTester.run` call and its constructor parser
// config), `vitest`, and the rule module under test (we only need the captured
// cases, not the rule object). The shared `helpers.js` is loaded for real
// because it provides the `input` tagged template and `setup`; both it and the
// copied test file get a small `require` shim injected because they call
// `require(...)`/`require.resolve(...)` at module scope (Vitest provides those
// in its runner; plain Node ESM does not).
//
// Each upstream test file is copied to a temp location before being imported,
// with its relative imports rewritten to sentinel specifiers that the hook
// chain intercepts. Cases are tagged with the parser they run under
// (`js`/`ts`/`flow`); the replay harness decides which parsers are in scope via
// parity.json (Flow syntax is not supported by Oxc and is quarantined there,
// with the dropped count logged — never silently truncated).
//
// Re-run with `pnpm run port:tests:simple-import-sort`, then `vp fmt --write`
// the fixtures: the JSON is emitted with `JSON.stringify` (matching the other
// sync tools), which does not collapse the short nested arrays in captured
// `options`, so the formatter normalizes them to match the committed style.

import { registerHooks } from 'node:module';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    license: string;
  }>;
};

type ParserKind = 'js' | 'ts' | 'flow';

type RawCase = Record<string, unknown> | string;
type CapturedRun = {
  name: string;
  parser: ParserKind;
  valid: RawCase[];
  invalid: RawCase[];
};
type ValidCase = { code: string; options?: unknown[]; parser: ParserKind };
type InvalidCase = {
  code: string;
  output: string;
  errors: number;
  options?: unknown[];
  parser: ParserKind;
};

const ROOT = process.cwd();
const HERE = dirname(fileURLToPath(import.meta.url));

// Shared globals that cross the hook-loaded module boundary: the eslint stub's
// `RuleTester.run` appends to CAPTURE_KEY; the vitest stub's `expect` records
// the last inline-snapshot argument under SNAPSHOT_KEY.
const CAPTURE_KEY = '__SIS_RUNS__';
const SNAPSHOT_KEY = '__SIS_LAST_SNAPSHOT__';

// A `require` shim injected at the top of the copied test file and the real
// helpers module. `require.resolve` on an uninstalled parser package returns
// the bare id instead of throwing, so the parser config can still be captured
// as data.
const REQUIRE_SHIM = [
  "import { createRequire as __sisCreateRequire } from 'node:module';",
  'const require = __sisCreateRequire(import.meta.url);',
  'const __sisResolve = require.resolve.bind(require);',
  'require.resolve = (id) => { try { return __sisResolve(id); } catch { return id; } };',
  '',
].join('\n');

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-simple-import-sort');
if (!plugin) {
  throw new Error('eslint-plugin-simple-import-sort is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const TESTS_DIR = join(SUBMODULE, 'test');
const HELPERS_FILE = join(TESTS_DIR, 'helpers.js');
const FIXTURES_DIR = join(ROOT, 'npm', 'simple-import-sort', 'test', 'fixtures');

// One upstream test file per rule.
const RULE_FILES: Array<{ rule: string; file: string }> = [
  { rule: 'imports', file: join(TESTS_DIR, 'imports.test.js') },
  { rule: 'exports', file: join(TESTS_DIR, 'exports.test.js') },
];

if (!existsSync(HELPERS_FILE)) {
  throw new Error(
    `Upstream tests not found at ${TESTS_DIR}. Run: git submodule update --init --depth 1 ${plugin.submodule}`,
  );
}

// Module-scope temp dir, set up by `main()` and consumed by `captureRuns`.
let tempDir = '';

async function main(): Promise<void> {
  mkdirSync(FIXTURES_DIR, { recursive: true });
  tempDir = mkdtempSync(join(tmpdir(), 'simple-import-sort-sync-'));
  registerStubHooks();

  const summary: string[] = [];

  try {
    for (const { rule, file } of RULE_FILES) {
      const runs = await captureRuns(file);
      const { valid, invalid, droppedDuplicates } = collapseRuns(runs);

      if (valid.length === 0 && invalid.length === 0) {
        throw new Error(`No upstream cases captured for rule "${rule}"`);
      }

      const fixture = {
        __generated: {
          source: plugin.npm,
          version: plugin.baselineVersion,
          sourceFile: `test/${rule}.test.js`,
          license: plugin.license,
          tool: 'tools/tasks/sync-simple-import-sort-tests.ts',
        },
        valid,
        invalid,
      };
      writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);

      const byParser = countByParser([...valid, ...invalid]);
      summary.push(
        `${rule}: ${valid.length} valid, ${invalid.length} invalid` +
          ` (${byParser.js} js, ${byParser.ts} ts, ${byParser.flow} flow;` +
          ` ${droppedDuplicates} cross-parser duplicate(s) collapsed)`,
      );
    }
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }

  console.log('Synced eslint-plugin-simple-import-sort fixtures from upstream:');
  for (const line of summary) {
    console.log(`- ${line}`);
  }
}

// --- capture ---------------------------------------------------------------

async function captureRuns(testFile: string): Promise<CapturedRun[]> {
  // Copy the test file to a temp dir and rewrite its relative imports to
  // sentinel specifiers the hook chain intercepts; inject a `require` shim so
  // the module-scope `require.resolve(...)` calls (for parser packages that are
  // not installed) do not throw.
  const original = readFileSync(testFile, 'utf8');
  const rewritten = REQUIRE_SHIM + rewriteImports(original);
  const tempFile = join(tempDir, `${nextId()}.mjs`);
  writeFileSync(tempFile, rewritten);

  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  await import(pathToFileURL(tempFile).href);

  const captured = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as Array<{
    name: string;
    config: { parser?: unknown };
    valid: RawCase[];
    invalid: RawCase[];
  }>;
  if (!captured || captured.length === 0) {
    throw new Error(`No RuleTester.run() call captured from ${testFile}`);
  }

  return captured.map((run) => ({
    name: run.name,
    parser: parserKindFromConfig(run.config?.parser),
    valid: run.valid,
    invalid: run.invalid,
  }));
}

// Rewrite the upstream test file's relative imports to sentinel bare specifiers
// the resolve hook recognizes. The rule import is stubbed; the helpers import
// is loaded for real (with a shim) from its sentinel.
function rewriteImports(source: string): string {
  return source
    .replace(/(['"])\.\.\/src\/index\.js\1/g, '"__sis_rule__"')
    .replace(/(['"])\.\/helpers\.js\1/g, '"__sis_helpers__"');
}

let idCounter = 0;
function nextId(): string {
  idCounter += 1;
  return `case-${idCounter}`;
}

function registerStubHooks(): void {
  const eslintStub = [
    'export class RuleTester {',
    '  constructor(config) { this.__config = config || {}; }',
    '  run(name, _rule, tests) {',
    `    (globalThis['${CAPTURE_KEY}'] ||= []).push({`,
    '      name,',
    '      config: this.__config,',
    '      valid: (tests && tests.valid) || [],',
    '      invalid: (tests && tests.invalid) || [],',
    '    });',
    '  }',
    '}',
    'export class Linter { getRules() { return new Map(); } }',
    "Linter.version = '8.57.0';",
  ].join('\n');

  // A capturing `expect`. `toMatchInlineSnapshot` (raw path) and `toBe` (the
  // path used by helpers' `expect2` wrapper) both record their argument so the
  // expected autofix output can be read back after invoking each `output`
  // callback. Everything else is a no-op.
  const vitestStub = [
    'function noop() {}',
    'function makeExpect() {',
    '  const fn = (actual) => ({',
    '    actual,',
    `    toBe(value) { globalThis['${SNAPSHOT_KEY}'] = { method: 'toBe', value }; },`,
    `    toMatchInlineSnapshot(value) { globalThis['${SNAPSHOT_KEY}'] = { method: 'toMatchInlineSnapshot', value: value ?? '' }; },`,
    '    toEqual: noop,',
    '    toStrictEqual: noop,',
    '    toThrow: noop,',
    '    toThrowErrorMatchingInlineSnapshot: noop,',
    '    toMatchSnapshot: noop,',
    '    not: { toBe: noop, toEqual: noop, toThrow: noop },',
    '  });',
    '  fn.addSnapshotSerializer = noop;',
    '  return fn;',
    '}',
    'export const expect = makeExpect();',
    'export function describe(_name, fn) { if (typeof fn === "function") { try { fn(); } catch {} } }',
    'export function test() {}',
    'export function it() {}',
    'export const vi = { fn: () => noop };',
    'export default { expect, describe, test, it, vi };',
  ].join('\n');

  const ruleStub = 'export default { rules: { imports: {}, exports: {} } };';

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'eslint') {
        return { url: 'stub:///eslint', shortCircuit: true };
      }
      if (specifier === 'vitest') {
        return { url: 'stub:///vitest', shortCircuit: true };
      }
      if (specifier === '__sis_rule__') {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      if (specifier === '__sis_helpers__') {
        return { url: 'stub:///helpers', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///eslint') {
        return { format: 'module', source: eslintStub, shortCircuit: true };
      }
      if (url === 'stub:///vitest') {
        return { format: 'module', source: vitestStub, shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'module', source: ruleStub, shortCircuit: true };
      }
      if (url === 'stub:///helpers') {
        // Helpers only needs `require('assert')`; base its `require` on the real
        // file path because `import.meta.url` here is the virtual stub URL.
        const helpersShim = [
          "import { createRequire as __sisCreateRequire } from 'node:module';",
          `const require = __sisCreateRequire(${JSON.stringify(pathToFileURL(HELPERS_FILE).href)});`,
          '',
        ].join('\n');
        const helpers = readFileSync(HELPERS_FILE, 'utf8');
        return { format: 'module', source: helpersShim + helpers, shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

// --- normalization ---------------------------------------------------------

// Collapse the 5 upstream runs (JavaScript / Flow / TypeScript over shared
// `baseTests`, plus Flow-specific and TypeScript-specific suites) into one
// valid/invalid set per rule, deduplicating the shared cases that run under
// multiple parsers. When the same (code, options) appears under several
// parsers we keep the most broadly supported one: js > ts > flow.
function collapseRuns(runs: CapturedRun[]): {
  valid: ValidCase[];
  invalid: InvalidCase[];
  droppedDuplicates: number;
} {
  const validByKey = new Map<string, ValidCase>();
  const invalidByKey = new Map<string, InvalidCase>();
  let droppedDuplicates = 0;

  for (const run of runs) {
    for (const raw of run.valid) {
      const normalized = normalizeValid(raw, run.parser);
      if (!normalized) {
        continue;
      }
      const key = caseKey(normalized.code, normalized.options);
      const existing = validByKey.get(key);
      if (!existing) {
        validByKey.set(key, normalized);
      } else {
        droppedDuplicates += 1;
        if (parserRank(normalized.parser) < parserRank(existing.parser)) {
          validByKey.set(key, normalized);
        }
      }
    }

    for (const raw of run.invalid) {
      const normalized = normalizeInvalid(raw, run.parser);
      if (!normalized) {
        continue;
      }
      const key = caseKey(normalized.code, normalized.options);
      const existing = invalidByKey.get(key);
      if (!existing) {
        invalidByKey.set(key, normalized);
      } else {
        droppedDuplicates += 1;
        if (parserRank(normalized.parser) < parserRank(existing.parser)) {
          invalidByKey.set(key, normalized);
        }
      }
    }
  }

  return {
    valid: [...validByKey.values()],
    invalid: [...invalidByKey.values()],
    droppedDuplicates,
  };
}

function normalizeValid(raw: RawCase, runParser: ParserKind): ValidCase | null {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (value == null || typeof value !== 'object' || typeof value.code !== 'string') {
    return null;
  }
  const options = Array.isArray(value.options) ? value.options : undefined;
  return {
    code: value.code,
    ...(options ? { options } : {}),
    parser: caseParser(value, runParser),
  };
}

function normalizeInvalid(raw: RawCase, runParser: ParserKind): InvalidCase | null {
  if (typeof raw !== 'object' || raw == null) {
    return null;
  }
  const value = raw as Record<string, unknown>;
  if (typeof value.code !== 'string') {
    return null;
  }
  const output = resolveOutput(value.output, value.code);
  const errors = resolveErrorCount(value.errors);
  if (output == null || errors == null) {
    return null;
  }
  const options = Array.isArray(value.options) ? value.options : undefined;
  return {
    code: value.code,
    output,
    errors,
    ...(options ? { options } : {}),
    parser: caseParser(value, runParser),
  };
}

// Resolve the expected autofix output. The common form is a callback that calls
// `expect(actual).toMatchInlineSnapshot(`...`)`; we invoke it with a sentinel
// and read the recorded snapshot back. A plain string is taken verbatim. A null
// output (ESLint's "no autofix") falls back to the unchanged code.
function resolveOutput(output: unknown, code: string): string | null {
  if (typeof output === 'string') {
    return output;
  }
  if (output === null) {
    return code;
  }
  if (typeof output !== 'function') {
    return null;
  }
  (globalThis as Record<string, unknown>)[SNAPSHOT_KEY] = null;
  try {
    (output as (actual: string) => void)('');
  } catch {
    return null;
  }
  const recorded = (globalThis as Record<string, unknown>)[SNAPSHOT_KEY] as {
    method: string;
    value: string;
  } | null;
  if (!recorded || typeof recorded.value !== 'string') {
    return null;
  }
  return cleanSnapshot(recorded.method, recorded.value);
}

// The upstream helpers indent inline snapshots and prefix every line with `|`.
// `toMatchInlineSnapshot` receives the raw indented template; `toBe` (via the
// `expect2` wrapper) receives the already pipe-stripped form. Normalize both to
// the real expected source.
function cleanSnapshot(method: string, value: string): string {
  if (method === 'toMatchInlineSnapshot') {
    return value.replace(/\n *\|/g, '\n').replace(/^\n|\n[^\S\n]*$/g, '');
  }
  // `toBe`: value is `strip(snapshot, { keepPipes: true })` — every line begins
  // with a `|` marker (edges already trimmed). Drop the markers.
  return value.replace(/^\|/, '').replace(/\n\|/g, '\n');
}

function resolveErrorCount(errors: unknown): number | null {
  if (typeof errors === 'number') {
    return errors;
  }
  if (Array.isArray(errors)) {
    return errors.length;
  }
  return null;
}

function caseParser(value: Record<string, unknown>, runParser: ParserKind): ParserKind {
  if (typeof value.parser === 'string') {
    return parserKindFromConfig(value.parser);
  }
  const languageOptions = value.languageOptions as { parser?: unknown } | undefined;
  if (languageOptions && 'parser' in languageOptions) {
    return parserKindFromConfig(languageOptions.parser);
  }
  return runParser;
}

function parserKindFromConfig(parser: unknown): ParserKind {
  if (typeof parser !== 'string') {
    return 'js';
  }
  if (parser.includes('typescript')) {
    return 'ts';
  }
  if (parser.includes('babel')) {
    return 'flow';
  }
  return 'js';
}

function parserRank(parser: ParserKind): number {
  return parser === 'js' ? 0 : parser === 'ts' ? 1 : 2;
}

function caseKey(code: string, options: unknown[] | undefined): string {
  return `${code} ${JSON.stringify(options ?? [])}`;
}

function countByParser(cases: Array<{ parser: ParserKind }>): Record<ParserKind, number> {
  const counts: Record<ParserKind, number> = { js: 0, ts: 0, flow: 0 };
  for (const entry of cases) {
    counts[entry.parser] += 1;
  }
  return counts;
}

void HERE;

await main();
