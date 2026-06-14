//! Rule `no-control-regex` (SonarJS key S6324).
//!
//! A control character (U+0000 to U+001F) in a regular expression is almost
//! certainly unintentional. The rule flags control characters written as a
//! hexadecimal escape (`\xNN`), a unicode escape (`\uNNNN` or `\u{NN}`), or a
//! control-letter escape (`\cX`), including occurrences inside character
//! classes (both endpoints of a range are checked).
//!
//! The conventional named escapes `\t`, `\n`, `\r`, `\v`, and `\f` are NOT
//! flagged; they are `CharacterKind::SingleEscape` and are a normal,
//! intentional way to write whitespace characters.
//!
//! **Flagged**: `/\x1f/`, `/\cA/`, `/[\x00-\x1f]/`, and the `\u`-escape
//! equivalents in the control range.
//!
//! **Not flagged**: `/\t/`, `/\n/`, `/\r/` (named escapes); `/\x20/` (value
//! `0x20` is above the control range); `/A/` (a literal letter).
//!
//! Narrow form: literal (raw) control characters and octal escapes are a
//! documented follow-up. Behaviour is reproduced from the public RSPEC
//! description (S6324) only; no upstream source, tests, fixtures, or message
//! strings were consulted or copied.

use oxc_ast::ast::RegExpLiteral;
use oxc_regular_expression::ast::{
    Character, CharacterClassContents, CharacterKind, Disjunction, Term,
};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-control-regex";

/// Returns `true` when `c` is a control character written in one of the
/// flagged escape forms (`\xNN`, `\uNNNN`/`\u{NN}`, or `\cX`).
fn is_control_char(c: &Character) -> bool {
    c.value <= 0x1F
        && matches!(
            c.kind,
            CharacterKind::ControlLetter
                | CharacterKind::HexadecimalEscape
                | CharacterKind::UnicodeEscape
        )
}

/// Collects spans of control characters found inside a `CharacterClass` body.
fn collect_in_class_content(content: &CharacterClassContents<'_>, out: &mut SmallVec<[Span; 8]>) {
    match content {
        CharacterClassContents::Character(c) => {
            if is_control_char(c) {
                out.push(c.span);
            }
        }
        CharacterClassContents::CharacterClassRange(r) => {
            if is_control_char(&r.min) {
                out.push(r.min.span);
            }
            if is_control_char(&r.max) {
                out.push(r.max.span);
            }
        }
        CharacterClassContents::NestedCharacterClass(cc) => {
            for nested in cc.body.iter() {
                collect_in_class_content(nested, out);
            }
        }
        CharacterClassContents::CharacterClassEscape(_)
        | CharacterClassContents::UnicodePropertyEscape(_)
        | CharacterClassContents::ClassStringDisjunction(_) => {}
    }
}

/// Collects spans of control characters found within a single `Term`.
fn collect_in_term(term: &Term<'_>, out: &mut SmallVec<[Span; 8]>) {
    match term {
        Term::Character(c) => {
            if is_control_char(c) {
                out.push(c.span);
            }
        }
        Term::CharacterClass(cc) => {
            for content in cc.body.iter() {
                collect_in_class_content(content, out);
            }
        }
        Term::CapturingGroup(g) => {
            collect_in_disjunction(&g.body, out);
        }
        Term::IgnoreGroup(g) => {
            collect_in_disjunction(&g.body, out);
        }
        Term::LookAroundAssertion(l) => {
            collect_in_disjunction(&l.body, out);
        }
        Term::Quantifier(q) => {
            collect_in_term(&q.body, out);
        }
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
    pub(crate) fn check_no_control_regex(&mut self, lit: &RegExpLiteral<'_>) {
        let spans = crate::regex_ast::with_parsed_regex_literal(lit, self.source_text, |pattern| {
            let mut out: SmallVec<[Span; 8]> = SmallVec::new();
            for alt in pattern.body.body.iter() {
                for term in alt.body.iter() {
                    collect_in_term(term, &mut out);
                }
            }
            out
        });
        for span in spans {
            self.report(RULE_NAME, "controlCharacter", span);
        }
    }
}
