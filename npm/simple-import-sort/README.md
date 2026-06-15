# @oxlint-plugins/oxlint-plugin-simple-import-sort

Rust-backed Oxlint plugin port of [`eslint-plugin-simple-import-sort`](https://github.com/lydell/eslint-plugin-simple-import-sort) — opinionated import/export sorting with autofixable diagnostics. The rule logic and sorting algorithm run in Rust through NAPI-RS; the JavaScript wrapper stays compatible with Oxlint's JS plugin API.

This is unofficial community work and is not an official Oxlint or eslint-plugin-simple-import-sort project.

## Usage

```jsonc
{
  "jsPlugins": [
    {
      "name": "simple-import-sort",
      "specifier": "@oxlint-plugins/oxlint-plugin-simple-import-sort",
    },
  ],
  "rules": {
    "simple-import-sort/imports": "error",
    "simple-import-sort/exports": "error",
  },
}
```

### `imports` rule — `groups` option

Custom groups control how imports are bucketed and separated by blank lines. Pass a 2-D array of regex strings; each outer array is a group (blank-line separator) and each inner array is a sub-group (no blank line between them). Patterns are matched against the import source string (with a `\0` prefix for side-effect imports and a `\0` suffix for type imports).

```jsonc
{
  "rules": {
    "simple-import-sort/imports": [
      "error",
      {
        "groups": [
          // Side-effect imports
          ["^\\u0000"],
          // Node built-ins
          ["^node:"],
          // Packages
          ["^@?\\w"],
          // Internal aliases
          ["^~"],
          // Relative imports
          ["^\\."],
        ],
      },
    ],
  },
}
```

## Rules

| Rule      | Description                                        | Status |
| --------- | -------------------------------------------------- | ------ |
| `imports` | Sort import declarations within each chunk         | ported |
| `exports` | Sort re-export declarations and named export lists | ported |

## Test parity

Upstream test cases are captured verbatim from the vendored submodule by `pnpm run port:tests:simple-import-sort` and replayed against this plugin in a Rust replay harness, so behavior tracks upstream as the submodule is bumped. All 88 upstream invalid cases are exercised; one is quarantined because it uses a construct (`import type X, {Y}`) that TypeScript itself rejects and Oxc declines to parse.

## Attribution

Rule behavior, sorting algorithm, and test cases are derived from [`eslint-plugin-simple-import-sort`](https://github.com/lydell/eslint-plugin-simple-import-sort) (MIT, © Simon Lydell). See [`LICENSE`](./LICENSE).
