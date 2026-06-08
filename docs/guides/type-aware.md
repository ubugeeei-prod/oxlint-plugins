# Type-Aware Rules

Type-aware rules must use Corsa through `@corsa-bind/napi`.

Do not create per-rule TypeScript compiler service integrations. The shared package `@oxlint-plugins/oxlint-plugin-type-aware` owns the Corsa boundary so rules can share:

- Corsa process lifecycle.
- Project initialization.
- Snapshot updates.
- Batched JSON calls.
- Cleanup.

The caller must provide a compatible Corsa executable. Published plugin packages should make type-aware mode explicit in their README and fail clearly when the executable is missing.

Performance rules still apply:

- Batch Corsa calls by file or project.
- Cache stable type facts per file version.
- Avoid Corsa calls inside hot AST visitors.
- Snapshot type-aware fixtures separately from syntax-only fixtures.
