//! Rule `concise-regex` (SonarJS key S6353).
//!
//! Some verbose character classes have a well-known shorthand escape that means
//! exactly the same thing. Spelling them out (`[0-9]`) instead of using the
//! shorthand (`\d`) is harder to read and offers no benefit.
//!
//! This narrow port flags ONLY the three character classes whose meaning is an
//! exact, unambiguous match for a shorthand, which keeps it free of false
//! positives:
//! - `[0-9]` → `\d`
//! - `[^0-9]` → `\D`
//! - `[A-Za-z0-9_]` (the three ranges in any order, plus `_`) → `\w`
//!
//! **Flagged**:
//! - `/[0-9]/`
//! - `/[^0-9]/`
//! - `/[A-Za-z0-9_]/`, `/[a-zA-Z0-9_]/`, `/[_0-9a-zA-Z]/` (order-insensitive)
//!
//! **Not flagged**:
//! - `/[0-9a]/` — an extra member.
//! - `/[a-z]/` — not one of the canonical forms.
//! - `/[A-Za-z0-9]/` — missing the `_`, so it is not `\w`.
//! - `/\d/`, `/\w/` — already concise.
//!
//! Behaviour is reproduced from the public RSPEC description (S6353) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{CharacterClass, CharacterClassContents, Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "concise-regex";

const DIGIT_MIN: u32 = b'0' as u32; // 0x30
const DIGIT_MAX: u32 = b'9' as u32; // 0x39
const UPPER_MIN: u32 = b'A' as u32; // 0x41
const UPPER_MAX: u32 = b'Z' as u32; // 0x5A
const LOWER_MIN: u32 = b'a' as u32; // 0x61
const LOWER_MAX: u32 = b'z' as u32; // 0x7A
const UNDERSCORE: u32 = b'_' as u32; // 0x5F

/// Returns `true` when `range` spans exactly `min..=max`.
fn is_range(content: &CharacterClassContents<'_>, min: u32, max: u32) -> bool {
    matches!(
        content,
        CharacterClassContents::CharacterClassRange(range)
            if range.min.value == min && range.max.value == max
    )
}

/// `[0-9]` (or `[^0-9]`): a single `0-9` range.
fn is_digit_class(class: &CharacterClass<'_>) -> bool {
    class.body.len() == 1 && is_range(&class.body[0], DIGIT_MIN, DIGIT_MAX)
}

/// `[A-Za-z0-9_]`: exactly the three word ranges plus a literal `_`, in any
/// order, and nothing else.
fn is_word_class(class: &CharacterClass<'_>) -> bool {
    if class.negative || class.body.len() != 4 {
        return false;
    }
    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;
    let mut has_underscore = false;
    for content in class.body.iter() {
        match content {
            CharacterClassContents::CharacterClassRange(range)
                if range.min.value == UPPER_MIN && range.max.value == UPPER_MAX =>
            {
                has_upper = true;
            }
            CharacterClassContents::CharacterClassRange(range)
                if range.min.value == LOWER_MIN && range.max.value == LOWER_MAX =>
            {
                has_lower = true;
            }
            CharacterClassContents::CharacterClassRange(range)
                if range.min.value == DIGIT_MIN && range.max.value == DIGIT_MAX =>
            {
                has_digit = true;
            }
            CharacterClassContents::Character(c) if c.value == UNDERSCORE => {
                has_underscore = true;
            }
            _ => return false,
        }
    }
    has_upper && has_lower && has_digit && has_underscore
}

/// Returns `true` when `class` is exactly one of the three canonical verbose
/// forms that have a shorthand (`\d`, `\D`, or `\w`).
fn is_concise_candidate(class: &CharacterClass<'_>) -> bool {
    // `[0-9]` (→ \d) and `[^0-9]` (→ \D) share the single-range body; the
    // `negative` flag only switches the shorthand letter, not whether it fires.
    is_digit_class(class) || is_word_class(class)
}

fn collect_in_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CharacterClass(class) => {
            if is_concise_candidate(class) {
                out.push(class.span);
            }
        }
        Term::CapturingGroup(group) => collect_in_disjunction(&group.body, out),
        Term::IgnoreGroup(group) => collect_in_disjunction(&group.body, out),
        Term::LookAroundAssertion(look) => collect_in_disjunction(&look.body, out),
        Term::Quantifier(quant) => collect_in_term(&quant.body, out),
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
    pub(crate) fn check_concise_regex(&mut self, lit: &RegExpLiteral<'_>) {
        let spans = crate::regex_ast::with_parsed_regex_literal(lit, self.source_text, |pattern| {
            let mut out: SmallVec<[Span; 8]> = SmallVec::new();
            collect_in_disjunction(&pattern.body, &mut out);
            out
        });
        for span in spans {
            self.report(RULE_NAME, "conciseRegex", span);
        }
    }
}
