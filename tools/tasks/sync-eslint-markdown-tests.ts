// Captures the upstream @eslint/markdown test suite straight from the vendored
// submodule and writes it to committed JSON fixtures, so our Vitest suite
// replays the real upstream cases and tracks behaviour as the submodule is
// bumped (oxc-style test syncing). Mirrors tools/tasks/sync-storybook-tests.ts
// and tools/tasks/sync-eslint-json-tests.ts.
//
// Upstream drives cases through ESLint's *declarative* `RuleTester`: each
// `tests/rules/<rule>.test.js` calls `ruleTester.run('<rule>', rule, { valid,
// invalid })` once at module top level. The files are ESM (`import`), use the
// `dedent` tagged template to author multi-line Markdown, and configure the
// RuleTester with a Markdown `language` (`markdown/commonmark` or
// `markdown/gfm`). A handful also `import { Linter }` for an ad-hoc perf check
// wrapped in a top-level mocha `it()`.
//
// We register synchronous module hooks (`module.registerHooks`) and stub:
//   - `eslint`                -> a capturing `RuleTester` whose `run(name, _rule,
//                                tests)` records the `valid`/`invalid` arrays
//                                (plus the constructor's `language`) instead of
//                                executing ESLint, and a no-op `Linter`.
//   - `dedent`                -> a verbatim reimplementation of dedent v1.7.1
//                                (the shallow submodule does not install it).
//   - `../../src/index.js` and `../../src/rules/<rule>.js` -> `export default
//                                {}`; the real modules pull deps the submodule
//                                has not installed, and we only need the cases.
// Global mocha hooks (`it`/`describe`/...) are stubbed as no-ops so the trailing
// `it()` perf blocks in a few files do not throw at import time.
//
// The upstream `.js` test files are ESM, so we drive them with dynamic
// `import()`; the stubs are emitted as real `.mjs` files in a temp dir and the
// resolve hook points bare/relative specifiers at them (cleaner than escaping
// dedent's regex-heavy source into an inline string).
//
// Cases carrying non-serialisable values (functions) are dropped and counted
// (no silent truncation). Re-run with `pnpm run port:tests:eslint-markdown`, then
// `vp fmt` the generated fixtures (the JSON is emitted with `JSON.stringify`,
// which always expands short arrays the formatter collapses onto one line).

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
    pinnedRef?: string;
    license: string;
  }>;
};

type RawCase =
  | string
  | {
      code?: string;
      output?: string | null;
      options?: unknown[];
      filename?: string;
      only?: boolean;
      language?: string;
      languageOptions?: Record<string, unknown>;
      errors?: unknown;
    };
type Capture = { language?: string; valid: RawCase[]; invalid: RawCase[] };

type NormError = {
  messageId?: string;
  message?: string;
  data?: Record<string, unknown>;
  line?: number;
  column?: number;
  endLine?: number;
  endColumn?: number;
};
type NormCase = {
  code: string;
  options?: unknown[];
  filename?: string;
  language?: string;
  languageOptions?: Record<string, unknown>;
  output?: string | null;
  errors?: NormError[] | number;
};
type NormalizedCapture = { valid: NormCase[]; invalid: NormCase[] };

type SyncGlobal = { captures: Record<string, Capture> };

const ROOT = process.cwd();
const HERE = dirname(fileURLToPath(import.meta.url));
const SYNC_KEY = '__eslintMarkdownSyncState__';
const DEDENT_STUB = pathToFileURL(join(HERE, 'markdown-sync-dedent.mjs')).href;

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-markdown');
if (!plugin) {
  throw new Error('eslint-markdown is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const TESTS_DIR = join(SUBMODULE, 'tests', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'eslint-markdown', 'test', 'fixtures');
const REF = plugin.pinnedRef ?? `v${plugin.baselineVersion}`;

if (!existsSync(TESTS_DIR)) {
  throw new Error(
    `Upstream tests not found at ${TESTS_DIR}. Run: git submodule update --init --depth 1 ${plugin.submodule}`,
  );
}

// Only generate fixtures for rules we actually ship, so the replay harness never
// references a rule that is not ported.
const ruleNames = getPortedRules();

mkdirSync(FIXTURES_DIR, { recursive: true });

const state: SyncGlobal = { captures: {} };
(globalThis as Record<string, unknown>)[SYNC_KEY] = state;

// Mocha globals used at the top level by a few perf-check `it()` blocks. No-op
// them so importing the test module never throws.
for (const name of ['describe', 'it', 'before', 'after', 'beforeEach', 'afterEach']) {
  (globalThis as Record<string, unknown>)[name] = () => {};
}

const tempDir = mkdtempSync(join(tmpdir(), 'eslint-markdown-sync-'));
const stubUrls = writeStubs(tempDir);
registerStubHooks(stubUrls);

const summary: string[] = [];
for (const rule of ruleNames) {
  const testFile = join(TESTS_DIR, `${rule}.test.js`);
  if (!existsSync(testFile)) {
    throw new Error(`Upstream test file missing for rule "${rule}": ${testFile}`);
  }

  await import(pathToFileURL(testFile).href);

  const captured = state.captures[rule];
  if (!captured) {
    throw new Error(`No RuleTester.run("${rule}") call captured from ${testFile}`);
  }

  const { cases, dropped } = normalizeCapture(captured);
  writeFixture(rule, `tests/rules/${rule}.test.js`, captured.language, cases);
  summary.push(
    `${rule}: ${cases.valid.length} valid, ${cases.invalid.length} invalid` +
      `${dropped > 0 ? `, ${dropped} dropped` : ''}`,
  );
}

rmSync(tempDir, { recursive: true, force: true });

console.log(`Synced @eslint/markdown fixtures from upstream ${REF}:`);
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ---------------------------------------------------------------

function getPortedRules(): string[] {
  const lib = readFileSync(join(ROOT, 'crates', 'eslint_markdown', 'src', 'lib.rs'), 'utf8');
  const block = /pub const RULE_NAMES:[^=]*=\s*\[([\s\S]*?)\]/.exec(lib);
  if (!block) {
    throw new Error('Could not read RULE_NAMES from crates/eslint_markdown/src/lib.rs');
  }
  const rules = [...block[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  if (rules.length === 0) {
    throw new Error('No rules found in RULE_NAMES.');
  }
  return rules.sort();
}

function writeFixture(
  rule: string,
  sourceFile: string,
  language: string | undefined,
  cases: NormalizedCapture,
): void {
  const fixture = {
    __generated: {
      source: plugin!.npm,
      ref: REF,
      sourceFile,
      license: plugin!.license,
      tool: 'tools/tasks/sync-eslint-markdown-tests.ts',
      language: language ?? null,
    },
    valid: cases.valid,
    invalid: cases.invalid,
  };
  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
}

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
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (value == null || typeof value !== 'object' || typeof value.code !== 'string') {
    return null;
  }

  const out: NormCase = { code: value.code };

  if (Array.isArray(value.options) && value.options.length > 0) {
    const options = safeClone(value.options);
    if (options === undefined) {
      return null;
    }
    out.options = options as unknown[];
  }
  if (typeof value.filename === 'string') {
    out.filename = value.filename;
  }
  // A per-case `language` override (the suite-level language lives in __generated).
  if (typeof value.language === 'string') {
    out.language = value.language;
  }
  if (value.languageOptions && typeof value.languageOptions === 'object') {
    const languageOptions = safeClone(value.languageOptions);
    if (languageOptions !== undefined) {
      out.languageOptions = languageOptions as Record<string, unknown>;
    }
  }

  if (isInvalid) {
    const errors = normalizeErrors(value.errors);
    if (errors !== undefined) {
      out.errors = errors;
    }
    if (typeof value.output === 'string' || value.output === null) {
      out.output = value.output;
    }
  }

  return out;
}

// Reduce each declared error to the fields we replay against: messageId (or
// message), the asserted `data` placeholders, and the 1-indexed location. A
// numeric `errors` is an expected count; a string error is a message assertion.
function normalizeErrors(errors: unknown): NormError[] | number | undefined {
  if (typeof errors === 'number') {
    return errors;
  }
  if (!Array.isArray(errors)) {
    return undefined;
  }
  const out: NormError[] = [];
  for (const error of errors) {
    if (typeof error === 'string') {
      out.push({ message: error });
      continue;
    }
    if (!error || typeof error !== 'object') {
      continue;
    }
    const e = error as Record<string, unknown>;
    const norm: NormError = {};
    if (typeof e.messageId === 'string') {
      norm.messageId = e.messageId;
    }
    if (typeof e.message === 'string') {
      norm.message = e.message;
    }
    if (e.data && typeof e.data === 'object') {
      const data = safeClone(e.data);
      if (data !== undefined) {
        norm.data = data as Record<string, unknown>;
      }
    }
    for (const key of ['line', 'column', 'endLine', 'endColumn'] as const) {
      if (typeof e[key] === 'number') {
        norm[key] = e[key] as number;
      }
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

function writeStubs(dir: string): { eslint: string; noop: string } {
  const eslintFile = join(dir, 'eslint.mjs');
  const noopFile = join(dir, 'noop.mjs');
  writeFileSync(eslintFile, eslintStub());
  writeFileSync(noopFile, 'export default {};\n');
  return {
    eslint: pathToFileURL(eslintFile).href,
    noop: pathToFileURL(noopFile).href,
  };
}

function registerStubHooks(stubs: { eslint: string; noop: string }): void {
  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'eslint') {
        return { url: stubs.eslint, shortCircuit: true };
      }
      if (specifier === 'dedent') {
        return { url: DEDENT_STUB, shortCircuit: true };
      }
      // The Markdown plugin entry and the rule-under-test modules: stub both.
      if (
        /(^|\/)src\/index\.js$/.test(specifier) ||
        /(^|\/)src\/rules\/[^/]+\.js$/.test(specifier)
      ) {
        return { url: stubs.noop, shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
  });
}

// Capturing RuleTester: records the `valid`/`invalid` arrays and the configured
// Markdown `language` keyed on the rule name, on the shared global so the value
// crosses the hook-loaded module boundary. `Linter` is a no-op (a few files use
// it for an ad-hoc perf check we do not replay).
function eslintStub(): string {
  return [
    `const KEY = ${JSON.stringify(SYNC_KEY)};`,
    'export class RuleTester {',
    '  constructor(config) {',
    '    this.language = config && config.language;',
    '  }',
    '  run(name, _rule, tests) {',
    '    const valid = (tests && tests.valid) || [];',
    '    const invalid = (tests && tests.invalid) || [];',
    '    const prev = globalThis[KEY].captures[name];',
    '    globalThis[KEY].captures[name] = prev',
    '      ? { language: prev.language, valid: prev.valid.concat(valid), invalid: prev.invalid.concat(invalid) }',
    '      : { language: this.language, valid, invalid };',
    '  }',
    '}',
    'export class Linter {',
    '  verify() { return []; }',
    '  verifyAndFix(code) { return { output: code, fixed: false, messages: [] }; }',
    '}',
    'export default { RuleTester, Linter };',
  ].join('\n');
}
