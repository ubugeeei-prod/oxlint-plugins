# oxlint-plugins

Rust-backed Oxlint plugin workspace for porting ESLint plugins through NAPI-RS.

The public package shape is an Oxlint JS plugin. Hot rule logic lives in Rust and is exposed through NAPI-RS so each plugin can be installed independently from npm.

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
- `npm/*`: individually installable npm packages, including oxlint plugins and shared JS helpers.
- `examples/*`: small usage examples outside the npm workspace graph.
- `docs/site`: ox-content + Void SDK website with rule status pages.
- `docs/guides`: project policy and contributor guides rendered by the website.
- `tools/tasks/*`: Node type-stripped TypeScript task scripts.
- `tools/vite/*`: Vite/Vite+ build helper files.
- `tools/license-exceptions.json`: audited license policy exceptions.
- `.github/workflows`: Blacksmith CI and trusted publishing release workflow.

## Sample Plugin

`@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers` demonstrates the intended shape:

- JS wrapper uses `@oxlint/plugins` and `createOnce`.
- Rust performs a file-level pre-scan through NAPI-RS.
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
