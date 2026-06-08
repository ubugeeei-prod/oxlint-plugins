# Motivation

This is unofficial community work. It is not an official Oxlint project.

The goal is to make the long tail of the ESLint plugin ecosystem available to Oxlint users without giving up performance.

Oxlint already implements many high-value rules natively in Rust. This workspace targets the remaining ecosystem:

- Rules that are valuable but not yet built into Oxlint.
- Plugin ecosystems with many specialized rules.
- Project-specific rules that should be fast enough to run everywhere.
- Candidate implementations that can later be proposed as Oxlint builtins.

The package boundary is pragmatic. Oxlint's public custom plugin API is currently JavaScript-compatible, so packages expose JavaScript plugins. Rule logic that benefits from native speed is implemented in Rust and called through NAPI-RS.

The long-term bar is native-quality behavior:

- Compatibility with the upstream ESLint rule.
- Snapshot-backed diagnostics.
- Clear documentation for every gap.
- No avoidable per-node overhead.
- A release process that can be audited through npm trusted publishing provenance.
