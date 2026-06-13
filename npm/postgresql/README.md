# @oxlint-plugins/oxlint-plugin-postgresql

Rust-backed Oxlint port of [`eslint-plugin-postgresql`](https://github.com/baseballyama/eslint-plugin-postgresql) — lints SQL embedded in template literals.

> **Status: scaffold.** No rules ported yet. The 89 upstream rules of `eslint-plugin-postgresql@0.22.1` are enumerated in [`status.json`](../../status.json) with `status: "pending"`. See [`docs/port-targets/eslint-plugin-postgresql.md`](../../docs/port-targets/eslint-plugin-postgresql.md) for the canonical rule inventory.

## Parser dependency

This plugin depends on the `postgresql-eslint-parser` port-target (no lint rules of its own, tracked separately in [`tools/port-targets.json`](../../tools/port-targets.json)). The parser must be available before any rule in this package can be ported.

## Porting

Rule logic lives in a domain Rust crate (`crates/postgresql`) exposed through NAPI-RS. Hot-path constraints (`CompactString`, `SmallVec`, `phf`, arena allocators, file-level pre-scan) apply. Type-aware rules must follow [`docs/guides/type-aware.md`](../../docs/guides/type-aware.md).

This is unofficial community work.

## License

MIT. See [`LICENSE`](./LICENSE).
