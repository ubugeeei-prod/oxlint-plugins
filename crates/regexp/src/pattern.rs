//! Group/alternative bookkeeping while walking a regexp pattern source.

use oxlint_plugins_carton::SmallVec;

use crate::helpers::{find_class_end, group_prefix, is_zero_quantifier, skip_escape};

#[derive(Clone, Copy)]
pub(crate) struct GroupState {
    pub(crate) check_empty: bool,
    pub(crate) capturing: bool,
    pub(crate) seen_pipe: bool,
    pub(crate) current_has_content: bool,
}

impl GroupState {
    fn top_level() -> Self {
        Self {
            check_empty: false,
            capturing: false,
            seen_pipe: false,
            current_has_content: false,
        }
    }

    fn group(check_empty: bool, capturing: bool) -> Self {
        Self {
            check_empty,
            capturing,
            seen_pipe: false,
            current_has_content: false,
        }
    }
}

#[derive(Default)]
pub(crate) struct PatternAnalysis {
    pub(crate) has_empty_character_class: bool,
    pub(crate) has_empty_group: bool,
    pub(crate) has_empty_capturing_group: bool,
    pub(crate) has_empty_alternative: bool,
    pub(crate) has_zero_quantifier: bool,
}

impl PatternAnalysis {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn scan(&mut self, pattern: &str) {
        let bytes = pattern.as_bytes();
        let mut groups = SmallVec::<[GroupState; 8]>::new();
        groups.push(GroupState::top_level());
        let mut index = 0;

        while index < bytes.len() {
            match bytes[index] {
                b'\\' => {
                    self.mark_content(&mut groups);
                    index = skip_escape(bytes, index);
                }
                b'[' => {
                    let close = find_class_end(bytes, index);
                    if let Some(close) = close {
                        if close == index + 1 {
                            self.has_empty_character_class = true;
                        }
                        self.mark_content(&mut groups);
                        index = close + 1;
                    } else {
                        self.mark_content(&mut groups);
                        index += 1;
                    }
                }
                b'(' => {
                    let (check_empty, capturing, next) = group_prefix(bytes, index);
                    groups.push(GroupState::group(check_empty, capturing));
                    index = next;
                }
                b')' => {
                    if groups.len() > 1
                        && let Some(group) = groups.pop()
                    {
                        if group.seen_pipe && !group.current_has_content {
                            self.has_empty_alternative = true;
                        }
                        if group.check_empty && !group.seen_pipe && !group.current_has_content {
                            self.has_empty_group = true;
                            if group.capturing {
                                self.has_empty_capturing_group = true;
                            }
                        }
                        self.mark_content(&mut groups);
                    }
                    index += 1;
                }
                b'|' => {
                    if let Some(group) = groups.last_mut() {
                        if !group.current_has_content {
                            self.has_empty_alternative = true;
                        }
                        group.seen_pipe = true;
                        group.current_has_content = false;
                    }
                    index += 1;
                }
                b'{' if is_zero_quantifier(bytes, index) => {
                    self.has_zero_quantifier = true;
                    index += 1;
                }
                b'*' | b'+' | b'?' | b'{' | b'}' | b'^' | b'$' => {
                    index += 1;
                }
                _ => {
                    self.mark_content(&mut groups);
                    index += 1;
                }
            }
        }

        if let Some(group) = groups.last()
            && group.seen_pipe
            && !group.current_has_content
        {
            self.has_empty_alternative = true;
        }
    }

    fn mark_content(&self, groups: &mut SmallVec<[GroupState; 8]>) {
        if let Some(group) = groups.last_mut() {
            group.current_has_content = true;
        }
    }
}
