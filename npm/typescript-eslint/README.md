# @oxlint-plugins/oxlint-plugin-typescript-eslint

Rust-backed Oxlint port of [`@typescript-eslint/eslint-plugin`](https://github.com/typescript-eslint/typescript-eslint).

> **Status: scaffold.** No rules ported yet. The 134 upstream rules of `@typescript-eslint/eslint-plugin@8.61.0` are enumerated in [`status.json`](../../status.json) with `status: "pending"`. See [`docs/port-targets/typescript-eslint-eslint-plugin.md`](../../docs/port-targets/typescript-eslint-eslint-plugin.md) for the canonical rule inventory.

Type-aware ports must follow [`docs/guides/type-aware.md`](../../docs/guides/type-aware.md) and use `@corsa-bind/napi` through `@oxlint-plugins/oxlint-plugin-type-aware`. Per-node Corsa calls are forbidden; batch type queries.

Oxlint already provides native coverage for several `typescript-eslint` rules; the per-rule `oxlintBuiltin` flag in `status.json` records that overlap as ports proceed.

This is unofficial community work and is not an official Oxlint or typescript-eslint project.

## License

MIT. See [`LICENSE`](./LICENSE).
