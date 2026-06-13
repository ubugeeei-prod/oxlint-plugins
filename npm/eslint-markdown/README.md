# @oxlint-plugins/oxlint-plugin-eslint-markdown

Rust-backed Oxlint plugin port of `@eslint/markdown`.

The native core scans Markdown source text and implements the upstream Markdown
rules through a NAPI adapter. Oxlint 1.68 does not currently route `.md` files
to `jsPlugins`, so integration coverage is limited to the direct ESLint
compatible adapter and native API.
