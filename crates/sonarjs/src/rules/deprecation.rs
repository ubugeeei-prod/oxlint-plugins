//! Rule `deprecation` (SonarJS key S1874).
//!
//! Clean-room port. Code marked with `@deprecated` should not be used.
//!
//! ## Conservative scope (zero-false-positive design)
//!
//! Only same-file function and class declarations are covered. A declaration
//! is considered deprecated when it is immediately preceded by a block comment
//! (i.e. a `/* … */` comment with only whitespace between its closing `*/` and
//! the `function`/`class` keyword) that contains the token `@deprecated`.
//! Identifier references that resolve to such a symbol and appear after the
//! declaration in source order are flagged.
//!
//! Cross-module imports, method calls, variable declarations, and references
//! appearing before the declaration in source order (hoisted calls) are
//! intentionally excluded to avoid false positives in the absence of type
//! information.
//!
//! Behaviour is reproduced from the public RSPEC S1874 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `/** @deprecated */ function old() {} old();` — reference to a
//!   same-file deprecated function declaration appears after the declaration
//!
//! ## Not flagged
//! - A function that is called but lacks a `@deprecated` block comment
//! - A deprecated function that is declared but never called
//! - Exported functions, class expressions, or variable declarations annotated
//!   with `@deprecated` (cross-module or variable-binding scope excluded)

use oxc_ast::ast::{Class, Function, IdentifierReference};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "deprecation";

impl<'a> Scanner<'a> {
    /// Returns `true` when `self.comment_spans` contains a block comment whose
    /// closing `*/` is separated from `decl_start` only by whitespace and
    /// whose text contains the token `@deprecated`.
    pub(crate) fn comment_is_deprecated_before(&self, decl_start: u32) -> bool {
        for &comment_span in &self.comment_spans {
            if comment_span.end > decl_start {
                continue;
            }
            let between = &self.source_text[comment_span.end as usize..decl_start as usize];
            if !between.chars().all(|c| c.is_whitespace()) {
                continue;
            }
            let comment_text = self.text(comment_span);
            if !comment_text.starts_with("/*") {
                continue;
            }
            if !comment_text.contains("@deprecated") {
                continue;
            }
            return true;
        }
        false
    }

    /// Checks whether a function declaration is preceded by a `@deprecated`
    /// block comment and, if so, records the symbol as deprecated.
    pub(crate) fn check_deprecation_function(&mut self, it: &Function<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let id = match &it.id {
            Some(id) => id,
            None => return,
        };
        let sym = match id.symbol_id.get() {
            Some(s) => s,
            None => return,
        };
        if self.comment_is_deprecated_before(it.span.start) {
            self.deprecated_symbols.push(sym);
        }
    }

    /// Checks whether a class declaration is preceded by a `@deprecated`
    /// block comment and, if so, records the symbol as deprecated.
    pub(crate) fn check_deprecation_class(&mut self, it: &Class<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let id = match &it.id {
            Some(id) => id,
            None => return,
        };
        let sym = match id.symbol_id.get() {
            Some(s) => s,
            None => return,
        };
        if self.comment_is_deprecated_before(it.span.start) {
            self.deprecated_symbols.push(sym);
        }
    }

    /// Reports an identifier reference that resolves to a locally-declared
    /// deprecated symbol.
    pub(crate) fn check_deprecation_reference(&mut self, ident: &IdentifierReference<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let scoping = match self.scoping {
            Some(s) => s,
            None => return,
        };
        let ref_id = match ident.reference_id.get() {
            Some(id) => id,
            None => return,
        };
        let sym = match scoping.get_reference(ref_id).symbol_id() {
            Some(s) => s,
            None => return,
        };
        if !self.deprecated_symbols.contains(&sym) {
            return;
        }
        self.report(RULE_NAME, "deprecatedUse", ident.span);
    }
}
