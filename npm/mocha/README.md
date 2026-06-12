# @oxlint-plugins/oxlint-plugin-mocha

Rust-backed Oxlint plugin port of `eslint-plugin-mocha`.

The JavaScript layer is an Oxlint/NAPI adapter. Rule parsing and Mocha AST checks
run in Rust through Oxc.

## Rules

- `mocha/consistent-interface`
- `mocha/consistent-spacing-between-blocks`
- `mocha/handle-done-callback`
- `mocha/max-top-level-suites`
- `mocha/no-async-suite`
- `mocha/no-empty-title`
- `mocha/no-exclusive-tests`
- `mocha/no-exports`
- `mocha/no-global-tests`
- `mocha/no-hooks`
- `mocha/no-hooks-for-single-case`
- `mocha/no-identical-title`
- `mocha/no-mocha-arrows`
- `mocha/no-nested-tests`
- `mocha/no-pending-tests`
- `mocha/no-return-and-callback`
- `mocha/no-return-from-async`
- `mocha/no-setup-in-describe`
- `mocha/no-sibling-hooks`
- `mocha/no-synchronous-tests`
- `mocha/no-top-level-hooks`
- `mocha/prefer-arrow-callback`
- `mocha/valid-suite-title`
- `mocha/valid-test-title`
