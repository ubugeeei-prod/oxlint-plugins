// Captures the upstream eslint-plugin-security test suite straight from the
// vendored submodule and writes it to committed JSON fixtures, so our Vitest
// suite replays the real upstream cases and tracks behavior as the submodule is
// bumped (oxc-style test syncing).
//
// The upstream tests drive cases through ESLint's `RuleTester`. We register
// synchronous module hooks (`module.registerHooks`) that stub `eslint`
// (capturing every `RuleTester.run` call — several upstream files run the same
// rule more than once) and the rule modules under test. Most rule modules are
// stubbed to `{}` (we only need the captured cases, not the rule object), but
// `detect-buffer-noassert` is loaded for real because its test file reads
// `rule.meta.__methodsToCheck` to generate one case per Buffer accessor; that
// rule has no third-party dependencies, so loading it is self-contained.
//
// Each upstream test file is copied to a temp location before being required,
// because bare specifiers resolved in-place under the submodule bypass the hook
// chain on Node 24. Regex message matchers (`message: /.../i`) are preserved as
// `{ __regex, __flags }` so the replay harness can apply them faithfully.
//
// Re-run with `pnpm run port:tests:security`.

import { createRequire, registerHooks } from 'node:module';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

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
type CapturedRun = { name: string; valid: RawCase[]; invalid: RawCase[] };

const ROOT = process.cwd();
const HERE = dirname(fileURLToPath(import.meta.url));

// `RuleTester.run` from the eslint stub appends each captured run here, keyed on
// the shared global so the value crosses the hook-loaded module boundary.
const CAPTURE_KEY = '__eslintSecuritySyncCapture__';
const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-security');
if (!plugin) {
  throw new Error('eslint-plugin-security is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const RULES_DIR = join(SUBMODULE, 'rules');
const TESTS_DIR = join(SUBMODULE, 'test', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'security', 'test', 'fixtures');

// The rule that must be loaded for real (its test reads `meta.__methodsToCheck`).
const REAL_RULE = 'detect-buffer-noassert';

// The rules currently implemented by our plugin. Only these get fixtures, so the
// replay harness never references a rule we have not ported yet.
const portedRules = getPortedRules();

if (!existsSync(TESTS_DIR)) {
  throw new Error(
    `Upstream tests not found at ${TESTS_DIR}. Run: git submodule update --init --depth 1 ${plugin.submodule}`,
  );
}

mkdirSync(FIXTURES_DIR, { recursive: true });

const tempDir = mkdtempSync(join(tmpdir(), 'security-sync-'));
registerStubHooks();

// rule name -> { valid, invalid } accumulated across every test file / run.
const grouped = new Map<string, { valid: Record<string, unknown>[]; invalid: Record<string, unknown>[] }>();
for (const rule of portedRules) {
  grouped.set(rule, { valid: [], invalid: [] });
}

const testFiles = listTestFiles(TESTS_DIR);
for (const file of testFiles) {
  const runs = captureRuns(file);
  for (const run of runs) {
    const rule = baseRuleName(run.name, portedRules);
    if (!rule) {
      // A run for a rule we have not ported; skip silently (not our target).
      continue;
    }
    const bucket = grouped.get(rule)!;
    for (const raw of run.valid) {
      const normalized = normalizeCase(raw);
      if (normalized) {
        bucket.valid.push(normalized);
      }
    }
    for (const raw of run.invalid) {
      const normalized = normalizeCase(raw);
      if (normalized) {
        bucket.invalid.push(normalized);
      }
    }
  }
}

rmSync(tempDir, { recursive: true, force: true });

const summary: string[] = [];
for (const rule of portedRules) {
  const bucket = grouped.get(rule)!;
  if (bucket.valid.length === 0 && bucket.invalid.length === 0) {
    throw new Error(`No upstream cases captured for rule "${rule}"`);
  }
  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceFile: `test/rules/${rule}.js`,
      license: plugin.license,
      tool: 'tools/tasks/sync-eslint-security-tests.ts',
    },
    valid: bucket.valid,
    invalid: bucket.invalid,
  };
  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
  summary.push(`${rule}: ${bucket.valid.length} valid, ${bucket.invalid.length} invalid`);
}

console.log('Synced eslint-plugin-security fixtures from upstream:');
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ---------------------------------------------------------------

function listTestFiles(dir: string): string[] {
  return readdirSync(dir)
    .filter((name) => name.endsWith('.js'))
    .map((name) => join(dir, name))
    .sort();
}

function captureRuns(testFile: string): CapturedRun[] {
  // Require a copy in a temp dir: bare specifiers resolved in-place under the
  // submodule bypass the hook chain, but resolve through the hooks from a temp
  // file. Rule requires are intercepted by specifier substring, so the original
  // `../../rules/...` path never needs to resolve on disk.
  const tempFile = join(tempDir, `${Date.now()}-${Math.random().toString(36).slice(2)}.cjs`);
  writeFileSync(tempFile, readFileSync(testFile, 'utf8'));

  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = [];
  const require = createRequire(tempFile);
  require(tempFile);

  const captured = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedRun[];
  if (!captured || captured.length === 0) {
    throw new Error(`No RuleTester.run() call captured from ${testFile}`);
  }
  return captured;
}

// Synchronous module hooks that replace the upstream test files' dependencies
// with capturing/no-op stubs (works for CommonJS via `module.registerHooks`).
function registerStubHooks(): void {
  const realRuleSource = readFileSync(join(RULES_DIR, `${REAL_RULE}.js`), 'utf8');

  const eslintStub = [
    'class RuleTester {',
    '  constructor(config) { this.config = config || {}; }',
    '  run(name, _rule, tests) {',
    `    const store = globalThis['${CAPTURE_KEY}'] || (globalThis['${CAPTURE_KEY}'] = []);`,
    '    store.push({',
    '      name,',
    '      defaults: this.config,',
    '      valid: (tests && tests.valid) || [],',
    '      invalid: (tests && tests.invalid) || [],',
    '    });',
    '  }',
    '}',
    'class Linter { getRules() { return new Map(); } }',
    "Linter.version = '8.57.0';",
    'module.exports = { RuleTester, Linter };',
  ].join('\n');

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'eslint') {
        return { url: 'stub:///eslint', shortCircuit: true };
      }
      if (specifier.startsWith('eslint/')) {
        return { url: 'stub:///eslint-internal', shortCircuit: true };
      }
      // Rule modules: load the real source only for the rule whose test reads
      // `meta.__methodsToCheck`; stub the rest so their third-party deps
      // (e.g. safe-regex) are never required.
      if (/[\\/]rules[\\/]/.test(specifier) || specifier.startsWith('../../rules/')) {
        if (specifier.endsWith(REAL_RULE)) {
          return { url: 'stub:///real-rule', shortCircuit: true };
        }
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url === 'stub:///eslint') {
        return { format: 'commonjs', source: eslintStub, shortCircuit: true };
      }
      if (url === 'stub:///eslint-internal') {
        return {
          format: 'commonjs',
          source: 'module.exports = { builtinRules: new Map() };',
          shortCircuit: true,
        };
      }
      if (url === 'stub:///real-rule') {
        return { format: 'commonjs', source: realRuleSource, shortCircuit: true };
      }
      if (url === 'stub:///rule') {
        return { format: 'commonjs', source: 'module.exports = {};', shortCircuit: true };
      }
      return nextLoad(url, context);
    },
  });
}

// Map an upstream run name back to the ported rule it exercises. Several files
// run the same rule under suffixed names (e.g. "detect-unsafe-regex (new
// RegExp)"); we attribute them to the longest matching ported rule id.
function baseRuleName(runName: string, rules: string[]): string | null {
  let match: string | null = null;
  for (const rule of rules) {
    if (runName === rule || runName.startsWith(`${rule} `)) {
      if (!match || rule.length > match.length) {
        match = rule;
      }
    }
  }
  return match;
}

// Recursively convert a captured value to a JSON-serializable form, preserving
// RegExp matchers as `{ __regex, __flags }` and dropping function-valued fields.
function toSerializable(value: unknown): unknown {
  if (value instanceof RegExp) {
    return { __regex: value.source, __flags: value.flags };
  }
  if (Array.isArray(value)) {
    return value.map(toSerializable);
  }
  if (value && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [key, entry] of Object.entries(value)) {
      const serialized = toSerializable(entry);
      if (serialized !== undefined) {
        out[key] = serialized;
      }
    }
    return out;
  }
  if (typeof value === 'function') {
    return undefined;
  }
  return value;
}

// Keep only cases that run under our Rust scanner (plain JS, no extra language
// or plugins). Returns null for cases we cannot replay.
function normalizeCase(raw: RawCase): Record<string, unknown> | null {
  const value = typeof raw === 'string' ? { code: raw } : raw;
  if (value == null || typeof value !== 'object') {
    return null;
  }
  if ('language' in value || 'plugins' in value || 'processor' in value) {
    return null;
  }
  const languageOptions = (value as { languageOptions?: { parser?: unknown } }).languageOptions;
  if (languageOptions && 'parser' in languageOptions) {
    return null;
  }

  const serialized = toSerializable(value) as Record<string, unknown>;
  if (typeof serialized.code !== 'string') {
    return null;
  }
  return serialized;
}

function getPortedRules(): string[] {
  // Prefer the built plugin's actual rule set; fall back to the upstream rules
  // directory when the native binding is not built (so the fixtures can be
  // regenerated without a full `vp build`). All upstream rules are ported.
  try {
    const pkgRequire = createRequire(join(ROOT, 'npm', 'security', 'index.js'));
    const loaded = pkgRequire(resolve(ROOT, 'npm', 'security', 'index.js')) as {
      rules?: Record<string, unknown>;
    };
    const rules = loaded.rules ? Object.keys(loaded.rules) : [];
    if (rules.length > 0) {
      return rules.sort();
    }
  } catch {
    // Native binding unavailable; fall back to the upstream rule inventory.
  }
  const rules = readdirSync(RULES_DIR)
    .filter((name) => name.endsWith('.js') && name.startsWith('detect-'))
    .map((name) => name.replace(/\.js$/, ''));
  if (rules.length === 0) {
    throw new Error('No rules found in the security plugin or upstream rules directory.');
  }
  return rules.sort();
}

void HERE;
