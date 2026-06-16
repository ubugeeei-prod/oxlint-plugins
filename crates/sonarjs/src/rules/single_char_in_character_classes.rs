//! Rule `single-char-in-character-classes` (SonarJS key S6397).
//!
//! A character class that contains exactly one literal character (`[a]`,
//! `[.]`) is redundant: the surrounding brackets add nothing, and the single
//! character (escaped if necessary) can be used directly. Such classes are a
//! sign of an over-complicated pattern.
//!
//! **Flagged** (a non-negated character class whose only element is one
//! literal character):
//! - `/[a]/`
//! - `/[.]/`
//! - `/(?:[5])/`, `/[a]+/` (at any nesting depth)
//!
//! **Not flagged**:
//! - `/[ab]/`, `/[a-z]/` — more than one element, or a range.
//! - `/[^a]/` — a negated class is meaningful (any char *except* `a`).
//! - `/a/` — no character class at all.
//!
//! Narrow form: only a single *literal character* element is reported; a
//! single class-escape element (`[\d]`) or a nested class is a documented
//! follow-up. Behaviour is reproduced from the public RSPEC description (S6397)
//! only; no upstream source, tests, fixtures, or message strings were consulted
//! or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{CharacterClass, CharacterClassContents, Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "single-char-in-character-classes";

/// Returns `true` when `class` is a non-negated character class whose only
/// element is a single literal character.
fn is_single_char_class(class: &CharacterClass<'_>) -> bool {
    !class.negative
        && class.body.len() == 1
        && matches!(class.body[0], CharacterClassContents::Character(_))
}

fn collect_in_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CharacterClass(class) => {
            if is_single_char_class(class) {
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
    pub(crate) fn check_single_char_in_character_classes_with_pattern(
        &mut self,
        _lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        collect_in_disjunction(&pattern.body, &mut out);
        for span in out {
            self.report(RULE_NAME, "singleCharInCharacterClass", span);
        }
    }
}
