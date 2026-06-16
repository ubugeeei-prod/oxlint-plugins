//! Rule `slow-regex` (SonarJS key S5852).
//!
//! Some regular expressions can take super-linear (often exponential) time to
//! match certain crafted inputs, because the backtracking engine explores an
//! explosive number of ways to split the input among nested repetitions. This
//! is the classic "Regular expression Denial of Service" (ReDoS) hazard.
//!
//! Detecting *every* super-linear regex is undecidable, so this port flags ONLY
//! the unambiguous textbook shape of a **nested unbounded quantifier**: an
//! unbounded quantifier applied to a group whose body itself contains another
//! unbounded quantifier — e.g. `(a+)+`, `(a*)*`, `(.*)+`, `(\d+)+`. That shape
//! is genuinely super-linear and reporting it carries no false positives.
//!
//! ### "Unbounded" quantifier
//! A `Term::Quantifier` is unbounded when it has no upper bound (`max == None`).
//! This covers `+` (min 1), `*` (min 0), and `{n,}` (min n). Bounded
//! quantifiers (`?`, `{3}`, `{2,5}`, `{2,3}`) have `max == Some(_)` and can only
//! repeat a finite number of times, so they are never super-linear and are
//! never flagged.
//!
//! **Flagged** (unbounded quantifier over a group that contains an unbounded
//! quantifier at the top level of one of its alternatives):
//! - `/(a+)+/`
//! - `/(a*)*/`
//! - `/(.*)+$/`
//! - `/(\d+)+/`
//!
//! **Not flagged**:
//! - `/(ab)+/` — the inner group has no quantifier.
//! - `/a+/` — a single quantifier, nothing nested.
//! - `/(a{2,5})+/` — the inner quantifier is bounded.
//! - `/(a+){2,3}/` — the outer quantifier is bounded.
//!
//! **Conservative scope (intentional under-report):** only the direct
//! nested-unbounded-quantifier-on-a-group shape is reported. Other ReDoS
//! constructs that require NFA/ambiguity analysis — overlapping alternation
//! (`(a|a)*`), quantified-overlap across concatenation, etc. — are
//! false-positive-prone and are deliberately NOT reported here.
//!
//! Behaviour is reproduced from the public RSPEC description (S5852) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "slow-regex";

/// A quantifier with no upper bound (`+`, `*`, `{n,}`) is "unbounded" and can
/// repeat arbitrarily many times.
fn is_unbounded(quant: &oxc_regular_expression::ast::Quantifier<'_>) -> bool {
    quant.max.is_none()
}

/// Returns `true` when `disj` has any alternative containing, at its top level,
/// an unbounded quantifier — the inner repetition that makes a wrapping
/// unbounded quantifier super-linear.
fn has_top_level_unbounded_quantifier(disj: &Disjunction<'_>) -> bool {
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            if matches!(term, Term::Quantifier(inner) if is_unbounded(inner)) {
                return true;
            }
        }
    }
    false
}

fn collect_in_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::Quantifier(quant) => {
            // A nested unbounded quantifier: an unbounded quantifier whose body
            // is a group that itself contains an unbounded quantifier.
            if is_unbounded(quant) {
                let inner = match &quant.body {
                    Term::CapturingGroup(group) => Some(&group.body),
                    Term::IgnoreGroup(group) => Some(&group.body),
                    _ => None,
                };
                if inner.is_some_and(has_top_level_unbounded_quantifier) {
                    out.push(quant.span);
                }
            }
            // Keep walking so nested occurrences anywhere are found.
            collect_in_term(&quant.body, out);
        }
        Term::CapturingGroup(group) => collect_in_disjunction(&group.body, out),
        Term::IgnoreGroup(group) => collect_in_disjunction(&group.body, out),
        Term::LookAroundAssertion(look) => collect_in_disjunction(&look.body, out),
        _ => {}
    }
}

fn collect_in_disjunction(disj: &Disjunction<'_>, out: &mut SmallVec<[Span; 8]>) {
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_in_term(term, out);
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_slow_regex_with_pattern(
        &mut self,
        _lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        collect_in_disjunction(&pattern.body, &mut out);
        for span in out {
            self.report(RULE_NAME, "slowRegex", span);
        }
    }
}
