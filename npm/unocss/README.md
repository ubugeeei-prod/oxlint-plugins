# @oxlint-plugins/oxlint-plugin-unocss

Rust-backed Oxlint plugin port of `@unocss/eslint-plugin`.

The JavaScript layer is an Oxlint/NAPI adapter. Class token scanning, ordering,
blocklist checks, and class-compile prefix checks run in Rust through Oxc.

## Scope and divergence from `@unocss/eslint-plugin`

This is a syntax-only port that does **not** load or invoke the UnoCSS engine,
so behavior differs from upstream:

- `order` / `order-attributify` sort classes with a built-in **heuristic bucket
  order**, not the real `presetUno`-generated order; results may differ, and the
  heuristic only recognizes common utility shapes (so it ignores, rather than
  reorders, class strings it does not recognize).
- `blocklist` matches utility **names supplied via rule options / `settings.unocss.blocklist`**
  by exact token, instead of resolving them against your `uno.config` blocklist.
- Only JS/TS/JSX is analyzed; `.vue` / `.svelte` templates (which Oxc cannot
  parse) are out of scope.

Full engine-backed parity is tracked upstream and intentionally deferred.
