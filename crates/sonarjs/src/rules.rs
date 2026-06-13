//! Clean-room rule implementations for the sonarjs port. Each module attaches
//! one `check_*` method to [`crate::scanner::Scanner`].

mod no_nested_template_literals;
