//! Clean-room rule implementations for the sonarjs port. Each module attaches
//! one `check_*` method to [`crate::scanner::Scanner`].

mod no_collapsible_if;
mod no_nested_conditional;
mod no_nested_switch;
mod no_nested_template_literals;
mod no_redundant_boolean;
