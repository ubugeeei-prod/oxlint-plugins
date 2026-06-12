# @oxlint-plugins/oxlint-plugin-simple-import-sort

Rust-backed Oxlint plugin port of `eslint-plugin-simple-import-sort`.

## Rules

- `simple-import-sort/imports`
- `simple-import-sort/exports`

The native implementation sorts import/export chunks and named specifiers and
returns autofix ranges through the Oxlint JavaScript plugin adapter. It covers
the common module-sorting path; the upstream plugin's exhaustive comment
printer is not duplicated yet.
