//! Rule `no-misleading-character-class` (SonarJS key S5868).
//!
//! In a regular expression *without* the `u` (unicode) or `v` (unicodeSets)
//! flag, a character class element that looks like a single perceived character
//! but is actually an astral code point (value `> 0xFFFF`, such as `👍`) is
//! encoded as a UTF-16 surrogate pair. The regex engine processes the pattern as
//! UTF-16 code units, so the single perceived character is silently split into
//! its two surrogate halves; the class then matches either surrogate half rather
//! than the intended character, and does not match what it appears to. With the
//! `u`/`v` flag the engine treats the code point as a single unit, which is
//! correct, so those regexes are never flagged.
//!
//! Detection mechanism: oxc's regex parser, when no `u`/`v` flag is present,
//! already splits an astral literal into two adjacent `Character` elements — a
//! high surrogate (`0xD800..=0xDBFF`) immediately followed by a low surrogate
//! (`0xDC00..=0xDFFF`). We flag a character class whose body contains such an
//! adjacent surrogate pair (and, defensively, any single character whose value
//! is already `> 0xFFFF`). A high surrogate paired with a low surrogate is
//! unambiguously a split astral code point, which keeps the rule free of false
//! positives.
//!
//! **Flagged** (astral code point inside a character class, no `u`/`v` flag):
//! - `/[👍]/`
//! - `/[a👍b]/`
//!
//! **Not flagged**:
//! - `/[👍]/u`, `/[👍]/v` — the flag makes the code point a single unit.
//! - `/[abc]/` — BMP-only character class.
//! - `/👍/` — astral char outside a character class (out of scope here).
//!
//! **Conservative scope (intentional under-report):** only the
//! astral-code-point-in-character-class case is handled, because it is the
//! clearest zero-false-positive signal. Other misleading constructs that the
//! ESLint-core rule of the same name detects — combining-mark sequences, ZWJ
//! emoji sequences, regional-indicator pairs — require Unicode segmentation and
//! are false-positive-prone, so they are deliberately not reported here.
//!
//! Behaviour is reproduced from the public RSPEC description (S5868) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{RegExpFlags, RegExpLiteral};
use oxc_regular_expression::ast::{CharacterClass, CharacterClassContents, Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

/// A code point that requires a UTF-16 surrogate pair (is outside the Basic
/// Multilingual Plane) is "astral".
const MAX_BMP_CODE_POINT: u32 = 0xFFFF;

pub(crate) const RULE_NAME: &str = "no-misleading-character-class";

fn is_high_surrogate(value: u32) -> bool {
    (0xD800..=0xDBFF).contains(&value)
}

fn is_low_surrogate(value: u32) -> bool {
    (0xDC00..=0xDFFF).contains(&value)
}

/// Returns `true` when the body of `class` contains an astral code point, either
/// as a single `Character` whose value is already `> 0xFFFF`, or as an adjacent
/// high-surrogate / low-surrogate `Character` pair (how the parser represents a
/// split astral literal when no `u`/`v` flag is set).
fn class_contains_astral_char(class: &CharacterClass<'_>) -> bool {
    let body = &class.body;
    for (i, content) in body.iter().enumerate() {
        let CharacterClassContents::Character(c) = content else {
            continue;
        };
        if c.value > MAX_BMP_CODE_POINT {
            return true;
        }
        if is_high_surrogate(c.value)
            && matches!(
                body.get(i + 1),
                Some(CharacterClassContents::Character(next)) if is_low_surrogate(next.value)
            )
        {
            return true;
        }
    }
    false
}

fn collect_in_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CharacterClass(class) => {
            if class_contains_astral_char(class) {
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
    pub(crate) fn check_no_misleading_character_class_with_pattern(
        &mut self,
        lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        // With the `u`/`v` flag the engine treats an astral code point as a
        // single unit, so the class is not misleading.
        if lit.regex.flags.intersects(RegExpFlags::U | RegExpFlags::V) {
            return;
        }
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        collect_in_disjunction(&pattern.body, &mut out);
        for span in out {
            self.report(RULE_NAME, "misleadingCharacterClass", span);
        }
    }
}
