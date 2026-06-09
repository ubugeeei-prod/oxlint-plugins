//! `no-use`: disallow ESLint directive comments (except an allowed set).

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::directive::parse_directive_comment;
use crate::loc::to_force_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Report every directive comment whose kind is not in `allowed`.
pub fn no_use(comments: &[Comment], allowed: &[&str]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();

    for comment in comments {
        let same_line = comment.loc.start.line == comment.loc.end.line;
        let Some(parsed) = parse_directive_comment(comment.kind, comment.value, same_line) else {
            continue;
        };

        if !allowed.contains(&parsed.kind.as_str()) {
            diagnostics.push(Diagnostic {
                message_id: CompactString::from("disallow"),
                data: DiagnosticData::default(),
                loc: to_force_location(comment.loc),
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use crate::directive::CommentKind;
    use crate::loc::{Location, Position};
    use crate::{Comment, no_use};

    fn comment(kind: CommentKind, value: &str) -> Comment<'_> {
        Comment {
            kind,
            value,
            loc: Location {
                start: Position { line: 1, column: 0 },
                end: Position {
                    line: 1,
                    column: value.len() as i32 + 4,
                },
            },
        }
    }

    #[test]
    fn reports_directives_outside_the_allow_list() {
        let comments = [
            comment(CommentKind::Block, "eslint-disable eqeqeq"),
            comment(CommentKind::Block, "eslint-enable"),
            comment(CommentKind::Line, "eslint-disable-line eqeqeq"),
            comment(CommentKind::Block, "not a directive"),
        ];
        // Allow eslint-enable only; the two disables remain reported.
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_use(&comments, &["eslint-enable"]));
        }
    }

    #[test]
    fn allows_everything_when_listed() {
        let comments = [comment(CommentKind::Block, "eslint-disable eqeqeq")];
        assert!(no_use(&comments, &["eslint-disable"]).is_empty());
    }
}
