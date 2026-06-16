//! Rule `no-regex-spaces` (SonarJS key S6326).
//!
//! A run of two or more consecutive literal space characters inside a regex
//! pattern is hard to count visually and should use an explicit quantifier
//! instead (e.g. ` {3}`). Spaces inside character classes are excluded.
//!
//! ## What is flagged
//!
//! A run of **two or more consecutive literal space characters** (U+0020) in
//! a regex literal pattern that are NOT inside a character class and NOT
//! expressed via a quantifier:
//!
//! ```js
//! /foo   bar/ // flagged — 3 consecutive spaces
//! /a  b/      // flagged — 2 consecutive spaces
//! ```
//!
//! ## What is NOT flagged
//!
//! - `/a b/`      — a single space.
//! - `/a {3}b/`   — one space + quantifier (parsed as a single `Quantifier`
//!   term, not two consecutive `Character` terms).
//! - `/[  ]{2}/`  — spaces inside a character class live in `CharacterClass`
//!   body, not in `Alternative.body`, so they are not detected.
//!
//! Behaviour is reproduced from the public RSPEC description (S6326) and the
//! ESLint `no-regex-spaces` rule specification only; no upstream source,
//! tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-regex-spaces";

/// Returns the character `Span` if `term` is a literal space character
/// (U+0020), otherwise `None`.
fn space_char_span(term: &Term<'_>) -> Option<Span> {
    match term {
        Term::Character(c) if c.value == 0x20 => Some(c.span),
        _ => None,
    }
}

/// Scans `terms` for maximal runs of two or more consecutive literal spaces,
/// pushing one `Span` per run onto `out`.  After detecting runs at this level,
/// recurses into each term so that nested group / quantifier / lookaround
/// bodies are also scanned.
fn scan_alternative(terms: &[Term<'_>], out: &mut SmallVec<[Span; 8]>) {
    // Detect maximal runs of consecutive literal space characters at this level.
    let mut i = 0;
    while i < terms.len() {
        if let Some(first) = space_char_span(&terms[i]) {
            let mut last = first;
            let mut j = i + 1;
            while j < terms.len() {
                if let Some(s) = space_char_span(&terms[j]) {
                    last = s;
                    j += 1;
                } else {
                    break;
                }
            }
            if j - i >= 2 {
                out.push(Span::new(first.start, last.end));
            }
            i = j;
        } else {
            i += 1;
        }
    }
    // Recurse into nested group / quantifier / lookaround bodies.
    for term in terms {
        recurse_term(term, out);
    }
}

/// Recurses into a [`Term`], descending into group, quantifier, and lookaround
/// bodies so that consecutive spaces at any nesting depth are found.
fn recurse_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CapturingGroup(group) => {
            scan_disjunction(&group.body, out);
        }
        Term::IgnoreGroup(group) => {
            scan_disjunction(&group.body, out);
        }
        Term::LookAroundAssertion(look) => {
            scan_disjunction(&look.body, out);
        }
        Term::Quantifier(quant) => {
            recurse_term(&quant.body, out);
        }
        _ => {}
    }
}

fn scan_disjunction(disj: &Disjunction<'_>, out: &mut SmallVec<[Span; 8]>) {
    for alt in disj.body.iter() {
        scan_alternative(&alt.body, out);
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_regex_spaces_with_pattern(
        &mut self,
        _lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        for alt in pattern.body.body.iter() {
            scan_alternative(&alt.body, &mut out);
        }
        for span in out {
            self.report(RULE_NAME, "multipleSpaces", span);
        }
    }
}
