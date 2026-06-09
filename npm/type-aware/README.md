# @oxlint-plugins/oxlint-plugin-type-aware

Shared Corsa-backed helper package for type-aware oxlint plugin ports.

Type-aware rules should use Corsa through `@corsa-bind/napi`. Do not wire individual rules directly to the TypeScript compiler service. Keeping the boundary here lets rule packages share initialization, snapshot updates, request batching, and lifecycle cleanup.

The caller must provide a compatible Corsa executable path.

## Corsa Oxlint Surface

This package now includes the `corsa-oxlint` helper surface:

- `OxlintUtils.RuleCreator()` and `getParserServices()`
- compatibility namespaces such as `ESLintUtils`, `TSESLint`, `TSESTree`, and `TSUtils`
- `RuleTester` helpers for Corsa-backed rule tests
- the Corsa native type-aware rule bridge under `rules`

The older `createCorsaTypeAwareSession()` and `pathToFileUri()` helpers remain exported for existing callers.

## Credits

The expanded Oxlint helper surface is derived from `corsa-oxlint` in
[`ubugeeei-prod/corsa-bind`](https://github.com/ubugeeei-prod/corsa-bind)
v0.43.0 (MIT).
