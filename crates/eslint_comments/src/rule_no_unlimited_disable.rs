//! `no-unlimited-disable`: disallow `eslint-disable` comments without rule names.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::directive::parse_directive_comment;
use crate::loc::to_force_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Disable kinds that must name at least one rule.
const DISABLE_KINDS: [&str; 3] = [
    "eslint-disable",
    "eslint-disable-line",
    "eslint-disable-next-line",
];

/// Report every `eslint-disable*` directive that disables all rules.
pub fn no_unlimited_disable(comments: &[Comment]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();

    for comment in comments {
        let same_line = comment.loc.start.line == comment.loc.end.line;
        let Some(parsed) = parse_directive_comment(comment.kind, comment.value, same_line) else {
            continue;
        };

        if !DISABLE_KINDS.contains(&parsed.kind.as_str()) {
            continue;
        }

        if parsed.has_empty_value() {
            diagnostics.push(Diagnostic {
                message_id: CompactString::from("unexpected"),
                data: DiagnosticData {
                    kind: Some(parsed.kind),
                    ..DiagnosticData::default()
                },
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
    use crate::{Comment, no_unlimited_disable};

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
    fn reports_unlimited_disables() {
        let comments = [
            comment(CommentKind::Block, "eslint-disable "),
            comment(CommentKind::Line, "eslint-disable-line"),
            comment(CommentKind::Line, "eslint-disable-next-line"),
        ];
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_unlimited_disable(&comments));
        }
    }

    #[test]
    fn ignores_scoped_disables_and_enables() {
        let comments = [
            comment(CommentKind::Block, "eslint-disable eqeqeq"),
            comment(CommentKind::Block, "eslint-enable"),
            comment(CommentKind::Line, "eslint-disable-line eqeqeq"),
            comment(CommentKind::Block, "not a directive"),
        ];
        assert!(no_unlimited_disable(&comments).is_empty());
    }
}
