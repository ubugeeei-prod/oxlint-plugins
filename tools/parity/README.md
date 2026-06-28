# Parity tooling

Mechanically proves that a ported oxlint rule behaves identically to the upstream ESLint
rule it replaces. See `docs/guides/parity-testing.md` for the full design.

```
tools/parity/
  capture/        Layer A+B: harvest upstream cases + run the upstream rule as an oracle
    core.cjs      shared primitives (normalize, oracle, self-validate, serialize)
    run.cjs       CLI for plain require()-loadable CJS upstreams: run.cjs <plugin-id> [--check]
    plugins.cjs   per-plugin capture config (test glob, exclusions, module stubs)
    *.capture.mjs vitest-hosted capture for ESM/transform-required upstreams
    vitest.config.ts  isolated config for the .capture.mjs specs
  corpora/        committed, machine-generated JSON ground truth (one file per rule)
    NOTICE        upstream attribution for the converted fixtures
  replay/
    runner.mjs    Layer C: drive a ported rule through oxlint/plugins-dev RuleTester
  divergences.json  known-divergences ledger (relaxes specific, reviewed expectations)
```

## Two capture paths

- **CJS upstreams** (`eslint-plugin-eslint-comments`): tests load with plain `require()`.
  `node tools/parity/capture/run.cjs <plugin-id>`.
- **ESM / transform-required upstreams** (`eslint-plugin-simple-import-sort`, and the TS
  plugins): tests use `import`, `require.resolve(parser)`, and vitest snapshot helpers that
  only work under a bundler transform. These run under vitest:
  `pnpm exec vitest run --config tools/parity/capture/vitest.config.ts`. The sink and oracle
  are identical (shared `core.cjs`); only the loader differs.

## Capture / regenerate a corpus

Requires `eslint` + the upstream plugin's runtime deps installed at the repo root (so the
vendored submodule rule can be `require()`d):

```sh
node tools/parity/capture/run.cjs eslint-plugin-eslint-comments
```

The harness monkeypatches `RuleTester.prototype.run` to harvest every fully-resolved case,
runs the real upstream rule through ESLint's `Linter` to materialize ground truth, and
**self-validates** that the oracle agrees with the upstream test's own assertion before it
writes anything. It aborts rather than freeze an unverified corpus.

`--check` regenerates into a scratch dir and fails if the committed corpora are stale (CI
drift gate; run after bumping a submodule pin).

## Replay

Each ported rule gets a vitest suite that loads its corpus and runs it through
`oxlint/plugins-dev` `RuleTester` (real oxc parser). Example:
`npm/eslint-comments/test/no-unlimited-disable.parity.test.mjs`.

Comparison asserts the rendered `message` (embeds interpolated data; most discriminating)
plus `line`/`endLine`/`endColumn`, and `column` unless the ledger suppresses it. `fixOutput`
is fed as RuleTester `output` so the port's fixer is applied and the resulting source string
is compared. Cases flagged `outOfScope` (non-JS language / custom parser) are skipped.

## Status

- `eslint-plugin-eslint-comments` — 8 corpora captured (176 cases, all self-validated).
  `no-unlimited-disable` is ported (pure JS) and replays green through oxlint's RuleTester
  with a single ledgered start-column divergence. Removing the ledger entry (asserting start
  column) or breaking the port both correctly turn the suite red — not false-green.
- `eslint-plugin-simple-import-sort` — **fully ported and replaying green**. `imports` (74) +
  `exports` (35) corpora captured via the vitest-hosted path; the faithful 1.2k-line port
  (`imports.js` + `exports.js` + `shared.js`, reused as-is because the oxc sourceCode/token/
  comment API is ESLint-compatible) reproduces **byte-exact autofix output on every replayable
  case**. Two valid cases that are valid only because an `eslint-disable*` directive suppresses
  the report are ledgered (oxlint's RuleTester does not apply inline disable directives).
- `@typescript-eslint/rule-tester` path (storybook/functional family) — capture mechanism
  proven by `capture/ts-rule-tester.test.mjs`: intercept its `run()` (after
  `core.installRuleTesterHooks` sets the `afterAll`/`describe`/`it` hooks it requires even at
  construction), oracle the real rule through `@typescript-eslint/parser` (TS/JSX), and
  materialize **suggestions** (messageId + applied output). Capturing the real TS plugins is
  **deferred**: their rules import heavy upstream runtimes (e.g. storybook rules import
  `storybook/internal/csf`), which must be installed before the oracle can run the genuine
  rule — stubbing those deps would corrupt the oracle. Replay of TS corpora uses oxlint
  `RuleTester` with `lang: 'ts' | 'tsx'`.
- Autofix replay path — proven by `npm/eslint-comments/test/fix-replay.test.mjs`: oxlint
  RuleTester compares the port's applied fix against the corpus `fixOutput`, honors per-case
  `recursive` for convergent multi-pass fixers (`!!!!x` → `x` over two passes), and goes red
  when the fix output diverges (no false-green on fixes). This path is now also exercised by
  the real `simple-import-sort` port above, which replays its full captured fix corpus
  byte-exactly.
