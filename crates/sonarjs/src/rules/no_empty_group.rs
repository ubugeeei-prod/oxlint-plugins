//! Rule `no-empty-group` (SonarJS key S6331).
//!
//! Clean-room port. A capturing group `(...)` or non-capturing group `(?:...)`
//! whose body is completely empty contributes nothing to the match and is almost
//! always a programmer mistake — a missing pattern, a forgotten character, or an
//! accidental deletion.
//!
//! ## What is flagged
//!
//! Any **regex literal** that contains an empty capturing or non-capturing group:
//!
//! ```js
//! /foo()bar/   // flagged — empty capturing group
//! /(?:)/       // flagged — empty non-capturing group
//! /(a)()/      // flagged — second group is empty
//! ```
//!
//! ## What is NOT flagged
//!
//! - `(a)`, `(?:abc)` — non-empty groups.
//! - `(a|)` — the *group* is not empty; it has two alternatives, one of which
//!   happens to be empty. That is a separate concern (empty alternative) outside
//!   the scope of this rule.
//! - `(a)?` — a non-empty group wrapped in a quantifier.
//!
//! ## Scope: regex literals only
//!
//! This rule detects empty groups in **regex literals** (e.g. `/foo()bar/`) only.
//! The `new RegExp("()")` string-argument form is out of scope for this PR and is
//! left as a follow-up.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-empty-group";

/// Returns `true` when the disjunction represents a completely empty group body —
/// exactly one alternative with no terms.
fn disjunction_is_empty(disj: &Disjunction<'_>) -> bool {
    disj.body.len() == 1 && disj.body[0].body.is_empty()
}

/// Recurses into a single [`Term`], pushing the span of any empty capturing or
/// non-capturing group onto `out`.  Also recurses into quantifier bodies and
/// lookaround assertion bodies so that empty groups at any nesting depth are found.
fn collect_empty_groups_term<'a>(term: &Term<'a>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CapturingGroup(group) => {
            if disjunction_is_empty(&group.body) {
                out.push(group.span);
            }
            collect_empty_groups(&group.body, out);
        }
        Term::IgnoreGroup(group) => {
            if disjunction_is_empty(&group.body) {
                out.push(group.span);
            }
            collect_empty_groups(&group.body, out);
        }
        Term::Quantifier(quant) => {
            collect_empty_groups_term(&quant.body, out);
        }
        Term::LookAroundAssertion(look) => {
            collect_empty_groups(&look.body, out);
        }
        _ => {}
    }
}

/// Walks every alternative in `disj` and every term within each alternative,
/// collecting the spans of any empty groups.
fn collect_empty_groups<'a>(disj: &Disjunction<'a>, out: &mut SmallVec<[Span; 8]>) {
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_empty_groups_term(term, out);
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_empty_group(&mut self, lit: &RegExpLiteral<'_>) {
        let spans = crate::regex_ast::with_parsed_regex_literal(lit, self.source_text, |pattern| {
            let mut out: SmallVec<[Span; 8]> = SmallVec::new();
            collect_empty_groups(&pattern.body, &mut out);
            out
        });
        for span in spans {
            self.report(RULE_NAME, "emptyGroup", span);
        }
    }
}
