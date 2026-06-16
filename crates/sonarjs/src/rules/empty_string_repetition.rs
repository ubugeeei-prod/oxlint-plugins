//! Rule `empty-string-repetition` (SonarJS key S5842).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC description
//! (S5842) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! Applying a quantifier that allows multiple repetitions (`*`, `+`, `{n,}`,
//! or `{n,m}` with m ≥ 2) to a sub-pattern that can match the empty string is
//! meaningless or dangerous: the body may consume zero characters on every
//! iteration, enabling catastrophic backtracking (ReDoS) or making the
//! repetition entirely redundant.
//!
//! ## What is flagged
//!
//! A repetition quantifier (`*`, `+`, `{n,}`, or `{n,m}` with m ≥ 2) whose
//! body can match the empty string:
//!
//! ```js
//! /(a*)*/   // flagged — outer * repeats a body that matches empty
//! /(a?)+/   // flagged — + repeats a body that matches empty (a? can be "")
//! /(?:)*/   // flagged — * repeats an always-empty non-capturing group
//! /()+/     // flagged — + repeats an empty capturing group
//! /(?:|a)*/ // flagged — * repeats a disjunction with an empty alternative
//! ```
//!
//! ## What is NOT flagged
//!
//! - `/(a+)*/` — `a+` requires ≥1 character; body cannot match empty.
//! - `/a*/`    — body `a` is a literal character; does not match empty.
//! - `/a?/`    — `?` has max=1; not a repetition quantifier.
//! - `/(abc)+/` — body `abc` requires three characters.
//! - `/[a-z]+/` — a character class always consumes one character.
//!
//! Only regex literals are inspected; `new RegExp(…)` is a documented
//! follow-up. No upstream source, tests, or messages were consulted.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Alternative, Disjunction, Quantifier, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "empty-string-repetition";

/// Returns `true` when `quant` allows the body to execute more than once.
/// A bare `?` or `{n,1}` quantifier (max ≤ 1) is not a repetition.
fn is_repetition_quantifier(quant: &Quantifier<'_>) -> bool {
    match quant.max {
        None => true,
        Some(max) => max >= 2,
    }
}

/// Returns `true` when `term` can produce a zero-length match.
fn term_matches_empty(term: &Term<'_>) -> bool {
    match term {
        // Groups delegate to their inner disjunction.
        Term::CapturingGroup(g) => disj_matches_empty(&g.body),
        Term::IgnoreGroup(g) => disj_matches_empty(&g.body),
        // A quantifier matches empty when its minimum is zero or its body does.
        Term::Quantifier(q) => {
            if q.min == 0 {
                true
            } else {
                term_matches_empty(&q.body)
            }
        }
        // Zero-width assertions never consume any input.
        Term::BoundaryAssertion(_) | Term::LookAroundAssertion(_) => true,
        // Backreferences are treated conservatively: the referenced group may
        // have captured the empty string.
        Term::IndexedReference(_) | Term::NamedReference(_) => true,
        // Character, CharacterClass, CharacterClassEscape, UnicodePropertyEscape,
        // Dot, and any future variants are assumed to consume at least one
        // character and therefore cannot match empty.
        _ => false,
    }
}

/// Returns `true` when every term in `alt` can match the empty string.
/// An alternative with zero terms trivially matches empty.
fn alt_matches_empty(alt: &Alternative<'_>) -> bool {
    alt.body.iter().all(|t| term_matches_empty(t))
}

/// Returns `true` when at least one alternative of `disj` can match empty.
fn disj_matches_empty(disj: &Disjunction<'_>) -> bool {
    disj.body.iter().any(|a| alt_matches_empty(a))
}

/// Inspects `term`: if it is a flagged repetition quantifier, records its
/// span. Then recurses into any nested term to find further violations.
fn collect_in_term<'a>(term: &Term<'a>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::Quantifier(quant) => {
            if is_repetition_quantifier(quant) && term_matches_empty(&quant.body) {
                out.push(quant.span);
            }
            collect_in_term(&quant.body, out);
        }
        Term::CapturingGroup(group) => collect_in_disj(&group.body, out),
        Term::IgnoreGroup(group) => collect_in_disj(&group.body, out),
        Term::LookAroundAssertion(look) => collect_in_disj(&look.body, out),
        _ => {}
    }
}

/// Walks all terms inside `disj` to find flagged repetition quantifiers.
fn collect_in_disj<'a>(disj: &Disjunction<'a>, out: &mut SmallVec<[Span; 8]>) {
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_in_term(term, out);
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_empty_string_repetition_with_pattern(
        &mut self,
        _lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        collect_in_disj(&pattern.body, &mut out);
        for span in out {
            self.report(RULE_NAME, "emptyStringRepetition", span);
        }
    }
}
