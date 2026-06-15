# @oxlint-plugins/oxlint-plugin-angular-eslint-template

Rust-backed Oxlint port of [`@angular-eslint/eslint-plugin-template`](https://github.com/angular-eslint/angular-eslint).

> **Status: scaffold.** No rules ported yet. The 39 upstream rules of `@angular-eslint/eslint-plugin-template@22.0.0` are enumerated in [`status.json`](../../status.json) with `status: "pending"`. See [`docs/port-targets/angular-eslint-eslint-plugin-template.md`](../../docs/port-targets/angular-eslint-eslint-plugin-template.md) for the canonical rule inventory.

## Scope

Angular template lint rules target Angular templates rather than JS/TS AST nodes. Oxlint does not currently route `.html` template files into JS plugins, so any future port of these rules will need either Oxlint language-plugin support or a pre-processing step on inline templates. Track upstream Oxlint capability before starting individual rule ports.

This is unofficial community work and is not an official Oxlint or Angular project.

## License

MIT. See [`LICENSE`](./LICENSE).
