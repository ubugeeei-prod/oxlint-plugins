//! Rule `unused-named-groups` (SonarJS key S5860).
//!
//! Clean-room port. Naming a capturing group in a regular expression
//! (`(?<year>\d{4})`) only pays off if the name is actually referenced — through
//! a backreference (`\k<year>`), a replacement string (`$<year>`), or the match
//! result's `groups` object (`m.groups.year`). A named group that is never
//! referenced is dead weight: it suggests a typo or forgotten code, and the
//! reader is misled into thinking the name carries meaning.
//!
//! Behaviour is reproduced from the public RSPEC S5860 description only; no
//! upstream eslint-plugin-sonarjs source, tests, fixtures, or message strings
//! were consulted or copied.
//!
//! ## Why a narrow, zero-false-positive subset
//!
//! The general rule needs dataflow/liveness analysis: a named group is "used"
//! whenever *any* downstream code reads `match.groups.<name>` on a result of the
//! regex, possibly many statements away and through aliases. That is beyond the
//! syntactic / scope-resolution capabilities available here, so reporting on an
//! arbitrary regex literal would produce false positives the moment the result
//! is consumed elsewhere.
//!
//! This port therefore restricts itself to the one context where the *complete*
//! set of possible group consumers is statically visible at the call site:
//!
//! - a `CallExpression` whose callee is a static member access named `replace`
//!   or `replaceAll`, AND
//! - whose FIRST argument is a regular-expression *literal* (`/.../`), AND
//! - whose SECOND argument is a *string literal*.
//!
//! In `str.replace(/(?<x>…)/, 'literal')` the match's `groups` object is never
//! exposed to user code: the only things that can reference a named group are
//! `\k<name>` backreferences inside the pattern and `$<name>` references inside
//! the replacement string. If a function replacement is used instead, the
//! callback receives a `groups` object and could read any name, so those calls
//! are deliberately skipped. Likewise a dynamic regex (`new RegExp`, a variable)
//! or a dynamic replacement is skipped. This guarantees no false positives at
//! the cost of under-reporting the (common) `match`/`exec` + `.groups` cases,
//! which is the project's preferred trade-off.
//!
//! ## Flagged
//! ```js
//! '2020-01'.replace(/(?<year>\d{4})-(?<month>\d{2})/, 'literal'); // both unused
//! 'abc'.replace(/(?<first>\w)/, 'X');                             // first unused
//! ```
//!
//! ## Not flagged
//! ```js
//! 'John Doe'.replace(/(?<first>\w+) (?<last>\w+)/, '$<last> $<first>'); // both referenced
//! 'aa'.replace(/(?<c>\w)\k<c>/, 'X');                                   // backreference
//! 'x'.replace(/(\d+)/, '$1');                                           // no named groups
//! str.replace(/(?<y>\d)/, m => m.groups.y);                            // function replacement
//! str.match(/(?<y>\d)/);                                                // not a replace call
//! ```

use oxc_ast::ast::{CallExpression, Expression};
use oxc_regular_expression::ast::{Disjunction, Term};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "unused-named-groups";

/// Named-group information collected from a parsed regex pattern: every named
/// capturing group (name + source span) and every named backreference name.
#[derive(Default)]
struct NamedGroupInfo<'a> {
    defs: SmallVec<[(&'a str, Span); 8]>,
    backrefs: SmallVec<[&'a str; 8]>,
}

fn collect_term<'a>(term: &Term<'a>, info: &mut NamedGroupInfo<'a>) {
    match term {
        Term::CapturingGroup(group) => {
            if let Some(name) = &group.name {
                info.defs.push((name.as_str(), group.span));
            }
            collect_disjunction(&group.body, info);
        }
        Term::IgnoreGroup(group) => {
            collect_disjunction(&group.body, info);
        }
        Term::Quantifier(quant) => {
            collect_term(&quant.body, info);
        }
        Term::LookAroundAssertion(look) => {
            collect_disjunction(&look.body, info);
        }
        Term::NamedReference(reference) => {
            info.backrefs.push(reference.name.as_str());
        }
        _ => {}
    }
}

fn collect_disjunction<'a>(disj: &Disjunction<'a>, info: &mut NamedGroupInfo<'a>) {
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_term(term, info);
        }
    }
}

/// Returns `true` when the replacement string contains a `$<name>` reference to
/// `name`, honouring `$$` (escaped dollar) so an escaped sequence is not mistaken
/// for a reference.
fn replacement_references(replacement: &str, name: &str) -> bool {
    let bytes = replacement.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        match bytes.get(i + 1) {
            // `$$` is an escaped literal dollar; consume both.
            Some(b'$') => i += 2,
            // Named reference `$<...>`.
            Some(b'<') => {
                if let Some(close) = replacement[i + 2..].find('>') {
                    if &replacement[i + 2..i + 2 + close] == name {
                        return true;
                    }
                    i += 2 + close + 1;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    false
}

impl Scanner<'_> {
    pub(crate) fn check_unused_named_groups(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();
        if method != "replace" && method != "replaceAll" {
            return;
        }
        if it.arguments.len() < 2 {
            return;
        }
        let Some(Expression::RegExpLiteral(regex)) = it.arguments[0]
            .as_expression()
            .map(Expression::get_inner_expression)
        else {
            return;
        };
        let Some(Expression::StringLiteral(replacement)) = it.arguments[1]
            .as_expression()
            .map(Expression::get_inner_expression)
        else {
            return;
        };

        let replacement_value = replacement.value.as_str();

        // The parsed pattern is owned by a temporary allocator inside the helper,
        // so all borrowing work happens in the closure; only owned report data
        // (name + span) escapes.
        let unused: SmallVec<[(CompactString, Span); 8]> =
            crate::regex_ast::with_parsed_regex_literal(regex, self.source_text, |pattern| {
                let mut info = NamedGroupInfo::default();
                collect_disjunction(&pattern.body, &mut info);
                info.defs
                    .iter()
                    .filter(|(name, _)| {
                        !info.backrefs.contains(name)
                            && !replacement_references(replacement_value, name)
                    })
                    .map(|(name, span)| (CompactString::from(*name), *span))
                    .collect()
            });

        for (name, span) in unused {
            let data = DiagnosticData {
                value: Some(name),
                format: None,
            };
            self.report_with_data(RULE_NAME, "unusedNamedGroups", data, span, None);
        }
    }
}
