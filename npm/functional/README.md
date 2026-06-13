# @oxlint-plugins/oxlint-plugin-functional

Rust-backed Oxlint plugin port of [`eslint-plugin-functional`](https://github.com/eslint-functional/eslint-plugin-functional).

The native scanner uses Oxc and reports every upstream rule id. Rules whose
behavior can be decided from syntax alone are ported to full upstream parity;
rules that depend on TypeScript's type checker are implemented as AST-safe checks
for the violations that can be proven from syntax, with the type-dependent cases
documented below.

## Upstream test parity

The upstream test suite is captured verbatim from the vendored submodule
(`eslint-plugin-functional` v10.0.0) into committed JSON fixtures by
`pnpm run port:tests:functional` and replayed against this plugin by
`npm/functional/test/upstream.test.mjs`, so behavior tracks upstream as the
submodule is bumped. For full-parity rules every upstream case is asserted
(reported messageIds and counts must match). For the remaining rules every case
is still captured, and the cases that require type information are skipped and
counted in the parity ledger — never silently dropped.

### Rule status

| Rule                            | Status                                                                  |
| ------------------------------- | ----------------------------------------------------------------------- |
| `functional-parameters`         | ✅ Full parity                                                          |
| `no-class-inheritance`          | ✅ Full parity                                                          |
| `no-classes`                    | ✅ Full parity                                                          |
| `no-let`                        | ✅ Full parity                                                          |
| `no-loop-statements`            | ✅ Full parity                                                          |
| `no-mixed-types`                | ✅ Full parity                                                          |
| `no-promise-reject`             | ✅ Full parity                                                          |
| `no-this-expressions`           | ✅ Full parity                                                          |
| `no-throw-statements`           | ✅ Full parity                                                          |
| `no-try-statements`             | ✅ Full parity                                                          |
| `prefer-property-signatures`    | ✅ Full parity                                                          |
| `readonly-type`                 | ✅ AST-only (no upstream test suite)                                    |
| `no-conditional-statements`     | 🟡 Syntactic core; `allowReturningBranches: "ifExhaustive"` needs types |
| `no-expression-statements`      | 🟡 Syntactic core; `ignoreVoid` / `ignoreSelfReturning` need types      |
| `prefer-readonly-type`          | 🟡 Syntactic core; `checkImplicit` needs types                          |
| `immutable-data`                | 🔵 Needs type information (array/map/set detection)                     |
| `no-return-void`                | 🔵 Needs type information (inferred return types)                       |
| `prefer-immutable-types`        | 🔵 Needs type information (`is-immutable-type`)                         |
| `prefer-tacit`                  | 🔵 Needs type information (call-signature arity)                        |
| `type-declaration-immutability` | 🔵 Needs type information (`is-immutable-type`)                         |

- ✅ Full parity — every upstream test case is asserted.
- 🟡 Syntactic core — the rule's syntax-only behavior is implemented and the
  syntactic cases pass; specific options/cases that require the TypeScript
  checker are captured and skipped (counted in the ledger).
- 🔵 Needs type information — the rule's core requires TypeScript types. The
  scanner emits AST-level approximations, but full parity requires the
  type-aware pipeline (`@oxlint-plugins/oxlint-plugin-type-aware`); all upstream
  cases are captured and skipped until then.
