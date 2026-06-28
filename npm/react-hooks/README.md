# @oxlint-plugins/oxlint-plugin-react-hooks

Rust-backed Oxlint port of [`eslint-plugin-react-hooks`](https://github.com/facebook/react/tree/main/packages/eslint-plugin-react-hooks).

> **Status: scaffold.** No rules ported yet. `rules-of-hooks` and `exhaustive-deps` are already provided by Oxlint's native `react` plugin (`react/rules-of-hooks`, `react/exhaustive-deps`) and are therefore out of scope here. The remaining React Compiler lint categories are enumerated in [`status.json`](../../status.json) with `status: "pending"` so the porting backlog is fully visible. See [`docs/port-targets/eslint-plugin-react-hooks.md`](../../docs/port-targets/eslint-plugin-react-hooks.md) for the canonical rule inventory generated from the pinned `eslint-plugin-react-hooks@7.1.1` submodule.

This is unofficial community work and is not an official Oxlint or React project.

## License

MIT. See [`LICENSE`](./LICENSE).
