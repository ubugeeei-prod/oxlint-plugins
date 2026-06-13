//! Per-rule check methods, each attached to [`crate::scanner::Scanner`]
//! through additional `impl` blocks.

mod array_some;
mod array_transforms;
mod comparisons;
mod date_regex;
mod dependencies;
mod includes_reduce;
mod inline_timer;
mod syntax;
