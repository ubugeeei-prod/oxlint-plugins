# oxlint-plugins playground

A browser playground that runs the Rust-backed oxlint plugin rules entirely
client-side. Paste code, toggle rules per package, and see diagnostics inline as
you type. Useful for trying the rules and for sharing a reproduction in a bug
report.

## How it works

- Every wasm-compatible plugin's core Rust crate is aggregated into one
  WebAssembly module (`crates/playground_wasm`) compiled with `wasm-pack`. The
  same rule logic that ships in the npm packages runs in the browser, so the
  diagnostics match the published plugins.
- `scripts/build-catalog.mjs` generates `src/catalog.json` with each rule's
  description and message templates, read from the npm plugins' `index.js` (and,
  for stylistic, from the WASM module since it renders messages in Rust).
- The UI (`src/main.ts`) uses CodeMirror for the editor and inline error
  markers, picks the language from the file extension, and scopes each plugin to
  matching files (so the JSON plugin only runs on `.json`, and so on).
- The current code, file name, and disabled rules are stored in the URL hash, so
  a link reproduces the exact state.

## Develop

```sh
pnpm --filter @oxlint-plugins/playground dev
```

`dev` and `build` first run `build:wasm` (wasm-pack) and `build:catalog`. Build
the static site with:

```sh
pnpm --filter @oxlint-plugins/playground build
```

The output in `dist/` is deployed to GitHub Pages by
`.github/workflows/deploy-playground.yml`.

## Coverage

Every plugin whose rule logic compiles to WebAssembly is included. Two are not:

- `postgresql` depends on the native `libpg_query` C library, which does not
  compile to `wasm32`.
- `eslint-comments` operates on comment tokens and disable-directive results
  supplied by the full lint run rather than on source text, so several of its
  rules cannot be reproduced standalone in the browser.
