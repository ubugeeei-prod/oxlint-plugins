import { defineConfig } from 'vitest/config';

// Dedicated config for capture specs that must load ESM / transform-required upstream test
// files (simple-import-sort, functional, …). These run via vite's transform pipeline, which
// plain `node require()` cannot do. Kept separate from the main suite so capture (which writes
// corpora and needs the upstream submodules) is an explicit step, not part of `vp test`.
//
//   pnpm exec vitest run --config tools/parity/capture/vitest.config.ts

export default defineConfig({
  test: {
    include: ['tools/parity/capture/**/*.capture.mjs'],
    testTimeout: 120_000,
    pool: 'forks',
  },
});
