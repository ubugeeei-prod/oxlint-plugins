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

/// A single (already non-negated) pattern matches a rule id, using the subset of
/// gitignore semantics that rule ids exercise (upstream matches with the
/// `ignore` package).
///
/// A pattern containing a non-trailing `/` is *anchored*: its slash-separated
/// segments must glob-match the rule id's leading segments, so `foo/*` matches
/// `foo/x` and — as a directory match — also `foo/x/y`. A pattern without a `/`
/// matches if it glob-matches *any* segment of the rule id (gitignore's "match
/// at any depth"), so a plugin name like `react` restricts every `react/*` rule
/// and `*semi*` matches `no-extra-semi`.
fn pattern_matches(pattern: &str, rule_id: &str) -> bool {
    // A purely trailing `/` (e.g. `foo/`) does not anchor; only an internal or
    // leading slash does. A leading slash anchors but is not itself a segment.
    let anchored = pattern.trim_end_matches('/').contains('/');
    let body = pattern.strip_prefix('/').unwrap_or(pattern);

    if anchored {
        let mut rule_segments = rule_id.split('/');
        for pattern_segment in body.split('/') {
            match rule_segments.next() {
                // A shorter anchored pattern is a directory match that covers
                // every deeper segment, so leftover rule segments still match.
                Some(rule_segment) if glob_match(pattern_segment, rule_segment) => {}
                _ => return false,
            }
        }
        true
    } else {
        rule_id.split('/').any(|segment| glob_match(body, segment))
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
        // An anchored pattern is a directory match: `foo/*` also covers deeper
        // segments (gitignore semantics, matching the upstream `ignore` package).
        assert!(super::pattern_matches("foo/*", "foo/bar/baz"));
        assert!(super::pattern_matches("no-undef", "no-undef"));
        assert!(!super::pattern_matches("no-undef", "no-undefined"));
    }

    #[test]
    fn slashless_pattern_matches_any_segment() {
        // A plugin name restricts every rule of that plugin (the common case
        // upstream supports via `ignore`'s match-at-any-depth behavior).
        assert!(super::pattern_matches("react", "react/no-array-index-key"));
        assert!(super::pattern_matches(
            "@typescript-eslint",
            "@typescript-eslint/no-unused-vars"
        ));
        // …but it must not match a different plugin whose name merely contains it.
        assert!(!super::pattern_matches("react", "preact/no-foo"));
    }

    #[test]
    fn anchored_pattern_stays_anchored() {
        assert!(!super::pattern_matches("foo/bar", "foo/barbaz"));
        assert!(super::pattern_matches("foo/bar", "foo/bar/deep"));
    }

    #[test]
    fn restricts_plugin_scoped_rules() {
        let comments = [block("eslint-disable react/no-foo, eqeqeq", 1)];
        // Banning the `react` plugin restricts `react/no-foo` but not `eqeqeq`.
        let diagnostics = no_restricted_disable(&comments, &["react"]);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].data.rule_id.as_deref(), Some("react/no-foo"));
    }
}
