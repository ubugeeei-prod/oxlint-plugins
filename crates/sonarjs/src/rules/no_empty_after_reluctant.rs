//! Rule `no-empty-after-reluctant` (SonarJS key S6019).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC description
//! (S6019) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! A **reluctant (lazy) quantifier** whose minimum repetition count is zero
//! (`*?`, `??`, or `{0,n}?`) can always produce a zero-length match. When
//! every term that follows it in the same regex alternative is also
//! empty-matchable (or there are no following terms at all), the engine has no
//! reason to ever backtrack into the quantifier, making it permanently match
//! empty. This is almost certainly a programmer mistake.
//!
//! ## What is flagged
//!
//! A reluctant quantifier with `min == 0` where every subsequent term in the
//! same alternative `can_match_empty`:
//!
//! ```js
//! /a*?/          // flagged — lazy star, nothing after
//! /a??/          // flagged — lazy ?, nothing after
//! /a*?$/         // flagged — $ is zero-width
//! /a*?(?=b)/     // flagged — lookahead is zero-width
//! ```
//!
//! ## What is NOT flagged
//!
//! - `/a*?b/`   — literal `b` cannot match empty; pattern is not pointless.
//! - `/a+?/`    — `min == 1`; quantifier cannot skip all repetitions.
//! - `/a*/`     — greedy; not a reluctant quantifier.
//!
//! ## `can_match_empty` helper (conservative — under-reports, never FPs)
//!
//! A term is considered empty-matchable when:
//! - It is a `Quantifier` with `min == 0` (e.g. `b*`, `b?`, `b{0,3}`).
//! - It is a `BoundaryAssertion` (`^`, `$`, `\b`, `\B`) — zero-width.
//! - It is a `LookAroundAssertion` (`(?=…)` etc.) — zero-width.
//! - It is a `CapturingGroup` or `IgnoreGroup` where at least one alternative
//!   can match empty (all terms in that alternative are empty-matchable).
//! - Everything else (characters, character classes, dot, back-references, …)
//!   is conservatively treated as NOT empty-matchable, so this rule will
//!   under-report rather than emit false positives.
//!
//! ## Scope
//!
//! Only regex literals (`/pattern/`) are inspected. `new RegExp(…)` is a
//! documented follow-up. This rule does not require semantic analysis
//! (`needs_semantic` is NOT set).

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Alternative, Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-empty-after-reluctant";

/// Returns `true` when `term` can produce a zero-length match.
///
/// Conservative: anything not explicitly listed here returns `false`, which
/// means the rule will miss some cases rather than generate false positives.
fn can_match_empty(term: &Term<'_>) -> bool {
    match term {
        // A quantifier with min == 0 can always skip all its repetitions.
        Term::Quantifier(q) => q.min == 0,
        // Boundary assertions (^, $, \b, \B) are zero-width by definition.
        Term::BoundaryAssertion(_) => true,
        // Lookaround assertions ((?=…), (?!…), etc.) are zero-width.
        Term::LookAroundAssertion(_) => true,
        // A group can match empty when any one of its alternatives can.
        Term::CapturingGroup(g) => disj_can_match_empty(&g.body),
        Term::IgnoreGroup(g) => disj_can_match_empty(&g.body),
        // Characters, character classes, dot, back-references, and anything
        // else are conservatively treated as requiring at least one character.
        _ => false,
    }
}

/// Returns `true` when every term in `alt` can match empty.
/// An empty alternative (no terms) trivially matches empty.
fn alt_can_match_empty(alt: &Alternative<'_>) -> bool {
    alt.body.iter().all(|t| can_match_empty(t))
}

/// Returns `true` when at least one alternative in `disj` can match empty
/// (disjunction semantics: any one branch is sufficient).
fn disj_can_match_empty(disj: &Disjunction<'_>) -> bool {
    disj.body.iter().any(|a| alt_can_match_empty(a))
}

/// Walks an alternative, reporting reluctant quantifiers (greedy == false,
/// min == 0) where every subsequent term in the same alternative can match
/// empty (or there are no subsequent terms).
///
/// Also recurses into nested groups and lookarounds so that patterns at any
/// nesting depth are found.
fn collect_in_alternative<'a>(alt: &Alternative<'a>, out: &mut SmallVec<[Span; 8]>) {
    let terms = &alt.body;
    for (i, term) in terms.iter().enumerate() {
        match term {
            Term::Quantifier(q) => {
                // Recurse into the quantifier body in case it is a group.
                collect_in_term(&q.body, out);
                // Only flag *lazy* quantifiers that allow zero repetitions.
                if q.greedy || q.min != 0 {
                    continue;
                }
                // Flag when every following term can match empty (or none follow).
                if terms[i + 1..].iter().all(|t| can_match_empty(t)) {
                    out.push(q.span);
                }
            }
            Term::CapturingGroup(g) => collect_in_disj(&g.body, out),
            Term::IgnoreGroup(g) => collect_in_disj(&g.body, out),
            Term::LookAroundAssertion(l) => collect_in_disj(&l.body, out),
            _ => {}
        }
    }
}

/// Recurses into a single term that is a group or lookaround body.
/// Non-group terms are silently ignored.
fn collect_in_term<'a>(term: &Term<'a>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CapturingGroup(g) => collect_in_disj(&g.body, out),
        Term::IgnoreGroup(g) => collect_in_disj(&g.body, out),
        Term::LookAroundAssertion(l) => collect_in_disj(&l.body, out),
        _ => {}
    }
}

fn collect_in_disj<'a>(disj: &Disjunction<'a>, out: &mut SmallVec<[Span; 8]>) {
    for alt in disj.body.iter() {
        collect_in_alternative(alt, out);
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_empty_after_reluctant(&mut self, lit: &RegExpLiteral<'_>) {
        let spans = crate::regex_ast::with_parsed_regex_literal(lit, self.source_text, |pattern| {
            let mut out: SmallVec<[Span; 8]> = SmallVec::new();
            collect_in_disj(&pattern.body, &mut out);
            out
        });
        for span in spans {
            self.report(RULE_NAME, "emptyAfterReluctant", span);
        }
    }
}
