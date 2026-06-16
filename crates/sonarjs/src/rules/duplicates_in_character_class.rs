//! Rule `duplicates-in-character-class` (SonarJS key S5869).
//!
//! A character class that lists the same character more than once (`[aa]`) is
//! redundant: the duplicate matches nothing extra and usually signals a typo or
//! a misunderstanding of the pattern.
//!
//! **Flagged** — a literal character that appears more than once in the same
//! character class:
//! - `/[aa]/`
//! - `/[abca]/` (the second `a`)
//!
//! **Not flagged**:
//! - `/[ab]/` — distinct characters.
//! - `/[a-z]/` — a range, not repeated literals.
//!
//! Narrow form: only exact duplicate *literal characters* within one class are
//! reported. Overlaps that involve ranges or class escapes (for example `\w`
//! overlapping `a`, or `\s` overlapping `\t`) are a documented follow-up, which
//! keeps the rule free of false positives. Behaviour is reproduced from the
//! public RSPEC description (S5869) only; no upstream source, tests, fixtures,
//! or message strings were consulted or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{CharacterClass, CharacterClassContents, Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "duplicates-in-character-class";

/// Pushes the span of every literal character that repeats an earlier literal
/// character in the same class body.
fn collect_duplicate_chars(class: &CharacterClass<'_>, out: &mut SmallVec<[Span; 8]>) {
    let mut seen: SmallVec<[u32; 8]> = SmallVec::new();
    for content in class.body.iter() {
        if let CharacterClassContents::Character(c) = content {
            if seen.contains(&c.value) {
                out.push(c.span);
            } else {
                seen.push(c.value);
            }
        }
    }
}

fn collect_in_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::CharacterClass(class) => collect_duplicate_chars(class, out),
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
    pub(crate) fn check_duplicates_in_character_class_with_pattern(
        &mut self,
        _lit: &RegExpLiteral<'_>,
        pattern: &oxc_regular_expression::ast::Pattern<'_>,
    ) {
        let mut out: SmallVec<[Span; 8]> = SmallVec::new();
        collect_in_disjunction(&pattern.body, &mut out);
        for span in out {
            self.report(RULE_NAME, "duplicateCharacter", span);
        }
    }
}
