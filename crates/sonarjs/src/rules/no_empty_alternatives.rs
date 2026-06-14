//! Rule `no-empty-alternatives` (SonarJS key S6323).
//!
//! Clean-room port. An alternation (`a|b|c`) with an empty alternative matches
//! the empty string for that branch, which is almost always a mistake: a
//! stray, leading, or trailing `|`, or a `||` typo. Such code should drop the
//! empty branch or, if matching nothing is intended, use an explicit optional.
//!
//! ## What is flagged
//!
//! A `|` alternation (a disjunction with **two or more** alternatives) where at
//! least one alternative is empty:
//!
//! ```js
//! /a|/      // flagged — trailing empty alternative
//! /|a/      // flagged — leading empty alternative
//! /a||b/    // flagged — empty middle alternative
//! /(?:a|)/  // flagged — empty alternative inside a group
//! ```
//!
//! ## What is NOT flagged
//!
//! - `/a|b/`, `/a|b|c/` — every alternative has content.
//! - `/(?:)/`, `/()/` — a group whose whole body is empty has a *single* empty
//!   alternative (no `|`); that is the separate `no-empty-group` rule.
//! - `/(a)?/` — an optional group, not an alternation.
//!
//! Detects empty alternatives in **regex literals** only; the
//! `new RegExp("a|")` string form is a documented follow-up. Behaviour is
//! reproduced from the public RSPEC description (S6323) only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-empty-alternatives";

/// Recurses into a [`Term`], descending into group, quantifier, and lookaround
/// bodies so empty alternatives at any nesting depth are found.
fn collect_empty_alternatives_term<'a>(term: &Term<'a>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CapturingGroup(group) => collect_empty_alternatives(&group.body, out),
        Term::IgnoreGroup(group) => collect_empty_alternatives(&group.body, out),
        Term::Quantifier(quant) => collect_empty_alternatives_term(&quant.body, out),
        Term::LookAroundAssertion(look) => collect_empty_alternatives(&look.body, out),
        _ => {}
    }
}

/// Walks `disj`: when it is a real alternation (two or more alternatives), the
/// span of every empty alternative is collected; then every term is recursed
/// into to find nested alternations.
fn collect_empty_alternatives<'a>(disj: &Disjunction<'a>, out: &mut SmallVec<[Span; 8]>) {
    if disj.body.len() >= 2 {
        for alt in disj.body.iter() {
            if alt.body.is_empty() {
                out.push(alt.span);
            }
        }
    }
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_empty_alternatives_term(term, out);
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_empty_alternatives(&mut self, lit: &RegExpLiteral<'_>) {
        let spans = crate::regex_ast::with_parsed_regex_literal(lit, self.source_text, |pattern| {
            let mut out: SmallVec<[Span; 8]> = SmallVec::new();
            collect_empty_alternatives(&pattern.body, &mut out);
            out
        });
        for span in spans {
            self.report(RULE_NAME, "emptyAlternative", span);
        }
    }
}
