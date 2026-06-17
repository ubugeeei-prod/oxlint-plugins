//! Shared native core for the Rust-backed oxlint plugins.
//!
//! Every plugin's NAPI boundary lives here so the whole workspace compiles into
//! a single native addon (`@oxlint-plugins/core`). Domain rule logic stays in
//! its own crate under `crates/*`; this crate only adapts that logic to NAPI.
//!
//! Each plugin gets its own module with a NAPI `namespace`, so exported function
//! names are grouped per plugin on the JS side (e.g.
//! `core.eslintComments.scanNoUse`) and can never collide across plugins as more
//! are ported.

pub mod cypress;
pub mod eslint_comments;
pub mod mocha;
pub mod no_forbidden_identifiers;
pub mod react_refresh;
pub mod security;
pub mod simple_import_sort;
pub mod stylistic;
pub mod unused_imports;
