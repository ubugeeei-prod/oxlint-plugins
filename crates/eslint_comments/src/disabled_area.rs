//! Clean-room port of upstream's `DisabledArea` bookkeeping.
//!
//! Walks the directive comments of a file in source order and records the
//! disabled regions, duplicate `eslint-disable` directives, unused
//! `eslint-enable` directives, and how many disables each enable closes. The
//! disabled-area rules (`disable-enable-pair`, `no-aggregating-enable`,
//! `no-duplicate-disable`, `no-unused-enable`) read this shared result.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::Comment;
use crate::directive::parse_directive_comment;
use crate::loc::{Position, lte};

/// Whether a disabled area came from a block (`eslint-disable`) or line
/// (`eslint-disable-line` / `-next-line`) directive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaKind {
    Block,
    Line,
}

/// A disabled region opened by a disable directive and (maybe) closed by an enable.
#[derive(Clone, Debug)]
pub struct Area {
    /// Index into the input comments of the directive that opened the area.
    pub comment: usize,
    /// The rule the area disables, or `None` for "all rules".
    pub rule_id: Option<CompactString>,
    pub kind: AreaKind,
    pub start: Position,
    pub end: Option<Position>,
}

/// A reference to a directive comment plus the rule it concerns.
#[derive(Clone, Debug)]
pub struct DirectiveRef {
    pub comment: usize,
    pub rule_id: Option<CompactString>,
}

/// The bookkeeping result for a file.
#[derive(Debug, Default)]
pub struct DisabledArea {
    pub areas: SmallVec<[Area; 8]>,
    pub duplicate_disable_directives: SmallVec<[DirectiveRef; 4]>,
    pub unused_enable_directives: SmallVec<[DirectiveRef; 4]>,
    /// `(enable comment index, number of disables it closed)`, in source order.
    pub related_counts: SmallVec<[(usize, u32); 4]>,
}

impl DisabledArea {
    /// Index of the most recent open area covering `location` for `rule_id`
    /// (matching a global `None` area or one with the same rule).
    fn open_area_at(&self, rule_id: Option<&str>, location: Position) -> Option<usize> {
        for (index, area) in self.areas.iter().enumerate().rev() {
            let rule_matches = area.rule_id.is_none() || area.rule_id.as_deref() == rule_id;
            let within = lte(area.start, location) && area.end.is_none_or(|end| lte(location, end));
            if rule_matches && within {
                return Some(index);
            }
        }
        None
    }

    fn disable(
        &mut self,
        comment: usize,
        location: Position,
        rule_ids: Option<&[CompactString]>,
        kind: AreaKind,
    ) {
        match rule_ids {
            Some(rule_ids) => {
                for rule_id in rule_ids {
                    if self.open_area_at(Some(rule_id), location).is_some() {
                        self.duplicate_disable_directives.push(DirectiveRef {
                            comment,
                            rule_id: Some(rule_id.clone()),
                        });
                    }
                    self.areas.push(Area {
                        comment,
                        rule_id: Some(rule_id.clone()),
                        kind,
                        start: location,
                        end: None,
                    });
                }
            }
            None => {
                if self.open_area_at(None, location).is_some() {
                    self.duplicate_disable_directives.push(DirectiveRef {
                        comment,
                        rule_id: None,
                    });
                }
                self.areas.push(Area {
                    comment,
                    rule_id: None,
                    kind,
                    start: location,
                    end: None,
                });
            }
        }
    }

    fn enable(
        &mut self,
        comment: usize,
        location: Position,
        rule_ids: Option<&[CompactString]>,
        kind: AreaKind,
    ) {
        let mut related: SmallVec<[usize; 8]> = SmallVec::new();
        let mut note_related = |area_comment: usize| {
            if !related.contains(&area_comment) {
                related.push(area_comment);
            }
        };

        match rule_ids {
            Some(rule_ids) => {
                for rule_id in rule_ids {
                    let mut used = false;
                    for area in self.areas.iter_mut().rev() {
                        if area.end.is_none()
                            && area.kind == kind
                            && area.rule_id.as_deref() == Some(rule_id.as_str())
                        {
                            note_related(area.comment);
                            area.end = Some(location);
                            used = true;
                        }
                    }
                    if !used {
                        self.unused_enable_directives.push(DirectiveRef {
                            comment,
                            rule_id: Some(rule_id.clone()),
                        });
                    }
                }
            }
            None => {
                let mut used = false;
                for area in self.areas.iter_mut().rev() {
                    if area.end.is_none() && area.kind == kind {
                        note_related(area.comment);
                        area.end = Some(location);
                        used = true;
                    }
                }
                if !used {
                    self.unused_enable_directives.push(DirectiveRef {
                        comment,
                        rule_id: None,
                    });
                }
            }
        }

        self.related_counts.push((comment, related.len() as u32));
    }
}

/// Split a directive value into rule ids, or `None` when it disables all rules.
fn split_rule_ids(value: &str) -> Option<SmallVec<[CompactString; 4]>> {
    if value.is_empty() {
        return None;
    }
    let ids: SmallVec<[CompactString; 4]> = value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(CompactString::from)
        .collect();
    if ids.is_empty() { None } else { Some(ids) }
}

/// Build the disabled-area bookkeeping for a file's comments.
pub fn build_disabled_area(comments: &[Comment]) -> DisabledArea {
    let mut state = DisabledArea::default();

    for (index, comment) in comments.iter().enumerate() {
        let same_line = comment.loc.start.line == comment.loc.end.line;
        let Some(parsed) = parse_directive_comment(comment.kind, comment.value, same_line) else {
            continue;
        };

        let rule_ids = split_rule_ids(&parsed.value);
        let rule_ids = rule_ids.as_deref();
        let line = comment.loc.start.line;

        match parsed.kind.as_str() {
            "eslint-disable" => {
                state.disable(index, comment.loc.start, rule_ids, AreaKind::Block);
            }
            "eslint-enable" => {
                state.enable(index, comment.loc.start, rule_ids, AreaKind::Block);
            }
            "eslint-disable-line" => {
                let start = Position { line, column: 0 };
                let end = Position {
                    line: line + 1,
                    column: -1,
                };
                state.disable(index, start, rule_ids, AreaKind::Line);
                state.enable(index, end, rule_ids, AreaKind::Line);
            }
            "eslint-disable-next-line" => {
                let start = Position {
                    line: line + 1,
                    column: 0,
                };
                let end = Position {
                    line: line + 2,
                    column: -1,
                };
                state.disable(index, start, rule_ids, AreaKind::Line);
                state.enable(index, end, rule_ids, AreaKind::Line);
            }
            _ => {}
        }
    }

    state
}
