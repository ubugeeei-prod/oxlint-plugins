//! Rule registry. Each rule is a small module exposing
//! `pub fn run(node, ancestors, ctx)` invoked for every node in the AST.

use crate::RuleDef;

mod no_cluster;
mod no_drop_database;
mod no_rename_table;
mod no_select_star;
mod no_set_not_null;

/// Every upstream rule name (89), in inventory order. Used by the JS adapter to
/// know the full surface even while only a subset is implemented.
pub const RULE_NAMES: [&str; 89] = [
    "align-column-definitions",
    "align-values",
    "consistent-as-for-column-alias",
    "consistent-as-for-table-alias",
    "consistent-between-over-and",
    "consistent-create-index-concurrently",
    "consistent-create-or-replace",
    "consistent-drop-index-concurrently",
    "consistent-explicit-inner-join",
    "consistent-explicit-outer-join",
    "consistent-fk-not-valid",
    "consistent-identity-over-serial",
    "consistent-jsonb-over-json",
    "consistent-reindex-concurrently",
    "consistent-text-over-varchar",
    "consistent-timestamptz",
    "no-add-check-constraint-without-not-valid",
    "no-add-column-not-null-without-default",
    "no-add-unique-constraint-directly",
    "no-alter-column-type",
    "no-char-type",
    "no-cluster",
    "no-composite-primary-key",
    "no-create-role",
    "no-cross-join",
    "no-distinct-on-without-order-by",
    "no-drop-column",
    "no-drop-database",
    "no-drop-not-null",
    "no-drop-schema-cascade",
    "no-drop-table-cascade",
    "no-equality-with-null",
    "no-grant-all",
    "no-grant-to-public",
    "no-group-by-ordinal",
    "no-having-without-group-by",
    "no-identifier-too-long",
    "no-implicit-join",
    "no-leading-wildcard-like",
    "no-money-type",
    "no-natural-join",
    "no-not-in-subquery",
    "no-numeric-without-precision",
    "no-on-delete-cascade",
    "no-order-by-ordinal",
    "no-rename-column",
    "no-rename-table",
    "no-rule",
    "no-security-definer-without-search-path",
    "no-select-into",
    "no-select-star",
    "no-set-not-null",
    "no-set-search-path",
    "no-syntax-error",
    "no-temporary-table",
    "no-time-type",
    "no-truncate-cascade",
    "no-unlogged-table",
    "no-unnecessary-quoted-identifier",
    "no-update-primary-key",
    "no-update-without-from-binding",
    "no-vacuum-full",
    "no-volatile-default-on-add-column",
    "no-with-recursive-without-limit",
    "plpgsql-keyword-case",
    "prefer-add-constraint-not-valid",
    "prefer-bigint-id",
    "prefer-cast-operator",
    "prefer-coalesce-over-case",
    "prefer-current-timestamp-over-now",
    "prefer-exists-over-in-subquery",
    "prefer-explicit-null-ordering",
    "prefer-in-list-over-or",
    "prefer-keyword-case",
    "prefer-not-equals-operator",
    "require-fk-include-columns",
    "require-if-exists",
    "require-index-on-fk-column",
    "require-limit",
    "require-named-constraint",
    "require-on-delete-action",
    "require-primary-key",
    "require-schema-qualified-table",
    "require-table-columns",
    "require-trailing-semicolon",
    "require-where-in-delete",
    "require-where-in-update",
    "snake-case-column-name",
    "snake-case-table-name",
];

/// Rules implemented in Rust so far (a growing subset of [`RULE_NAMES`]).
pub const IMPLEMENTED_RULE_NAMES: &[&str] = &[
    "no-cluster",
    "no-drop-database",
    "no-rename-table",
    "no-select-star",
    "no-set-not-null",
];

/// Dispatch table consulted by [`crate::scan_postgresql`].
pub(crate) const REGISTRY: &[RuleDef] = &[
    RuleDef {
        name: "no-cluster",
        run: no_cluster::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-drop-database",
        run: no_drop_database::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-rename-table",
        run: no_rename_table::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-select-star",
        run: no_select_star::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-set-not-null",
        run: no_set_not_null::run,
        uses_parse_error: false,
    },
];
