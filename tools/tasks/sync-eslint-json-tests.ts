// Captures the upstream @eslint/json test suite straight from the vendored
// submodule and writes it to committed JSON fixtures, so our Vitest suite
// replays the real upstream RuleTester cases and tracks behavior as the
// submodule is bumped (oxc-style test syncing).
//
// The upstream tests drive cases through ESLint's `RuleTester` configured with
// the `json/json` language. They are ES modules that import `eslint`
// (RuleTester), the plugin entry (`../../src/index.js`), and the rule under
// test (`../../src/rules/<rule>.js`). We register synchronous module hooks
// (`module.registerHooks`) that replace all three with stubs: a capturing
// `RuleTester` that records the `valid`/`invalid` arrays, and no-op modules for
// the plugin and rule (which pull deps the submodule has not installed). Each
// upstream test file is copied to a temp location before being imported,
// because the relative `../../src/...` specifiers resolved in-place under the
// submodule would otherwise reach the real (uninstalled) modules.
//
// Every captured case is JSON-serializable (code + options + language +
// errors + output), so nothing is dropped; the per-case `language`
// (json/json | json/jsonc | json/json5) is preserved for the replay label.
// Re-run with `pnpm run port:tests:eslint-json`.

import { createRequire, registerHooks } from 'node:module';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    license: string;
  }>;
};

type RawCase = Record<string, unknown> | string;
type CapturedTests = { name: string; valid: RawCase[]; invalid: RawCase[] };

const ROOT = process.cwd();
const HERE = dirname(fileURLToPath(import.meta.url));

// The capturing `RuleTester.run` stub writes here, keyed on the shared global so
// the value crosses the hook-loaded module boundary.
const CAPTURE_KEY = '__eslintJsonSyncCapture__';

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-json');
if (!plugin) {
  throw new Error('eslint-json is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const TESTS_DIR = join(SUBMODULE, 'tests', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'eslint-json', 'test', 'fixtures');

// The default RuleTester language for every upstream eslint-json rule test.
const DEFAULT_LANGUAGE = 'json/json';

const portedRules = getPortedRules();

if (!existsSync(TESTS_DIR)) {
  throw new Error(
    `Upstream tests not found at ${TESTS_DIR}. Run: git submodule update --init ${plugin.submodule}`,
  );
}

mkdirSync(FIXTURES_DIR, { recursive: true });

const tempDir = mkdtempSync(join(tmpdir(), 'eslint-json-sync-'));
registerStubHooks();

const summary: string[] = [];
for (const rule of portedRules) {
  const testFile = join(TESTS_DIR, `${rule}.test.js`);
  if (!existsSync(testFile)) {
    throw new Error(`Upstream test file missing for rule "${rule}": ${testFile}`);
  }

  const captured = await captureTests(rule, testFile);
  const valid = captured.valid.map(normalizeCase).filter(isPresent);
  const invalid = captured.invalid.map(normalizeCase).filter(isPresent);
  const dropped = captured.valid.length - valid.length + (captured.invalid.length - invalid.length);
  if (dropped > 0) {
    throw new Error(
      `${rule}: ${dropped} upstream case(s) were not JSON-serializable; eslint-json cases must all replay.`,
    );
  }

  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceFile: `tests/rules/${rule}.test.js`,
      license: plugin.license,
      tool: 'tools/tasks/sync-eslint-json-tests.ts',
    },
    valid,
    invalid,
  };

  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
  summary.push(`${rule}: ${valid.length} valid, ${invalid.length} invalid`);
}

rmSync(tempDir, { recursive: true, force: true });

console.log('Synced eslint-json fixtures from upstream:');
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ---------------------------------------------------------------

async function captureTests(rule: string, testFile: string): Promise<CapturedTests> {
  // Import a copy in a temp dir so the relative `../../src/...` specifiers in
  // the upstream test file are intercepted by the resolve hook (by pattern)
  // rather than reaching the real, uninstalled modules under the submodule.
  const tempFile = join(tempDir, `${rule}.test.mjs`);
  writeFileSync(tempFile, readFileSync(testFile, 'utf8'));

  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = null;
  await import(`${pathToFileURL(tempFile).href}?rule=${rule}`);

  const captured = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedTests | null;
  if (!captured || !captured.name) {
    throw new Error(`No RuleTester.run() call captured from ${testFile}`);
  }
  return captured;
}

// Synchronous module hooks that replace the upstream test file's imports
// (`eslint`, the plugin entry, and the rule module) with capturing/no-op stubs.
function registerStubHooks(): void {
  const ruleTesterStub = [
    'export class RuleTester {',
    '  constructor() {}',
    '  run(name, rule, tests) {',
    `    globalThis['${CAPTURE_KEY}'] = {`,
    '      name,',
    '      valid: (tests && tests.valid) || [],',
    '      invalid: (tests && tests.invalid) || [],',
    '    };',
    '  }',
    '}',
    'export default { RuleTester };',
  ].join('\n');
  // The plugin entry and rule modules are only needed as RuleTester arguments,
  // which the capturing stub ignores; an empty default export is enough.
  const noopStub = 'export default {};';

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'eslint') {
        return { url: 'stub:///eslint', shortCircuit: true };
      }
      if (specifier.includes('/src/index.js') || specifier.endsWith('/src/index.js')) {
        return { url: 'stub:///plugin', shortCircuit: true };
      }
      if (specifier.includes('/src/rules/')) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///eslint') {
        return { format: 'module', source: ruleTesterStub, shortCircuit: true };
      }
      if (url === 'stub:///plugin' || url === 'stub:///rule') {
        return { format: 'module', source: noopStub, shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

// Normalize one captured case into a JSON-serializable record. A bare string is
// shorthand for `{ code }`. The per-case `language` defaults to the RuleTester's
// `json/json`. Returns null only if the case has no string `code`.
function normalizeCase(raw: RawCase): Record<string, unknown> | null {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (value == null || typeof value !== 'object') {
    return null;
  }

  let clone: Record<string, unknown>;
  try {
    clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
  } catch {
    return null;
  }
  if (typeof clone.code !== 'string') {
    return null;
  }
  if (typeof clone.language !== 'string') {
    clone.language = DEFAULT_LANGUAGE;
  }
  return clone;
}

function isPresent<T>(value: T | null): value is T {
  return value != null;
}

function getPortedRules(): string[] {
  const pkgRequire = createRequire(join(ROOT, 'npm', 'eslint-json', 'index.js'));
  const loaded = pkgRequire(resolve(ROOT, 'npm', 'eslint-json', 'index.js')) as {
    rules?: Record<string, unknown>;
  };
  const rules = loaded.rules ? Object.keys(loaded.rules) : [];
  if (rules.length === 0) {
    throw new Error('No rules found on the eslint-json plugin; build it first (vp build).');
  }
  return rules.sort();
}

void HERE;
