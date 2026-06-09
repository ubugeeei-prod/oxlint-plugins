//! `disable-enable-pair`: require an `eslint-enable` for every `eslint-disable`.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::disabled_area::build_disabled_area;
use crate::loc::{Position, lte, to_rule_id_location};
use crate::{Comment, Diagnostic, DiagnosticData};

/// Report every disabled area that is never closed by an `eslint-enable`.
///
/// With `allow_whole_file`, areas that start at or before the first token are
/// treated as intentional whole-file disables; `first_token_start` is the first
/// non-comment token's position (or `None` when the file has no tokens).
pub fn disable_enable_pair(
    comments: &[Comment],
    allow_whole_file: bool,
    first_token_start: Option<Position>,
) -> SmallVec<[Diagnostic; 8]> {
    let mut diagnostics = SmallVec::new();

    if allow_whole_file && first_token_start.is_none() {
        return diagnostics;
    }

    let state = build_disabled_area(comments);

    for area in &state.areas {
        if area.end.is_some() {
            continue;
        }
        if allow_whole_file && first_token_start.is_some_and(|first| lte(area.start, first)) {
            continue;
        }

        let comment = &comments[area.comment];
        let loc = to_rule_id_location(comment.value, comment.loc, area.rule_id.as_deref());
        let message_id = if area.rule_id.is_some() {
            "missingRulePair"
        } else {
            "missingPair"
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
    use crate::{Comment, disable_enable_pair};

    fn block<'a>(value: &'a str, line: u32, end_column: i32) -> Comment<'a> {
        Comment {
            kind: CommentKind::Block,
            value,
            loc: Location {
                start: Position { line, column: 0 },
                end: Position {
                    line,
                    column: end_column,
                },
            },
        }
    }

    #[test]
    fn reports_unpaired_disables() {
        let comments = [
            block("eslint-disable", 2, 19),
            block("eslint-disable no-undef", 4, 27),
        ];
        insta::assert_debug_snapshot!(disable_enable_pair(&comments, false, None));
    }

    #[test]
    fn paired_disable_is_ok() {
        let comments = [
            block("eslint-disable no-undef", 1, 27),
            block("eslint-enable no-undef", 3, 26),
        ];
        assert!(disable_enable_pair(&comments, false, None).is_empty());
    }

    #[test]
    fn allow_whole_file_skips_leading_disable() {
        let comments = [block("eslint-disable", 1, 19)];
        let first_token = Position { line: 2, column: 0 };
        assert!(disable_enable_pair(&comments, true, Some(first_token)).is_empty());
    }
}
