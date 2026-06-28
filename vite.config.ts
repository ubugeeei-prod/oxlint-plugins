import { execFileSync } from 'node:child_process';

import { defineConfig } from 'vite-plus';

function workspaceNativeBuild() {
  return {
    name: 'oxlint-plugins-workspace-native-build',
    apply: 'build' as const,
    buildStart() {
      execFileSync('pnpm', ['run', 'build:rust'], { stdio: 'inherit' });
      execFileSync('pnpm', ['run', 'build:workspace'], { stdio: 'inherit' });
    },
  };
}

export default defineConfig({
  plugins: [workspaceNativeBuild()],
  build: {
    lib: {
      entry: 'tools/vite/build-entry.ts',
      formats: ['es'],
      fileName: 'index',
    },
    outDir: 'dist',
    sourcemap: true,
  },
  fmt: {
    ignorePatterns: [
      'docs/site/dist/**',
      'docs/site/.void/**',
      'docs/site/.wrangler/**',
      'docs/port-targets/**',
      'npm/eslint-comments/test/fixtures/**',
      'npm/eslint-json/test/fixtures/**',
      'npm/functional/test/fixtures/**',
      'dist/**',
      'target/**',
      'node_modules/**',
      'upstream/**',
      'npm/**/native.js',
      'npm/**/native.d.ts',
      'playground/src/wasm/**',
      'playground/src/catalog.json',
      'playground/dist/**',
    ],
    singleQuote: true,
    semi: true,
    sortPackageJson: true,
  },
  lint: {
    ignorePatterns: [
      'docs/site/dist/**',
      'docs/site/.void/**',
      'docs/site/.wrangler/**',
      'docs/port-targets/**',
      'npm/eslint-comments/test/fixtures/**',
      'npm/eslint-json/test/fixtures/**',
      'npm/functional/test/fixtures/**',
      'dist/**',
      'target/**',
      'node_modules/**',
      'upstream/**',
      'npm/**/native.js',
      'npm/**/native.d.ts',
      'playground/src/wasm/**',
      'playground/src/catalog.json',
      'playground/dist/**',
    ],
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  test: {
    include: ['npm/**/*.test.ts', 'npm/**/*.test.mjs', 'test/**/*.test.mjs', 'tools/**/*.test.ts'],
    setupFiles: ['test/setup.mjs'],
    testTimeout: 120_000,
  },
  run: {
    cache: {
      scripts: false,
      tasks: true,
    },
  },
});
