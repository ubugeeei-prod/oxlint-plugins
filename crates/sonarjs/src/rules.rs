//! Clean-room rule implementations for the sonarjs port. Each module attaches
//! one `check_*` method to [`crate::scanner::Scanner`].

mod arguments_usage;
mod comma_or_logical_or_case;
mod no_all_duplicated_branches;
mod no_collapsible_if;
mod no_duplicate_in_composite;
mod no_identical_conditions;
mod no_identical_expressions;
mod no_labels;
mod no_nested_conditional;
mod no_nested_switch;
mod no_nested_template_literals;
mod no_redundant_boolean;
mod non_existent_operator;
