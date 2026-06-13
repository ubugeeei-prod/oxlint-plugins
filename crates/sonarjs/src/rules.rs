//! Clean-room rule implementations for the sonarjs port. Each module attaches
//! one `check_*` method to [`crate::scanner::Scanner`].

mod arguments_usage;
mod class_prototype;
mod comma_or_logical_or_case;
mod constructor_for_side_effects;
mod elseif_without_else;
mod for_in;
mod generator_without_yield;
mod max_switch_cases;
mod max_union_size;
mod no_all_duplicated_branches;
mod no_built_in_override;
mod no_case_label_in_switch;
mod no_collapsible_if;
mod no_delete_var;
mod no_duplicate_in_composite;
mod no_empty_character_class;
mod no_exclusive_tests;
mod no_identical_conditions;
mod no_identical_expressions;
mod no_labels;
mod no_nested_conditional;
mod no_nested_switch;
mod no_nested_template_literals;
mod no_redundant_boolean;
mod non_existent_operator;
