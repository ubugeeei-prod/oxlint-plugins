# oxlint-plugins

Rust-backed Oxlint plugin workspace for porting ESLint plugins through NAPI-RS.

The public package shape is an Oxlint JS plugin. Hot rule logic lives in Rust and is exposed through NAPI-RS. All plugins share a single native addon (`@oxlint-plugins/core`); each `@oxlint-plugins/oxlint-plugin-*` package is a thin JavaScript facade that depends on it, so installing many plugins links the native code once instead of once per plugin. `@oxlint-plugins/oxlint` bundles the whole suite behind one config entry.

This is unofficial community work. It is not an official Oxlint project, and builtin migration should happen only through normal upstream review.

## Commands

```sh
nix develop
vp install
vp build
vp lint
vp fmt
vp test
vp run bench:check
vp run bench
cargo test --workspace --all-features
vp run new eslint-plugin-react/jsx-no-bind
vp run profile bench
vp run release patch
```

`vp run release major|minor|patch` bumps versions, verifies locally, commits, tags, and pushes. The tag triggers trusted publishing through GitHub Actions.

## Layout

- `crates/_carton`: shared allocation and fast-hash primitives.
- `crates/stylistic`: stylistic-domain Rust rule logic. Add future domains like `import`, `react`, or `security` instead of one crate per rule.
- `npm/core`: `@oxlint-plugins/core`, the single NAPI-RS native addon shared by every plugin. Holds only the NAPI boundary (one namespaced module per plugin); rule logic stays in the domain crates.
- `npm/oxlint`: `@oxlint-plugins/oxlint`, the convenience bundle that aggregates every plugin into one combined plugin with a recommended config.
- `npm/*`: individually installable npm packages. The `oxlint-plugin-*` packages are thin JavaScript facades over `@oxlint-plugins/core`; `type-aware` is a shared JS helper.
- `examples/*`: small usage examples outside the npm workspace graph.
- `docs/site`: ox-content + Void SDK website with rule status pages.
- `docs/guides`: project policy and contributor guides rendered by the website.
- `tools/tasks/*`: Node type-stripped TypeScript task scripts.
- `tools/vite/*`: Vite/Vite+ build helper files.
- `tools/license-exceptions.json`: audited license policy exceptions.
- `tools/port-targets.json`: manifest of the ESLint plugins we intend to port (single source of truth for rule enumeration and release tracking).
- `upstream/*`: upstream port-target sources vendored as shallow git submodules, pinned to each plugin's baseline version. For behavioral reference only; never copy upstream code without honoring its license.
- `docs/port-targets`: generated, per-plugin rule inventories. Run `pnpm run port:rules` to regenerate from the submodules.
- `.github/workflows`: Blacksmith CI and trusted publishing release workflow.

## Port Targets

`tools/port-targets.json` lists the ESLint plugins used by flyle-nexus that Oxlint does not yet support natively (`eslint-plugin-svelte` is excluded; it is handled by [rsvelte](https://github.com/baseballyama/rsvelte), and `eslint-plugin-vue` is excluded; it is handled by [vize](https://vizejs.dev/)). Their sources are vendored under `upstream/` as submodules.

```sh
git submodule update --init --depth 1   # fetch upstream sources
pnpm run port:rules                      # regenerate docs/port-targets/*
```

`pnpm run port:rules` enumerates every rule of each target straight from its submodule and fails if a plugin's rule count drifts from the manifest, so the porting backlog stays complete. See `docs/port-targets/README.md` for the generated inventory.

## Sample Plugin

`@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers` demonstrates the intended shape:

- JS wrapper uses `@oxlint/plugins` and `createOnce`.
- The package is a thin facade: it imports its native functions from `@oxlint-plugins/core` (`require('@oxlint-plugins/core').noForbiddenIdentifiers`) rather than shipping its own `.node`.
- The NAPI boundary lives in `npm/core/src/no_forbidden_identifiers.rs`, namespaced so exported names never collide across plugins; the file-level pre-scan runs there.
- Rust tests use `insta` snapshots.
- Vitest covers wrapper reports and skip behavior.

See `docs/guides/porting.md`, `docs/guides/testing-strategy.md`, and `docs/guides/trusted-publishing.md`.

## Credits

Credited to [@ubugeeei](https://github.com/ubugeeei), [@baseballyama](https://github.com/baseballyama), [Blacksmith](https://www.blacksmith.sh/), and [OpenAI](https://openai.com/).

## Motivation And Policy

See `docs/guides/motivation.md`, `docs/guides/governance.md`, and `docs/guides/license-compliance.md` before porting existing ecosystem rules.
Type-aware ports must also follow `docs/guides/type-aware.md`.
Performance policy lives in `docs/guides/performance.md` and is the highest-priority engineering constraint.
Environment setup is Nix-first; see `docs/guides/environment.md`.

## Docs Site

```sh
vp run docs:dev
vp run docs:build
vp run status:sync
```

The status page is generated from `status.json`.
