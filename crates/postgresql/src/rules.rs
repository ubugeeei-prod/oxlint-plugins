//! Rule registry. Each rule is a small module exposing
//! `pub fn run(node, ancestors, ctx)` invoked for every node in the AST.

use crate::RuleDef;

mod no_alter_column_type;
mod no_cluster;
mod no_create_role;
mod no_cross_join;
mod no_distinct_on_without_order_by;
mod no_drop_column;
mod no_drop_database;
mod no_drop_not_null;
mod no_equality_with_null;
mod no_group_by_ordinal;
mod no_having_without_group_by;
mod no_implicit_join;
mod no_leading_wildcard_like;
mod no_order_by_ordinal;
mod no_rename_column;
mod no_rename_table;
mod no_rule;
mod no_select_into;
mod no_select_star;
mod no_set_not_null;
mod no_set_search_path;
mod no_temporary_table;
mod no_truncate_cascade;
mod no_unlogged_table;
mod no_vacuum_full;
mod no_with_recursive_without_limit;
mod require_limit;
mod require_where_in_delete;
mod require_where_in_update;

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
    "no-alter-column-type",
    "no-cluster",
    "no-create-role",
    "no-cross-join",
    "no-distinct-on-without-order-by",
    "no-drop-column",
    "no-drop-database",
    "no-drop-not-null",
    "no-equality-with-null",
    "no-group-by-ordinal",
    "no-having-without-group-by",
    "no-implicit-join",
    "no-leading-wildcard-like",
    "no-order-by-ordinal",
    "no-rename-column",
    "no-rename-table",
    "no-rule",
    "no-select-into",
    "no-select-star",
    "no-set-not-null",
    "no-set-search-path",
    "no-temporary-table",
    "no-truncate-cascade",
    "no-unlogged-table",
    "no-vacuum-full",
    "no-with-recursive-without-limit",
    "require-limit",
    "require-where-in-delete",
    "require-where-in-update",
];

/// Dispatch table consulted by [`crate::scan_postgresql`].
pub(crate) const REGISTRY: &[RuleDef] = &[
    RuleDef {
        name: "no-alter-column-type",
        run: no_alter_column_type::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-cluster",
        run: no_cluster::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-create-role",
        run: no_create_role::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-cross-join",
        run: no_cross_join::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-distinct-on-without-order-by",
        run: no_distinct_on_without_order_by::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-drop-column",
        run: no_drop_column::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-drop-database",
        run: no_drop_database::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-drop-not-null",
        run: no_drop_not_null::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-equality-with-null",
        run: no_equality_with_null::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-group-by-ordinal",
        run: no_group_by_ordinal::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-having-without-group-by",
        run: no_having_without_group_by::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-implicit-join",
        run: no_implicit_join::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-leading-wildcard-like",
        run: no_leading_wildcard_like::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-order-by-ordinal",
        run: no_order_by_ordinal::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-rename-column",
        run: no_rename_column::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-rename-table",
        run: no_rename_table::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-rule",
        run: no_rule::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-select-into",
        run: no_select_into::run,
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
    RuleDef {
        name: "no-set-search-path",
        run: no_set_search_path::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-temporary-table",
        run: no_temporary_table::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-truncate-cascade",
        run: no_truncate_cascade::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-unlogged-table",
        run: no_unlogged_table::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-vacuum-full",
        run: no_vacuum_full::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "no-with-recursive-without-limit",
        run: no_with_recursive_without_limit::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "require-limit",
        run: require_limit::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "require-where-in-delete",
        run: require_where_in_delete::run,
        uses_parse_error: false,
    },
    RuleDef {
        name: "require-where-in-update",
        run: require_where_in_update::run,
        uses_parse_error: false,
    },
];
