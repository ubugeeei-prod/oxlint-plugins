//! `no-duplicate-disable`: disallow `eslint-disable` comments that re-disable
//! a rule already disabled by an earlier, still-open directive.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::disabled_area::build_disabled_area;
use crate::loc::to_rule_id_location;
use crate::{Comment, Diagnostic, DiagnosticData};

/// Report each disable directive that duplicates an already-active disable.
pub fn no_duplicate_disable(comments: &[Comment]) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();
    let state = build_disabled_area(comments);

    for item in &state.duplicate_disable_directives {
        let comment = &comments[item.comment];
        let loc = to_rule_id_location(comment.value, comment.loc, item.rule_id.as_deref());
        let message_id = if item.rule_id.is_some() {
            "duplicateRule"
        } else {
            "duplicate"
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
    use crate::{Comment, no_duplicate_disable};

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
    fn reports_duplicate_disable() {
        let comments = [
            block("eslint-disable no-undef", 1),
            block("eslint-disable no-undef", 2),
        ];
        #[allow(clippy::disallowed_macros)]
        {
            insta::assert_debug_snapshot!(no_duplicate_disable(&comments));
        }
    }

    #[test]
    fn distinct_rules_are_ok() {
        let comments = [
            block("eslint-disable no-undef", 1),
            block("eslint-disable no-unused-vars", 2),
        ];
        assert!(no_duplicate_disable(&comments).is_empty());
    }
}
