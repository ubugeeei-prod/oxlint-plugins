// Builds the playground WebAssembly module with wasm-pack.
//
// The Rust aggregator crate (crates/playground_wasm) is compiled to a
// browser-targeted wasm package written into playground/src/wasm so Vite can
// bundle it. Run via `pnpm run build:wasm`.
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');
const outDir = resolve(repoRoot, 'playground', 'src', 'wasm');

execFileSync(
  'wasm-pack',
  ['build', 'crates/playground_wasm', '--target', 'web', '--release', '--out-dir', outDir],
  { cwd: repoRoot, stdio: 'inherit' },
);
