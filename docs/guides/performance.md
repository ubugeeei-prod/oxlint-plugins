# Performance

Performance is the primary product requirement.

Every rule port should be designed as if it will run on a very large monorepo on every commit.

## Required Shape

- Prefer one Rust call per file over one Rust call per AST node.
- Use `createOnce` and `before` to build per-file state.
- Return `false` from `before` when the rule can skip a file.
- Batch Corsa calls for type-aware rules.
- Keep NAPI payloads small and stable.
- Avoid allocations in hot paths.
- Use `CompactString` for owned short strings in rule state.
- Use `SmallVec` for small bounded collections.
- Use `GhostCell`, `GhostToken`, `BumpAllocator`, `ArenaVec`, and `ArenaString` from `oxlint_plugins_carton` for short-lived or reusable per-file rule data.
- Use `phf` for static string lookup tables.
- Use `profile!`, `Profiler`, and `ProfilingAllocator` from `oxlint_plugins_carton` for local performance investigations.
- Do not use slow hash collections.
- Release and bench builds use fat LTO.

## Banned By Default

- `std::collections::HashMap`
- `std::collections::HashSet`
- `std::collections::BTreeMap`
- `std::collections::BTreeSet`
- Owned `String` in rule state. Use `CompactString`; NAPI/public ABI boundaries are the only default exception.
- Owned `Vec<T>` for small bounded rule state. Use `SmallVec`; NAPI/public ABI boundaries are the only default exception.
- `to_string` in Rust rule code. It hides owned string allocation; use `CompactString::from`, borrowed data, or a documented ABI-boundary conversion.
- `mod.rs`. Use named module files so rule/domain boundaries stay explicit.
- Per-node Corsa calls.
- Per-node NAPI calls unless the rule proves there is no viable batch or pre-scan shape.

## Benchmarking

Add a Rust benchmark for each non-trivial rule core:

```sh
vp run bench
```

CI compiles benchmarks with:

```sh
vp run bench:check
```

Pull requests run the benchmark workflow on Blacksmith and update a PR comment with the latest benchmark output for same-repository PRs.

Benchmark results are not a substitute for correctness snapshots. A port needs both.
