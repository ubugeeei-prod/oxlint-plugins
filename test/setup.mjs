import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = dirname(dirname(fileURLToPath(import.meta.url)));

// All plugins share one native addon, built by `@oxlint-plugins/core`. Normally
// `vp build` produces it before `vp test`; this is the fallback for direct test
// runs.
const nativeBinding = resolve(root, 'npm/core/native.js');

if (!existsSync(nativeBinding)) {
  const result = spawnSync('pnpm', ['--filter', '@oxlint-plugins/core', 'build'], {
    cwd: root,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    throw new Error('Failed to build NAPI bindings required by Vitest.');
  }
}
