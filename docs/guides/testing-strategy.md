# Testing Strategy

Agentic porting should bias toward more tests, not fewer.

Required layers for every real plugin or rule port:

1. Rust unit tests for the pure rule core.
2. `insta` snapshots for diagnostics, options, and edge cases.
3. Vitest tests for the JS Oxlint plugin wrapper.
4. LSP diagnostic and quick-fix fixture tests for editor-facing behavior.
5. Oxlint integration fixtures when the public JS plugin API supports the behavior.
6. Package dry-run checks for installable artifacts.
7. Production pnpm audit checks for publishable packages.
8. CI verification on Blacksmith before merge and again before tag publishing.

The CI contract is `pnpm run verify`. Keep workflow YAML thin: setup the runner, install with `vp install --frozen-lockfile`, then run the workspace verification script. This prevents local, PR, and release checks from drifting.

Performance-sensitive rules should include tests that prove file-level skipping works. Prefer a Rust pre-scan or batched Rust result over one NAPI call per AST node.

Do not use `std::collections::HashMap` or `std::collections::HashSet` in rule crates. Use `oxlint_plugins_carton::FastHashMap` or `FastHashSet` when a map is actually necessary.

## Upstream test syncing

For ports of plugins with their own test suites, capture the upstream cases
verbatim into committed JSON fixtures and replay them, so behavior tracks
upstream as the submodule is bumped (oxc-style test syncing):

- `pnpm run port:tests:eslint-comments` — classic ESLint `RuleTester` suites.
- `pnpm run port:tests:functional` — `eslint-vitest-rule-tester` suites
  (imperative `valid()`/`invalid()` calls). Each captured case is tagged
  `typeAware` based on the upstream config it ran under. Type-aware cases need
  TypeScript type information the syntax-only Rust port does not have, so the
  replay harness skips and counts them (no silent truncation) and asserts full
  parity per rule via a `FULL_PARITY` allowlist that grows one entry per rule PR.
- `pnpm run port:tests:unocss` — `eslint-vitest-rule-tester` declarative `run()`
  suites. Cases are grouped by block name (each `run()` call) and tagged with the
  detected parser (`vue`, `svelte`, `jsx`, `js`). Vue/svelte-parser blocks are
  always skipped (template languages); js/jsx-parser blocks are asserted and
  individual divergences are quarantined in `test/parity.json`.

Re-run the relevant sync script after bumping a submodule and commit the
regenerated fixtures.
