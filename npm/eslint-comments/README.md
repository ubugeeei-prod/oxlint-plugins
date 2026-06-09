# @oxlint-plugins/oxlint-plugin-eslint-comments

Rust-backed Oxlint port of [`@eslint-community/eslint-plugin-eslint-comments`](https://github.com/eslint-community/eslint-plugin-eslint-comments) — additional rules for ESLint directive comments. The JavaScript wrapper stays compatible with Oxlint's JS plugin API while the directive parsing and rule logic run in Rust through NAPI-RS.

This is unofficial community work and is not an official Oxlint or eslint-community project.

## Usage

```jsonc
{
  "jsPlugins": [
    {
      "name": "eslint-comments",
      "specifier": "@oxlint-plugins/oxlint-plugin-eslint-comments",
    },
  ],
  "rules": {
    "eslint-comments/no-unlimited-disable": "error",
  },
}
```

## Rules

| Rule                    | Description                                                | Status |
| ----------------------- | ---------------------------------------------------------- | ------ |
| `disable-enable-pair`   | require an `eslint-enable` for every `eslint-disable`      | ported |
| `no-aggregating-enable` | disallow one `eslint-enable` for multiple `eslint-disable` | ported |
| `no-unlimited-disable`  | disallow `eslint-disable` comments without rule names      | ported |
| `no-use`                | disallow ESLint directive-comments                         | ported |
| `require-description`   | require descriptions in ESLint directive-comments          | ported |

Remaining upstream rules (`no-duplicate-disable`, `no-restricted-disable`, `no-unused-disable`, `no-unused-enable`) are tracked in [issue #4](https://github.com/ubugeeei-prod/oxlint-plugins/issues/4) and land one rule per pull request.

## Performance Shape

Each rule calls into Rust once per file with the file's comments and reports the returned diagnostics. There is no per-AST-node NAPI traffic.

## Test parity

Upstream test cases are captured verbatim from the vendored submodule by `pnpm run port:tests:eslint-comments` and replayed against this plugin in Vitest, so behavior tracks upstream as the submodule is bumped.

## Attribution

Rule behavior, diagnostic message strings, and test cases are derived from `@eslint-community/eslint-plugin-eslint-comments` (v4.7.2, MIT, © 2016 Toru Nagashima). See [`LICENSE`](./LICENSE).
