//! Rule `anchor-precedence` (SonarJS key S5850).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC description
//! (S5850) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! In a regular expression like `/^a|b|c$/`, operator precedence means the `^`
//! anchors only the first alternative and `$` only the last:
//! `(^a)|(b)|(c$)`. The author almost certainly intended `/^(a|b|c)$/`.
//!
//! ## What is flagged
//!
//! A **top-level** alternation with two or more alternatives where the first
//! alternative begins with `^` or the last alternative ends with `$`, but the
//! anchor is not consistently applied across every branch:
//!
//! ```js
//! /^a|b|c$/   // flagged — ^ anchors only first, $ anchors only last
//! /^a|b/      // flagged — ^ anchors only the first alternative
//! /a|b$/      // flagged — $ anchors only the last alternative
//! ```
//!
//! ## What is NOT flagged
//!
//! - `/^(a|b|c)$/` — one top-level alternative; nested `|` is inside a group.
//! - `/^a$|^b$|^c$/` — every branch is fully anchored (first ends with `$`).
//! - `/a|b|c/` — no anchors at all.
//! - `/^\s+|\s+$/g` — exactly two alternatives with complementary anchors
//!   (the "trim" idiom), explicitly excluded.
//! - Any pattern where a middle alternative carries its own anchor (likely
//!   intentional mixed anchoring).
//!
//! Only the top-level disjunction is inspected; the rule never recurses into
//! groups or lookarounds.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{Alternative, BoundaryAssertionKind, Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "anchor-precedence";

/// Returns `true` when the first term of `alt` is a `^` start-boundary
/// assertion.
fn starts_with_caret(alt: &Alternative<'_>) -> bool {
    match alt.body.first() {
        Some(Term::BoundaryAssertion(a)) => matches!(a.kind, BoundaryAssertionKind::Start),
        _ => false,
    }
}

/// Returns `true` when the last term of `alt` is a `$` end-boundary
/// assertion.
fn ends_with_dollar(alt: &Alternative<'_>) -> bool {
    match alt.body.last() {
        Some(Term::BoundaryAssertion(a)) => matches!(a.kind, BoundaryAssertionKind::End),
        _ => false,
    }
}

/// Checks the top-level disjunction for the anchor-precedence problem and
/// pushes the disjunction span when a violation is found.
fn check_top_level_disjunction(disj: &Disjunction<'_>, out: &mut SmallVec<[Span; 8]>) {
    let alts = &disj.body;
    let len = alts.len();
    if len < 2 {
        return;
    }

    let first = match alts.first() {
        Some(a) => a,
        None => return,
    };
    let last = match alts.last() {
        Some(a) => a,
        None => return,
    };

    // Complementary-anchor ("trim") exception: exactly two alternatives where
    // the first starts with `^` and the second ends with `$`.  This covers
    // patterns like `/^\s+|\s+$/g` that are not a precedence mistake.
    if len == 2 && starts_with_caret(first) && ends_with_dollar(last) {
        return;
    }

    let has_start_anchor = starts_with_caret(first);
    let has_end_anchor = ends_with_dollar(last);
    if !has_start_anchor && !has_end_anchor {
        return;
    }

    // If the first alternative is fully anchored (it also ends with `$`) or
    // the last starts with `^`, the author is using intentional varied
    // anchoring across branches — not a precedence mistake.
    if ends_with_dollar(first) || starts_with_caret(last) {
        return;
    }

    // If any middle alternative carries its own anchor the pattern is more
    // likely intentional.
    for alt in alts.iter().skip(1).take(len - 2) {
        if starts_with_caret(alt) || ends_with_dollar(alt) {
            return;
        }
    }

    out.push(disj.span);
}

impl Scanner<'_> {
    pub(crate) fn check_anchor_precedence(&mut self, lit: &RegExpLiteral<'_>) {
        let spans = crate::regex_ast::with_parsed_regex_literal(lit, self.source_text, |pattern| {
            let mut out: SmallVec<[Span; 8]> = SmallVec::new();
            check_top_level_disjunction(&pattern.body, &mut out);
            out
        });
        for span in spans {
            self.report(RULE_NAME, "anchorPrecedence", span);
        }
    }
}
