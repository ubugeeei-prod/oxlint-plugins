// Captures the upstream @unocss/eslint-plugin test suite straight from the
// vendored submodule and writes it to committed JSON fixtures, so our Vitest
// suite replays the real upstream cases and tracks behaviour as the submodule
// is bumped (oxc-style test syncing).
//
// Upstream drives cases through a *declarative* `run()` API from
// `eslint-vitest-rule-tester`. Each test file calls `run({ name, rule,
// languageOptions?, settings?, valid, invalid })` once or more at module top
// level. We register synchronous module hooks (`module.registerHooks`) that:
//
//   - stub `eslint-vitest-rule-tester` → a capturing `run()` that records each
//     block keyed by `name`, plus a faithful `$` / `html` unindent tag and a
//     no-op `RuleTester` class.
//   - stub `vue-eslint-parser`          → marker object (default export).
//   - stub `svelte-eslint-parser`       → marker object (default export).
//   - stub `vitest`                     → `{ expect }` that captures inline
//     snapshots passed to `toMatchInlineSnapshot`.
//   - stub rule modules (`./order`, etc.)      → `{ default: {} }`.
//   - stub `@typescript-eslint/types`   → `{ AST_TOKEN_TYPES: {} }`.
//   - pass through `node:url`           → real module.
//
// Parser detection per block:
//   `languageOptions.parser === vueMarker`                → 'vue'
//   `languageOptions.parser === svelteMarker` or `.default` → 'svelte'
//   `languageOptions.parserOptions?.ecmaFeatures?.jsx`    → 'jsx'
//   otherwise                                             → 'js'
//
// Snapshot capture: `toMatchInlineSnapshot` is called inside the `output`
// function of invalid cases. The vitest stub writes the snapshot argument to
// `globalThis[SYNC_KEY].lastSnapshot` before returning; the normalise pass
// reads it immediately after calling the output function.
//
// Snapshot parsing: after trim, if the value starts and ends with `"` we
// strip those outer quotes and unescape `\"` → `"` inside to recover the
// actual fixed output string (vitest serialises strings that way).
//
// enforce-class-compile uses the `$` tag for BOTH `code` and `output`
// fields of invalid cases — `output` is a plain string there, not a function.
//
// Re-run with `pnpm run port:tests:unocss`, then `vp fmt` the generated
// fixtures (the JSON is emitted with `JSON.stringify`, which always expands
// short arrays the formatter collapses onto one line).

import { registerHooks } from 'node:module';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
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

type Parser = 'vue' | 'svelte' | 'jsx' | 'js';

type RawCase = {
  code: string;
  options?: unknown[];
  filename?: string;
  output?: string | null | ((o: unknown) => unknown);
  errors?: unknown[];
  parser: Parser;
};

type NormCase = {
  code: string;
  options?: unknown[];
  filename?: string;
  output?: string | null;
  errors?: unknown[];
  parser: Parser;
};

type Capture = { valid: RawCase[]; invalid: RawCase[] };

type BlockCapture = {
  parser: Parser;
  valid: NormCase[];
  invalid: NormCase[];
};

type FixtureBlocks = Record<string, BlockCapture>;

type SyncGlobal = {
  captures: Record<string, Capture>;
  vueMarker: object;
  svelteMarker: object;
  lastSnapshot: string | undefined;
};

const ROOT = process.cwd();
const SYNC_KEY = '__unocssSyncState__';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'unocss-eslint-plugin');
if (!plugin) {
  throw new Error('unocss-eslint-plugin is not registered in tools/port-targets.json');
}

const PKG = join(ROOT, plugin.submodule, plugin.packageSubdir ?? '.');
const SRC_RULES_DIR = join(PKG, 'src', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'unocss', 'test', 'fixtures');
const REF = plugin.pinnedRef ?? `v${plugin.baselineVersion}`;

if (!existsSync(SRC_RULES_DIR)) {
  throw new Error(
    `Upstream sources not found under ${PKG}. Run: git submodule update --init ${plugin.submodule}`,
  );
}

mkdirSync(FIXTURES_DIR, { recursive: true });

// Shared markers for parser detection — the stubs export these same objects.
const vueMarker: object = { __parser: 'vue' };
const svelteMarker: object = { __parser: 'svelte' };

const state: SyncGlobal = {
  captures: {},
  vueMarker,
  svelteMarker,
  lastSnapshot: undefined,
};
(globalThis as Record<string, unknown>)[SYNC_KEY] = state;

registerStubHooks();

// Rule files that have a corresponding .test.ts in src/rules/.
const TEST_RULES: Array<{ rule: string; testFile: string }> = [
  { rule: 'order', testFile: 'order.test.ts' },
  { rule: 'blocklist', testFile: 'blocklist.test.ts' },
  { rule: 'enforce-class-compile', testFile: 'enforce-class-compile.test.ts' },
];

// order-attributify has no test file in the upstream submodule.
const NO_TEST_RULES: string[] = ['order-attributify'];

const summary: string[] = [];

for (const { rule, testFile } of TEST_RULES) {
  const testFilePath = join(SRC_RULES_DIR, testFile);
  if (!existsSync(testFilePath)) {
    writeFixture(rule, [], {}, true);
    summary.push(`${rule}: no upstream test file (empty fixture written)`);
    continue;
  }

  // Reset captures for this file import.
  state.captures = {};

  await import(pathToFileURL(testFilePath).href);

  const blocks: Record<string, Capture> = { ...state.captures };
  const { cases, dropped } = normalizeBlocks(blocks);
  writeFixture(rule, Object.keys(blocks), cases, false);

  let total = 0;
  for (const blk of Object.values(cases.blocks)) {
    total += blk.valid.length + blk.invalid.length;
  }
  summary.push(
    `${rule}: ${total} cases across ${Object.keys(blocks).length} block(s)` +
      `${dropped > 0 ? `, ${dropped} dropped` : ''}`,
  );
}

for (const rule of NO_TEST_RULES) {
  writeFixture(rule, [], {}, true);
  summary.push(`${rule}: no upstream test file (empty fixture written)`);
}

console.log(`Synced @unocss/eslint-plugin fixtures from upstream ${REF}:`);
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ----------------------------------------------------------------

function normalizeBlocks(blocks: Record<string, Capture>): {
  cases: { blocks: FixtureBlocks };
  dropped: number;
} {
  let dropped = 0;
  const result: FixtureBlocks = {};

  for (const [name, capture] of Object.entries(blocks)) {
    const valid: NormCase[] = [];
    const invalid: NormCase[] = [];

    // Determine the parser from the first case in the block (all cases in a
    // single run() call share the same languageOptions).
    const allCases = [...capture.valid, ...capture.invalid];
    const parser: Parser = allCases.length > 0 ? allCases[0].parser : 'js';

    for (const raw of capture.valid) {
      const norm = normalizeCase(raw, false);
      if (norm) {
        valid.push(norm);
      } else {
        dropped++;
      }
    }
    for (const raw of capture.invalid) {
      const norm = normalizeCase(raw, true);
      if (norm) {
        invalid.push(norm);
      } else {
        dropped++;
      }
    }

    result[name] = { parser, valid, invalid };
  }

  return { cases: { blocks: result }, dropped };
}

function normalizeCase(raw: RawCase, isInvalid: boolean): NormCase | null {
  if (!raw || typeof raw.code !== 'string') {
    return null;
  }

  const out: NormCase = { code: raw.code, parser: raw.parser };

  if (Array.isArray(raw.options) && raw.options.length > 0) {
    const opts = safeClone(raw.options);
    if (opts === undefined) {
      return null;
    }
    out.options = opts as unknown[];
  }

  if (typeof raw.filename === 'string') {
    out.filename = raw.filename;
  }

  if (isInvalid) {
    // `output` can be:
    //   - a string  → direct output (enforce-class-compile uses `$` tagged strings)
    //   - null      → explicitly no output
    //   - a function → calls expect(output).toMatchInlineSnapshot(snap);
    //     the vitest stub captures the `snap` argument into state.lastSnapshot.
    if (typeof raw.output === 'function') {
      state.lastSnapshot = undefined;
      // Call with a dummy value; the function body is:
      //   output => expect(output).toMatchInlineSnapshot(`"..."`)
      // Our vitest stub's expect().toMatchInlineSnapshot captures the snap argument.
      raw.output('__sentinel__');
      const snap = state.lastSnapshot;
      state.lastSnapshot = undefined;
      if (typeof snap === 'string') {
        out.output = parseSnapshot(snap);
      }
      // If no snapshot was captured, leave output undefined.
    } else if (typeof raw.output === 'string') {
      out.output = raw.output;
    } else if (raw.output === null) {
      out.output = null;
    }

    if (Array.isArray(raw.errors)) {
      const errors = normalizeErrors(raw.errors);
      if (errors.length > 0) {
        out.errors = errors;
      }
    }
  }

  return out;
}

// Parse a vitest inline snapshot string to the actual expected output.
// Template literal form: "\n  \"<div>...</div>\"\n" → trim → "\"...\"" → strip quotes → unescape
// String literal form:   "   \"...\"   " → trim → "\"...\"" → strip quotes → unescape
// If no outer quotes: return trimmed value as-is.
function parseSnapshot(snap: string): string {
  const trimmed = snap.trim();
  if (trimmed.startsWith('"') && trimmed.endsWith('"') && trimmed.length >= 2) {
    // Strip outer double-quote delimiters and unescape \" → "
    return trimmed.slice(1, -1).replace(/\\"/g, '"');
  }
  return trimmed;
}

function normalizeErrors(errors: unknown[]): unknown[] {
  return errors
    .map((error) => {
      if (typeof error === 'string') {
        return { messageId: error };
      }
      if (!error || typeof error !== 'object') {
        return null;
      }
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
      if (Object.keys(out).length === 0) {
        return null;
      }
      return out;
    })
    .filter(Boolean);
}

function writeFixture(
  rule: string,
  blockNames: string[],
  cases: { blocks?: FixtureBlocks },
  noUpstreamTests: boolean,
): void {
  const sourceFiles =
    blockNames.length > 0 ? [...new Set(blockNames.map(() => `src/rules/${rule}.test.ts`))] : [];
  const fixture: Record<string, unknown> = {
    __generated: {
      source: plugin!.npm,
      ref: REF,
      sourceFiles,
      license: plugin!.license,
      tool: 'tools/tasks/sync-unocss-tests.ts',
      noUpstreamTests: noUpstreamTests || undefined,
    },
    blocks: cases.blocks ?? {},
  };
  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
}

function safeClone(value: unknown): unknown {
  try {
    return JSON.parse(JSON.stringify(value));
  } catch {
    return undefined;
  }
}

// --- stubs ------------------------------------------------------------------

function registerStubHooks(): void {
  const stubSource: Record<string, string> = {
    'eslint-vitest-rule-tester': ruleTesterStub(),
    'vue-eslint-parser': vueParserStub(),
    'svelte-eslint-parser': svelteParserStub(),
    vitest: vitestStub(),
    rule: 'export default {};',
    'ts-types': 'export const AST_TOKEN_TYPES = {};',
  };

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'eslint-vitest-rule-tester') {
        return { url: 'stub:///eslint-vitest-rule-tester', shortCircuit: true };
      }
      if (specifier === 'vue-eslint-parser') {
        return { url: 'stub:///vue-eslint-parser', shortCircuit: true };
      }
      if (specifier === 'svelte-eslint-parser') {
        return { url: 'stub:///svelte-eslint-parser', shortCircuit: true };
      }
      if (specifier === 'vitest') {
        return { url: 'stub:///vitest', shortCircuit: true };
      }
      if (specifier === '@typescript-eslint/types') {
        return { url: 'stub:///ts-types', shortCircuit: true };
      }
      // Relative rule imports (e.g. './order', './blocklist', './enforce-class-compile')
      if (
        /^\.\//.test(specifier) &&
        !specifier.endsWith('.test.ts') &&
        !specifier.endsWith('.test.js')
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

// The capturing `run()` stub plus the `$` / `html` unindent tag.
// `$` is exported as `unindent` in eslint-vitest-rule-tester, and its real
// implementation uses the COOKED string value (str[0], not strings.raw[0]).
// Using .raw would incorrectly keep `\`` as `\\`` instead of the literal backtick.
// See: eslint-vitest-rule-tester/dist/index.mjs:
//   const lines = (typeof str === "string" ? str : str[0]).split("\n");
function ruleTesterStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'function state() { return globalThis[KEY]; }',
    'export function $(strings, ...values) {',
    // Use cooked strings[0] (the TemplateStringsArray object), NOT strings.raw[0].
    // This preserves literal backticks (from \` in template literals) correctly.
    '  const str = typeof strings === "string" ? strings : strings[0];',
    '  const lines = str.split("\\n");',
    '  const nonBlank = lines.filter((l) => l.trim().length > 0);',
    '  const commonIndent = nonBlank.reduce((min, l) => {',
    '    const m = l.match(/^(\\s*)/);',
    '    return Math.min(min, m ? m[1].length : 0);',
    '  }, Infinity);',
    '  const indent = Number.isFinite(commonIndent) ? commonIndent : 0;',
    '  let head = 0;',
    '  while (head < lines.length && lines[head].trim() === "") head++;',
    '  let tail = lines.length;',
    '  while (tail > head && lines[tail - 1].trim() === "") tail--;',
    '  return lines.slice(head, tail).map((l) => l.slice(indent)).join("\\n");',
    '}',
    'export { $ as html };',
    'export class RuleTester {}',
    'export function run(config) {',
    '  const captures = state().captures;',
    '  const name = config && config.name ? config.name : "unknown";',
    '  const lang = config && config.languageOptions;',
    '  const vue = state().vueMarker;',
    '  const svelte = state().svelteMarker;',
    '  let parser = "js";',
    '  if (lang && lang.parser) {',
    '    if (lang.parser === vue || (lang.parser && lang.parser.default === vue)) parser = "vue";',
    '    else if (lang.parser === svelte || (lang.parser && lang.parser.default === svelte)) parser = "svelte";',
    '  } else if (lang && lang.parserOptions && lang.parserOptions.ecmaFeatures && lang.parserOptions.ecmaFeatures.jsx) {',
    '    parser = "jsx";',
    '  }',
    '  function toCase(input) {',
    '    if (typeof input === "string") return { code: input, parser };',
    '    return { ...(input || {}), parser };',
    '  }',
    '  const valid = (config && config.valid ? config.valid : []).map(toCase);',
    '  const invalid = (config && config.invalid ? config.invalid : []).map(toCase);',
    '  captures[name] = { valid, invalid };',
    '}',
    'export default { run, $, html: $, RuleTester };',
  ].join('\n');
}

function vueParserStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'const marker = globalThis[KEY].vueMarker;',
    'export default marker;',
    'export { marker as parse };',
  ].join('\n');
}

function svelteParserStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'const marker = globalThis[KEY].svelteMarker;',
    'export default marker;',
    'export { marker as parse };',
  ].join('\n');
}

// The vitest stub captures the snapshot argument passed to toMatchInlineSnapshot
// by writing it to `globalThis[SYNC_KEY].lastSnapshot`. The normalise pass reads
// this immediately after calling the output function.
function vitestStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'function state() { return globalThis[KEY]; }',
    'function chain() {',
    '  return new Proxy(function () {}, {',
    '    get(_t, prop) {',
    '      if (prop === "toMatchInlineSnapshot") {',
    '        return function (snap) { state().lastSnapshot = snap; };',
    '      }',
    '      return function () { return chain(); };',
    '    },',
    '    apply() { return chain(); },',
    '  });',
    '}',
    'export function expect(_value) { return chain(); }',
    'export function describe(_n, fn) { if (typeof fn === "function") fn(); }',
    'export function it() {}',
    'export const test = it;',
    'export function beforeAll() {} export function afterAll() {}',
    'export function beforeEach() {} export function afterEach() {}',
    'export default { expect, describe, it, test };',
  ].join('\n');
}
