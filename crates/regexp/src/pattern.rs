//! Group/alternative bookkeeping while walking a regexp pattern source.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{
    BraceQuantifierShape, class_bracket_changes_meaning, class_contains_backspace_escape,
    class_first_collapsible_run, class_first_duplicate_literal, class_first_obscure_range,
    class_has_case_pair, class_has_misleading_unicode, class_has_unsorted_literal_elements,
    class_has_useless_range, class_has_useless_string_literal, class_is_digit_range,
    class_is_useless_single_literal, class_is_word_char_set, class_matches_anything,
    class_negated_shorthand_letter, find_class_end, find_class_end_nested,
    fixed_count_lazy_brace_end, group_prefix, is_zero_quantifier, parse_brace_quantifier,
    skip_escape,
};

#[derive(Clone, Copy)]
pub(crate) struct GroupState {
    pub(crate) check_empty: bool,
    pub(crate) capturing: bool,
    /// 1-based capture group number (0 for non-capturing groups).
    pub(crate) capture_number: u32,
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
    /// Byte offset where the current alternative began. For the first
    /// alternative this equals `body_start`; for subsequent alts it is the
    /// position immediately after the previous `|`. Used by
    /// `prefer-character-class` to inspect each alternative's literal shape.
    pub(crate) current_alt_start: usize,
    /// `true` while every alternative observed so far in this group has
    /// consisted of exactly one ASCII alphanumeric byte. Used by
    /// `prefer-character-class`.
    pub(crate) all_alts_single_literal: bool,
    /// First byte of the most recently completed alternative whose body was
    /// a single ASCII alphanumeric. `0` means no such alt has been recorded
    /// yet. Used by `sort-alternatives` to detect out-of-order alternations.
    pub(crate) prev_alt_first_byte: u8,
    /// `true` while every completed single-literal alternative observed so
    /// far has appeared in ascending byte order. `sort-alternatives`.
    pub(crate) alts_in_order: bool,
    /// Number of `|` separators seen so far in this group. Total number of
    /// alternatives = `pipe_count + 1`. Used by `prefer-character-class` to
    /// enforce the upstream default of `minAlternatives: 3` (i.e. at least
    /// two `|` separators).
    pub(crate) pipe_count: u32,
}

impl GroupState {
    fn top_level() -> Self {
        Self {
            check_empty: false,
            capturing: false,
            capture_number: 0,
            is_lookaround: false,
            is_non_capturing: false,
            body_start: 0,
            seen_pipe: false,
            current_has_content: false,
            current_alt_atom_count: 0,
            last_atom_was_lookaround: false,
            current_alt_start: 0,
            all_alts_single_literal: true,
            prev_alt_first_byte: 0,
            alts_in_order: true,
            pipe_count: 0,
        }
    }

    fn group(
        check_empty: bool,
        capturing: bool,
        capture_number: u32,
        is_lookaround: bool,
        is_non_capturing: bool,
        body_start: usize,
    ) -> Self {
        Self {
            check_empty,
            capturing,
            capture_number,
            is_lookaround,
            is_non_capturing,
            body_start,
            seen_pipe: false,
            current_has_content: false,
            current_alt_atom_count: 0,
            last_atom_was_lookaround: false,
            current_alt_start: body_start,
            all_alts_single_literal: true,
            prev_alt_first_byte: 0,
            alts_in_order: true,
            pipe_count: 0,
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
    /// `(?:X+)+` etc. — a non-capturing group whose body is a single
    /// quantified ASCII alphanumeric atom AND is itself followed by a
    /// quantifier. `no-trivially-nested-quantifier`.
    pub(crate) has_trivially_nested_quantifier: bool,
    /// `(?:a|b|c)` — a non-capturing group whose alternation has at least
    /// two alternatives and every alternative is exactly one ASCII
    /// alphanumeric literal byte. `prefer-character-class`.
    pub(crate) has_preferable_character_class: bool,
    /// `(?:b|a)` — a non-capturing alternation whose single-literal alts are
    /// not in ascending byte order. `sort-alternatives`.
    pub(crate) has_unsorted_alternatives: bool,
    /// `(?=$)` / `(?<=^)` etc. — a lookaround whose body is exactly one
    /// `^` or `$` anchor. Such lookarounds are equivalent to the anchor
    /// itself. `prefer-predefined-assertion`.
    pub(crate) has_preferable_predefined_assertion: bool,
    /// `(?=X*)` / `(?=X?)` — a lookaround whose body always succeeds
    /// because the inner quantifier accepts the empty match.
    /// `optimal-lookaround-quantifier`.
    pub(crate) has_suboptimal_lookaround_quantifier: bool,
    /// `(?:a|a)` — alternation that contains the same single-literal
    /// alternative twice in a row. `no-dupe-disjunctions`.
    pub(crate) has_dupe_disjunctions: bool,
    /// Number of capturing groups opened so far during the scan. Used by
    /// `no-useless-backreference` to detect forward references.
    pub(crate) capture_count: u32,
    /// `\N` was encountered where N > capture_count at the point of
    /// observation, so the back-reference can never match a real capture.
    /// `no-useless-backreference`.
    pub(crate) has_useless_backreference: bool,
    /// `[^\\d]` (or `\\D`/`\\s`/`\\S`/`\\w`/`\\W`) — a negated character
    /// class whose body is exactly one predefined shorthand and can be
    /// replaced by the negated shorthand. `negation`.
    pub(crate) has_negation_shorthand: bool,
    /// `{n}?` or `{n,n}?` — fixed-count brace quantifier with a lazy
    /// modifier. The lazy modifier is a no-op because the engine always
    /// matches exactly `n` repetitions. `no-useless-lazy`.
    pub(crate) has_useless_lazy: bool,
    /// A character class or quantifier element is a single *unit* that the
    /// regex engine actually matches as MULTIPLE Unicode code points — either
    /// a multi-code-point grapheme (combining / ZWJ / regional-indicator
    /// sequence) placed literally in a class, or an astral character that is
    /// seen as a surrogate pair (two UTF-16 code units) in non-`u`/`v` mode.
    /// A class/quantifier element consisting of a *single* code point (a lone
    /// ZWJ, combining mark, variation selector, or an astral char under the
    /// `u`/`v` flag) is NOT flagged. `no-misleading-unicode-character`.
    pub(crate) has_misleading_unicode_character: bool,
    /// Bitmask of capturing-group indices (1-based, bits 1..=31) whose closing
    /// `)` is immediately followed by `?` or `*` — meaning the group's capture
    /// may be absent at match time. Used by `no-potentially-useless-backreference`
    /// to detect `\N` that references such an optionally-quantified group.
    pub(crate) optionally_quantified_groups: u32,
    /// `true` when a backreference `\N` targets a capturing group that was
    /// preceded by `?` or `*` (so the group may not have matched). Only the
    /// clear syntactic case is flagged; alternative branches are deferred.
    /// `no-potentially-useless-backreference`.
    pub(crate) has_potentially_useless_backreference: bool,
}

impl PatternAnalysis {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Scan `pattern` and populate the analysis fields.
    ///
    /// `v_mode` must be `true` when the regex carries the `v` flag. Only
    /// v-mode allows nested `[...]` classes (set-operation operands); in
    /// non-v mode every unescaped `[` inside a class is a literal character
    /// and must not be treated as opening a nested class.
    ///
    /// `unicode_mode` must be `true` when the regex carries either the `u` or
    /// `v` flag. Under those flags an astral character is a single code point,
    /// so it is no longer "misleading"; without them the same astral character
    /// is matched as a surrogate pair (two UTF-16 code units).
    pub(crate) fn scan(&mut self, pattern: &str, v_mode: bool, unicode_mode: bool) {
        let bytes = pattern.as_bytes();
        let mut groups = SmallVec::<[GroupState; 8]>::new();
        groups.push(GroupState::top_level());
        let mut index = 0;

        while index < bytes.len() {
            match bytes[index] {
                b'\\' => {
                    if let Some(&next) = bytes.get(index + 1)
                        && matches!(next, b'1'..=b'9')
                    {
                        let n = u32::from(next - b'0');
                        if n > self.capture_count {
                            self.has_useless_backreference = true;
                        }
                        // `no-potentially-useless-backreference`: flag \N when
                        // group N (already opened, so n <= capture_count) is
                        // directly followed by `?` or `*` in the pattern. Only
                        // the first 31 groups are tracked (bitmask width).
                        if n >= 1
                            && n <= self.capture_count
                            && n <= 31
                            && self.optionally_quantified_groups & (1 << n) != 0
                        {
                            self.has_potentially_useless_backreference = true;
                        }
                    }
                    self.mark_content(&mut groups);
                    index = skip_escape(bytes, index);
                }
                b'[' => {
                    // In v-mode, use the depth-aware variant so that nested
                    // classes (set-operation operands such as `[\w--[ab]]`)
                    // are correctly bounded and their inner `]` is not
                    // mistaken for the closing `]` of the outer class.
                    // In non-v mode, `[` inside a class is a literal
                    // character; use the flat variant so that patterns like
                    // `[[]` (matching a literal `[`) are not mis-parsed as
                    // an unterminated class.
                    let close = if v_mode {
                        find_class_end_nested(bytes, index)
                    } else {
                        find_class_end(bytes, index)
                    };
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
                            && !class_bracket_changes_meaning(bytes, index, ch as u8)
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
                            && let Some(byte) = class_first_duplicate_literal(bytes, index, v_mode)
                        {
                            self.first_dupe_class_literal = Some(byte as char);
                        }
                        if self.first_collapsible_run.is_none()
                            && let Some(run) = class_first_collapsible_run(bytes, index)
                        {
                            self.first_collapsible_run = Some(run);
                        }
                        if !self.has_negation_shorthand
                            && class_negated_shorthand_letter(bytes, index).is_some()
                        {
                            self.has_negation_shorthand = true;
                        }
                        if !self.has_misleading_unicode_character
                            && class_has_misleading_unicode(bytes, index, v_mode, unicode_mode)
                        {
                            self.has_misleading_unicode_character = true;
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
                    if prefix.capturing {
                        self.capture_count = self.capture_count.saturating_add(1);
                    }
                    let capture_number = if prefix.capturing {
                        self.capture_count
                    } else {
                        0
                    };
                    groups.push(GroupState::group(
                        prefix.check_empty,
                        prefix.capturing,
                        capture_number,
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
                        // `no-potentially-useless-backreference`: record which
                        // capturing groups are directly followed by `?` or `*`.
                        // Only track groups 1..=31 (bitmask width). The check
                        // uses index+1 because `index` currently points at `)`.
                        if group.capturing
                            && group.capture_number >= 1
                            && group.capture_number <= 31
                            && matches!(bytes.get(index + 1).copied(), Some(b'?') | Some(b'*'))
                        {
                            self.optionally_quantified_groups |= 1 << group.capture_number;
                        }
                        // `(?=...)?` and friends: a `?` immediately after the
                        // closing paren of a lookaround makes the assertion
                        // optional, which is always meaningless because the
                        // assertion does not consume input.
                        if group.is_lookaround && bytes.get(index + 1) == Some(&b'?') {
                            self.has_optional_assertion = true;
                        }
                        // `(?=$)` / `(?<=^)` etc.: a lookaround whose body is
                        // exactly the `$` or `^` anchor is equivalent to the
                        // bare anchor. Only fires for non-empty single-byte
                        // bodies so empty lookarounds stay with
                        // `no-empty-lookarounds-assertion`.
                        if group.is_lookaround && !group.seen_pipe && index == group.body_start + 1
                        {
                            let byte = bytes[group.body_start];
                            if matches!(byte, b'^' | b'$') {
                                self.has_preferable_predefined_assertion = true;
                            }
                        }
                        // `(?=X*)` / `(?=X?)`: lookaround body is exactly one
                        // quantified atom whose quantifier accepts the empty
                        // match, so the assertion always succeeds. Narrow:
                        // body is 2 bytes — ASCII alphanumeric followed by
                        // `*` or `?`. `(?=X+)` is excluded because it
                        // requires a non-empty match.
                        if group.is_lookaround && !group.seen_pipe && index == group.body_start + 2
                        {
                            let body0 = bytes[group.body_start];
                            let body1 = bytes[group.body_start + 1];
                            if body0.is_ascii_alphanumeric() && matches!(body1, b'*' | b'?') {
                                self.has_suboptimal_lookaround_quantifier = true;
                            }
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
                            // `body_start` is three bytes past the opening `(`,
                            // so the `(` of `(?:` sits at `body_start - 3`.
                            let open_pos = group.body_start.saturating_sub(3);
                            if byte.is_ascii_alphanumeric()
                                && !non_capturing_group_removal_would_change_meaning(
                                    bytes, open_pos, byte,
                                )
                            {
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
                        // `(?:X+)+` and similar: non-capturing wrapper whose
                        // body is a single ASCII alphanumeric atom followed
                        // by `*`/`+`/`?`, and is itself followed by a
                        // quantifier. Both quantifiers apply to the same bare
                        // atom so the outer wrapper carries no meaning.
                        // `(?:a|b|c)` with all-single-literal alternatives ->
                        // `prefer-character-class`. We tracked per-alt
                        // single-literal shape during scanning; combined with
                        // the final-alt check here, an alternation with at
                        // least two `|` separators (i.e. ≥ 3 total alts) and
                        // every alt being a bare alphanumeric letter/digit is
                        // reported. The threshold of 3 matches upstream's
                        // default `minAlternatives` option.
                        //
                        // `sort-alternatives` shares the same single-literal
                        // tracking but fires for ≥ 2 alts (pipe_count ≥ 1).
                        if group.is_non_capturing && group.seen_pipe {
                            let alt_len = index - group.current_alt_start;
                            let final_alt_simple = alt_len == 1
                                && bytes[group.current_alt_start].is_ascii_alphanumeric();
                            if group.all_alts_single_literal && final_alt_simple {
                                // `prefer-character-class` requires ≥ 3 alternatives.
                                if group.pipe_count >= 2 {
                                    self.has_preferable_character_class = true;
                                }
                                // `sort-alternatives` fires for ≥ 2 alternatives
                                // when at least one transition violated ascending order.
                                let final_byte = bytes[group.current_alt_start];
                                let mut alts_in_order = group.alts_in_order;
                                if group.prev_alt_first_byte != 0 {
                                    if final_byte < group.prev_alt_first_byte {
                                        alts_in_order = false;
                                    }
                                    if final_byte == group.prev_alt_first_byte {
                                        self.has_dupe_disjunctions = true;
                                    }
                                }
                                if !alts_in_order {
                                    self.has_unsorted_alternatives = true;
                                }
                            }
                        }
                        // Multi-byte bodies, escapes, classes, and braced
                        // quantifiers are deferred to keep the check sound.
                        if group.is_non_capturing
                            && !group.seen_pipe
                            && index == group.body_start + 2
                        {
                            let body0 = bytes[group.body_start];
                            let body1 = bytes[group.body_start + 1];
                            let outer_q = matches!(
                                bytes.get(index + 1).copied(),
                                Some(b'*' | b'+' | b'?' | b'{')
                            );
                            if body0.is_ascii_alphanumeric()
                                && matches!(body1, b'*' | b'+' | b'?')
                                && outer_q
                            {
                                self.has_trivially_nested_quantifier = true;
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
                        // Check the closing alternative for `prefer-character-class`
                        // and `sort-alternatives`: each alt must be exactly one
                        // ASCII alphanumeric byte. Both rules also need
                        // ordering checks for single-literal alts.
                        let mut alt_first_byte: u8 = 0;
                        if group.all_alts_single_literal {
                            let alt_len = index - group.current_alt_start;
                            if alt_len != 1
                                || !bytes[group.current_alt_start].is_ascii_alphanumeric()
                            {
                                group.all_alts_single_literal = false;
                            } else {
                                alt_first_byte = bytes[group.current_alt_start];
                                if group.prev_alt_first_byte != 0 {
                                    if alt_first_byte < group.prev_alt_first_byte {
                                        group.alts_in_order = false;
                                    }
                                    if alt_first_byte == group.prev_alt_first_byte {
                                        self.has_dupe_disjunctions = true;
                                    }
                                }
                            }
                        }
                        if alt_first_byte != 0 {
                            group.prev_alt_first_byte = alt_first_byte;
                        }
                        group.seen_pipe = true;
                        group.pipe_count = group.pipe_count.saturating_add(1);
                        group.current_has_content = false;
                        group.current_alt_atom_count = 0;
                        group.last_atom_was_lookaround = false;
                        group.current_alt_start = index + 1;
                    }
                    index += 1;
                }
                b'{' if is_zero_quantifier(bytes, index) => {
                    self.has_zero_quantifier = true;
                    if !self.has_useless_lazy && fixed_count_lazy_brace_end(bytes, index).is_some()
                    {
                        self.has_useless_lazy = true;
                    }
                    index += 1;
                }
                b'{' => {
                    if !self.has_useless_lazy && fixed_count_lazy_brace_end(bytes, index).is_some()
                    {
                        self.has_useless_lazy = true;
                    }
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

/// Returns `true` when removing the `(?:BYTE)` group whose opening `(` sits at
/// `open_pos` inside `bytes` would change the regex meaning by merging `inner`
/// with the token that immediately precedes the group.
///
/// The checks cover the following hazardous contexts:
///
/// * `\(?:X)` — bare backslash; removal creates a new escape sequence `\X`.
/// * `\N(?:D)` — digit after `\`; removal extends a backref or octal escape
///   (`\1(?:0)` → `\10`).
/// * `\x(?:H)` / `\xH(?:H)` — hex escape needs exactly two hex digits.
/// * `\u(?:H)` / … / `\uHHH(?:H)` — unicode escape needs four hex digits.
/// * `\c(?:A)` — control escape `\cX` requires a letter.
/// * `{(?:D)` / `{N,(?:D)` — digit inside a brace quantifier body.
fn non_capturing_group_removal_would_change_meaning(
    bytes: &[u8],
    open_pos: usize,
    inner: u8,
) -> bool {
    // Inside a `{...}` brace quantifier: `{(?:D)` or `{N,(?:D)}`.
    // Scan backward past digits and commas; if we hit `{`, the inner digit
    // would become part of the quantifier body and change its meaning.
    if inner.is_ascii_digit() && open_pos > 0 {
        let mut scan = open_pos - 1;
        loop {
            let b = bytes[scan];
            if b == b'{' {
                return true;
            }
            if b != b',' && !b.is_ascii_digit() {
                break;
            }
            if scan == 0 {
                break;
            }
            scan -= 1;
        }
    }

    if open_pos == 0 {
        return false;
    }
    let p1 = bytes[open_pos - 1];

    // Direct backslash immediately before `(?:`.
    if p1 == b'\\' {
        return true;
    }

    if open_pos < 2 {
        return false;
    }
    let p2 = bytes[open_pos - 2];

    // `\N(?:D)` — backslash + digit before the group; inner digit would extend
    // a backref (`\1(?:0)` → `\10`) or octal (`\0(?:1)` → `\01`).
    if p2 == b'\\' && p1.is_ascii_digit() && inner.is_ascii_digit() {
        return true;
    }

    // `\x(?:H)` — `\x` needs two hex digits; inner hex digit completes it.
    if p2 == b'\\' && p1 == b'x' && inner.is_ascii_hexdigit() {
        return true;
    }

    // `\u(?:H)` — `\u` needs four hex digits; inner hex digit starts filling them.
    if p2 == b'\\' && p1 == b'u' && inner.is_ascii_hexdigit() {
        return true;
    }

    // `\c(?:A)` — control escape; inner letter would form `\cA`.
    if p2 == b'\\' && p1 == b'c' && inner.is_ascii_alphabetic() {
        return true;
    }

    if open_pos < 3 {
        return false;
    }
    let p3 = bytes[open_pos - 3];

    // `\xH(?:H)` — one hex digit already consumed after `\x`.
    if p3 == b'\\' && p2 == b'x' && p1.is_ascii_hexdigit() && inner.is_ascii_hexdigit() {
        return true;
    }

    // `\uH(?:H)` — one hex digit after `\u`, three more needed.
    if p3 == b'\\' && p2 == b'u' && p1.is_ascii_hexdigit() && inner.is_ascii_hexdigit() {
        return true;
    }

    if open_pos < 4 {
        return false;
    }
    let p4 = bytes[open_pos - 4];

    // `\uHH(?:H)` — two hex digits after `\u`.
    if p4 == b'\\'
        && p3 == b'u'
        && p2.is_ascii_hexdigit()
        && p1.is_ascii_hexdigit()
        && inner.is_ascii_hexdigit()
    {
        return true;
    }

    if open_pos < 5 {
        return false;
    }
    let p5 = bytes[open_pos - 5];

    // `\uHHH(?:H)` — three hex digits after `\u`.
    if p5 == b'\\'
        && p4 == b'u'
        && p3.is_ascii_hexdigit()
        && p2.is_ascii_hexdigit()
        && p1.is_ascii_hexdigit()
        && inner.is_ascii_hexdigit()
    {
        return true;
    }

    false
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
