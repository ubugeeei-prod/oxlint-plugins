# @oxlint-plugins/oxlint-plugin-react-refresh

Rust-backed Oxlint port of [`eslint-plugin-react-refresh`](https://github.com/ArnaudBarre/eslint-plugin-react-refresh).

This is unofficial community work and is not an official Oxlint, React, or eslint-plugin-react-refresh project.

## Usage

```jsonc
{
  "jsPlugins": [
    {
      "name": "react-refresh",
      "specifier": "@oxlint-plugins/oxlint-plugin-react-refresh",
    },
  ],
  "rules": {
    "react-refresh/only-export-components": "error",
  },
}
```

## Rules

| Rule                     | Description                                                  | Status |
| ------------------------ | ------------------------------------------------------------ | ------ |
| `only-export-components` | validate exports that can safely participate in Fast Refresh | ported |

## Configs

- `recommended`: enables `react-refresh/only-export-components`.
- `vite`: enables the rule with `allowConstantExport: true`.
- `next`: enables the rule with Next.js route segment and metadata export names allowed.

## Attribution

Rule behavior, diagnostic message strings, and test cases are derived from `eslint-plugin-react-refresh` (v0.5.2, MIT, Copyright 2023 Arnaud Barré). See [`LICENSE`](./LICENSE).
