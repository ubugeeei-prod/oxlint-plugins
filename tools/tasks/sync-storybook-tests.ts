// Captures the upstream eslint-plugin-storybook test suite straight from the
// vendored submodule and writes it to committed JSON fixtures, so our Vitest
// suite replays the real upstream cases and tracks behaviour as the submodule is
// bumped (oxc-style test syncing). Mirrors tools/tasks/sync-functional-tests.ts.
//
// Upstream drives cases through a classic *declarative* RuleTester: each
// `<rule>.test.ts` calls `ruleTester.run('<rule>', rule, { valid, invalid })`
// once at module top level (no async `it()` blocks, no snapshots). The shared
// `../test-utils.ts` wraps `@typescript-eslint/rule-tester` and injects a default
// `filename: 'MyComponent.stories.js'` into every case. We register synchronous
// module hooks (`module.registerHooks`) that stub:
//   - `../test-utils.ts`      -> a capturing RuleTester whose `run(name, _rule,
//                                {valid, invalid})` records the arrays (applying
//                                the same default filename as upstream) instead of
//                                executing ESLint.
//   - `./<rule>.ts` / `../rules/<rule>.ts` -> `export default {}`; the real rule
//                                module pulls Storybook deps the shallow submodule
//                                has not installed, and we only need the cases.
//   - `ts-dedent`             -> a faithful reimplementation (not installed).
//   - `@typescript-eslint/utils` -> an `AST_NODE_TYPES` proxy yielding member
//                                names (some tests tag errors with `type:
//                                AST_NODE_TYPES.X`, which we do not replay).
//   - `vitest`                -> a `vi` mock stub (one test calls `vi.mock`/`vi.fn`
//                                to fake `fs`); the calls must not crash.
//
// The upstream `.ts` test files run directly through Node's native type stripping
// (Node >= 24). Cases carrying non-serialisable values (functions) are dropped and
// counted (no silent truncation).
//
// Re-run with `pnpm run port:tests:storybook`, then `vp fmt` the generated
// fixtures (the JSON is emitted with `JSON.stringify`, which always expands short
// arrays the formatter collapses onto one line).

import { registerHooks } from 'node:module';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    packageSubdir?: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

type RawSuggestion = { messageId?: string; output?: string };
type RawError = {
  messageId?: string;
  data?: Record<string, unknown>;
  suggestions?: RawSuggestion[];
};
type RawCase = {
  code?: string;
  output?: string | null;
  options?: unknown[];
  filename?: string;
  errors?: unknown;
};
type Capture = { valid: RawCase[]; invalid: RawCase[] };

type NormSuggestion = { messageId: string; output?: string };
type NormError = {
  messageId: string;
  data?: Record<string, unknown>;
  suggestions?: NormSuggestion[];
};
type NormCase = {
  code: string;
  options?: unknown[];
  filename: string;
  output?: string;
  errors?: NormError[];
};

type SyncGlobal = { captures: Record<string, Capture> };

const ROOT = process.cwd();
const SYNC_KEY = '__storybookSyncState__';
const DEFAULT_FILENAME = 'MyComponent.stories.js';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-storybook');
if (!plugin) {
  throw new Error('eslint-plugin-storybook is not registered in tools/port-targets.json');
}

const PKG = join(ROOT, plugin.submodule, plugin.packageSubdir ?? '.');
const SRC_RULES_DIR = join(PKG, 'src', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'storybook', 'test', 'fixtures');
const REF = plugin.pinnedRef ?? `v${plugin.baselineVersion}`;

if (!existsSync(SRC_RULES_DIR)) {
  throw new Error(
    `Upstream sources not found under ${PKG}. Run: git submodule update --init ${plugin.submodule}`,
  );
}

mkdirSync(FIXTURES_DIR, { recursive: true });

const state: SyncGlobal = { captures: {} };
(globalThis as Record<string, unknown>)[SYNC_KEY] = state;

registerStubHooks();

const ruleNames = readdirSync(SRC_RULES_DIR)
  .filter((file) => file.endsWith('.test.ts'))
  .map((file) => file.slice(0, -'.test.ts'.length))
  .sort();

const summary: string[] = [];

for (const rule of ruleNames) {
  const testFile = join(SRC_RULES_DIR, `${rule}.test.ts`);
  await import(pathToFileURL(testFile).href);

  const captured = state.captures[rule] ?? { valid: [], invalid: [] };
  const { cases, dropped } = normalizeCapture(captured);
  writeFixture(rule, `src/rules/${rule}.test.ts`, cases);

  summary.push(
    `${rule}: ${cases.valid.length} valid, ${cases.invalid.length} invalid` +
      `${dropped > 0 ? `, ${dropped} dropped` : ''}`,
  );
}

console.log(`Synced eslint-plugin-storybook fixtures from upstream ${REF}:`);
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ---------------------------------------------------------------

function writeFixture(rule: string, sourceFile: string, cases: Capture | NormalizedCapture): void {
  const fixture = {
    __generated: {
      source: plugin!.npm,
      ref: REF,
      sourceFile,
      license: plugin!.license,
      tool: 'tools/tasks/sync-storybook-tests.ts',
    },
    valid: cases.valid,
    invalid: cases.invalid,
  };
  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
}

type NormalizedCapture = { valid: NormCase[]; invalid: NormCase[] };

function normalizeCapture(capture: Capture): { cases: NormalizedCapture; dropped: number } {
  let dropped = 0;
  const valid: NormCase[] = [];
  const invalid: NormCase[] = [];

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

// Keep only JSON-serialisable cases with a string `code`. A case carrying a
// function (e.g. a custom parser) does not survive the round-trip and is dropped.
function normalizeCase(raw: RawCase, isInvalid: boolean): NormCase | null {
  if (raw == null || typeof raw.code !== 'string') {
    return null;
  }

  const out: NormCase = {
    code: raw.code,
    filename: typeof raw.filename === 'string' ? raw.filename : DEFAULT_FILENAME,
  };

  if (Array.isArray(raw.options) && raw.options.length > 0) {
    const options = safeClone(raw.options);
    if (options === undefined) {
      return null;
    }
    out.options = options as unknown[];
  }

  if (isInvalid) {
    const errors = normalizeErrors(raw.errors);
    if (errors.length > 0) {
      out.errors = errors;
    }
    if (typeof raw.output === 'string') {
      out.output = raw.output;
    }
  }

  return out;
}

// Reduce each declared error to the fields we replay against: the messageId, the
// asserted `data` placeholders, and the suggestion outputs (a `<messageId,
// output>` pair). Position/`type` fields are not replayed (the Rust port reports
// faithful messageIds but may flag different spans).
function normalizeErrors(errors: unknown): NormError[] {
  if (!Array.isArray(errors)) {
    return [];
  }
  const out: NormError[] = [];
  for (const error of errors) {
    if (typeof error === 'string') {
      out.push({ messageId: error });
      continue;
    }
    if (!error || typeof error !== 'object') {
      continue;
    }
    const e = error as RawError;
    if (typeof e.messageId !== 'string') {
      continue;
    }
    const norm: NormError = { messageId: e.messageId };
    if (e.data && typeof e.data === 'object') {
      const data = safeClone(e.data);
      if (data !== undefined) {
        norm.data = data as Record<string, unknown>;
      }
    }
    if (Array.isArray(e.suggestions)) {
      const suggestions: NormSuggestion[] = [];
      for (const suggestion of e.suggestions) {
        if (
          suggestion &&
          typeof suggestion === 'object' &&
          typeof suggestion.messageId === 'string'
        ) {
          const s: NormSuggestion = { messageId: suggestion.messageId };
          if (typeof suggestion.output === 'string') {
            s.output = suggestion.output;
          }
          suggestions.push(s);
        }
      }
      norm.suggestions = suggestions;
    }
    out.push(norm);
  }
  return out;
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
    'test-utils': testUtilsStub(),
    rule: 'export default {};',
    dedent: dedentStub(),
    'ts-utils': tsUtilsStub(),
    vitest: vitestStub(),
  };

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'ts-dedent') {
        return { url: 'stub:///dedent', shortCircuit: true };
      }
      if (specifier === 'vitest') {
        return { url: 'stub:///vitest', shortCircuit: true };
      }
      if (specifier === '@typescript-eslint/utils') {
        return { url: 'stub:///ts-utils', shortCircuit: true };
      }
      if (/(^|\/)test-utils\.ts$/.test(specifier)) {
        return { url: 'stub:///test-utils', shortCircuit: true };
      }
      // Relative rule imports: `./<rule>.ts` or `../rules/<rule>.ts` (but not the
      // test file itself or the shared test-utils).
      if (
        /^\.{1,2}\//.test(specifier) &&
        specifier.endsWith('.ts') &&
        !specifier.endsWith('.test.ts')
      ) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
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

// Capturing RuleTester matching upstream `src/test-utils.ts`: every case is
// merged onto the default `{ filename: 'MyComponent.stories.js' }`, string cases
// become `{ filename, code }`.
function testUtilsStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    `const DEFAULT = { filename: ${JSON.stringify(DEFAULT_FILENAME)} };`,
    'function toValid(input) {',
    '  if (typeof input === "string") return { ...DEFAULT, code: input };',
    '  return { ...DEFAULT, ...(input || {}) };',
    '}',
    'function toInvalid(input) { return { ...DEFAULT, ...(input || {}) }; }',
    'const ruleTester = {',
    '  run(name, _rule, tests) {',
    '    const valid = (tests && tests.valid ? tests.valid : []).map(toValid);',
    '    const invalid = (tests && tests.invalid ? tests.invalid : []).map(toInvalid);',
    '    globalThis[KEY].captures[name] = { valid, invalid };',
    '  },',
    '};',
    'export default ruleTester;',
  ].join('\n');
}

function tsUtilsStub(): string {
  return [
    'export const AST_NODE_TYPES = new Proxy({}, { get(_t, prop) { return String(prop); } });',
    'export const ESLintUtils = {};',
    'export const TSESLint = {};',
    'export default { AST_NODE_TYPES };',
  ].join('\n');
}

function vitestStub(): string {
  return [
    'function noop() {}',
    'function fn() { const f = () => {}; f.mockReturnValue = () => f; f.mockImplementation = () => f; return f; }',
    'const vi = {',
    '  fn,',
    '  mock: noop,',
    '  mocked: (x) => x,',
    '  spyOn: () => ({ mockReturnValue: () => {}, mockImplementation: () => {} }),',
    '  importActual: async () => ({}),',
    '  resetAllMocks: noop,',
    '  restoreAllMocks: noop,',
    '};',
    'export { vi };',
    'export function describe(_n, fn) { if (typeof fn === "function") fn(); }',
    'export function it() {}',
    'export const test = it;',
    'export function expect() { return new Proxy(function () {}, { get() { return () => {}; }, apply() {} }); }',
    'export function beforeAll() {} export function afterAll() {}',
    'export function beforeEach() {} export function afterEach() {}',
    'export default { vi, describe, it, test, expect };',
  ].join('\n');
}

// Reimplementation of `ts-dedent` (the submodule does not install it): concatenate
// the tagged-template parts, then remove the minimum leading whitespace shared by
// all non-empty lines and trim surrounding blank lines.
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
    'export { dedent };',
    'export default dedent;',
  ].join('\n');
}
