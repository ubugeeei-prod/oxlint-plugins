# @oxlint-plugins/oxlint-plugin-angular-eslint

Rust-backed Oxlint plugin port of `@angular-eslint/eslint-plugin` v22.0.0.

This package exposes the `@angular-eslint` plugin and a native API for scanning
Angular TypeScript source through the Rust implementation.

`no-input-rename` is an Oxc AST port covering `@Input()` aliases, `input()` and
`input.required()` signal aliases, and `inputs` metadata. It forwards the
upstream `allowedNames` option and preserves the selector-composition,
`aria-*`, and `hostDirectives` exceptions from angular-eslint v22.0.0. The
checked-in fixture replays all 46 authored valid and 35 authored invalid cases
from commit `7ee4556badebf8c140ffdefdd0b07b02820d5e96`.

The current native diagnostic ABI contains message data and locations, but not
fix or suggestion edit payloads. The plugin therefore exposes the upstream
`fixable`, `hasSuggestions`, schema, and message metadata while reporting
`noInputRename` diagnostics without edits. Fix/suggestion transport can be
added separately without changing this rule's detection semantics.
