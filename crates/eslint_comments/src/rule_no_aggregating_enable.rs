//! `no-aggregating-enable`: disallow an `eslint-enable` that closes many disables.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::disabled_area::build_disabled_area;
use crate::loc::to_force_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Report `eslint-enable` comments that re-enable two or more `eslint-disable`
/// comments at once.
pub fn no_aggregating_enable(comments: &[Comment]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();
    let state = build_disabled_area(comments);

    for &(comment_index, count) in &state.related_counts {
        if count >= 2 {
            let comment = &comments[comment_index];
            diagnostics.push(Diagnostic {
                message_id: CompactString::from("aggregatingEnable"),
                data: DiagnosticData {
                    count: Some(count),
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
    use crate::{Comment, no_aggregating_enable};

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
    fn reports_aggregating_enable() {
        // One bare enable closes two separate disables.
        let comments = [
            block("eslint-disable no-undef", 1),
            block("eslint-disable no-unused-vars", 2),
            block("eslint-enable", 3),
        ];
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_aggregating_enable(&comments));
        }
    }

    #[test]
    fn single_pair_is_ok() {
        let comments = [
            block("eslint-disable no-undef", 1),
            block("eslint-enable no-undef", 2),
        ];
        assert!(no_aggregating_enable(&comments).is_empty());
    }
}
