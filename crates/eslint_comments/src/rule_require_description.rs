//! `require-description`: require descriptions in ESLint directive comments.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::directive::parse_directive_comment;
use crate::loc::to_force_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Report directive comments without a description, unless their kind is ignored.
pub fn require_description(comments: &[Comment], ignored: &[&str]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();

    for comment in comments {
        let same_line = comment.loc.start.line == comment.loc.end.line;
        let Some(parsed) = parse_directive_comment(comment.kind, comment.value, same_line) else {
            continue;
        };

        if ignored.contains(&parsed.kind.as_str()) {
            continue;
        }

        let has_description = parsed.description.as_deref().is_some_and(|d| !d.is_empty());
        if !has_description {
            diagnostics.push(Diagnostic {
                message_id: CompactString::from("missingDescription"),
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
    use crate::{Comment, require_description};

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
    fn reports_directives_without_descriptions() {
        let comments = [
            comment(CommentKind::Block, "eslint-disable eqeqeq"),
            comment(CommentKind::Block, "eslint-disable eqeqeq -- because"),
            comment(CommentKind::Line, "eslint-disable-line eqeqeq"),
        ];
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(require_description(&comments, &[]));
        }
    }

    #[test]
    fn skips_ignored_kinds() {
        let comments = [comment(CommentKind::Block, "eslint-disable eqeqeq")];
        assert!(require_description(&comments, &["eslint-disable"]).is_empty());
    }
}
