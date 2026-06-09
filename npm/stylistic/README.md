# @oxlint-plugins/oxlint-plugin-stylistic

Rust-backed oxlint plugin port of the native stylistic scanner from `corsa-oxlint`.
The plugin batches enabled stylistic rules into one source-wide native scan and
then reports Oxlint-compatible diagnostics from JavaScript.

## Usage

```jsonc
{
  "jsPlugins": [
    {
      "name": "stylistic",
      "specifier": "@oxlint-plugins/oxlint-plugin-stylistic",
    },
  ],
  "settings": {
    "corsaStylistic": {
      "rules": {
        "quotes": ["single"],
        "no-trailing-spaces": [],
      },
    },
  },
  "rules": {
    "stylistic/quotes": "error",
    "stylistic/no-trailing-spaces": "error",
  },
}
```

Rule options can also be supplied directly from each rule entry, for example
`"stylistic/quotes": ["error", "single"]`. For the fastest multi-rule path,
mirror enabled option payloads in `settings.corsaStylistic.rules` so all
configured rules share a single native call per source file.

## JS API

```js
const {
  nativeStylisticRuleMetas,
  runNativeStylisticLint,
} = require('@oxlint-plugins/oxlint-plugin-stylistic/api');

runNativeStylisticLint('const label = "value";\n', {
  rules: [{ name: 'quotes', options: ['single'] }],
});
```

Use `./native` only when you intentionally need the raw NAPI-RS generated
bindings.

## Credits

Rule scanning logic is derived from `corsa-oxlint/stylistic` in
[`ubugeeei-prod/corsa-bind`](https://github.com/ubugeeei-prod/corsa-bind)
v0.43.0 (MIT).
