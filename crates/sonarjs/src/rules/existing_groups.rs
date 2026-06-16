//! Rule `existing-groups` (SonarJS key S6328).
//!
//! Clean-room port. A `String.prototype.replace` / `replaceAll` *replacement
//! string* may reference a capturing group from the search regular expression
//! via `$N` (numeric, 1-based) or `$<name>` (named). When the referenced group
//! does not exist, JavaScript does not throw: the reference is silently treated
//! as a literal (numeric refs out of range) or as the empty string (unknown
//! named refs), which almost always produces wrong output. This rule flags such
//! dangling references.
//!
//! Behaviour is reproduced from the public RSPEC S6328 description and the two
//! Noncompliant / Compliant examples published there only; no upstream
//! eslint-plugin-sonarjs source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! ## Deterministic, zero-false-positive subset
//!
//! Only a fully static call is analysed. The rule fires only when a
//! `CallExpression`:
//! - has a callee that is a static member expression whose property is
//!   `replace` or `replaceAll`, AND
//! - whose FIRST argument is a regular-expression *literal* (`/.../`), AND
//! - whose SECOND argument is a *string literal*.
//!
//! If either argument is dynamic (a variable, template literal, `new RegExp`,
//! etc.) the call is skipped: the group set / replacement text cannot be
//! verified statically, so we never guess.
//!
//! ## Group information (AST approach)
//!
//! The regex literal is parsed with the shared
//! [`crate::regex_ast::with_parsed_regex_literal`] helper and the resulting
//! `oxc_regular_expression` `Pattern` is walked recursively
//! (`Disjunction` -> `Alternative` -> `Term`, descending through quantifier
//! bodies, lookaround bodies and nested groups). Each `CapturingGroup` node
//! increments the capturing-group count; named capturing groups additionally
//! contribute their name to a name set. Non-capturing groups (`IgnoreGroup`,
//! e.g. `(?:...)`) and lookarounds are *not* counted, matching JS semantics.
//! The AST is preferred over text scanning because it already resolves
//! escaping, character classes, and the `(?<name>` / `(?<=` / `(?<!`
//! disambiguation correctly.
//!
//! ## JavaScript replacement-string semantics handled
//!
//! Walking the replacement string literal value character by character:
//! - `$$` is an escaped literal dollar; both characters are consumed and it is
//!   NOT a reference.
//! - `$&` (whole match), `` $` `` (text before match), `$'` (text after match)
//!   are specials, NOT group references.
//! - `$<name>` is a NAMED group reference. It is only ever a reference when the
//!   search regex actually has named capture groups: if the regex has NO named
//!   groups, JS emits `$<name>` literally, so it is never flagged. When named
//!   groups are present, `$<name>` is flagged if `name` is not one of them.
//! - `$` followed by a digit is a NUMERIC group reference (see below).
//! - any other `$x` is not a reference and is skipped.
//!
//! ## Conservative numeric rule (two-digit under-reporting)
//!
//! JS `GetSubstitution` reads up to TWO digits after `$`. We inspect the FIRST
//! digit `d`:
//! - `d == 0`: if another ASCII digit follows (`$0D`, e.g. `$01`) the pair can
//!   resolve to a valid two-digit group, so it is conservatively treated as
//!   valid and NOT flagged (this under-reports the impossible `$00`). A bare
//!   `$0` with no following digit can never resolve (groups are 1-based) -> flag.
//! - else if `d` (as a single-digit value) is greater than the total
//!   capturing-group count -> no valid one-digit interpretation -> flag.
//! - otherwise NOT flagged.
//!
//! This deliberately under-reports the two-digit form `$NN`: e.g. `$12` against
//! a regex with 3 groups is left alone, because JS may interpret it as group
//! `$1` followed by literal `2`. Under-reporting (missing a real bug) is
//! acceptable here; over-reporting (a false positive on valid code) is not, so
//! we only flag when *no* interpretation can be valid.
//!
//! ## Examples
//!
//! Flagged:
//! ```js
//! 'John Doe'.replace(/(\w+)\s(\w+)/, '$1, $0 $1');                 // $0 never exists
//! 'a'.replace(/(a)/, '$2');                                        // only 1 group
//! 'John Doe'.replace(/(?<firstName>\w+)/, '$<surname>');          // no such name
//! ```
//!
//! Not flagged:
//! ```js
//! 'John Doe'.replace(/(\w+)\s(\w+)/, '$2, $1 $2');                 // valid numeric
//! 'John Doe'.replace(/(?<firstName>\w+)/, '$<firstName>');        // valid named
//! 'a$b'.replace(/a/, '$$');                                        // escaped dollar
//! 'a'.replace(/a/, '$&');                                          // whole-match special
//! 'a'.replace(re, repl);                                           // dynamic args skipped
//! ```

use oxc_ast::ast::{CallExpression, Expression};
use oxc_regular_expression::ast::{Disjunction, Term};
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "existing-groups";

/// Capturing-group information extracted from a parsed regex pattern: the total
/// number of capturing groups and the set of named-group names.
#[derive(Default)]
struct GroupInfo<'a> {
    count: u32,
    names: SmallVec<[&'a str; 8]>,
}

fn collect_groups_term<'a>(term: &Term<'a>, info: &mut GroupInfo<'a>) {
    match term {
        Term::CapturingGroup(group) => {
            info.count += 1;
            if let Some(name) = &group.name {
                info.names.push(name.as_str());
            }
            collect_groups(&group.body, info);
        }
        Term::IgnoreGroup(group) => {
            collect_groups(&group.body, info);
        }
        Term::Quantifier(quant) => {
            collect_groups_term(&quant.body, info);
        }
        Term::LookAroundAssertion(look) => {
            collect_groups(&look.body, info);
        }
        _ => {}
    }
}

fn collect_groups<'a>(disj: &Disjunction<'a>, info: &mut GroupInfo<'a>) {
    for alt in disj.body.iter() {
        for term in alt.body.iter() {
            collect_groups_term(term, info);
        }
    }
}

/// Returns `true` when `replacement` contains at least one `$` reference that
/// cannot resolve against `info`.
fn replacement_has_dangling_reference(replacement: &str, info: &GroupInfo<'_>) -> bool {
    let bytes = replacement.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        // `$` at end of string: literal dollar, not a reference.
        let Some(&next) = bytes.get(i + 1) else {
            break;
        };
        match next {
            // `$$` escaped dollar; `$&`/`` $` ``/`$'` whole-match specials.
            b'$' | b'&' | b'`' | b'\'' => {
                i += 2;
            }
            // Named reference `$<name>`.
            b'<' => {
                // Per JS spec, if the search regex has NO named capture groups,
                // `$<name>` is emitted literally and is not a dangling
                // reference; skip the `$` and continue.
                if info.names.is_empty() {
                    i += 1;
                } else if let Some(close) = replacement[i + 2..].find('>') {
                    let name = &replacement[i + 2..i + 2 + close];
                    if !info.names.contains(&name) {
                        return true;
                    }
                    i += 2 + close + 1;
                } else {
                    // No closing `>`: not a valid named reference; skip the `$`.
                    i += 1;
                }
            }
            // Numeric reference; JS `GetSubstitution` reads up to two digits.
            b'0'..=b'9' => {
                if next == b'0' {
                    // Leading zero. If another ASCII digit follows (`$0D`), the
                    // pair can resolve to a valid two-digit group such as `$01`
                    // (= group 1); conservatively treat it as valid. A bare `$0`
                    // (no following digit) can never resolve: groups are 1-based.
                    if bytes.get(i + 2).is_some_and(u8::is_ascii_digit) {
                        i += 2;
                    } else {
                        return true;
                    }
                } else {
                    // First digit `1..=9`: flag only when the single-digit value
                    // exceeds the capturing-group count. The two-digit form (e.g.
                    // `$12`) is left to the conservative under-report.
                    let d = u32::from(next - b'0');
                    if d > info.count {
                        return true;
                    }
                    i += 2;
                }
            }
            // Any other `$x` is not a reference.
            _ => {
                i += 1;
            }
        }
    }
    false
}

impl Scanner<'_> {
    pub(crate) fn check_existing_groups(&mut self, it: &CallExpression<'_>) {
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

        // The parsed pattern (and its group names) are owned by a temporary
        // allocator inside the helper, so the entire dangling-reference check is
        // performed within the closure, returning only an owned `bool`.
        let replacement_value = replacement.value.as_str();
        let dangling =
            crate::regex_ast::with_parsed_regex_literal(regex, self.source_text, |pattern| {
                let mut info = GroupInfo::default();
                collect_groups(&pattern.body, &mut info);
                replacement_has_dangling_reference(replacement_value, &info)
            });
        if dangling {
            self.report(RULE_NAME, "existingGroups", it.span);
        }
    }
}
