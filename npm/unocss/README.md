# @oxlint-plugins/oxlint-plugin-unocss

Rust-backed Oxlint plugin port of `@unocss/eslint-plugin`.

The JavaScript layer is an Oxlint/NAPI adapter. Class token scanning, ordering,
blocklist checks, and class-compile prefix checks run in Rust through Oxc.
