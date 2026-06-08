# @oxlint-plugins/oxlint-plugin-type-aware

Shared helper package for type-aware oxlint plugin ports.

Type-aware rules should use Corsa through `@corsa-bind/napi`. Do not wire individual rules directly to the TypeScript compiler service. Keeping the boundary here lets rule packages share initialization, snapshot updates, request batching, and lifecycle cleanup.

The caller must provide a compatible Corsa executable path.
