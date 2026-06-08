# @oxlint-plugins/oxlint-plugin-no-forbidden-identifiers

Sample Rust-backed oxlint plugin. The JavaScript plugin shape stays compatible with Oxlint's JS plugin API, while the file-level pre-scan runs in Rust through NAPI-RS.

## Usage

```jsonc
{
  "jsPlugins": [
    {
      "name": "forbidden",
      "specifier": "@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers",
    },
  ],
  "rules": {
    "forbidden/no-forbidden-identifiers": ["error", { "names": ["ctx"] }],
  },
}
```

## Performance Shape

The rule calls into Rust once per file to pre-scan source text. If no configured names are present, the Oxlint visitor is skipped for that file. Identifier reporting remains in the JS wrapper so the rule stays compatible with the current public Oxlint plugin API.

## JS API

The package also exposes a NAPI-backed JS API:

```js
const {
  scanForbiddenIdentifiers,
} = require('@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers/api');

scanForbiddenIdentifiers('const event = data.error;');
```

Use `./native` only when you intentionally need the raw NAPI-RS generated bindings.

## Current Limits

This sample intentionally handles ASCII identifier boundaries only. Real ports should snapshot the upstream ESLint behavior and document any Unicode, parser, or type-aware gaps before release.
