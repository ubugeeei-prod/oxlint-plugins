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
    name: '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers',
    dir: 'npm/no-forbidden-identifiers',
    requiredFiles: ['index.js', 'api.js', 'api.d.ts', 'native.js', 'native.d.ts'],
  },
  {
    name: '@oxlint-plugins/oxlint-plugin-eslint-comments',
    dir: 'npm/eslint-comments',
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
