# @oxlint-plugins/oxlint-plugin-postgresql

Rust-backed Oxlint plugin port of `eslint-plugin-postgresql` v0.22.1.

The current port scans SQL source text in Rust through NAPI. Oxlint does not
parse `.sql` files as a first-class language target yet, so the package exposes
the native scanner API and an ESLint/Oxlint-compatible rule adapter for
environments that provide SQL source text to ESLint.

## Rules

Implemented rules:

- `postgresql/consistent-identity-over-serial`
- `postgresql/consistent-jsonb-over-json`
- `postgresql/consistent-text-over-varchar`
- `postgresql/consistent-timestamptz`
- `postgresql/no-char-type`
- `postgresql/no-cluster`
- `postgresql/no-create-role`
- `postgresql/no-cross-join`
- `postgresql/no-drop-database`
- `postgresql/no-drop-schema-cascade`
- `postgresql/no-drop-table-cascade`
- `postgresql/no-equality-with-null`
- `postgresql/no-grant-all`
- `postgresql/no-grant-to-public`
- `postgresql/no-money-type`
- `postgresql/no-natural-join`
- `postgresql/no-not-in-subquery`
- `postgresql/no-select-into`
- `postgresql/no-select-star`
- `postgresql/no-set-search-path`
- `postgresql/no-temporary-table`
- `postgresql/no-time-type`
- `postgresql/no-truncate-cascade`
- `postgresql/no-unlogged-table`
- `postgresql/no-vacuum-full`
- `postgresql/prefer-cast-operator`
- `postgresql/prefer-current-timestamp-over-now`
- `postgresql/prefer-not-equals-operator`
- `postgresql/require-trailing-semicolon`
- `postgresql/require-where-in-delete`
- `postgresql/require-where-in-update`

## API

```js
const { scanPostgresql } = require('@oxlint-plugins/oxlint-plugin-postgresql/api');

const diagnostics = scanPostgresql('SELECT * FROM users;', 'query.sql');
```

## Attribution

Rule behavior, diagnostic message strings, and fixture ideas are derived from
`eslint-plugin-postgresql` (v0.22.1, MIT). See `LICENSE`.
