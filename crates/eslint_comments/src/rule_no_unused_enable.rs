//! `no-unused-enable`: disallow `eslint-enable` comments that re-enable rules
//! which were never disabled.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::disabled_area::build_disabled_area;
use crate::loc::to_rule_id_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Report each `eslint-enable` directive that does not close any open disable.
pub fn no_unused_enable(comments: &[Comment]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();
    let state = build_disabled_area(comments);

    for item in &state.unused_enable_directives {
        let comment = &comments[item.comment];
        let loc = to_rule_id_location(comment.value, comment.loc, item.rule_id.as_deref());
        let message_id = if item.rule_id.is_some() {
            "unusedRule"
        } else {
            "unused"
        };

        diagnostics.push(Diagnostic {
            message_id: CompactString::from(message_id),
            data: DiagnosticData {
                rule_id: item.rule_id.clone(),
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
    use crate::{Comment, no_unused_enable};

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
    fn reports_unused_enable() {
        // Enable a rule that was never disabled.
        let comments = [block("eslint-enable no-undef", 1)];
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_unused_enable(&comments));
        }
    }

    #[test]
    fn matched_enable_is_ok() {
        let comments = [
            block("eslint-disable no-undef", 1),
            block("eslint-enable no-undef", 2),
        ];
        assert!(no_unused_enable(&comments).is_empty());
    }
}
