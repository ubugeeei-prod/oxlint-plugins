# @oxlint-plugins/oxlint-plugin-cypress

Rust-backed Oxlint plugin port of `eslint-plugin-cypress`.

The JavaScript layer is an Oxlint/NAPI adapter. Rule parsing and Cypress AST
checks run in Rust through Oxc.
