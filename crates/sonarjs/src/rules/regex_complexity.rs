//! Rule `regex-complexity` (SonarJS key S5843).
//!
//! A regular expression that combines many alternations, repetitions and
//! nested groups quickly becomes hard to read and to maintain. This rule scores
//! the *structural* complexity of each regex literal and reports the literal
//! when that score exceeds a configurable threshold (default **20**).
//!
//! ## How the score is computed
//!
//! Scoring walks the parsed regex AST while tracking a `nesting` level that
//! starts at `0` and is increased by one whenever the walk descends into a
//! group (capturing or non-capturing) or a look-around assertion. The score is
//! then accumulated as follows:
//!
//! - An **alternation** (`a|b|c`, i.e. a disjunction with two or more
//!   alternatives) adds `1 + nesting` for its first `|` and an extra `nesting`
//!   for every additional `|`. After scoring, the alternatives are walked at
//!   `nesting + 1`.
//! - A **quantifier / repetition** (`*`, `+`, `?`, `{n,m}` and lazy variants)
//!   adds `1 + nesting`, and its quantified sub-expression is walked at
//!   `nesting + 1`.
//! - A **back-reference** (`\1`, `\k<name>`) adds `1 + nesting`.
//! - Groups and look-arounds themselves add nothing directly; they only raise
//!   the `nesting` level for the constructs they contain, so deeply nested
//!   alternations and repetitions cost progressively more.
//!
//! Plain characters, character classes, dots, escapes and boundary assertions
//! (`^`, `$`, `\b`) contribute nothing.
//!
//! ## Scope (intentional under-report)
//!
//! Only regex **literals** (`/.../`) are scored, because they are the only form
//! whose pattern is reliably available as a parsed AST. Patterns built from a
//! string passed to `new RegExp("...")` are not scored here, since faithfully
//! reconstructing them would require constant/dataflow analysis that the runtime
//! does not provide. The rule therefore under-reports rather than risk false
//! positives.
//!
//! Behaviour is reproduced from the public RSPEC description (S5843) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use compact_str::ToCompactString;
use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Disjunction, Term};

use crate::scanner::Scanner;
use crate::types::DiagnosticData;

pub(crate) const RULE_NAME: &str = "regex-complexity";

/// Accumulates the structural-complexity score of `disj` (and everything nested
/// inside it) into `total`, given the current `nesting` level.
fn score_disjunction(disj: &Disjunction<'_>, nesting: u32, total: &mut u32) {
    let n_alts = disj.body.len();
    // A disjunction with two or more alternatives is an alternation: it both
    // contributes to the score and raises the nesting for its children.
    let child_nesting = if n_alts >= 2 {
        // First `|` costs `1 + nesting`; each subsequent `|` costs `nesting`.
        *total += 1 + nesting + (n_alts as u32 - 2) * nesting;
        nesting + 1
    } else {
        nesting
    };
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            score_term(term, child_nesting, total);
        }
    }
}

/// Accumulates the score of a single [`Term`] into `total`.
fn score_term(term: &Term<'_>, nesting: u32, total: &mut u32) {
    match term {
        Term::Quantifier(quant) => {
            *total += 1 + nesting;
            score_term(&quant.body, nesting + 1, total);
        }
        Term::CapturingGroup(group) => {
            score_disjunction(&group.body, nesting + 1, total);
        }
        Term::IgnoreGroup(group) => {
            score_disjunction(&group.body, nesting + 1, total);
        }
        Term::LookAroundAssertion(look) => {
            score_disjunction(&look.body, nesting + 1, total);
        }
        Term::IndexedReference(_) | Term::NamedReference(_) => {
            *total += 1 + nesting;
        }
        _ => {}
    }
}

impl Scanner<'_> {
    pub(crate) fn check_regex_complexity_with_pattern(
        &mut self,
        lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut total: u32 = 0;
        score_disjunction(&pattern.body, 0, &mut total);
        if total > self.options.regex_complexity_threshold {
            let data = DiagnosticData {
                value: Some(total.to_compact_string()),
                format: None,
            };
            self.report_with_data(RULE_NAME, "complexity", data, lit.span, None);
        }
    }
}
