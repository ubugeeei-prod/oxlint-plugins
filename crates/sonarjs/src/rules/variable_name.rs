//! Rule `variable-name` (SonarJS key S117).
//!
//! Clean-room port. Local variable and function-parameter names should follow
//! the conventional camelCase style. The default SonarJS convention is the
//! regular expression `^[_a-z][a-zA-Z0-9]*$`: a name starts with an underscore
//! or an ASCII lowercase letter, followed by ASCII letters or digits.
//!
//! ## Narrow form
//!
//! This port enforces the unambiguous, configuration-independent default
//! format and only on bindings whose violation is unambiguous, deliberately
//! under-reporting rather than risking false positives:
//!
//! - Only *local* variables are checked: a `let`/`const`/`var` declarator is
//!   considered only when it appears inside a function body (the scanner's
//!   `function_nesting_depth` is non-zero). Module/global declarations are out
//!   of scope for S117 and are never reported.
//! - Function and arrow-function parameters are checked, since parameters are
//!   inherently local.
//! - Only plain binding identifiers are inspected. Destructuring patterns
//!   (`const { a_b } = x`, `function f({ a_b }) {}`) are skipped, as the binding
//!   shape there is ambiguous to attribute cleanly.
//! - SCREAMING_SNAKE_CASE constant-style names (only uppercase letters, digits
//!   and underscores, e.g. `MAX_VALUE`) are exempted. Such names are an
//!   idiomatic constant convention; flagging them would be a practical false
//!   positive, so the port stays silent on them.
//!
//! ```js
//! function f(bad_param) {      // Noncompliant: snake_case parameter
//!   let My_Local = 1;          // Noncompliant: not camelCase
//!   let goodLocal = 2;         // Compliant
//!   const MAX = 3;             // Compliant (constant-style, exempted)
//! }
//! const topLevel = 1;          // Not a local variable: never reported
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description (S117) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use compact_str::ToCompactString;
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, FormalParameters, Function, VariableDeclarator,
};

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "variable-name";

/// The SonarJS default naming convention for local variables and parameters.
const DEFAULT_FORMAT: &str = "^[_a-z][a-zA-Z0-9]*$";

impl Scanner<'_> {
    pub(crate) fn check_variable_name_declarator(&mut self, declarator: &VariableDeclarator<'_>) {
        // Only local variables (declared inside a function) are in scope for S117.
        if self.function_nesting_depth == 0 {
            return;
        }
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return;
        };
        self.report_bad_variable_name(id.name.as_str(), id.span);
    }

    pub(crate) fn check_variable_name_function_params(&mut self, function: &Function<'_>) {
        // Skip overload/ambient signatures (no body): they carry no real bindings.
        if function.body.is_none() {
            return;
        }
        self.check_param_names(&function.params);
    }

    pub(crate) fn check_variable_name_arrow_params(&mut self, arrow: &ArrowFunctionExpression<'_>) {
        self.check_param_names(&arrow.params);
    }

    fn check_param_names(&mut self, params: &FormalParameters<'_>) {
        for param in &params.items {
            if let BindingPattern::BindingIdentifier(id) = &param.pattern {
                self.report_bad_variable_name(id.name.as_str(), id.span);
            }
        }
    }

    fn report_bad_variable_name(&mut self, name: &str, span: oxc_span::Span) {
        if matches_default_format(name) || is_constant_name(name) {
            return;
        }
        let data = DiagnosticData {
            value: Some(name.to_compact_string()),
            format: Some(DEFAULT_FORMAT.to_compact_string()),
        };
        self.report_with_data(RULE_NAME, "renameVariable", data, span, None);
    }
}

/// Matches the default convention `^[_a-z][a-zA-Z0-9]*$` without building a
/// regex: first char is `_` or ASCII lowercase, the rest ASCII alphanumeric.
fn matches_default_format(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_lowercase()) && chars.all(|ch| ch.is_ascii_alphanumeric())
}

/// True for SCREAMING_SNAKE_CASE constant-style names: only uppercase ASCII
/// letters, digits and underscores, with at least one uppercase letter. These
/// are an idiomatic constant convention and are intentionally exempted to avoid
/// practical false positives.
fn is_constant_name(name: &str) -> bool {
    let mut has_uppercase = false;
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            has_uppercase = true;
        } else if !(ch.is_ascii_digit() || ch == '_') {
            return false;
        }
    }
    has_uppercase
}
