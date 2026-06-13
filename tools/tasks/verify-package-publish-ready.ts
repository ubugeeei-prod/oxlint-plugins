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
    name: '@oxlint-plugins/oxlint-plugin-angular-eslint',
    dir: 'npm/angular-eslint',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-type-aware',
    dir: 'npm/type-aware',
    requiredFiles: ['dist/index.mjs', 'dist/index.d.mts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
    dir: 'npm/no-forbidden-identifiers',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-e18e',
    dir: 'npm/e18e',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-comments',
    dir: 'npm/eslint-comments',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-json',
    dir: 'npm/eslint-json',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-markdown',
    dir: 'npm/eslint-markdown',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-react-refresh',
    dir: 'npm/react-refresh',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-regexp',
    dir: 'npm/regexp',
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
    name: '@oxlint-plugins/oxlint-plugin-mocha',
    dir: 'npm/mocha',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-playwright',
    dir: 'npm/playwright',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-simple-import-sort',
    dir: 'npm/simple-import-sort',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-unused-imports',
    dir: 'npm/unused-imports',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-storybook',
    dir: 'npm/storybook',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-stylistic',
    dir: 'npm/stylistic',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-testing-library',
    dir: 'npm/testing-library',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
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
