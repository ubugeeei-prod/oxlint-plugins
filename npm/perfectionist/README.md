# @oxlint-plugins/oxlint-plugin-perfectionist

Rust-backed Oxlint plugin port of `eslint-plugin-perfectionist`.

The JavaScript layer is an Oxlint/NAPI adapter. Representative sorting checks
for imports, exports, object members, types, JSX props, collections, modules,
and declarations run in Rust through Oxc.

## `sort-array-includes`

The rule implements the `eslint-plugin-perfectionist` v5.10.0 option contract
for array literals and `new Array(...)` expressions immediately followed by a
non-computed `.includes(...)` call.

Supported options include alphabetical, natural, line-length, custom,
subgroup, and unsorted ordering; fallback sorts; locale, case, special
character, and custom alphabet handling; groups and custom groups; comment and
newline partitions; spacing policies; conditional configuration; and shared
`settings.perfectionist` defaults. Fixes preserve comments, sparse-array and
spread boundaries, UTF-16 offsets, and CRLF text.

React-specific behavior and JSX/TSX syntax are intentionally outside this
rule's port.

## `sort-sets`

The rule implements the `eslint-plugin-perfectionist` v5.10.0 option contract
for array literals and `new Array(...)` expressions used as the first argument
of `new Set(...)`.

It shares the complete comparator, grouping, custom-group, partition, newline,
conditional-configuration, settings, diagnostic, and fixer engine with
`sort-array-includes`. Set-specific AST matching ignores calls, non-`Set`
constructors, non-array inputs, and arrays outside the first argument. Fixes
preserve comments, sparse-array and spread boundaries, UTF-16 offsets, and
CRLF text.

React-specific behavior and JSX/TSX syntax are intentionally outside this
rule's port.
