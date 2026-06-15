# @oxlint-plugins/oxlint-plugin-sonarjs

Rust-backed Oxlint port of [`eslint-plugin-sonarjs`](https://github.com/SonarSource/SonarJS).

> **Status: scaffold.** No rules ported yet. The 269 upstream rules whose `implementation === 'original'` (excluding three source-only assertion rules) are enumerated in [`status.json`](../../status.json) with `status: "pending"`. See [`docs/port-targets/eslint-plugin-sonarjs.md`](../../docs/port-targets/eslint-plugin-sonarjs.md) for the canonical inventory.

## Clean-room policy (LGPL-3.0)

The upstream SonarJS project is distributed under **LGPL-3.0**, which the workspace license policy blocks (`tools/tasks/check-license-policy.ts` rejects any `(L|A)GPL-` license). Every rule in this package MUST be implemented clean-room: behaviour is reproduced from public RSPEC documentation, observed output, and inferred specification only. You must NOT copy any of the following from upstream into this directory or `crates/`:

- source code or pseudocode
- diagnostic message strings
- test inputs or expected outputs
- fixtures
- internal helper functions
- comments

See [`docs/guides/license-compliance.md`](../../docs/guides/license-compliance.md) before opening any port PR against this package.

## Porting

Rule logic lives in a domain Rust crate (`crates/sonarjs`) exposed through NAPI-RS. Hot-path constraints (`CompactString`, `SmallVec`, `phf`, arena allocators, file-level pre-scan) apply.

## License

This package is MIT-licensed. The upstream SonarJS plugin is LGPL-3.0 and is not redistributed here. See [`LICENSE`](./LICENSE).
