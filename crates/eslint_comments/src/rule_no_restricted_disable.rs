//! `no-restricted-disable`: disallow `eslint-disable` comments for rules that
//! match a configured gitignore-style pattern list.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::disabled_area::build_disabled_area;
use crate::loc::to_rule_id_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Glob match where `*` matches any run of non-`/` characters and every other
/// byte is literal (the subset of gitignore globbing rule ids exercise).
fn glob_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let mut pi = 0;
    let mut ti = 0;
    let mut star: Option<(usize, usize)> = None;

    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some((pi, ti));
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some((star_pi, star_ti)) = star {
            if t[star_ti] == b'/' {
                return false;
            }
            star = Some((star_pi, star_ti + 1));
            pi = star_pi + 1;
            ti = star_ti + 1;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

/// A single pattern matches a rule id. A pattern without `/` matches the rule
/// id's last segment (gitignore "match at any level"); a pattern with `/` is
/// matched against the whole rule id.
fn pattern_matches(pattern: &str, rule_id: &str) -> bool {
    if pattern.contains('/') {
        glob_match(pattern, rule_id)
    } else {
        let basename = rule_id.rsplit('/').next().unwrap_or(rule_id);
        glob_match(pattern, basename)
    }
}

/// Whether the rule id is restricted by the ordered pattern list. Patterns are
/// applied in order; a `!`-prefixed pattern un-restricts a previous match
/// (gitignore negation). The last matching pattern wins.
fn is_restricted(patterns: &[&str], rule_id: &str) -> bool {
    let mut restricted = false;
    for pattern in patterns {
        let (negated, body) = match pattern.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, *pattern),
        };
        if pattern_matches(body, rule_id) {
            restricted = !negated;
        }
    }
    restricted
}

/// Join the patterns with `,` for the message when a whole-file disable (no
/// rule id) is restricted, matching upstream's `String(context.options)`.
fn joined_patterns(patterns: &[&str]) -> CompactString {
    let mut joined = CompactString::default();
    for (index, pattern) in patterns.iter().enumerate() {
        if index > 0 {
            joined.push(',');
        }
        joined.push_str(pattern);
    }
    joined
}

/// Report each disabled rule that the pattern list restricts (including
/// whole-file disables, which have no rule id).
pub fn no_restricted_disable(comments: &[Comment], patterns: &[&str]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();
    if patterns.is_empty() {
        return diagnostics;
    }

    let state = build_disabled_area(comments);

    for area in &state.areas {
        let restricted = match area.rule_id.as_deref() {
            None => true,
            Some(rule_id) => is_restricted(patterns, rule_id),
        };
        if !restricted {
            continue;
        }

        let comment = &comments[area.comment];
        let loc = to_rule_id_location(comment.value, comment.loc, area.rule_id.as_deref());
        let rule_id = match &area.rule_id {
            Some(rule_id) => rule_id.clone(),
            None => joined_patterns(patterns),
        };

        diagnostics.push(Diagnostic {
            message_id: CompactString::from("disallow"),
            data: DiagnosticData {
                rule_id: Some(rule_id),
                ..DiagnosticData::default()
            },
            loc,
        });
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use crate::directive::CommentKind;
    use crate::loc::{Location, Position};
    use crate::{Comment, no_restricted_disable};

    fn block<'a>(value: &'a str, line: u32) -> Comment<'a> {
        Comment {
            kind: CommentKind::Block,
            value,
            loc: Location {
                start: Position { line, column: 0 },
                end: Position {
                    line,
                    column: value.len() as i32 + 4,
                },
            },
        }
    }

    #[test]
    fn restricts_named_and_negated_patterns() {
        let comments = [
            block("eslint-disable eqeqeq", 1),
            block("eslint-disable no-undef", 2),
        ];
        // Restrict everything except eqeqeq → only no-undef is reported.
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_restricted_disable(&comments, &["*", "!eqeqeq"]));
        }
    }

    #[test]
    fn no_patterns_reports_nothing() {
        let comments = [block("eslint-disable eqeqeq", 1)];
        assert!(no_restricted_disable(&comments, &[]).is_empty());
    }

    #[test]
    fn wildcards_and_paths() {
        assert!(super::pattern_matches("*semi*", "semi-style"));
        assert!(super::pattern_matches("foo/*", "foo/bar"));
        assert!(!super::pattern_matches("foo/*", "foo/bar/baz"));
        assert!(super::pattern_matches("no-undef", "no-undef"));
        assert!(!super::pattern_matches("no-undef", "no-undefined"));
    }
}
