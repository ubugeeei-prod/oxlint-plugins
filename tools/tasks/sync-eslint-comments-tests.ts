// Captures the upstream @eslint-community/eslint-plugin-eslint-comments test
// suite straight from the vendored submodule and writes it to committed JSON
// fixtures, so our Vitest suite replays the real upstream cases and tracks
// behavior as the submodule is bumped (oxc-style test syncing).
//
// The upstream tests drive cases through ESLint's `RuleTester`. We register
// synchronous module hooks (`module.registerHooks`) that stub `eslint`
// (capturing `RuleTester.run` cases), `semver` (so the `>=9.6.0` language-plugin
// branches are skipped), `@eslint/*`, and the rule modules under test (which
// pull deps the submodule has not installed). Each upstream test file is copied
// to a temp location before being required, because bare specifiers resolved
// in-place under the submodule bypass the hook chain on Node 24.
//
// Cases that need a non-JS language or extra plugins cannot run through espree
// in our replay harness; they are dropped and the count is logged (no silent
// truncation). Re-run with `pnpm run port:tests:eslint-comments`.

import { createRequire, registerHooks } from 'node:module';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

type Manifest = {
  submoduleRoot: string;
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

// `RuleTester.run` from the eslint stub writes captured cases here, keyed on the
// shared global so the value crosses the hook-loaded module boundary.
const CAPTURE_KEY = '__eslintCommentsSyncCapture__';
const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-eslint-comments');
if (!plugin) {
  throw new Error('eslint-plugin-eslint-comments is not registered in tools/port-targets.json');
}

const SUBMODULE = join(ROOT, plugin.submodule);
const TESTS_DIR = join(SUBMODULE, 'tests', 'lib', 'rules');
const FIXTURES_DIR = join(ROOT, 'npm', 'eslint-comments', 'test', 'fixtures');

// The rules currently implemented by our plugin. Only these get fixtures, so the
// replay harness never references a rule we have not ported yet.
const portedRules = getPortedRules();

if (!existsSync(TESTS_DIR)) {
  throw new Error(
    `Upstream tests not found at ${TESTS_DIR}. Run: git submodule update --init --depth 1 ${plugin.submodule}`,
  );
}

mkdirSync(FIXTURES_DIR, { recursive: true });

const tempDir = mkdtempSync(join(tmpdir(), 'eslint-comments-sync-'));
registerStubHooks();

const summary: string[] = [];
for (const rule of portedRules) {
  const testFile = join(TESTS_DIR, `${rule}.js`);
  if (!existsSync(testFile)) {
    throw new Error(`Upstream test file missing for rule "${rule}": ${testFile}`);
  }

  const captured = captureTests(rule, testFile);
  const valid = captured.valid.map(normalizeCase).filter(isPresent);
  const invalid = captured.invalid.map(normalizeCase).filter(isPresent);
  const dropped = captured.valid.length - valid.length + (captured.invalid.length - invalid.length);

  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceFile: `tests/lib/rules/${rule}.js`,
      license: plugin.license,
      tool: 'tools/tasks/sync-eslint-comments-tests.ts',
    },
    valid,
    invalid,
  };

  writeFileSync(join(FIXTURES_DIR, `${rule}.json`), `${JSON.stringify(fixture, null, 2)}\n`);
  summary.push(
    `${rule}: ${valid.length} valid, ${invalid.length} invalid${
      dropped > 0 ? ` (${dropped} non-JS case(s) dropped)` : ''
    }`,
  );
}

rmSync(tempDir, { recursive: true, force: true });

console.log('Synced eslint-comments fixtures from upstream:');
for (const line of summary) {
  console.log(`- ${line}`);
}

// --- helpers ---------------------------------------------------------------

function captureTests(rule: string, testFile: string): CapturedTests {
  // Require a copy in a temp dir: bare specifiers resolved in-place under the
  // submodule bypass the hook chain, but resolve correctly from a temp file.
  const tempFile = join(tempDir, `${rule}.cjs`);
  writeFileSync(tempFile, readFileSync(testFile, 'utf8'));

  (globalThis as Record<string, unknown>)[CAPTURE_KEY] = null;
  const require = createRequire(tempFile);
  require(tempFile);

  const captured = (globalThis as Record<string, unknown>)[CAPTURE_KEY] as CapturedTests | null;
  if (!captured || !captured.name) {
    throw new Error(`No RuleTester.run() call captured from ${testFile}`);
  }
  return captured;
}

// Synchronous module hooks that replace the upstream test files' dependencies
// with capturing/no-op stubs (works for CommonJS via `module.registerHooks`).
function registerStubHooks(): void {
  const stubSource: Record<string, string> = {
    eslint: [
      'class RuleTester {',
      '  run(name, rule, tests) {',
      `    globalThis['${CAPTURE_KEY}'] = {`,
      '      name,',
      '      valid: (tests && tests.valid) || [],',
      '      invalid: (tests && tests.invalid) || [],',
      '    };',
      '  }',
      '}',
      'class Linter {',
      '  getRules() { return new Map(); }',
      '}',
      "Linter.version = '8.57.0';",
      'module.exports = { RuleTester, Linter };',
    ].join('\n'),
    // `eslint/use-at-your-own-risk`: some tests read `builtinRules` to register
    // core rules in the RuleTester (which the stub ignores), so an empty Map is
    // enough.
    'eslint-internal': 'module.exports = { builtinRules: new Map() };',
    // Minimal `semver.satisfies` evaluated against the stub Linter.version
    // (8.57.0). This makes version guards like `>=7.0.0` true while keeping the
    // `>=9.6.0` language-plugin branches false, so v9.6 CSS/language cases (also
    // dropped by normalizeCase) are excluded. Unparseable comparators default to
    // true (permissive); such cases carry `language`/`plugins` and are dropped.
    semver: [
      'function parse(v) { return String(v).replace(/^[v=]+/, "").split(".").map(Number); }',
      'function cmp(a, b) { for (let i = 0; i < 3; i++) { const x = a[i] || 0, y = b[i] || 0; if (x !== y) return x < y ? -1 : 1; } return 0; }',
      'function satisfies(version, range) {',
      '  const v = parse(version);',
      '  return String(range).split("||").some((group) =>',
      '    group.trim().split(/\\s+/).filter(Boolean).every((part) => {',
      '      const m = /^(>=|<=|>|<|=)?\\s*v?(\\d+\\.\\d+\\.\\d+)/.exec(part);',
      '      if (!m) return true;',
      '      const c = cmp(v, parse(m[2]));',
      '      switch (m[1]) {',
      '        case ">=": return c >= 0;',
      '        case "<=": return c <= 0;',
      '        case ">": return c > 0;',
      '        case "<": return c < 0;',
      '        default: return c === 0;',
      '      }',
      '    }),',
      '  );',
      '}',
      'module.exports = { satisfies };',
    ].join('\n'),
    css: 'module.exports = { default: {} };',
    // The rule under test pulls runtime deps the submodule has not installed;
    // we only need the captured cases, not the rule object.
    rule: 'module.exports = {};',
  };

  registerHooks({
    resolve(specifier, context, nextResolve) {
      if (specifier === 'eslint') {
        return { url: 'stub:///eslint', shortCircuit: true };
      }
      if (specifier.startsWith('eslint/')) {
        return { url: 'stub:///eslint-internal', shortCircuit: true };
      }
      if (specifier === 'semver') {
        return { url: 'stub:///semver', shortCircuit: true };
      }
      if (specifier.startsWith('@eslint/')) {
        return { url: 'stub:///css', shortCircuit: true };
      }
      if (specifier.includes('lib/rules/')) {
        return { url: 'stub:///rule', shortCircuit: true };
      }
      return nextResolve(specifier, context);
    },
    load(url, context, nextLoad) {
      if (url.startsWith('stub:///')) {
        return {
          format: 'commonjs',
          source: stubSource[url.slice('stub:///'.length)],
          shortCircuit: true,
        };
      }
      return nextLoad(url, context);
    },
  });
}

// Keep only JSON-serializable cases that run under espree (plain JS, no extra
// language or plugins). Returns null for cases we cannot replay.
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

  try {
    const clone = JSON.parse(JSON.stringify(value)) as Record<string, unknown>;
    // A round-trip drop of any function-valued field (e.g. a custom parser)
    // means the case is not replayable; treat structural loss as a drop.
    if (!('code' in clone) || typeof clone.code !== 'string') {
      return null;
    }
    return clone;
  } catch {
    return null;
  }
}

function isPresent<T>(value: T | null): value is T {
  return value != null;
}

function getPortedRules(): string[] {
  const pkgRequire = createRequire(join(ROOT, 'npm', 'eslint-comments', 'index.js'));
  const loaded = pkgRequire(resolve(ROOT, 'npm', 'eslint-comments', 'index.js')) as {
    rules?: Record<string, unknown>;
  };
  const rules = loaded.rules ? Object.keys(loaded.rules) : [];
  if (rules.length === 0) {
    throw new Error('No rules found on the eslint-comments plugin; build it first (vp build).');
  }
  return rules.sort();
}

void HERE;
