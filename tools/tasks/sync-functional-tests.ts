// Captures the upstream eslint-plugin-functional test suite straight from the
// vendored submodule and writes it to committed JSON fixtures, so our Vitest
// suite replays the real upstream cases and tracks behavior as the submodule is
// bumped (oxc-style test syncing). Mirrors tools/tasks/sync-eslint-comments-tests.ts.
//
// Upstream drives cases through `eslint-vitest-rule-tester`'s `createRuleTester`,
// which returns imperative `valid()`/`invalid()` functions called inside async
// Vitest `it()` blocks (not a declarative array as ESLint's classic RuleTester
// uses). We register synchronous module hooks (`module.registerHooks`) that stub:
//   - `vitest`            -> collects `it()` callbacks and runs them after import,
//                            so the `valid()`/`invalid()` calls execute; `expect`
//                            and `toMatchSnapshot` are chainable no-ops.
//   - `eslint-vitest-rule-tester` -> `createRuleTester({ name, configs })` returns
//                            capturing `valid`/`invalid`; the `configs` object
//                            (stubbed below) tells us whether the block is the
//                            type-aware TypeScript config or the syntax-only one.
//   - `../utils/configs`  -> marker objects (`esLatestConfig` / `typescriptConfig`)
//                            so each case is tagged `typeAware: true|false`.
//   - `dedent`            -> a faithful reimplementation (the package is not
//                            installed in the submodule).
//   - `#/rules/<name>`    -> `{ name: '<name>', rule: {} }`; the real rule module
//                            pulls deps the shallow submodule has not installed,
//                            and we only need the captured cases, not the rule.
//   - `is-immutable-type` -> an `Immutability` proxy yielding member names, so the
//                            type-aware rules' enum option values serialize readably.
//
// Many upstream invalid tests assert their reported errors via
// `toMatchSnapshot()` rather than inline `errors`, so the captured expectations
// are completed from the committed Vitest `__snapshots__` (the `expect(result)
// .toMatchSnapshot()` call is matched to its snapshot key and the messageIds are
// extracted). When a case has neither inline `errors` nor a snapshot, the
// upstream `invalid()` only asserts "at least one error", so `errors` is left
// omitted and the replay harness checks `>= 1`.
//
// The upstream `.ts` test files run directly through Node's native type stripping
// (Node >= 24). Type-aware cases (TypeScript `projectService` config) are captured
// and tagged but cannot be replayed by the syntax-only Rust port; the replay
// harness skips and counts them (no silent truncation). `readonly-type` has no
// upstream test file; its fixture is written empty and logged.
//
// Re-run with `pnpm run port:tests:functional`.

import { registerHooks } from 'node:module';
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

type Manifest = {
  submoduleRoot: string;
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

type CapturedCase = {
  code: string;
  options?: unknown[];
  filename?: string;
  errors?: unknown;
  output?: string | null;
  typeAware: boolean;
  // Authoritative expected messageIds extracted from the upstream Vitest
  // snapshot when the test relied on `toMatchSnapshot()` instead of declaring
  // inline `errors`. Preferred over `errors` during normalization.
  snapshotMessageIds?: string[];
};
type Capture = { valid: CapturedCase[]; invalid: CapturedCase[] };
type CollectedTest = { name: string; fullName: string; fn: () => unknown };

type SyncGlobal = {
  capture: Capture;
  // Describe-name stack (maintained while test files are imported) and the
  // running snapshot key state (maintained while collected `it`s execute), so
  // `expect(result).toMatchSnapshot()` can be matched to the upstream snapshot.
  stack: string[];
  currentTest: { fullName: string; snapCounter: number } | null;
  snapshotsByKey: Record<string, string[]>;
  collected: CollectedTest[];
};

const ROOT = process.cwd();
const SYNC_KEY = '__functionalSyncState__';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-functional');
if (!plugin) {
  throw new Error('eslint-plugin-functional is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const SRC_RULES_DIR = join(SUBMODULE, 'src', 'rules');
const TESTS_DIR = join(SUBMODULE, 'tests', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'functional', 'test', 'fixtures');
const REF = plugin.pinnedRef ?? `v${plugin.baselineVersion}`;

if (!existsSync(SRC_RULES_DIR) || !existsSync(TESTS_DIR)) {
  throw new Error(
    `Upstream sources not found under ${SUBMODULE}. Run: git submodule update --init ${plugin.submodule}`,
  );
}

mkdirSync(FIXTURES_DIR, { recursive: true });

const state: SyncGlobal = {
  capture: { valid: [], invalid: [] },
  stack: [],
  currentTest: null,
  snapshotsByKey: {},
  collected: [],
};
(globalThis as Record<string, unknown>)[SYNC_KEY] = state;

registerStubHooks();

const ruleNames = readdirSync(SRC_RULES_DIR)
  .filter((file) => file.endsWith('.ts') && file !== 'index.ts')
  .map((file) => file.slice(0, -3))
  .sort();

const summary: string[] = [];

for (const rule of ruleNames) {
  const testFiles = testFilesForRule(rule);

  if (testFiles.length === 0) {
    writeFixture(rule, [], { valid: [], invalid: [] }, true);
    summary.push(`${rule}: no upstream test file (empty fixture written)`);
    continue;
  }

  state.capture = { valid: [], invalid: [] };
  state.collected = [];
  state.stack = [];
  state.currentTest = null;
  state.snapshotsByKey = loadSnapshots(testFiles);

  // Sequential import + run keeps the shared capture buffer deterministic:
  // `it()` callbacks are only registered during import (building the describe
  // stack), then run in order so the imperative `valid()`/`invalid()` calls
  // inside them fire and are captured. Before each `it` runs we publish its full
  // name + a fresh snapshot counter so `toMatchSnapshot()` resolves the matching
  // upstream snapshot key (`<describe> > … > <it> <n>`).
  for (const file of testFiles) {
    await import(pathToFileURL(file).href);
  }
  for (const test of state.collected) {
    state.currentTest = { fullName: test.fullName, snapCounter: 0 };
    await test.fn();
  }
  state.currentTest = null;

  const captured = state.capture;
  const result = normalizeCapture(captured);
  const relFiles = testFiles.map((file) => file.slice(SUBMODULE.length + 1));
  writeFixture(rule, relFiles, result.cases, false);

  const tAware =
    result.cases.valid.filter((c) => c.typeAware).length +
    result.cases.invalid.filter((c) => c.typeAware).length;
  summary.push(
    `${rule}: ${result.cases.valid.length} valid, ${result.cases.invalid.length} invalid` +
      ` (${tAware} type-aware)${result.dropped > 0 ? `, ${result.dropped} dropped` : ''}`,
  );
}

console.log(`Synced eslint-plugin-functional fixtures from upstream ${REF}:`);
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ---------------------------------------------------------------

function testFilesForRule(rule: string): string[] {
  const files: string[] = [];
  const direct = join(TESTS_DIR, `${rule}.test.ts`);
  if (existsSync(direct)) {
    files.push(direct);
  }
  const dir = join(TESTS_DIR, rule);
  if (existsSync(dir) && statSync(dir).isDirectory()) {
    for (const entry of readdirSync(dir).sort()) {
      if (entry.endsWith('.test.ts')) {
        files.push(join(dir, entry));
      }
    }
  }
  return files;
}

// Parse the Vitest snapshot files alongside the given test files into a map of
// snapshot key -> ordered list of expected messageIds. Many upstream invalid
// tests assert via `toMatchSnapshot()` rather than inline `errors`, and the
// snapshot is the authoritative expected output. A snapshot body nests the same
// messages under a `steps` array, so only the messageIds before `"steps"` (the
// top-level `messages`) are taken to avoid double-counting.
function loadSnapshots(testFiles: string[]): Record<string, string[]> {
  const map: Record<string, string[]> = {};
  for (const file of testFiles) {
    const snapFile = join(dirname(file), '__snapshots__', `${basename(file)}.snap`);
    if (!existsSync(snapFile)) {
      continue;
    }
    const text = readFileSync(snapFile, 'utf8');
    const entry = /exports\[`([\s\S]*?)`\]\s*=\s*`([\s\S]*?)`;/g;
    let match: RegExpExecArray | null;
    while ((match = entry.exec(text)) !== null) {
      const key = match[1];
      const topLevel = match[2].split('"steps"')[0];
      const ids = [...topLevel.matchAll(/"messageId":\s*"([^"]+)"/g)].map((m) => m[1]);
      map[key] = ids;
    }
  }
  return map;
}

function writeFixture(
  rule: string,
  sourceFiles: string[],
  cases: Capture,
  noUpstreamTests: boolean,
): void {
  const fixture = {
    __generated: {
      source: plugin!.npm,
      ref: REF,
      sourceFiles,
      license: plugin!.license,
      tool: 'tools/tasks/sync-functional-tests.ts',
      noUpstreamTests: noUpstreamTests || undefined,
    },
    valid: cases.valid,
    invalid: cases.invalid,
  };
  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
}

function normalizeCapture(capture: Capture): { cases: Capture; dropped: number } {
  let dropped = 0;
  const valid: CapturedCase[] = [];
  const invalid: CapturedCase[] = [];

  for (const raw of capture.valid) {
    const normalized = normalizeCase(raw, false);
    if (normalized) {
      valid.push(normalized);
    } else {
      dropped += 1;
    }
  }
  for (const raw of capture.invalid) {
    const normalized = normalizeCase(raw, true);
    if (normalized) {
      invalid.push(normalized);
    } else {
      dropped += 1;
    }
  }

  return { cases: { valid, invalid }, dropped };
}

// Keep only JSON-serializable cases with a string `code`. A case carrying a
// function (e.g. a custom parser) does not survive the round-trip and is dropped.
function normalizeCase(raw: CapturedCase, isInvalid: boolean): CapturedCase | null {
  if (raw == null || typeof raw.code !== 'string') {
    return null;
  }

  const out: CapturedCase = { code: raw.code, typeAware: raw.typeAware };

  if (Array.isArray(raw.options) && raw.options.length > 0) {
    const options = safeClone(raw.options);
    if (options === undefined) {
      return null;
    }
    out.options = options as unknown[];
  }
  if (typeof raw.filename === 'string') {
    out.filename = raw.filename;
  }
  if (isInvalid) {
    // Prefer the snapshot's messageIds (authoritative, present even when the
    // test declared no inline `errors`); otherwise fall back to inline `errors`.
    // When neither is present the upstream `invalid()` call only asserts "at
    // least one error" — leave `errors` omitted so the harness checks that.
    if (Array.isArray(raw.snapshotMessageIds)) {
      out.errors = raw.snapshotMessageIds.map((messageId) => ({ messageId }));
    } else {
      const inline = normalizeErrors(raw.errors);
      const hasInline = typeof inline === 'number' || (Array.isArray(inline) && inline.length > 0);
      if (hasInline) {
        out.errors = inline;
      }
    }
    if (typeof raw.output === 'string') {
      out.output = raw.output;
    }
  }

  return out;
}

// Reduce each declared error to the fields we replay against: the messageId plus
// any position/data the upstream case asserted. A bare string is a messageId.
function normalizeErrors(errors: unknown): unknown {
  if (typeof errors === 'number') {
    return errors;
  }
  if (!Array.isArray(errors)) {
    return [];
  }
  return errors.map((error) => {
    if (typeof error === 'string') {
      return { messageId: error };
    }
    if (error && typeof error === 'object') {
      const e = error as Record<string, unknown>;
      const out: Record<string, unknown> = {};
      for (const key of ['messageId', 'message', 'line', 'column', 'endLine', 'endColumn']) {
        if (key in e) {
          out[key] = e[key];
        }
      }
      if (e.data && typeof e.data === 'object') {
        const data = safeClone(e.data);
        if (data !== undefined) {
          out.data = data;
        }
      }
      if (Array.isArray(e.suggestions)) {
        out.suggestionCount = e.suggestions.length;
      }
      return out;
    }
    return {};
  });
}

function safeClone(value: unknown): unknown {
  try {
    return JSON.parse(JSON.stringify(value));
  } catch {
    return undefined;
  }
}

// --- stubs -----------------------------------------------------------------

function registerStubHooks(): void {
  const stubSource: Record<string, string> = {
    vitest: vitestStub(),
    'rule-tester': ruleTesterStub(),
    dedent: dedentStub(),
    configs: configsStub(),
    immutability: immutabilityStub(),
    empty: 'export default {};',
  };

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'vitest') {
        return { url: 'stub:///vitest', shortCircuit: true };
      }
      if (specifier === 'eslint-vitest-rule-tester') {
        return { url: 'stub:///rule-tester', shortCircuit: true };
      }
      if (specifier === 'dedent') {
        return { url: 'stub:///dedent', shortCircuit: true };
      }
      if (specifier === 'is-immutable-type') {
        return { url: 'stub:///immutability', shortCircuit: true };
      }
      if (/(^|\/)utils\/configs$/.test(specifier) || specifier.endsWith('utils/configs')) {
        return { url: 'stub:///configs', shortCircuit: true };
      }
      if (specifier.startsWith('#/rules')) {
        const rule = specifier.slice('#/rules'.length).replace(/^\//, '') || 'index';
        return { url: `stub:///rulemod?name=${encodeURIComponent(rule)}`, shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url.startsWith('stub:///rulemod')) {
        const name = new URL(url).searchParams.get('name') ?? 'index';
        return {
          format: 'module',
          source:
            name === 'index'
              ? 'export const rules = {};'
              : `export const name = ${JSON.stringify(name)};\nexport const rule = {};`,
          shortCircuit: true,
        };
      }
      if (url.startsWith('stub:///')) {
        const key = url.slice('stub:///'.length);
        const source = stubSource[key];
        if (source !== undefined) {
          return { format: 'module', source, shortCircuit: true };
        }
      }
      return nextLoad(url, context);
    },
  });
}

function vitestStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'function state() { return globalThis[KEY]; }',
    'function callRow(fn, row) { return Array.isArray(row) ? fn(...row) : fn(row); }',
    'function fullName(name) { return [...state().stack, name].join(" > "); }',
    'export function describe(name, fn) {',
    '  state().stack.push(String(name));',
    '  try { if (typeof fn === "function") fn(); } finally { state().stack.pop(); }',
    '}',
    'describe.only = describe; describe.skip = function () {}; describe.todo = function () {};',
    'describe.each = (table) => (name, fn) => {',
    '  for (const row of table || []) {',
    '    state().stack.push(String(name));',
    '    try { callRow(fn, row); } finally { state().stack.pop(); }',
    '  }',
    '};',
    'export function it(name, fn) {',
    '  if (typeof fn === "function") state().collected.push({ name, fullName: fullName(name), fn });',
    '}',
    'it.only = it; it.skip = function () {}; it.todo = function () {};',
    'it.each = (table) => (name, fn) => {',
    '  for (const row of table || [])',
    '    state().collected.push({ name, fullName: fullName(name), fn: () => callRow(fn, row) });',
    '};',
    'export const test = it;',
    // `expect(value)` keeps `value` bound so `toMatchSnapshot()` can attach the
    // matching upstream snapshot (keyed `<full test name> <n>`) to the invalid
    // case the result came from. Every other matcher is a chainable no-op.
    'function chainFor(value) {',
    '  return new Proxy(function () {}, {',
    '    get(_t, prop) {',
    '      if (prop === "toMatchSnapshot" || prop === "toMatchInlineSnapshot") {',
    '        return function () {',
    '          const ct = state().currentTest;',
    '          if (ct) {',
    '            const ids = state().snapshotsByKey[ct.fullName + " " + ++ct.snapCounter];',
    '            if (ids && value && typeof value === "object" && value.__caseRef) {',
    '              value.__caseRef.snapshotMessageIds = ids;',
    '            }',
    '          }',
    '          return chainFor(value);',
    '        };',
    '      }',
    '      if (prop === "resolves" || prop === "rejects" || prop === "not") return chainFor(value);',
    '      return function () { return chainFor(value); };',
    '    },',
    '    apply() { return chainFor(value); },',
    '  });',
    '}',
    'export function expect(value) { return chainFor(value); }',
    'expect.any = function () { return {}; }; expect.objectContaining = function (o) { return o; };',
    'expect.stringContaining = function (s) { return s; }; expect.arrayContaining = function (a) { return a; };',
    'export function beforeAll() {} export function afterAll() {}',
    'export function beforeEach() {} export function afterEach() {}',
    'export function vi() {} export const vitest = {};',
    'export default { describe, it, test, expect, beforeAll, afterAll, beforeEach, afterEach };',
  ].join('\n');
}

function ruleTesterStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'function state() { return globalThis[KEY]; }',
    'function isTypeAware(configs) {',
    '  if (!configs) return false;',
    '  const list = Array.isArray(configs) ? configs : [configs];',
    '  return list.some((c) => c && c.__configKind === "ts");',
    '}',
    'function toCase(input, typeAware) {',
    '  const base = typeof input === "string" ? { code: input } : { ...(input || {}) };',
    '  base.typeAware = typeAware;',
    '  return base;',
    '}',
    'export function createRuleTester(opts) {',
    '  const typeAware = isTypeAware(opts && opts.configs);',
    '  const valid = async (input) => {',
    '    const items = Array.isArray(input) ? input : [input];',
    '    for (const item of items) state().capture.valid.push(toCase(item, typeAware));',
    '    return { result: { messages: [], output: null, fixed: false, steps: [] } };',
    '  };',
    '  const invalid = async (input) => {',
    '    const items = Array.isArray(input) ? input : [input];',
    '    let last = null;',
    '    for (const item of items) { last = toCase(item, typeAware); state().capture.invalid.push(last); }',
    '    return { result: { __caseRef: last, messages: [], output: null, fixed: false, steps: [] } };',
    '  };',
    '  return { valid, invalid, rule: opts && opts.rule, name: opts && opts.name };',
    '}',
    'export default { createRuleTester };',
  ].join('\n');
}

function configsStub(): string {
  return [
    'export const esLatestConfig = { __configKind: "es" };',
    'export const typescriptConfig = { __configKind: "ts" };',
    'export default { esLatestConfig, typescriptConfig };',
  ].join('\n');
}

// Reimplementation of the `dedent` package (the submodule does not install it):
// concatenate the tagged-template parts, then remove the minimum leading
// whitespace shared by all non-empty lines and trim the surrounding blank lines.
// (dedent's rarely-used `\`-line-continuation escape is not reproduced; the
// upstream functional test files do not rely on it.)
function dedentStub(): string {
  return [
    'function dedent(strings, ...values) {',
    '  const raw = typeof strings === "string" ? [strings] : (strings.raw || strings);',
    '  let result = "";',
    '  for (let i = 0; i < raw.length; i++) {',
    '    result += raw[i];',
    '    if (i < values.length) result += String(values[i]);',
    '  }',
    '  const lines = result.split("\\n");',
    '  let mindent = null;',
    '  for (const line of lines) {',
    '    const m = line.match(/^(\\s+)\\S/);',
    '    if (m) { const indent = m[1].length; mindent = mindent === null ? indent : Math.min(mindent, indent); }',
    '  }',
    '  const out = mindent === null || mindent === 0 ? lines : lines.map((l) => l.slice(mindent));',
    '  return out.join("\\n").trim();',
    '}',
    'export default dedent;',
  ].join('\n');
}

function immutabilityStub(): string {
  // Member access yields the member name as a string, so option values like
  // `Immutability.Immutable` serialize to "Immutable" (these cases are
  // type-aware and skipped by the replay harness anyway).
  return [
    'export const Immutability = new Proxy({}, { get(_t, prop) { return String(prop); } });',
    'export default { Immutability };',
  ].join('\n');
}
