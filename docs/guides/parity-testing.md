# Upstream Parity Testing

How we guarantee that a Rust+NAPI oxlint port behaves identically to the original
upstream ESLint plugin it replaces.

This extends `docs/guides/testing-strategy.md`. It does not replace its layers; it adds a
new mandatory **Layer 0: upstream parity replay** and the tooling that feeds it.

## Thesis

We do not hand-author parity. For every ported rule we:

1. **Capture** the upstream test corpus at runtime (not by static parsing).
2. **Materialize ground truth** by executing the _real upstream rule_ as an oracle.
3. **Freeze** that into a versioned JSON corpus committed in-repo.
4. **Replay** it against the oxlint port through the real oxc parser, using `RuleTester`
   from `oxlint/plugins-dev`, which asserts the match and fails CI on any divergence.

Genuinely unavoidable differences (parser AST quirks, etc.) live in an explicit, reviewed
**divergences ledger**; everything else must match or CI is red.

## Architecture

```
A. CAPTURE  (Node, against the pinned upstream submodule)
   import upstream test files with patched tester/Linter sink
   → fully-resolved input cases (code / options / settings / languageOptions)
                         │
B. ORACLE   (Node, real ESLint Linter runs the real upstream rule)
   linter.verify / verifyAndFix → messageId, data, line/column, fixOutput, suggestions
   → emits a versioned JSON corpus committed under tools/parity/corpora/
                         │  (frozen corpus, in git)
C. REPLAY   (in-repo vitest, runs the built oxlint port)
   RuleTester from oxlint/plugins-dev → real oxc parse + visitor
   → RuleTester asserts each case; honor the divergences ledger
```

Capture (A) and Oracle (B) run **in separate Node processes** per plugin to avoid sink
self-pollution (see "Pitfalls"). Replay (C) is a separate vitest process that depends only
on the committed corpus + the built port — it never imports upstream or ESLint, so PR CI
needs no submodule checkout.

### Why dynamic capture, not static extraction

Upstream `valid`/`invalid` arrays are rarely static literals. They use `semver.satisfies`
version gating with spreads, `require()` of ESLint core rules, `dedent`/`String.raw`
template builders, `.map()` loops, function-computed `output`, and sidecar snapshot files
(`regexp` `.eslintsnap`, `functional` `.snap`). The concrete set of cases — and their
expected output — only exists _after the test file's JS has executed_.

So we capture at the tester boundary, where every value is already resolved:

- **Universal sink:** monkeypatch `Linter.prototype.verify` / `verifyAndFix` before importing
  any test file. Every tester family ultimately calls `Linter.verify`, so this records every
  `(code, resolvedConfig)` regardless of how the framework drove it.
- **Structured layer:** also patch `RuleTester.prototype.run` (and family equivalents) to
  record the as-authored `{valid, invalid}` grouping, names, and `only`, which the raw
  `verify` stream lacks.
- **Hook neutralization:** stub `describe`/`it`/`afterAll` so upstream assertions never fire
  and the ts-eslint "all cases ran" check never throws — we harvest inputs, not run their
  asserts.

Static AST extraction would require re-implementing a JS+TS interpreter to be correct:
strictly more work for strictly less fidelity. It is rejected as a primary strategy.

### Why the oracle layer is mandatory

Expected behavior is frequently missing from the test source itself: `errors: <number>`
count-only assertions (`simple-import-sort`, `security`, `eslint-comments`), plain-string
messages with no `messageId`, and snapshot suites with nothing inline. For each captured
input we run the real upstream rule through ESLint's `Linter` to obtain the authoritative
`{ messageId, message, data, line, column, endLine, endColumn, fix, suggestions }`. This:

- **fills every gap** — count-only and string-only cases get real messageIds, locations, fixes;
- **self-validates** — we assert the oracle output is _consistent with_ whatever the upstream
  test did assert (count matches; any asserted messageId/loc is a subset). If the oracle
  disagrees with the upstream's own inline assertion (e.g. wrong ESLint version), we **fail
  the capture build** rather than freezing garbage.

## How the port is compared: use RuleTester's native assertions

The only public test API in `oxlint/plugins-dev` is `RuleTester`. Its `run()` returns `void`
and **throws on mismatch** — there is no function that lints a string and returns diagnostics.
That is fine, because the verified `InvalidTestCase` surface already accepts everything we need
and asserts it internally:

```ts
interface InvalidTestCase {
  code: string;
  options?: unknown[];
  filename?: string;
  settings?: object;
  languageOptions?: LanguageOptions;
  errors:
    | number
    | Array<{
        messageId?: string;
        message?: string | RegExp;
        data?: object;
        line?: number;
        column?: number;
        endLine?: number;
        endColumn?: number;
        suggestions?: Array<{ messageId?: string; desc?: string; data?: object; output: string }>;
      }>;
  output?: string | null;
}
// Config also supports: recursive?: boolean | number  (fix passes; true = 10)
//                       eslintCompat?: boolean         (1-based column, loc end===start)
```

So replay is: translate each corpus case into this shape, feed the **oracle-derived expected
values**, and let RuleTester assert. **Do not build a custom diagnostic differ or a `rangeText`
scheme** — RuleTester already compares `messageId`, `data`, `line`/`column`/`endLine`/`endColumn`,
`output`, and `suggestions` for us.

- **Location is verified** through RuleTester's own `line`/`column` checks. Set
  `eslintCompat: true` so the column base and `loc`-only `end === start` match ESLint.
- **Autofix is verified** by feeding `output = fixOutput` (post-application full source). Never
  compare fix `{range,text}` objects — oxc byte offsets vs ESLint UTF-16 offsets are not
  comparable; compare the _resulting source string_.
- **Multi-pass fixes**: the oracle uses `verifyAndFix` (multi-pass). Record the pass behavior
  and set RuleTester `recursive` to match (default is single-pass), or false-failures appear on
  rules whose fix needs multiple passes (`simple-import-sort`, sort rules).
- **String-only rules** (`security`, `eslint-comments`, `postgresql`) emit no `messageId`. For
  these the port **must** reproduce byte-identical `message` text, and `message` becomes the
  compared key — a hard requirement, not a divergence.
- **Suggestions** are compared as `{ messageId | desc, data, output }`, where `output` is the
  source after applying the suggestion. Suggestion fix ranges are never compared.

Where oxc and ESTree genuinely disagree on `line`/`column` (column base aside), the case goes
into the divergences ledger rather than being silently dropped from comparison.

## Corpus schema

One file per rule: `tools/parity/corpora/<plugin-id>/<rule>.json`, committed, machine-generated.
Validated in CI against `tools/parity/schema/corpus.v1.json`.

```jsonc
{
  "corpusVersion": 1,
  "provenance": {
    "plugin": "eslint-plugin-eslint-comments",
    "pinnedRef": "v4.7.2",
    "submoduleSha": "…",
    "eslintVersion": "9.x", // oracle ESLint version (self-validation depends on it)
    "license": "MIT",
    // NOTE: no capturedAt / capturedNode — non-deterministic fields break `--check` (see Drift)
  },
  "languageDefaults": {
    "sourceType": "module",
    "parserOptions": {
      /* … */
    },
  },
  "cases": [
    {
      "kind": "invalid", // or "valid"
      "code": "/*eslint-disable */",
      "options": [],
      "filename": "file.js",
      "settings": {},
      "languageOptions": {},
      "expectedErrors": [
        {
          "messageId": "disabled", // null if the rule emits no messageId (string-only)
          "message": "…", // compared only when messageId is null; else advisory
          "data": { "…": "…" },
          "line": 1,
          "column": 1,
          "endLine": 1,
          "endColumn": 20,
          "suggestions": [
            { "messageId": "…", "data": {}, "output": "<full source after this suggestion>" },
          ],
        },
      ],
      "fixOutput": "<full source after verifyAndFix>", // null ⇒ no fix
      "recursive": false, // mirror the upstream pass behavior for this case
      "upstreamAssertion": { "style": "count|plain-string|messageId|snapshot|location" },
    },
  ],
}
```

- `valid` cases have `expectedErrors: []` and `fixOutput: null`.
- `fixOutput` and suggestion `output` are **post-application full source strings**, never fix
  ranges.
- `corpusVersion` bumps on any schema change; the replay runner refuses a mismatched version.
  Bump deliberately — it forces regenerating every plugin's corpus at once.

## Pipeline

- **Capture+oracle:** `tools/parity/capture/run.mjs <plugin-id>` imports the upstream test files
  (with the patched sink), resolves cases, runs each through the oracle Linter, runs the
  self-validation assertions, and emits one corpus file per rule. It **aborts the whole plugin**
  if any oracle/assertion mismatch — never writes a half-trusted corpus. Per-family intake
  adapters live in `tools/parity/capture/adapters/`.
- **Replay:** `tools/parity/replay/<plugin>.parity.test.ts` (generated, one suite per ported
  rule) loads the committed corpus and drives `RuleTester` from `oxlint/plugins-dev`. Hermetic,
  fast, part of `pnpm run verify` on every PR.

## License matrix

| Plugin                           | License      | Persist converted corpus?           | Notes                                                                                                                                     |
| -------------------------------- | ------------ | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| eslint-plugin-eslint-comments    | MIT          | Yes                                 | NOTICE entry + `provenance.license`.                                                                                                      |
| eslint-plugin-functional         | MIT          | Yes                                 | NOTICE entry.                                                                                                                             |
| eslint-plugin-regexp             | MIT          | Yes                                 | NOTICE entry.                                                                                                                             |
| eslint-plugin-simple-import-sort | MIT          | Yes                                 | NOTICE entry.                                                                                                                             |
| eslint-plugin-storybook          | MIT          | Yes                                 | NOTICE entry.                                                                                                                             |
| @unocss/eslint-plugin            | MIT          | Yes                                 | NOTICE entry.                                                                                                                             |
| eslint-markdown                  | MIT          | Yes                                 | Language plugin — see "Out of replay scope".                                                                                              |
| eslint-plugin-postgresql         | MIT          | Yes                                 | SQL parser — see "Out of replay scope".                                                                                                   |
| eslint-plugin-security           | Apache-2.0   | Yes                                 | Apache §4: carry LICENSE, retain notices, **state modifications** per modified file ("derived/converted"). Upstream ships no NOTICE file. |
| eslint-json                      | Apache-2.0   | Yes                                 | Language plugin — see "Out of replay scope".                                                                                              |
| eslint-plugin-sonarjs (SonarJS)  | **disputed** | **Hold — legal review in progress** | See below.                                                                                                                                |

For every "Yes" row: a single `tools/parity/corpora/NOTICE` enumerates each upstream repo,
license, copyright holder, pinned SHA, and that fixtures were converted/derived under that
license; per-corpus `provenance` duplicates the essentials. MIT and Apache both require the
license text + copyright notice to travel with substantial copies — an SPDX string alone is not
enough.

### SonarJS — handle separately

`upstream/SonarJS/LICENSE.txt` is the **Sonar Source-Available License (SSAL)**, while its
`package.json` says `LGPL-3.0-only` — a genuine conflict. SSAL carries a field-of-use clause
that can reach a product "ported to a different platform or programming language" acting as a
substitute for SonarQube, **even when free**. That is a restriction on _use_, which clean-room
does not cure. Per maintainer direction this is **under legal review and provisionally proceeding**.
Until it clears:

- Keep SonarJS **out of the persisted-corpus path**. Run capture/oracle only in a throwaway,
  gitignored scratch dir; a CI check fails if any `corpora/eslint-plugin-sonarjs/**` file is ever
  committed.
- The committed parity artifact is **hand-authored independent cases** under
  `tools/parity/handwritten/sonarjs/` (our own code, our own expectations), never upstream text.
- Fix `tools/port-targets.json` to record the SSAL/LGPL conflict and the hold status.

## Out of replay scope (for now)

`oxlint/plugins-dev` `RuleTester` only parses JS dialects (`lang: "js"|"jsx"|"ts"|"tsx"|"dts"`,
no custom parser). The language/parser plugins therefore **cannot** run Layer-0 replay:

- **eslint-json** (momoa JSON parser), **eslint-markdown** (mdast/gfm) — already noted as
  outside oxlint's JS/TS scope in `port-targets.json`.
- **eslint-plugin-postgresql** (postgresql-eslint-parser, SQL in template literals).

Track these as future language targets. Their corpora can still be _captured_ for the day a
language-aware harness exists, but they are excluded from the per-PR replay gate until then.

## Known divergences ledger

`tools/parity/divergences.json` (machine-enforced) + `tools/parity/DIVERGENCES.md` (human).
Every entry is an explicit allowlisted exception naming specific `caseId`s; the replay runner
converts those cases from "must match" to known-divergent (xfail). **An xfail that starts
passing also fails CI**, so the ledger cannot rot into permanent green-by-omission. Admissible
classes only (anything else requires a code fix, not a ledger entry):

1. `parser-ast-quirk` — node `type` names / AST shape differences (oxc vs ESTree/momoa).
2. `location-coordinate` — a specific `line`/`column` that genuinely cannot match.
3. `core-rule-dependency` — cases that require an ESLint core rule (e.g.
   `eslint-comments/no-restricted-disable` wiring `no-undef`), captured as `oracle: "manual"`.

Each entry carries an `expires` date and a `ledgerReviewedSha`; entries are reviewed in every
porting PR.

## Drift control

- Corpora regenerate when a submodule pin moves. `tools/parity/capture/run.mjs --all --check`
  regenerates into a scratch dir and `git diff`s — CI fails if a committed corpus would change.
  **Capture output must be byte-deterministic**: no wall-clock/Node-version fields in the schema,
  sorted object keys, stable case ordering, and pin the oracle ESLint version per plugin.
- A per-PR `parity:audit` compares the live submodule's _case-identity set_ against the committed
  corpus and **fails (not warns)** if a corpus shrank or upstream grew uncovered cases — so a sink
  that silently stops intercepting after an upstream refactor is caught, not masked.
- Tie regeneration into the existing `port:rules` / upstream-release-tracking tooling.

## Pitfalls (learned from design review)

- **Sink self-pollution:** `verifyAndFix` calls `verify` internally up to 10×; if capture and
  oracle share a process with the sink installed, intermediate fix passes get recorded as fake
  inputs. Run them in separate processes / phases.
- **Monkeypatch realm:** each submodule may resolve its own `eslint`. Patch the `Linter` the test
  file actually imports (resolve `eslint` from the test file's location), or the sink silently
  records nothing.
- **Side-effecting tests:** some upstream tests spawn the `eslint` CLI (`eslint-comments`
  `no-unused-disable`) or mock `fs` (`storybook` `no-uninstalled-addons`). Apply per-rule
  exclusions _before_ import; mark them `oracle: "manual"` and cover via the ledger.
- **ESM named imports** (`import { it } from "vitest"`) can't be intercepted by assigning
  `globalThis.it`; the imperative families (`functional`, `unocss`) need a runner-driven adapter,
  and the universal `Linter.verify` sink is what makes them capturable at all.

## Rollout

| Phase | Target                                                                            | Proves                                                    |
| ----- | --------------------------------------------------------------------------------- | --------------------------------------------------------- |
| 0     | `tools/parity/` skeleton + one rule end-to-end                                    | the capture→oracle→replay loop                            |
| 1     | **eslint-plugin-eslint-comments** (MIT, pure JS, core RuleTester)                 | messageId + fix + options; exclude the CLI-spawn rule     |
| 2     | eslint-plugin-simple-import-sort                                                  | multi-pass `recursive` fixes + count-only→oracle backfill |
| 3     | security (Apache) / functional, storybook, unocss, regexp (TS + snapshot testers) | TS transpile + imperative/snapshot adapters               |
| —     | json, markdown, postgresql                                                        | deferred (language/parser plugins)                        |
| —     | SonarJS                                                                           | hold for legal review; hand-authored cases only           |

Recommended first prototype: **eslint-plugin-eslint-comments** — small, pure JS, MIT (persistable),
exercising messageId, fixes, and options in one pass.
