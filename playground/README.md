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

Every implemented plugin runs in the playground. Two need extra plumbing:

- `eslint-comments` operates on comments rather than source text, so the adapter
  recovers the comment list and first-token position from an oxc parse, and
  feeds the other plugins' diagnostics to `no-unused-disable` as the file's lint
  problems.
- `postgresql` parses SQL with libpg_query, a C library with no
  `wasm32-unknown-unknown` build. The Rust core is feature-gated so the
  playground builds without it; the frontend parses `.sql` with
  `@libpg-query/parser` (the same libpg_query compiled via Emscripten) and
  passes the parse tree to the Rust rules. Its ~2 MB wasm is loaded lazily, only
  when a `.sql` file is linted.
