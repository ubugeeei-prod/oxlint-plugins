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
