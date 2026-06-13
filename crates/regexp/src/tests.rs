use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{
    implemented_regexp_rule_names, scan_regexp,
    types::{Diagnostic, DiagnosticData},
};

fn ids(source: &str) -> SmallVec<[(&'static str, &'static str); 8]> {
    scan_regexp(source, "fixture.js")
        .into_iter()
        .map(|diagnostic| (diagnostic.rule_name, diagnostic.message_id))
        .collect()
}

fn diagnostics(source: &str) -> SmallVec<[Diagnostic; 16]> {
    scan_regexp(source, "fixture.js")
}

fn first_data(source: &str, rule_name: &'static str) -> DiagnosticData {
    diagnostics(source)
        .into_iter()
        .find(|d| d.rule_name == rule_name)
        .map(|d| d.data)
        .expect("expected diagnostic for rule")
}

fn rule_ids_for(source: &str, rule_name: &'static str) -> SmallVec<[&'static str; 8]> {
    diagnostics(source)
        .into_iter()
        .filter(|d| d.rule_name == rule_name)
        .map(|d| d.message_id)
        .collect()
}

fn rule_names_for(source: &str) -> SmallVec<[&'static str; 8]> {
    ids(source).into_iter().map(|(name, _)| name).collect()
}

#[test]
fn exposes_initial_regexp_rule_names() {
    assert_eq!(
        implemented_regexp_rule_names(),
        &[
            "no-invalid-regexp",
            "no-empty-character-class",
            "no-empty-group",
            "no-empty-capturing-group",
            "no-empty-alternative",
            "no-zero-quantifier",
            "no-octal",
            "no-control-character",
            "sort-flags",
            "require-unicode-regexp",
            "no-escape-backspace",
            "prefer-plus-quantifier",
            "prefer-star-quantifier",
            "prefer-question-quantifier",
            "no-useless-two-nums-quantifier",
            "prefer-named-capture-group",
            "match-any",
            "no-legacy-features",
            "prefer-d",
            "prefer-w",
            "letter-case",
            "no-non-standard-flag",
            "no-invisible-character",
            "hexadecimal-escape",
            "unicode-escape",
            "no-useless-range",
            "no-empty-lookarounds-assertion",
            "prefer-regexp-exec",
        ]
    );
}

#[test]
fn ignores_non_regexp_callers() {
    assert!(ids("const a = Foo('[]', 'u');").is_empty());
    assert!(ids("const a = new Bar('[', 'u');").is_empty());
    assert!(ids("const a = RegExp;").is_empty());
}

#[test]
fn ignores_dynamic_constructor_arguments() {
    // Non-literal pattern arguments cannot be statically analysed at all.
    assert!(ids("const a = new RegExp(pattern, 'u');").is_empty());
    assert!(ids("const a = RegExp();").is_empty());
    // When the flags argument is non-literal we still scan the pattern and
    // assume no `u`/`v` flag, which surfaces `require-unicode-regexp` only.
    assert_eq!(
        rule_names_for("const a = new RegExp('a', flags);").as_slice(),
        &["require-unicode-regexp"]
    );
}

#[test]
fn parses_syntactically_invalid_module_safely() {
    // Parser failure should yield no diagnostics rather than panic.
    assert!(scan_regexp("const = ;", "fixture.js").is_empty());
}

mod no_invalid_regexp {
    use super::*;

    #[test]
    fn accepts_well_formed_patterns() {
        assert!(rule_ids_for("const a = /a+/u;", "no-invalid-regexp").is_empty());
        assert!(
            rule_ids_for("const a = new RegExp('a+', 'gimsu');", "no-invalid-regexp").is_empty()
        );
        assert!(
            rule_ids_for(
                "const a = new RegExp('(?:a|b)+', 'v');",
                "no-invalid-regexp"
            )
            .is_empty()
        );
    }

    #[test]
    fn reports_unclosed_constructs() {
        assert_eq!(
            rule_ids_for("const a = new RegExp('[', 'u');", "no-invalid-regexp").as_slice(),
            &["error"]
        );
        assert_eq!(
            rule_ids_for("const a = new RegExp('(?:', 'u');", "no-invalid-regexp").as_slice(),
            &["error"]
        );
        assert_eq!(
            rule_ids_for("const a = new RegExp('\\\\u{', 'u');", "no-invalid-regexp").as_slice(),
            &["error"]
        );
    }

    #[test]
    fn reports_duplicate_flags() {
        let data = first_data("const a = new RegExp('a', 'gg');", "no-invalid-regexp");
        assert_eq!(data.flag.as_ref().map(CompactString::as_str), Some("g"));

        // Duplicate detection short-circuits before further checks.
        assert_eq!(
            ids("const a = new RegExp('a', 'ii');").as_slice(),
            &[("no-invalid-regexp", "duplicateFlag")]
        );
    }

    #[test]
    fn reports_conflicting_u_and_v_flags() {
        assert_eq!(
            ids("const a = new RegExp('a', 'uv');").as_slice(),
            &[("no-invalid-regexp", "uvFlag")]
        );
        assert_eq!(
            ids("const a = RegExp('a', 'vu');").as_slice(),
            &[("no-invalid-regexp", "uvFlag")]
        );
    }

    #[test]
    fn does_not_validate_literal_patterns() {
        // Literal regexps are already parser-validated by the JS engine; we
        // intentionally skip constructor-style validation for them.
        assert!(rule_ids_for("const a = /a+/u;", "no-invalid-regexp").is_empty());
    }
}

mod no_empty_character_class {
    use super::*;

    #[test]
    fn reports_empty_classes_in_various_positions() {
        assert_eq!(
            rule_ids_for("const a = /[]/u;", "no-empty-character-class").as_slice(),
            &["empty"]
        );
        assert_eq!(
            rule_ids_for("const a = /abc[]def/u;", "no-empty-character-class").as_slice(),
            &["empty"]
        );
        assert_eq!(
            rule_ids_for(
                "const a = new RegExp('[]', 'u');",
                "no-empty-character-class"
            )
            .as_slice(),
            &["empty"]
        );
    }

    #[test]
    fn accepts_non_empty_or_negated_classes() {
        assert!(rule_ids_for("const a = /[a]/u;", "no-empty-character-class").is_empty());
        assert!(rule_ids_for("const a = /[^]/u;", "no-empty-character-class").is_empty());
        // A `]` escaped inside the class still has content.
        assert!(rule_ids_for("const a = /[\\]]/u;", "no-empty-character-class").is_empty());
    }
}

mod no_empty_group {
    use super::*;

    #[test]
    fn reports_empty_non_capturing_groups() {
        assert_eq!(
            rule_ids_for("const a = /(?:)/u;", "no-empty-group").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /a(?:)b/u;", "no-empty-group").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn reports_empty_named_capturing_group_as_group_too() {
        // Named capturing groups participate in both empty-group rules.
        assert_eq!(
            rule_ids_for("const a = /(?<name>)/u;", "no-empty-group").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_lookaround_groups() {
        // Lookarounds are not checked for emptiness; they are valid even when empty.
        assert!(rule_ids_for("const a = /(?=)/u;", "no-empty-group").is_empty());
        assert!(rule_ids_for("const a = /(?!)/u;", "no-empty-group").is_empty());
        assert!(rule_ids_for("const a = /(?<=)/u;", "no-empty-group").is_empty());
        assert!(rule_ids_for("const a = /(?<!)/u;", "no-empty-group").is_empty());
    }

    #[test]
    fn accepts_non_empty_groups() {
        assert!(rule_ids_for("const a = /(?:a)/u;", "no-empty-group").is_empty());
        assert!(rule_ids_for("const a = /(?:a|b)/u;", "no-empty-group").is_empty());
    }
}

mod no_empty_capturing_group {
    use super::*;

    #[test]
    fn reports_anonymous_and_named_empty_captures() {
        assert!(
            rule_ids_for("const a = /()/u;", "no-empty-capturing-group").contains(&"unexpected")
        );
        assert!(
            rule_ids_for("const a = /(?<name>)/u;", "no-empty-capturing-group")
                .contains(&"unexpected")
        );
        assert!(
            rule_ids_for("const a = /a()b/u;", "no-empty-capturing-group").contains(&"unexpected")
        );
    }

    #[test]
    fn accepts_non_empty_captures() {
        assert!(rule_ids_for("const a = /(a)/u;", "no-empty-capturing-group").is_empty());
        assert!(rule_ids_for("const a = /(?<name>a)/u;", "no-empty-capturing-group").is_empty());
    }
}

mod no_empty_alternative {
    use super::*;

    #[test]
    fn reports_top_level_empty_alternatives() {
        assert_eq!(
            rule_ids_for("const a = /a|/u;", "no-empty-alternative").as_slice(),
            &["empty"]
        );
        assert_eq!(
            rule_ids_for("const a = /|a/u;", "no-empty-alternative").as_slice(),
            &["empty"]
        );
        assert_eq!(
            rule_ids_for("const a = /a||b/u;", "no-empty-alternative").as_slice(),
            &["empty"]
        );
    }

    #[test]
    fn reports_empty_alternatives_inside_groups() {
        assert_eq!(
            rule_ids_for("const a = /(a|)/u;", "no-empty-alternative").as_slice(),
            &["empty"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:a|)/u;", "no-empty-alternative").as_slice(),
            &["empty"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:|b)/u;", "no-empty-alternative").as_slice(),
            &["empty"]
        );
    }

    #[test]
    fn accepts_filled_alternatives() {
        assert!(rule_ids_for("const a = /a|b/u;", "no-empty-alternative").is_empty());
        assert!(rule_ids_for("const a = /(?:a|b|c)/u;", "no-empty-alternative").is_empty());
    }
}

mod no_zero_quantifier {
    use super::*;

    #[test]
    fn reports_zero_braced_quantifiers() {
        assert_eq!(
            rule_ids_for("const a = /a{0}/u;", "no-zero-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /a{0,0}/u;", "no-zero-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:abc){0}/u;", "no-zero-quantifier").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn accepts_quantifiers_with_a_nonzero_upper_bound() {
        assert!(rule_ids_for("const a = /a{1}/u;", "no-zero-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{0,1}/u;", "no-zero-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{0,}/u;", "no-zero-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{2,5}/u;", "no-zero-quantifier").is_empty());
    }
}

mod no_octal {
    use super::*;

    #[test]
    fn reports_real_octal_escapes() {
        let data = first_data("const a = /\\07/u;", "no-octal");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("\\07"));

        let data = first_data("const a = /\\012/u;", "no-octal");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("\\012"));
    }

    #[test]
    fn ignores_nul_and_non_octal_digits() {
        // `\0` alone is the NUL escape, not an octal sequence.
        assert!(rule_ids_for("const a = /\\0/u;", "no-octal").is_empty());
        // `\08` is `\0` followed by literal `8` (which is not an octal digit).
        assert!(rule_ids_for("const a = /\\08/u;", "no-octal").is_empty());
    }
}

mod no_control_character {
    use super::*;

    #[test]
    fn reports_hex_and_unicode_control_escapes() {
        let data = first_data(
            "const a = new RegExp('\\x01', 'u');",
            "no-control-character",
        );
        assert_eq!(
            data.char_text.as_ref().map(CompactString::as_str),
            Some("U+0001")
        );

        let data = first_data(
            "const a = new RegExp('\\u0002', 'u');",
            "no-control-character",
        );
        assert_eq!(
            data.char_text.as_ref().map(CompactString::as_str),
            Some("U+0002")
        );

        let data = first_data(
            "const a = new RegExp('\\u{3}', 'u');",
            "no-control-character",
        );
        assert_eq!(
            data.char_text.as_ref().map(CompactString::as_str),
            Some("U+0003")
        );
    }

    #[test]
    fn accepts_named_or_printable_escapes() {
        // Named escapes (`\t`, `\n`, `\r`) are NOT reported.
        assert!(rule_ids_for("const a = /\\t/u;", "no-control-character").is_empty());
        assert!(rule_ids_for("const a = /\\n/u;", "no-control-character").is_empty());
        // Printable characters above U+001F are fine.
        assert!(rule_ids_for("const a = /a/u;", "no-control-character").is_empty());
        // `\u0041` ('A') is above the control range.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\u0041', 'u');",
                "no-control-character"
            )
            .is_empty()
        );
    }
}

mod sort_flags {
    use super::*;

    #[test]
    fn reports_unsorted_flags_with_data() {
        let data = first_data("const a = /a/mi;", "sort-flags");
        assert_eq!(data.flags.as_ref().map(CompactString::as_str), Some("mi"));
        assert_eq!(
            data.sorted_flags.as_ref().map(CompactString::as_str),
            Some("im")
        );
    }

    #[test]
    fn accepts_already_sorted_or_empty_flags() {
        assert!(rule_ids_for("const a = /a/gim;", "sort-flags").is_empty());
        assert!(rule_ids_for("const a = /a/u;", "sort-flags").is_empty());
        assert!(rule_ids_for("const a = /a/;", "sort-flags").is_empty());
        assert!(rule_ids_for("const a = new RegExp('a');", "sort-flags").is_empty());
    }
}

mod require_unicode_regexp {
    use super::*;

    #[test]
    fn reports_when_neither_u_nor_v_is_present() {
        assert_eq!(
            rule_ids_for("const a = /a/;", "require-unicode-regexp").as_slice(),
            &["require"]
        );
        assert_eq!(
            rule_ids_for("const a = /a/g;", "require-unicode-regexp").as_slice(),
            &["require"]
        );
        assert_eq!(
            rule_ids_for("const a = new RegExp('a');", "require-unicode-regexp").as_slice(),
            &["require"]
        );
    }

    #[test]
    fn accepts_u_or_v_flag() {
        assert!(rule_ids_for("const a = /a/u;", "require-unicode-regexp").is_empty());
        assert!(rule_ids_for("const a = /a/v;", "require-unicode-regexp").is_empty());
        assert!(rule_ids_for("const a = /a/gu;", "require-unicode-regexp").is_empty());
        assert!(
            rule_ids_for("const a = new RegExp('a', 'u');", "require-unicode-regexp").is_empty()
        );
    }
}

mod no_escape_backspace {
    use super::*;

    #[test]
    fn reports_backspace_escape_inside_character_class() {
        assert_eq!(
            rule_ids_for("const a = /[\\b]/u;", "no-escape-backspace").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /a[\\b]b/u;", "no-escape-backspace").as_slice(),
            &["unexpected"]
        );
        // Mixed-content classes still report when `\b` is one element.
        assert_eq!(
            rule_ids_for("const a = /[a\\b]/u;", "no-escape-backspace").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_word_boundary_outside_character_class() {
        // `\b` outside a character class is the word-boundary assertion, not a
        // backspace; the rule must not flag it.
        assert!(rule_ids_for("const a = /\\bword/u;", "no-escape-backspace").is_empty());
        assert!(rule_ids_for("const a = /a\\bb/u;", "no-escape-backspace").is_empty());
    }

    #[test]
    fn ignores_other_classes() {
        assert!(rule_ids_for("const a = /[a-z]/u;", "no-escape-backspace").is_empty());
        // `\\b` (escaped backslash followed by `b`) is just the letter `b`.
        assert!(rule_ids_for("const a = /[\\\\b]/u;", "no-escape-backspace").is_empty());
    }
}

mod prefer_plus_quantifier {
    use super::*;

    #[test]
    fn reports_one_or_more_braced_form() {
        let data = first_data("const a = /a{1,}/u;", "prefer-plus-quantifier");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("{1,}"));
    }

    #[test]
    fn accepts_other_quantifiers_and_plus_itself() {
        assert!(rule_ids_for("const a = /a+/u;", "prefer-plus-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{2,}/u;", "prefer-plus-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{1,3}/u;", "prefer-plus-quantifier").is_empty());
    }
}

mod prefer_star_quantifier {
    use super::*;

    #[test]
    fn reports_zero_or_more_braced_form() {
        let data = first_data("const a = /a{0,}/u;", "prefer-star-quantifier");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("{0,}"));
    }

    #[test]
    fn accepts_star_and_other_quantifiers() {
        assert!(rule_ids_for("const a = /a*/u;", "prefer-star-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{1,}/u;", "prefer-star-quantifier").is_empty());
        // `{0,N}` is a different shape; not flagged by this rule.
        assert!(rule_ids_for("const a = /a{0,5}/u;", "prefer-star-quantifier").is_empty());
    }
}

mod prefer_question_quantifier {
    use super::*;

    #[test]
    fn reports_zero_or_one_braced_form() {
        let data = first_data("const a = /a{0,1}/u;", "prefer-question-quantifier");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("{0,1}"));
    }

    #[test]
    fn accepts_question_mark_and_unrelated_quantifiers() {
        assert!(rule_ids_for("const a = /a?/u;", "prefer-question-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{0,2}/u;", "prefer-question-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{1,2}/u;", "prefer-question-quantifier").is_empty());
    }
}

mod no_useless_two_nums_quantifier {
    use super::*;

    #[test]
    fn reports_equal_bounds_quantifier() {
        let data = first_data("const a = /a{3,3}/u;", "no-useless-two-nums-quantifier");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("{3,3}"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("{3}")
        );
    }

    #[test]
    fn accepts_distinct_bounds_and_canonical_form() {
        assert!(rule_ids_for("const a = /a{3}/u;", "no-useless-two-nums-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{2,5}/u;", "no-useless-two-nums-quantifier").is_empty());
        // `{0,0}` is no-zero-quantifier's responsibility; we do not double-report.
        assert!(rule_ids_for("const a = /a{0,0}/u;", "no-useless-two-nums-quantifier").is_empty());
    }
}

mod prefer_named_capture_group {
    use super::*;

    #[test]
    fn reports_anonymous_capturing_groups() {
        assert_eq!(
            rule_ids_for("const a = /(a)/u;", "prefer-named-capture-group").as_slice(),
            &["required"]
        );
        // Even when alternation is present, the unnamed capture is flagged.
        assert_eq!(
            rule_ids_for("const a = /(foo|bar)/u;", "prefer-named-capture-group").as_slice(),
            &["required"]
        );
    }

    #[test]
    fn ignores_named_captures_and_non_capturing_groups() {
        assert!(rule_ids_for("const a = /(?<name>a)/u;", "prefer-named-capture-group").is_empty());
        assert!(rule_ids_for("const a = /(?:a)/u;", "prefer-named-capture-group").is_empty());
        assert!(rule_ids_for("const a = /(?=a)/u;", "prefer-named-capture-group").is_empty());
        assert!(rule_ids_for("const a = /(?!a)/u;", "prefer-named-capture-group").is_empty());
        assert!(rule_ids_for("const a = /(?<=a)/u;", "prefer-named-capture-group").is_empty());
        assert!(rule_ids_for("const a = /(?<!a)/u;", "prefer-named-capture-group").is_empty());
    }

    #[test]
    fn reports_once_per_literal_even_with_multiple_anonymous_captures() {
        // Per the existing pattern, each literal emits at most one diagnostic
        // per rule; multiple anonymous captures collapse to one report.
        assert_eq!(
            rule_ids_for("const a = /(a)(b)/u;", "prefer-named-capture-group").as_slice(),
            &["required"]
        );
    }
}

mod match_any {
    use super::*;

    #[test]
    fn reports_anti_pair_character_classes() {
        assert_eq!(
            rule_ids_for("const a = /[\\s\\S]/u;", "match-any").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[\\d\\D]/u;", "match-any").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[\\w\\W]/u;", "match-any").as_slice(),
            &["unexpected"]
        );
        // Order does not matter.
        assert_eq!(
            rule_ids_for("const a = /[\\S\\s]/u;", "match-any").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_non_anti_pair_classes() {
        assert!(rule_ids_for("const a = /[a-z]/u;", "match-any").is_empty());
        // Only one of the pair is present.
        assert!(rule_ids_for("const a = /[\\s]/u;", "match-any").is_empty());
        // Mixed shorthand families are NOT anti-pairs.
        assert!(rule_ids_for("const a = /[\\s\\D]/u;", "match-any").is_empty());
        // Three or more elements are not "exactly an anti-pair".
        assert!(rule_ids_for("const a = /[\\s\\Sa]/u;", "match-any").is_empty());
        // Negated classes never match anything; do not flag.
        assert!(rule_ids_for("const a = /[^\\s\\S]/u;", "match-any").is_empty());
    }
}

mod no_legacy_features {
    use super::*;

    #[test]
    fn reports_indexed_capture_properties() {
        let data = first_data("RegExp.$1;", "no-legacy-features");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("$1"));
        assert_eq!(
            rule_ids_for("RegExp.$9;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
    }

    #[test]
    fn reports_named_legacy_properties() {
        assert_eq!(
            rule_ids_for("RegExp.input;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
        assert_eq!(
            rule_ids_for("RegExp.$_;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
        assert_eq!(
            rule_ids_for("RegExp.lastMatch;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
        assert_eq!(
            rule_ids_for("RegExp.lastParen;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
        assert_eq!(
            rule_ids_for("RegExp.leftContext;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
        assert_eq!(
            rule_ids_for("RegExp.rightContext;", "no-legacy-features").as_slice(),
            &["staticProperty"]
        );
    }

    #[test]
    fn ignores_non_regexp_member_access_and_modern_apis() {
        assert!(rule_ids_for("Foo.$1;", "no-legacy-features").is_empty());
        assert!(rule_ids_for("RegExp.prototype;", "no-legacy-features").is_empty());
        assert!(rule_ids_for("regexp.lastMatch;", "no-legacy-features").is_empty());
        // `$10` is NOT one of the legacy indices ($1–$9).
        assert!(rule_ids_for("RegExp.$10;", "no-legacy-features").is_empty());
    }
}

mod prefer_d {
    use super::*;

    #[test]
    fn reports_digit_ranges_with_replacement() {
        let data = first_data("const a = /[0-9]/u;", "prefer-d");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("[0-9]"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\d")
        );

        let data = first_data("const a = /[^0-9]/u;", "prefer-d");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("[^0-9]")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\D")
        );
    }

    #[test]
    fn ignores_supersets_and_subranges() {
        // Subset of the digit range; not equivalent to \d.
        assert!(rule_ids_for("const a = /[1-9]/u;", "prefer-d").is_empty());
        // Superset that includes letters; not equivalent.
        assert!(rule_ids_for("const a = /[0-9a]/u;", "prefer-d").is_empty());
        // Already \d; should not flag.
        assert!(rule_ids_for("const a = /\\d/u;", "prefer-d").is_empty());
    }
}

mod prefer_w {
    use super::*;

    #[test]
    fn reports_word_char_set_in_any_order() {
        assert_eq!(
            rule_ids_for("const a = /[a-zA-Z0-9_]/u;", "prefer-w").as_slice(),
            &["unexpected"]
        );
        // Reordered elements still match.
        assert_eq!(
            rule_ids_for("const a = /[_0-9A-Za-z]/u;", "prefer-w").as_slice(),
            &["unexpected"]
        );

        let data = first_data("const a = /[^a-zA-Z0-9_]/u;", "prefer-w");
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\W")
        );
    }

    #[test]
    fn ignores_classes_missing_or_adding_elements() {
        // Missing underscore.
        assert!(rule_ids_for("const a = /[a-zA-Z0-9]/u;", "prefer-w").is_empty());
        // Adds extra range.
        assert!(rule_ids_for("const a = /[a-zA-Z0-9_-]/u;", "prefer-w").is_empty());
        // Already \w.
        assert!(rule_ids_for("const a = /\\w/u;", "prefer-w").is_empty());
    }
}

mod letter_case {
    use super::*;

    #[test]
    fn reports_uppercase_hex_escapes() {
        let data = first_data("const a = new RegExp('\\\\xAB', 'u');", "letter-case");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("\\xAB"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\xab")
        );

        let data = first_data("const a = new RegExp('\\\\uABCD', 'u');", "letter-case");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\uABCD")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\uabcd")
        );

        let data = first_data("const a = new RegExp('\\\\u{1F4A9}', 'u');", "letter-case");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\u{1F4A9}")
        );
    }

    #[test]
    fn ignores_lowercase_and_decimal_only_escapes() {
        assert!(rule_ids_for("const a = /\\xab/u;", "letter-case").is_empty());
        assert!(rule_ids_for("const a = /\\uabcd/u;", "letter-case").is_empty());
        // Decimal-only digits are already lowercase-equivalent.
        assert!(rule_ids_for("const a = new RegExp('\\\\u0041', 'u');", "letter-case").is_empty());
    }
}

mod no_non_standard_flag {
    use super::*;

    #[test]
    fn reports_first_non_standard_flag() {
        // `q` is not a valid JS regex flag. The constructor parser also errors
        // out on it, but `no-non-standard-flag` must still report its own diag.
        let names = rule_names_for("const a = new RegExp('a', 'gq');");
        assert!(names.contains(&"no-non-standard-flag"));
        let data = first_data("const a = new RegExp('a', 'gq');", "no-non-standard-flag");
        assert_eq!(data.flag.as_ref().map(CompactString::as_str), Some("q"));
    }

    #[test]
    fn ignores_canonical_flag_set() {
        // The disallowed-macros lint forbids `format!` in this crate, so we
        // enumerate canonical flag combinations explicitly.
        let sources = [
            "const a = new RegExp('a', 'd');",
            "const a = new RegExp('a', 'g');",
            "const a = new RegExp('a', 'i');",
            "const a = new RegExp('a', 'm');",
            "const a = new RegExp('a', 's');",
            "const a = new RegExp('a', 'u');",
            "const a = new RegExp('a', 'v');",
            "const a = new RegExp('a', 'y');",
            "const a = new RegExp('a', 'gimsuy');",
            "const a = new RegExp('a', 'gv');",
        ];
        for source in sources {
            assert!(
                rule_ids_for(source, "no-non-standard-flag").is_empty(),
                "expected no diagnostic for canonical flags in source: {source}",
            );
        }
    }
}

mod no_invisible_character {
    use super::*;

    #[test]
    fn reports_invisible_characters_in_pattern() {
        // U+00A0 NO-BREAK SPACE literal inside the pattern.
        let data = first_data("const a = /a\u{00A0}b/u;", "no-invisible-character");
        assert_eq!(
            data.char_text.as_ref().map(CompactString::as_str),
            Some("U+00A0")
        );
        // U+200B ZERO WIDTH SPACE — invisible to the eye.
        let names = rule_names_for("const a = /a\u{200B}b/u;");
        assert!(names.contains(&"no-invisible-character"));
        // U+FEFF BOM in the middle of a pattern.
        let names = rule_names_for("const a = /a\u{FEFF}b/u;");
        assert!(names.contains(&"no-invisible-character"));
    }

    #[test]
    fn ignores_visible_and_escaped_characters() {
        assert!(rule_ids_for("const a = /ab/u;", "no-invisible-character").is_empty());
        // Plain ASCII space is not invisible.
        assert!(rule_ids_for("const a = /a b/u;", "no-invisible-character").is_empty());
        // Escaped hex sequence for U+00A0 is not the literal invisible char.
        assert!(
            rule_ids_for(
                "const a = new RegExp('a\\\\xa0b', 'u');",
                "no-invisible-character"
            )
            .is_empty()
        );
    }
}

mod hexadecimal_escape {
    use super::*;

    #[test]
    fn reports_hex_x_escapes_with_unicode_replacement() {
        let data = first_data(
            "const a = new RegExp('\\\\xab', 'u');",
            "hexadecimal-escape",
        );
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("\\xab"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\u{ab}")
        );
        // Uppercase digits are normalised in the replacement.
        let data = first_data(
            "const a = new RegExp('\\\\xAB', 'u');",
            "hexadecimal-escape",
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\u{ab}")
        );
    }

    #[test]
    fn ignores_unicode_escapes_and_unrelated_escapes() {
        assert!(rule_ids_for("const a = /\\uabcd/u;", "hexadecimal-escape").is_empty());
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\u{ab}', 'u');",
                "hexadecimal-escape"
            )
            .is_empty()
        );
        assert!(rule_ids_for("const a = /\\d/u;", "hexadecimal-escape").is_empty());
    }
}

mod unicode_escape {
    use super::*;

    #[test]
    fn reports_fixed_unicode_escapes_with_codepoint_replacement() {
        let data = first_data("const a = new RegExp('\\\\uabcd', 'u');", "unicode-escape");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\uabcd")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\u{abcd}")
        );
    }

    #[test]
    fn ignores_codepoint_form_and_other_escapes() {
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\u{abcd}', 'u');",
                "unicode-escape"
            )
            .is_empty()
        );
        assert!(rule_ids_for("const a = /\\xab/u;", "unicode-escape").is_empty());
        assert!(rule_ids_for("const a = /\\d/u;", "unicode-escape").is_empty());
    }
}

mod no_useless_range {
    use super::*;

    #[test]
    fn reports_single_char_ranges() {
        let data = first_data("const a = /[a-a]/u;", "no-useless-range");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("a-a"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("a")
        );
        assert_eq!(
            rule_ids_for("const a = /[0-0]/u;", "no-useless-range").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[a-ab]/u;", "no-useless-range").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_real_ranges_and_unrelated_classes() {
        assert!(rule_ids_for("const a = /[a-z]/u;", "no-useless-range").is_empty());
        assert!(rule_ids_for("const a = /[0-9]/u;", "no-useless-range").is_empty());
        // Bare repeated characters without a `-` in between are not ranges.
        assert!(rule_ids_for("const a = /[aa]/u;", "no-useless-range").is_empty());
    }
}

mod no_empty_lookarounds_assertion {
    use super::*;

    #[test]
    fn reports_each_empty_lookaround_shape() {
        assert_eq!(
            rule_ids_for("const a = /(?=)/u;", "no-empty-lookarounds-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?!)/u;", "no-empty-lookarounds-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<=)/u;", "no-empty-lookarounds-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<!)/u;", "no-empty-lookarounds-assertion").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_filled_lookarounds_and_empty_non_lookaround_groups() {
        assert!(rule_ids_for("const a = /(?=a)/u;", "no-empty-lookarounds-assertion").is_empty());
        // Empty non-capturing group is `no-empty-group`'s responsibility.
        assert!(rule_ids_for("const a = /(?:)/u;", "no-empty-lookarounds-assertion").is_empty());
    }
}

mod prefer_regexp_exec {
    use super::*;

    #[test]
    fn reports_string_match_with_non_global_regexp() {
        assert_eq!(
            rule_ids_for("str.match(/foo/u);", "prefer-regexp-exec").as_slice(),
            &["unexpected"]
        );
        // Other call shapes still match if the property is `match`.
        assert_eq!(
            rule_ids_for("obj.prop.match(/foo/);", "prefer-regexp-exec").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_global_regexps_and_unrelated_calls() {
        assert!(rule_ids_for("str.match(/foo/gu);", "prefer-regexp-exec").is_empty());
        assert!(rule_ids_for("str.match(/foo/g);", "prefer-regexp-exec").is_empty());
        // Non-literal argument — we cannot be sure of the flags.
        assert!(rule_ids_for("str.match(pattern);", "prefer-regexp-exec").is_empty());
        // Different method name.
        assert!(rule_ids_for("str.replace(/foo/u, 'bar');", "prefer-regexp-exec").is_empty());
    }
}

#[test]
fn brace_quantifier_rules_ignore_quantifiers_inside_character_classes() {
    // Inside `[...]` braces are literal characters, not quantifier syntax.
    assert!(rule_ids_for("const a = /[a{1,}]/u;", "prefer-plus-quantifier").is_empty());
    assert!(rule_ids_for("const a = /[a{0,}]/u;", "prefer-star-quantifier").is_empty());
    assert!(rule_ids_for("const a = /[a{0,1}]/u;", "prefer-question-quantifier").is_empty());
    assert!(rule_ids_for("const a = /[a{3,3}]/u;", "no-useless-two-nums-quantifier").is_empty());
}

#[test]
fn reports_multiple_rules_for_a_single_regex() {
    // `/()/mi` triggers: empty group, empty capturing group, sort-flags, require-unicode-regexp.
    let rules = rule_names_for("const a = /()/mi;");
    assert!(rules.contains(&"no-empty-group"));
    assert!(rules.contains(&"no-empty-capturing-group"));
    assert!(rules.contains(&"sort-flags"));
    assert!(rules.contains(&"require-unicode-regexp"));
}

#[test]
fn reports_each_literal_independently() {
    assert_eq!(
        rule_names_for("const a = /[]/u; const b = /a|/u;").as_slice(),
        &["no-empty-character-class", "no-empty-alternative"]
    );
}
