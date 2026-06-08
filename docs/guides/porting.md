# Porting Workflow

Generate an AI handoff prompt:

```sh
vp run new eslint-plugin-react
vp run new eslint-plugin-react/jsx-no-bind
vp run new @typescript-eslint/eslint-plugin/no-explicit-any
```

The prompt is written to `prompts/generated/` and is intentionally ignored by Git.

Each port should create one independently installable npm package under `npm/`. Rust rule logic belongs in domain crates such as `crates/stylistic`, `crates/import`, `crates/react`, or `crates/security`; do not create one Rust crate per rule. Shared performance primitives belong in `crates/_carton`.

Use the current public Oxlint JS plugin API as the package boundary. Rust native builtin proposals for Oxlint should be prepared only after behavior is covered with snapshots and documented parity notes.

Before copying any upstream fixture, message, or docs excerpt, read `docs/guides/license-compliance.md`.

For type-aware rules, use `@corsa-bind/napi` through `@oxlint-plugins/oxlint-plugin-type-aware`. Do not create one-off TypeScript compiler integrations inside rule packages.
