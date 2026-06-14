# @oxlint-plugins/postgresql-eslint-parser

A Rust-backed port of [`postgresql-eslint-parser`](https://github.com/baseballyama/postgresql-eslint-parser)
(MIT) — an ESLint custom parser for PostgreSQL SQL. SQL is parsed with
[`libpg_query`](https://github.com/pganalyze/libpg_query) (the real PostgreSQL 17
parser, statically linked through the [`pg_query`](https://crates.io/crates/pg_query)
crate) and enriched into the same ESLint-shaped AST the upstream TypeScript
parser produces.

## Usage

```js
const parser = require('@oxlint-plugins/postgresql-eslint-parser');

const { ast, visitorKeys, scopeManager } = parser.parseForESLint('SELECT * FROM users;');
// ast.type === 'Program'; ast.body[0].type === 'SelectStmt'
```

As an ESLint custom parser:

```js
// eslint.config.js
const pgParser = require('@oxlint-plugins/postgresql-eslint-parser');

module.exports = [
  {
    files: ['**/*.sql'],
    languageOptions: { parser: pgParser },
  },
];
```

The public API mirrors upstream:

- `parseForESLint(code)` → `{ ast, visitorKeys, scopeManager }` (`scopeManager`
  is always `null`, matching upstream).
- `parse(code)` → the `Program` AST node.

## Supported platforms

`libpg_query` builds on Linux and macOS only, so prebuilt binaries are published
for `x86_64`/`aarch64` Linux (glibc) and macOS. Windows is intentionally not
supported (upstream `libpg_query` does not target it).

## Attribution

Ported from `postgresql-eslint-parser@0.5.4` (MIT, © Yuichiro Yamashita). See
[`LICENSE`](./LICENSE).
