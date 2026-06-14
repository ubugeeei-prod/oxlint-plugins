// Captures the upstream postgresql-eslint-parser fixture suite straight from the
// vendored submodule and writes it to committed JSON fixtures, so our Vitest
// suite replays the real upstream `parseForESLint` expectations and tracks
// behaviour as the submodule is bumped (oxc-style test syncing). Mirrors the
// other tools/tasks/sync-*-tests.ts scripts.
//
// Upstream authors each case as an on-disk fixture directory under
// tests/fixtures/<name>/ containing:
//   - input.sql                  the SQL source
//   - output.ast.ts              a TS module `export default {…} satisfies Program`
//   - output.visitorKeys.json    the expected visitor-key map
//   - output.scopeManager.json   always empty -> scopeManager is null
//
// We import the AST module directly: Node strips the `import type { Program }`
// and the `satisfies Program` so the referenced src/ast.ts is never loaded, and
// the module evaluates to a plain object. The AST is already parent-free (the
// upstream fixture writer strips `parent`), but we strip again defensively.
//
// Re-run with `pnpm run port:tests:postgresql-parser`, then `vp fmt` the JSON
// (JSON.stringify expands short arrays the formatter collapses onto one line).

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
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

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, '..', '..');

const manifest = JSON.parse(
  readFileSync(join(repoRoot, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const target = manifest.plugins.find((plugin) => plugin.id === 'postgresql-eslint-parser');
if (!target) {
  throw new Error('postgresql-eslint-parser not found in tools/port-targets.json');
}

const fixturesSrc = join(repoRoot, target.submodule, 'tests', 'fixtures');
if (!existsSync(fixturesSrc)) {
  throw new Error(
    `Upstream fixtures not found at ${fixturesSrc}. Did you run \`git submodule update --init ${target.submodule}\`?`,
  );
}

const outDir = join(repoRoot, 'npm', 'postgresql-eslint-parser', 'test', 'fixtures');
mkdirSync(outDir, { recursive: true });

const generated = {
  source: target.npm,
  ref: target.pinnedRef ?? `v${target.baselineVersion}`,
  sourceDir: `${target.submodule}/tests/fixtures`,
  license: target.license,
  tool: 'tools/tasks/sync-postgresql-parser-tests.ts',
};

const stripParent = (value: unknown): unknown => {
  if (Array.isArray(value)) {
    return value.map(stripParent);
  }
  if (value !== null && typeof value === 'object') {
    const result: Record<string, unknown> = {};
    for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
      if (key !== 'parent') {
        result[key] = stripParent(child);
      }
    }
    return result;
  }
  return value;
};

const readJsonOrNull = (path: string): unknown => {
  if (!existsSync(path)) {
    return null;
  }
  const raw = readFileSync(path, 'utf8').trim();
  return raw ? JSON.parse(raw) : null;
};

const dirs = readdirSync(fixturesSrc)
  .filter((name) => statSync(join(fixturesSrc, name)).isDirectory())
  .sort();

let count = 0;
for (const name of dirs) {
  const dir = join(fixturesSrc, name);
  const inputPath = join(dir, 'input.sql');
  if (!existsSync(inputPath)) {
    continue;
  }

  const input = readFileSync(inputPath, 'utf8');
  const astModule = (await import(pathToFileURL(join(dir, 'output.ast.ts')).href)) as {
    default: unknown;
  };
  const ast = stripParent(astModule.default);
  const visitorKeys = readJsonOrNull(join(dir, 'output.visitorKeys.json'));
  const scopeManager = readJsonOrNull(join(dir, 'output.scopeManager.json'));

  const fixture = { __generated: generated, input, ast, visitorKeys, scopeManager };
  writeFileSync(join(outDir, `${name}.json`), `${JSON.stringify(fixture, null, 2)}\n`, 'utf8');
  count += 1;
}

console.log(`Wrote ${count} postgresql-eslint-parser fixtures to ${outDir}`);
