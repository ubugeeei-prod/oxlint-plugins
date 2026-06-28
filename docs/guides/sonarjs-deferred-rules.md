# SonarJS port: deferred rules and feasibility notes

The `eslint-plugin-sonarjs` port (`npm/sonarjs`, issue #9) implements rules as a
Rust `Scanner` that has access to:

- the raw source text and comments,
- the Oxc **syntactic** AST (via `Visit` hooks), and
- **optional** Oxc semantic scoping (`scoping` / `nodes`): reference → declaration
  resolution and `symbol_is_mutated`, built only when a rule needs it.

It deliberately does **not** have a TypeScript type checker, a dataflow/CFG
engine, or filesystem access. Five of the original 269 rules cannot be faithfully
reproduced inside that envelope and are intentionally left unimplemented. This
note records _why_, and _what infrastructure_ each would actually need, so the
next person does not re-derive it.

> Reminder: upstream SonarJS is LGPL-3.0. All analysis below is from the public
> RSPEC behaviour only. See [`license-compliance.md`](./license-compliance.md).

## Deferred rules

| Rule                        | Key   | Blocked on                                               | Tractable path                               |
| --------------------------- | ----- | -------------------------------------------------------- | -------------------------------------------- |
| `no-reference-error`        | S3827 | Full runtime/ambient **globals set** + symbol resolution | **Type-aware (Corsa)** — clean               |
| `no-gratuitous-expressions` | S2589 | Symbolic execution (value/nullness along paths)          | CFG + SE, or a narrow **type-driven** subset |
| `no-dead-store`             | S1854 | **Liveness** dataflow over a CFG                         | Oxc CFG + a hand-written liveness pass       |
| `no-implicit-dependencies`  | S4328 | `package.json` dependency list (project/host)            | Corsa host, or a small scanner-side FS read  |
| `future-reserved-words`     | S1527 | **Parser mode** (strict module rejects the targets)      | Parse the file as a `Script` for this rule   |

## Available infrastructure

### Type information — `@corsa-bind/napi` (Corsa)

Corsa is a TypeScript-compiler binding (external npm dependency, pinned in the
pnpm catalog at `^0.43.0`; **not** part of this repo, so nothing here is a PR to
corsa-bind — it is only consumed). Today it is wired up **only** in the
`npm/type-aware` package (`parser_services.ts` exposes `getTypeAtLocation` /
`getSymbolAtLocation` / `getTypeChecker`). The `npm/sonarjs` package is pure Rust
and currently receives **no** type checker, so no sonarjs rule is type-aware yet.

Using Corsa for a sonarjs rule therefore means one of:

1. add the rule to `npm/type-aware` (Corsa already wired, but that package is the
   _typescript-eslint_ port — a sonarjs-keyed rule there is a category mismatch), or
2. introduce a Corsa-backed type-aware path into `npm/sonarjs` itself (keeps
   sonarjs rules in sonarjs, but is a new, unprecedented integration).

The real cost of the type-dependent rules is this placement/architecture
decision, not the Corsa API calls.

### Control-flow graph — Oxc

`oxc_semantic` builds an `oxc_cfg::ControlFlowGraph` (petgraph-based: basic
blocks + typed edges + instructions). Enable with `SemanticBuilder::with_cfg(true)`
and read via `semantic.cfg()`. Oxlint's built-in control-flow rules
(`no-fallthrough`, `no-unreachable`, `getter-return`, `no-this-before-super`,
`consistent-return`, …) are built on it.

Oxc provides the **CFG primitive only** — there is no general dataflow framework
(no liveness / reaching-definitions / nullness). A rule like `no-dead-store`
would obtain the CFG from Oxc and implement the backward liveness pass itself.
The sonarjs scanner already constructs a `Semantic` for scoping, so enabling
`with_cfg(true)` is the entry point.

## Per-rule assessment

- **`no-reference-error` (S3827)** — "read of an undeclared identifier that throws
  `ReferenceError`" is exactly symbol resolution. `getSymbolAtLocation` returning
  `undefined` (TS "Cannot find name"), with the full globals set (`lib.d.ts`,
  ambient declarations, project), is the canonical answer. **Feasible with Corsa**;
  an AST/scoping-only implementation would massively over-report. Best done on the
  type-aware side. (The TDZ sub-case is already covered by
  `no-variable-usage-before-declaration`.)

- **`no-gratuitous-expressions` (S2589)** — flags conditions whose value is already
  determined. Faithful detection needs SonarJS-style symbolic execution. Types
  cover only a narrow slice (a condition typed `never` / an always-truthy
  non-nullable value, comparisons of disjoint literal types). A type-driven subset
  is possible; full parity needs CFG + SE.

- **`no-dead-store` (S1854)** — a value assigned to a local and never read before
  being overwritten or going out of scope. This is **liveness**, not types. Needs
  Oxc's CFG plus a self-written live-variable analysis; closures, labels, and
  branches make a no-false-positive narrow subset hard without the real pass.

- **`no-implicit-dependencies` (S4328)** — imports of packages not declared in
  `package.json`. Not a type problem: it needs the dependency list. Reachable via
  Corsa's file host or a small filesystem read added to the scanner, but that is a
  project/host capability, not an AST or type query.

- **`future-reserved-words` (S1527)** — `scan_sonarjs` parses every file as a
  strict ES module, so future-reserved words (`await`, `let`, `static`, `yield`,
  `implements`, `enum`, …) are rejected by the parser _as bindings_ before any
  rule runs; the binding-based check can never fire. The fix is parser-mode, not
  type/dataflow: parse the source as a `Script` (sloppy mode) for this rule. Cheap,
  Rust-only, no Corsa.

## Suggested order

1. `future-reserved-words` — Rust-only, sloppy-parse switch; highest value/cost.
2. `no-reference-error` — Corsa; pending the placement decision above.
3. `no-dead-store` — its own change to stand up CFG + liveness in the sonarjs core.
4. `no-gratuitous-expressions` / `no-implicit-dependencies` — narrow subsets / host
   access; lower priority.
