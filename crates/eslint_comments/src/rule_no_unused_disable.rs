//! `no-unused-disable`: disallow `eslint-disable` comments that suppress no
//! problem.
//!
//! Upstream implements this by patching ESLint's `Linter#verify` to enable
//! `reportUnusedDisableDirectives` and is deprecated in favor of that built-in.
//! There is no equivalent hook in the Oxlint plugin API, so this is an
//! approximation: it reuses the shared disabled-area ranges and marks a disable
//! as unused when no reported problem matches its rule within its range. The
//! problems come from `sourceCode.getDisableDirectives().problems` at runtime
//! (so this rule is exercised with synthetic problems in tests, not via the
//! espree replay harness).

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::disabled_area::build_disabled_area;
use crate::loc::{lte, to_rule_id_location};
use crate::{Comment, Diagnostic, DiagnosticData, Problem};

/// Report each disable directive whose range contains no matching problem.
pub fn no_unused_disable(comments: &[Comment], problems: &[Problem]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();
    let state = build_disabled_area(comments);

    for area in &state.areas {
        let suppressed_something = problems.iter().any(|problem| {
            let rule_matches = match area.rule_id.as_deref() {
                None => true,
                Some(rule_id) => problem.rule_id == Some(rule_id),
            };
            let within = lte(area.start, problem.position)
                && area.end.is_none_or(|end| lte(problem.position, end));
            rule_matches && within
        });

        if suppressed_something {
            continue;
        }

        let comment = &comments[area.comment];
        let loc = to_rule_id_location(comment.value, comment.loc, area.rule_id.as_deref());
        let message_id = if area.rule_id.is_some() {
            "unusedRule"
        } else {
            "unused"
        };

        diagnostics.push(Diagnostic {
            message_id: CompactString::from(message_id),
            data: DiagnosticData {
                rule_id: area.rule_id.clone(),
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
    use crate::{Comment, Problem, no_unused_disable};

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
    fn reports_disable_without_matching_problem() {
        let comments = [block("eslint-disable no-undef", 1)];
        // A problem for a different rule does not justify the disable.
        let problems = [Problem {
            rule_id: Some("eqeqeq"),
            position: Position { line: 5, column: 0 },
        }];
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_unused_disable(&comments, &problems));
        }
    }

    #[test]
    fn used_disable_is_ok() {
        let comments = [block("eslint-disable no-undef", 1)];
        let problems = [Problem {
            rule_id: Some("no-undef"),
            position: Position { line: 3, column: 4 },
        }];
        assert!(no_unused_disable(&comments, &problems).is_empty());
    }
}
