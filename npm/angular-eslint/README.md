# @oxlint-plugins/oxlint-plugin-angular-eslint

Rust-backed Oxlint plugin port of `@angular-eslint/eslint-plugin` v22.0.0.

This package exposes the `@angular-eslint` plugin and a native API for scanning
Angular TypeScript source through the Rust implementation.

`consistent-component-styles` is an Oxc AST port of the upstream `string`
(default) and `array` modes. It distinguishes Component decorator metadata
from comments, strings, directives, and unrelated object properties, preserves
the four upstream diagnostic IDs and exact report spans, and exposes the
upstream schema, messages, and fixability metadata. The checked-in fixture
replays all 21 authored valid and 20 authored invalid cases from commit
`7ee4556badebf8c140ffdefdd0b07b02820d5e96`. An audit of v22.1.0 found no
semantic, fixture, or documentation drift; its only rule-source change is
formatting.

`no-input-rename` is an Oxc AST port covering `@Input()` aliases, `input()` and
`input.required()` signal aliases, and `inputs` metadata. It forwards the
upstream `allowedNames` option and preserves the selector-composition,
`aria-*`, and `hostDirectives` exceptions from angular-eslint v22.0.0. The
checked-in fixture replays all 46 authored valid and 35 authored invalid cases
from commit `7ee4556badebf8c140ffdefdd0b07b02820d5e96`.

The current native diagnostic ABI contains message data and locations, but not
fix or suggestion edit payloads. The plugin therefore exposes upstream
fixability and suggestion metadata where applicable, plus exact schemas and
messages, while reports remain location-only. Edit transport can be added
separately without changing these rules' detection semantics.
