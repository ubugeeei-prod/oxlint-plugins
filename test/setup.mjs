import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const nativeBinding = resolve(root, 'npm/no-forbidden-identifiers/native.js');

if (!existsSync(nativeBinding)) {
  const result = spawnSync(
    'pnpm',
    ['--filter', '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers', 'build'],
    {
      cwd: root,
      stdio: 'inherit',
    },
  );

  if (result.status !== 0) {
    throw new Error('Failed to build NAPI bindings required by Vitest.');
  }
}
