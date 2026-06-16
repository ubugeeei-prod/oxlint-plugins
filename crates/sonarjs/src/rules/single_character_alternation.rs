//! Rule `single-character-alternation` (SonarJS key S6035).
//!
//! Clean-room port. A regex disjunction (`a|b|c`) where every alternative is
//! a single literal character can be written more clearly and efficiently as a
//! character class (`[abc]`). This rule flags such disjunctions.
//!
//! ## What is flagged
//!
//! A `|` alternation (a disjunction with two or more alternatives) where
//! **every** alternative consists of exactly one term that is a literal
//! `Character` node (including escape sequences like `\.`, `\n`, `\t`):
//!
//! ```js
//! /a|b|c/        // flagged — all alternatives are single chars
//! /(a|b|c)/      // flagged — disjunction inside a group
//! /x(1|2|3)y/    // flagged — nested disjunction
//! /\.|,/         // flagged — escaped char counts as a single character
//! ```
//!
//! ## What is NOT flagged
//!
//! - `/ab|c/` — first alternative has two terms (multi-char)
//! - `/\d|x/` — `\d` is a CharacterClassEscape, not a Character
//! - `/a/`    — no disjunction at all
//! - `/a|/`   — empty alternative disqualifies (zero terms)
//!
//! Detects qualifying disjunctions in **regex literals** only; the
//! `new RegExp(...)` string form is a documented follow-up. Behaviour is
//! reproduced from the public RSPEC description (S6035) and observed IDE
//! behaviour only; no upstream source, tests, fixtures, or message strings
//! were consulted or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "single-character-alternation";

/// Returns `true` when `term` is exactly a single literal `Character` node.
/// `CharacterClassEscape` (`\d`, `\w`, …) intentionally returns `false`.
fn is_single_character(term: &Term<'_>) -> bool {
    matches!(term, Term::Character(_))
}

/// Returns `true` when `disj` is a qualifying single-character alternation:
/// ≥2 alternatives and every alternative has exactly one term that is a
/// literal `Character`.
fn is_qualifying_disjunction(disj: &Disjunction<'_>) -> bool {
    if disj.body.len() < 2 {
        return false;
    }
    for alt in disj.body.iter() {
        if alt.body.len() != 1 {
            return false;
        }
        if !is_single_character(&alt.body[0]) {
            return false;
        }
    }
    true
}

/// Recurses into a [`Term`], descending into group, quantifier, and lookaround
/// bodies to find nested disjunctions.
fn collect_single_char_alternations_term<'a>(term: &Term<'a>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CapturingGroup(group) => {
            collect_single_char_alternations(&group.body, out);
        }
        Term::IgnoreGroup(group) => {
            collect_single_char_alternations(&group.body, out);
        }
        Term::Quantifier(quant) => {
            collect_single_char_alternations_term(&quant.body, out);
        }
        Term::LookAroundAssertion(look) => {
            collect_single_char_alternations(&look.body, out);
        }
        _ => {}
    }
}

/// Walks `disj`: when it qualifies as a single-character alternation, its span
/// is collected; then every nested term is recursed into to find more.
fn collect_single_char_alternations<'a>(disj: &Disjunction<'a>, out: &mut SmallVec<[Span; 8]>) {
    if is_qualifying_disjunction(disj) {
        out.push(disj.span);
    }
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_single_char_alternations_term(term, out);
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_single_character_alternation_with_pattern(
        &mut self,
        _lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        collect_single_char_alternations(&pattern.body, &mut out);
        for span in out {
            self.report(RULE_NAME, "singleCharAlternation", span);
        }
    }
}
