'use strict';

// Per-plugin capture configuration.
//
// This is the single source of truth for how each upstream port-target's test
// suite is harvested. Keep it small and declarative; per-family quirks live in
// adapters, not here.
//
// Fields:
//   submodule    repo-relative path to the vendored upstream submodule
//   pinnedRef    the submodule tag we capture against (must match port-targets.json)
//   license      SPDX id of the upstream (gates whether converted fixtures may be persisted)
//   copyright    upstream copyright holder, recorded in corpus provenance + NOTICE
//   testGlob     repo-relative glob of rule test files (CJS RuleTester suites)
//   excludeFiles basenames to skip BEFORE require() — used for tests with unsafe
//                import-time side effects (CLI spawn, fs mocks). Tracked in the ledger.
//   stubModules  module ids to replace with an inert stub at import time, for optional
//                deps that only feed out-of-scope (non-JS) language cases.

/** @type {Record<string, import('./types.js').PluginCaptureConfig>} */
const PLUGINS = {
  'eslint-plugin-eslint-comments': {
    submodule: 'upstream/eslint-plugin-eslint-comments',
    pinnedRef: 'v4.7.2',
    license: 'MIT',
    copyright: 'Toru Nagashima',
    testGlob: 'upstream/eslint-plugin-eslint-comments/tests/lib/rules/*.js',
    // no-unused-disable spawns the real eslint CLI via cross-spawn at test time;
    // capturing it requires a running ESLint binary and is order/fs dependent.
    // Ported behavior for it is covered by hand-authored cases + ledger DIV-ESC-001.
    excludeFiles: ['no-unused-disable.js'],
    // 7 of 9 suites build CSS-language valid cases behind `semver >= 9.6.0`, pulling
    // in @eslint/css purely for those cases. We stub it: CSS is out of oxlint's JS/TS
    // scope, and such cases are flagged `outOfScope` and never sent to the oracle.
    stubModules: ['@eslint/css'],
  },
};

module.exports = { PLUGINS };
