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

`indent` preserves the complete stable v5.10.0 JavaScript, JSX, and TypeScript
visitor and option contract through Oxlint's Oxc-provided ESTree, including
tabs, every node-specific offset option, ignored selectors, comments, and
recursive whitespace fixes. The native API also exposes a source-wide fallback
for deterministic indentation diagnostics and UTF-8 byte-range fixes.

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

`jsx-curly-brace-presence` follows the complete stable v5.10.0 JSX/TSX
contract for `props`, `children`, and `propElementValues`. The native Oxc AST
port preserves comments, HTML entities, escapes, multiline text, adjacent
expressions, JSX elements/fragments, and the upstream first-pass and recursive
fix behavior.

`jsx-first-prop-new-line` follows the stable `@stylistic/eslint-plugin` v5.10.0
`always`, `never`, `multiline`, `multiline-multiprop`, and `multiprop` modes.
Its fixes preserve TypeScript generic component arguments and match upstream's
raw replacement boundaries.

`exp-jsx-props-style` follows the experimental rule shipped in the stable
v5.10.0 package. It supports `singleLine.maxItems` and
`multiLine.minItems`/`maxItemsPerLine`, uses the first prop to choose between
wrapping and collapsing multiline props, and leaves comment-separated
boundaries unfixable while applying every other safe fix.

`jsx-newline` follows the pinned stable JSX/TSX contract for required and
prevented newlines between adjacent elements and expressions, including the
`allowMultilines` option and comment-preserving fixes.

`jsx-props-no-multi-spaces` preserves the stable v5.10.0 deprecated rule for
existing configurations. It rejects blank lines between consecutive JSX props
and fixes inline gaps to one space across JSX names, TypeScript generics,
namespaced attributes, and spread props. New configurations should use the
more general `no-multi-spaces` rule.

`jsx-sort-props` follows the complete stable v5.10.0 comparison precedence:
callbacks, shorthand and multiline placement, custom reserved-first and
reserved-last lists, case and locale handling, plus alphabetical order. Spread
attributes remain ordering barriers, while attached JSX comments move with the
same attribute blocks as upstream.

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
v0.43.0 (MIT). Additional stable ports, including `jsx-first-prop-new-line`,
`jsx-newline`, `jsx-pascal-case`, `jsx-quotes`, `jsx-curly-brace-presence`,
`jsx-curly-newline`, `exp-jsx-props-style`, `jsx-indent`,
`jsx-props-no-multi-spaces`, `jsx-sort-props`, `jsx-wrap-multilines`, `indent`,
and `semi`, follow
[`@stylistic/eslint-plugin`](https://github.com/eslint-stylistic/eslint-stylistic)
v5.10.0 (MIT).
