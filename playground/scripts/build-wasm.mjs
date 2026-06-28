// Builds the playground WebAssembly module with wasm-pack and stages the
// @libpg-query/parser wasm.
//
// The Rust aggregator crate (crates/playground_wasm) is compiled to a
// browser-targeted wasm package written into playground/src/wasm so Vite can
// bundle it. The @libpg-query/parser Emscripten module fetches its wasm from
// the site root at runtime, so copy it into public/. Run via
// `pnpm run build:wasm`.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');
const playgroundDir = resolve(here, '..');
const outDir = resolve(playgroundDir, 'src', 'wasm');

execFileSync(
  'wasm-pack',
  ['build', 'crates/playground_wasm', '--target', 'web', '--release', '--out-dir', outDir],
  { cwd: repoRoot, stdio: 'inherit' },
);

const require = createRequire(import.meta.url);
const libpgWasm = resolve(
  dirname(require.resolve('@libpg-query/parser/package.json')),
  'wasm',
  'libpg-query.wasm',
);
const publicDir = resolve(playgroundDir, 'public');
mkdirSync(publicDir, { recursive: true });
copyFileSync(libpgWasm, resolve(publicDir, 'libpg-query.wasm'));
