# @oxlint-plugins/oxlint-plugin-react

Rust-backed Oxlint port of [`eslint-plugin-react`](https://github.com/jsx-eslint/eslint-plugin-react).

> **Status: scaffold.** No rules ported yet. Each upstream rule is enumerated in [`status.json`](../../status.json) with `status: "pending"` so the porting backlog is fully visible. See [`docs/port-targets/eslint-plugin-react.md`](../../docs/port-targets/eslint-plugin-react.md) for the canonical rule inventory generated from the pinned `eslint-plugin-react@7.37.5` submodule.

This is unofficial community work and is not an official Oxlint or `jsx-eslint` project.

## Porting

Each rule is ported behind the public Oxlint JS plugin API; see [`docs/guides/porting.md`](../../docs/guides/porting.md) and [`docs/guides/performance.md`](../../docs/guides/performance.md). Rule logic lives in a domain Rust crate (`crates/react`) exposed through NAPI-RS. Hot-path constraints (`CompactString`, `SmallVec`, `phf`, arena allocators, file-level pre-scan) apply.

## License

MIT. See [`LICENSE`](./LICENSE).
