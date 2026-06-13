//! Group/alternative bookkeeping while walking a regexp pattern source.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{
    BraceQuantifierShape, class_contains_backspace_escape, class_first_collapsible_run,
    class_first_duplicate_literal, class_first_obscure_range, class_has_case_pair,
    class_has_unsorted_literal_elements, class_has_useless_range, class_has_useless_string_literal,
    class_is_digit_range, class_is_useless_single_literal, class_is_word_char_set,
    class_matches_anything, find_class_end, group_prefix, is_zero_quantifier,
    parse_brace_quantifier, skip_escape,
};

#[derive(Clone, Copy)]
pub(crate) struct GroupState {
    pub(crate) check_empty: bool,
    pub(crate) capturing: bool,
    pub(crate) is_lookaround: bool,
    pub(crate) is_non_capturing: bool,
    /// Byte offset of the body start (after the prefix). Used by
    /// `no-useless-non-capturing-group` to read the body character.
    pub(crate) body_start: usize,
    pub(crate) seen_pipe: bool,
    pub(crate) current_has_content: bool,
    /// Number of top-level atoms in the current alternative. Each literal
    /// character, escape, character class, or completed nested group counts
    /// as one atom. Reset by `|`. Used to detect "body is exactly one
    /// nested group" cases (`no-trivially-nested-assertion` and
    /// `no-extra-lookaround-assertions`).
    pub(crate) current_alt_atom_count: u32,
    /// `true` if the most recently completed atom in the current alternative
    /// was a lookaround group closing. Pairs with `current_alt_atom_count`
    /// to identify single-assertion bodies.
    pub(crate) last_atom_was_lookaround: bool,
}

impl GroupState {
    fn top_level() -> Self {
        Self {
            check_empty: false,
            capturing: false,
            is_lookaround: false,
            is_non_capturing: false,
            body_start: 0,
            seen_pipe: false,
            current_has_content: false,
            current_alt_atom_count: 0,
            last_atom_was_lookaround: false,
        }
    }

    fn group(
        check_empty: bool,
        capturing: bool,
        is_lookaround: bool,
        is_non_capturing: bool,
        body_start: usize,
    ) -> Self {
        Self {
            check_empty,
            capturing,
            is_lookaround,
            is_non_capturing,
            body_start,
            seen_pipe: false,
            current_has_content: false,
            current_alt_atom_count: 0,
            last_atom_was_lookaround: false,
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
    pub(crate) has_escape_backspace_in_class: bool,
    /// First braced quantifier rewritable as `+`, e.g. `{1,}`.
    pub(crate) first_plus_quantifier: Option<CompactString>,
    /// First braced quantifier rewritable as `*`, e.g. `{0,}`.
    pub(crate) first_star_quantifier: Option<CompactString>,
    /// First braced quantifier rewritable as `?`, e.g. `{0,1}`.
    pub(crate) first_question_quantifier: Option<CompactString>,
    /// First `{n,n}` (with `n >= 1`) and its `{n}` replacement.
    pub(crate) first_useless_two_nums_quantifier: Option<(CompactString, CompactString)>,
    /// At least one anonymous capturing group `(...)` (i.e. capturing but not
    /// named) — `prefer-named-capture-group`.
    pub(crate) has_unnamed_capturing_group: bool,
    /// At least one `[\s\S]`/`[\d\D]`/`[\w\W]`-shaped character class —
    /// `match-any`.
    pub(crate) has_match_any_class: bool,
    /// First `[0-9]`-shaped class and whether it was negated. `Some(false)` →
    /// `[0-9]` (suggest `\d`), `Some(true)` → `[^0-9]` (suggest `\D`).
    /// `prefer-d`.
    pub(crate) first_digit_class: Option<bool>,
    /// First `[a-zA-Z0-9_]`-shaped class (any order) and whether it was
    /// negated. `Some(false)` → `\w`, `Some(true)` → `\W`. `prefer-w`.
    pub(crate) first_word_class: Option<bool>,
    /// First useless single-character range like `[a-a]`. The captured `char`
    /// is the repeated endpoint. `no-useless-range`.
    pub(crate) first_useless_range: Option<char>,
    /// At least one lookaround assertion (`(?=)`, `(?!)`, `(?<=)`, `(?<!)`)
    /// with an empty body. `no-empty-lookarounds-assertion`.
    pub(crate) has_empty_lookaround: bool,
    /// First single-literal class `[X]` and the bare character it could be
    /// replaced with. `no-useless-character-class`.
    pub(crate) first_useless_single_literal_class: Option<char>,
    /// At least one lookaround assertion followed by an optional `?` quantifier
    /// (`(?=a)?`). The `?` is always meaningless because the assertion either
    /// succeeds or fails without consuming input. `no-optional-assertion`.
    pub(crate) has_optional_assertion: bool,
    /// At least one lazy quantifier whose minimum is zero (`*?`, `??`,
    /// `{0,N}?`, `{0,}?`). Such quantifiers always prefer the empty match and
    /// rarely express the author's intent. `confusing-quantifier`.
    pub(crate) has_confusing_quantifier: bool,
    /// First `X-Y` range whose endpoints cross ASCII character categories
    /// (e.g. `A-z`). The captured chars are the original endpoints.
    /// `no-obscure-range`.
    pub(crate) first_obscure_range: Option<(char, char)>,
    /// First ASCII literal that appears more than once inside the same
    /// `[...]` class. `no-dupe-characters-character-class`.
    pub(crate) first_dupe_class_literal: Option<char>,
    /// First run of three or more consecutive ASCII letters/digits inside the
    /// same `[...]` class. `prefer-range`.
    pub(crate) first_collapsible_run: Option<(char, char)>,
    /// The pattern contains at least one unescaped `.` outside a character
    /// class. Used by `no-useless-flag` to decide whether the `s` flag has
    /// any effect.
    pub(crate) has_unescaped_dot: bool,
    /// The pattern contains at least one unescaped `^` or `$` outside a
    /// character class. Used by `no-useless-flag` to decide whether the `m`
    /// flag has any effect.
    pub(crate) has_unescaped_anchor: bool,
    /// At least one character class contains both the lower- and upper-case
    /// form of an ASCII letter (e.g. `[aA]`). `use-ignore-case`.
    pub(crate) has_case_pair_class: bool,
    /// First single-character `\q{X}` string literal inside a class, holding
    /// the bare character it could be simplified to. `grapheme-string-literal`.
    pub(crate) first_useless_string_literal: Option<char>,
    /// At least one `(?:X)` non-capturing group whose body is a single
    /// regular ASCII alphanumeric character and is not followed by a
    /// quantifier — the wrapper is useless. `no-useless-non-capturing-group`.
    pub(crate) has_useless_non_capturing_group: bool,
    /// At least one `(?:X){n}` non-capturing group whose body is a single
    /// regular ASCII alphanumeric character AND is followed by a quantifier —
    /// the wrapper is unnecessary because the quantifier could apply to the
    /// bare atom. `prefer-quantifier`.
    pub(crate) has_preferable_quantifier_group: bool,
    /// At least one `[...]` class whose body is all-alphanumeric-literal but
    /// out of sorted order. `sort-character-class-elements`.
    pub(crate) has_unsorted_class_elements: bool,
    /// `(?:(?=...))` — a non-capturing group whose entire body is exactly
    /// one nested lookaround. `no-trivially-nested-assertion`.
    pub(crate) has_trivially_nested_assertion: bool,
    /// `(?=(?=...))` — a lookaround whose entire body is exactly one nested
    /// lookaround. `no-extra-lookaround-assertions`.
    pub(crate) has_extra_lookaround_assertion: bool,
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
                        if !self.has_escape_backspace_in_class
                            && class_contains_backspace_escape(bytes, index)
                        {
                            self.has_escape_backspace_in_class = true;
                        }
                        if !self.has_match_any_class && class_matches_anything(bytes, index) {
                            self.has_match_any_class = true;
                        }
                        if self.first_digit_class.is_none()
                            && let Some(shape) = class_is_digit_range(bytes, index)
                        {
                            self.first_digit_class = Some(shape.negated);
                        }
                        if self.first_word_class.is_none()
                            && let Some(shape) = class_is_word_char_set(bytes, index)
                        {
                            self.first_word_class = Some(shape.negated);
                        }
                        if self.first_useless_range.is_none()
                            && let Some(ch) = class_has_useless_range(bytes, index)
                        {
                            self.first_useless_range = Some(ch);
                        }
                        if self.first_useless_single_literal_class.is_none()
                            && let Some(ch) = class_is_useless_single_literal(bytes, index)
                        {
                            self.first_useless_single_literal_class = Some(ch);
                        }
                        if self.first_obscure_range.is_none()
                            && let Some(range) = class_first_obscure_range(bytes, index)
                        {
                            self.first_obscure_range = Some(range);
                        }
                        if !self.has_case_pair_class && class_has_case_pair(bytes, index) {
                            self.has_case_pair_class = true;
                        }
                        if !self.has_unsorted_class_elements
                            && class_has_unsorted_literal_elements(bytes, index)
                        {
                            self.has_unsorted_class_elements = true;
                        }
                        if self.first_useless_string_literal.is_none()
                            && let Some(byte) = class_has_useless_string_literal(bytes, index)
                        {
                            self.first_useless_string_literal = Some(byte as char);
                        }
                        if self.first_dupe_class_literal.is_none()
                            && let Some(byte) = class_first_duplicate_literal(bytes, index)
                        {
                            self.first_dupe_class_literal = Some(byte as char);
                        }
                        if self.first_collapsible_run.is_none()
                            && let Some(run) = class_first_collapsible_run(bytes, index)
                        {
                            self.first_collapsible_run = Some(run);
                        }
                        self.mark_content(&mut groups);
                        index = close + 1;
                    } else {
                        self.mark_content(&mut groups);
                        index += 1;
                    }
                }
                b'(' => {
                    let prefix = group_prefix(bytes, index);
                    if prefix.capturing && !prefix.named {
                        self.has_unnamed_capturing_group = true;
                    }
                    groups.push(GroupState::group(
                        prefix.check_empty,
                        prefix.capturing,
                        prefix.is_lookaround,
                        prefix.is_non_capturing,
                        prefix.next,
                    ));
                    index = prefix.next;
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
                        if group.is_lookaround && !group.seen_pipe && !group.current_has_content {
                            self.has_empty_lookaround = true;
                        }
                        // `(?=...)?` and friends: a `?` immediately after the
                        // closing paren of a lookaround makes the assertion
                        // optional, which is always meaningless because the
                        // assertion does not consume input.
                        if group.is_lookaround && bytes.get(index + 1) == Some(&b'?') {
                            self.has_optional_assertion = true;
                        }
                        // `(?:X)` with a single ASCII-alphanumeric body. The
                        // wrapper carries no meaning regardless of what follows
                        // because the quantifier (if any) could apply directly
                        // to the bare atom. Two complementary rules cover this:
                        // `no-useless-non-capturing-group` when no quantifier
                        // follows, `prefer-quantifier` when one does. We only
                        // consider non-capturing groups with no `|` and exactly
                        // one literal byte between `:` and `)`.
                        if group.is_non_capturing
                            && !group.seen_pipe
                            && index == group.body_start + 1
                        {
                            let byte = bytes[group.body_start];
                            let next = bytes.get(index + 1).copied();
                            let followed_by_quantifier =
                                matches!(next, Some(b'*' | b'+' | b'?' | b'{'));
                            if byte.is_ascii_alphanumeric() {
                                if followed_by_quantifier {
                                    self.has_preferable_quantifier_group = true;
                                } else {
                                    self.has_useless_non_capturing_group = true;
                                }
                            }
                        }
                        // Bodies that consist of exactly one nested lookaround
                        // atom (and nothing else) are reported as trivially
                        // nested. `no-trivially-nested-assertion` targets the
                        // `(?:` outer wrapper, `no-extra-lookaround-assertions`
                        // targets the `(?=` / `(?!` / `(?<=` / `(?<!` outer
                        // wrapper. Both require: no `|` seen, atom_count == 1,
                        // and the single atom was itself a lookaround.
                        if !group.seen_pipe
                            && group.current_alt_atom_count == 1
                            && group.last_atom_was_lookaround
                        {
                            if group.is_non_capturing {
                                self.has_trivially_nested_assertion = true;
                            }
                            if group.is_lookaround {
                                self.has_extra_lookaround_assertion = true;
                            }
                        }
                        if group.is_lookaround {
                            self.mark_atom_from_lookaround(&mut groups);
                        } else {
                            self.mark_content(&mut groups);
                        }
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
                        group.current_alt_atom_count = 0;
                        group.last_atom_was_lookaround = false;
                    }
                    index += 1;
                }
                b'{' if is_zero_quantifier(bytes, index) => {
                    self.has_zero_quantifier = true;
                    index += 1;
                }
                b'{' => {
                    if let Some((end, original, shape)) = parse_brace_quantifier(bytes, index) {
                        self.record_brace_quantifier(original, shape);
                        // `{0,}?` and `{0,1}?` are lazy quantifiers whose
                        // minimum is zero — `confusing-quantifier` flags them.
                        if matches!(
                            shape,
                            BraceQuantifierShape::Star | BraceQuantifierShape::Question
                        ) && bytes.get(end) == Some(&b'?')
                        {
                            self.has_confusing_quantifier = true;
                        }
                        // Advance past the `}` so we do not re-scan the digits inside.
                        // Skipping the body is safe: digits are not quantifiable on
                        // their own and contain no further regexp syntax we need to
                        // observe for the other rules tracked here.
                        index = end;
                        continue;
                    }
                    index += 1;
                }
                b'*' => {
                    if bytes.get(index + 1) == Some(&b'?') {
                        self.has_confusing_quantifier = true;
                    }
                    index += 1;
                }
                b'?' => {
                    if bytes.get(index + 1) == Some(&b'?') {
                        self.has_confusing_quantifier = true;
                    }
                    index += 1;
                }
                b'^' | b'$' => {
                    self.has_unescaped_anchor = true;
                    index += 1;
                }
                b'+' | b'}' => {
                    index += 1;
                }
                b'.' => {
                    self.has_unescaped_dot = true;
                    self.mark_content(&mut groups);
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

    fn record_brace_quantifier(&mut self, original: &str, shape: BraceQuantifierShape) {
        match shape {
            BraceQuantifierShape::Plus => {
                if self.first_plus_quantifier.is_none() {
                    self.first_plus_quantifier = Some(CompactString::from(original));
                }
            }
            BraceQuantifierShape::Star => {
                if self.first_star_quantifier.is_none() {
                    self.first_star_quantifier = Some(CompactString::from(original));
                }
            }
            BraceQuantifierShape::Question => {
                if self.first_question_quantifier.is_none() {
                    self.first_question_quantifier = Some(CompactString::from(original));
                }
            }
            BraceQuantifierShape::EqualTwoNums(value) => {
                if self.first_useless_two_nums_quantifier.is_none() {
                    let mut replacement = CompactString::new("{");
                    push_u64_decimal(&mut replacement, value);
                    replacement.push('}');
                    self.first_useless_two_nums_quantifier =
                        Some((CompactString::from(original), replacement));
                }
            }
        }
    }

    fn mark_content(&self, groups: &mut SmallVec<[GroupState; 8]>) {
        if let Some(group) = groups.last_mut() {
            group.current_has_content = true;
            group.current_alt_atom_count = group.current_alt_atom_count.saturating_add(1);
            group.last_atom_was_lookaround = false;
        }
    }

    fn mark_atom_from_lookaround(&self, groups: &mut SmallVec<[GroupState; 8]>) {
        if let Some(group) = groups.last_mut() {
            group.current_has_content = true;
            group.current_alt_atom_count = group.current_alt_atom_count.saturating_add(1);
            group.last_atom_was_lookaround = true;
        }
    }
}

/// Append the decimal representation of `value` to `target` without allocating
/// an intermediate `String`. Stays on the stack via a small fixed buffer; a
/// `u64` has at most 20 decimal digits.
fn push_u64_decimal(target: &mut CompactString, value: u64) {
    let mut buf = [0u8; 20];
    let mut cursor = buf.len();
    let mut remaining = value;
    if remaining == 0 {
        cursor -= 1;
        buf[cursor] = b'0';
    } else {
        while remaining > 0 {
            cursor -= 1;
            buf[cursor] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
        }
    }
    if let Ok(text) = std::str::from_utf8(&buf[cursor..]) {
        target.push_str(text);
    }
}
