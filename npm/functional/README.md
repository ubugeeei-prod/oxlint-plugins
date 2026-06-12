# @oxlint-plugins/oxlint-plugin-functional

Rust-backed Oxlint plugin port of `eslint-plugin-functional`.

The native scanner uses Oxc and reports all upstream rule ids. Rules that need TypeScript's checker are implemented as AST-safe checks for the violations that can be proven from syntax alone.
