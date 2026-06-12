import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = dirname(dirname(fileURLToPath(import.meta.url)));

// Packages whose Vitest suites depend on NAPI bindings. Normally `vp build`
// produces these before `vp test`; this is the fallback for direct test runs.
const nativePackages = [
  {
    name: '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
    binding: 'npm/no-forbidden-identifiers/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-comments',
    binding: 'npm/eslint-comments/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-security',
    binding: 'npm/security/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-cypress',
    binding: 'npm/cypress/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-stylistic',
    binding: 'npm/stylistic/native.js',
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-react-refresh',
    binding: 'npm/react-refresh/native.js',
  },
];

for (const pkg of nativePackages) {
  if (existsSync(resolve(root, pkg.binding))) {
    continue;
  }

  const result = spawnSync('pnpm', ['--filter', pkg.name, 'build'], {
    cwd: root,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    throw new Error(`Failed to build NAPI bindings required by Vitest for ${pkg.name}.`);
  }
}
