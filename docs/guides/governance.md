# Governance

This repository is unofficial community work. Do not present packages, rules, or migration proposals as official Oxlint work unless upstream has accepted them.

## Rule Porting Policy

Ports should start from observed upstream behavior, not from a loose rewrite of the README.

For each plugin or rule:

1. Identify the upstream package, version, license, docs, and tests.
2. Check whether Oxlint already has a native rule.
3. Decide whether the port should implement the rule, delegate to a builtin, or document that it is unnecessary.
4. Keep the npm ruleset package independently installable.
5. Add enough tests for an AI agent to keep changing the rule without eroding behavior.

## Performance Policy

Performance is a feature, not a cleanup item.

- Do not use `std::collections::HashMap` or `std::collections::HashSet` in rule code.
- Prefer `oxlint_plugins_carton::FastHashMap`, `FastHashSet`, `CompactString`, `SmallVec`, `GhostCell`, `GhostToken`, `BumpAllocator`, `ArenaVec`, and `ArenaString`.
- Treat `String` and `Vec<T>` as public ABI types only. Compact them at the NAPI boundary before rule logic runs.
- Do not use `to_string` in rule code.
- Use `phf` for static string lookup tables.
- Do not add `mod.rs`; use named module files.
- Avoid allocations in hot paths.
- Avoid one NAPI call per AST node when a per-file pre-scan or batched result is possible.
- Use `createOnce`, `before`, and `after` so per-file state is explicit and reusable.
- If a rule must allocate or use a slower data structure, document why near the implementation and cover it with tests.

## Review Policy

PR titles must be conventional and must not mention Codex.

Before merge, CI must pass. If GitHub Actions are configured, prefer Actions results as the final verification source.

## Release Policy

Releases are tag-driven:

```sh
vp run release major
vp run release minor
vp run release patch
```

The release workflow uses Blacksmith for verification and GitHub-hosted runners only for npm trusted publishing, because npm's trusted publisher support is limited to GitHub-hosted runners for GitHub Actions.

## Status Policy

Every package and rule must have a status entry in `status.json`. The website renders this status file as the status page.

Run this after publishing or unpublishing packages:

```sh
vp run status:sync
```
