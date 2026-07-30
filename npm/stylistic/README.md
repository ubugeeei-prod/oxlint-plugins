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

`type-annotation-spacing` follows the stable `@stylistic/eslint-plugin` v5.10.0
behavior, including the global `before`/`after` settings, context-specific
`variable`, `property`, `parameter`, and `returnType` overrides, and the
deprecated `arrow` override (including `"ignore"`).
`member-delimiter-style` follows the same stable baseline for interfaces and
type literals, including per-container overrides and multiline detection.

`semi` follows the complete stable v5.10.0 JavaScript and TypeScript contract:
`always`/`never`, one-line block and class-body omissions, statement
continuation policies, class-field ASI hazards, and conservative surrounding
token fixes. TypeScript and TSX files use the same native Oxc-AST path.

`wrap-iife` is a native Oxc AST port of the stable v5.10.0 rule. It supports
`"outside"`, `"inside"`, and `"any"`, including `functionPrototypeMethods`,
optional chains, and comment-preserving code fixes for JavaScript, TypeScript,
and TSX.

`jsx-curly-newline` is an Oxc JSX/TSX AST port of the stable v5.10.0 rule. It
supports `consistent`, `never`, and the complete `singleline`/`multiline`
object policy, with comment-safe whitespace fixes and exact UTF-16 locations
through the JavaScript plugin.

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

Most rule scanning logic is derived from `corsa-oxlint/stylistic` in
[`ubugeeei-prod/corsa-bind`](https://github.com/ubugeeei-prod/corsa-bind)
v0.43.0 (MIT). Additional stable ports, including `jsx-quotes`, `jsx-curly-newline`,
and `semi`, follow
[`@stylistic/eslint-plugin`](https://github.com/eslint-stylistic/eslint-stylistic)
v5.10.0 (MIT).
