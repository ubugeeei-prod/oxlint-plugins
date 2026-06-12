import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

type PackageToPack = {
  name: string;
  dir: string;
  requiredFiles: string[];
};

const packages: PackageToPack[] = [
  {
    name: '@oxlint-plugins/oxlint-plugin-type-aware',
    dir: 'npm/type-aware',
    requiredFiles: ['dist/index.mjs', 'dist/index.d.mts'],
  },
  {
    // Shared native core: the only package that ships a NAPI binding.
    name: '@oxlint-plugins/core',
    dir: 'npm/core',
    requiredFiles: ['index.js', 'native.js', 'native.d.ts'],
  },
  {
    // Thin JavaScript facades over the shared core; they ship no native binding.
    name: '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
    dir: 'npm/no-forbidden-identifiers',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-comments',
    dir: 'npm/eslint-comments',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-react-refresh',
    dir: 'npm/react-refresh',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-security',
    dir: 'npm/security',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-cypress',
    dir: 'npm/cypress',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-stylistic',
    dir: 'npm/stylistic',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts'],
  },
  {
    // Convenience bundle: aggregates the facades into one combined plugin.
    name: '@oxlint-plugins/oxlint',
    dir: 'npm/oxlint',
    requiredFiles: ['index.js'],
  },
];

for (const pkg of packages) {
  for (const file of pkg.requiredFiles) {
    const path = resolve(pkg.dir, file);
    if (!existsSync(path)) {
      console.error(`${pkg.name} is missing ${file}. Run vp build first.`);
      process.exit(1);
    }
  }

  execFileSync('pnpm', ['pack', '--dry-run', '--json'], {
    cwd: pkg.dir,
    stdio: 'inherit',
  });
}
