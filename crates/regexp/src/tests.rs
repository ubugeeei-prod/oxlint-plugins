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
            "prefer-regexp-test",
            "no-missing-g-flag",
            "no-useless-character-class",
            "no-empty-string-literal",
            "no-optional-assertion",
            "require-unicode-sets-regexp",
            "confusing-quantifier",
            "prefer-named-replacement",
            "no-obscure-range",
            "prefer-unicode-codepoint-escapes",
            "no-dupe-characters-character-class",
            "prefer-range",
            "no-useless-escape",
            "no-useless-quantifier",
            "prefer-named-backreference",
            "no-useless-flag",
            "no-lazy-ends",
            "no-useless-dollar-replacements",
            "prefer-escape-replacement-dollar-char",
            "use-ignore-case",
            "control-character-escape",
            "grapheme-string-literal",
            "no-useless-non-capturing-group",
            "prefer-quantifier",
            "no-useless-string-literal",
            "sort-character-class-elements",
            "no-trivially-nested-assertion",
            "no-extra-lookaround-assertions",
            "no-trivially-nested-quantifier",
            "prefer-character-class",
            "sort-alternatives",
            "prefer-predefined-assertion",
            "optimal-lookaround-quantifier",
            "no-dupe-disjunctions",
            "no-useless-backreference",
            "negation",
            "no-useless-lazy",
            "no-misleading-unicode-character",
            "no-standalone-backslash",
            "no-potentially-useless-backreference",
            "strict",
            "no-useless-assertions",
            "optimal-quantifier-concatenation",
            "no-contradiction-with-assertion",
            "no-useless-set-operand",
        ]
    );
}

mod no_useless_set_operand {
    use super::*;

    #[test]
    fn reports_redundant_shorthand_operands() {
        for src in [
            // Intersection with a subset operand → superset is useless.
            "const a = /[\\w&&\\d]/v;",
            // Intersection of disjoint sets → empty.
            "const a = /[\\w&&\\s]/v;",
            // Negated class still has redundant operands.
            "const a = /[^\\w&&\\s]/v;",
            // Subtraction of a disjoint set → removes nothing.
            "const a = /[\\w--\\s]/v;",
            // Subtraction where left ⊆ right → empty.
            "const a = /[\\d--\\w]/v;",
        ] {
            assert_eq!(
                rule_ids_for(src, "no-useless-set-operand").as_slice(),
                &["unexpected"],
                "expected report for {src}"
            );
        }
    }

    #[test]
    fn ignores_meaningful_or_unsupported_shapes() {
        // The one upstream-valid shorthand case: removing a proper subset.
        assert!(rule_ids_for("const a = /[\\w--\\d]/v;", "no-useless-set-operand").is_empty());
        // Not v-mode → set operations are not in effect (and `&&` is literal).
        assert!(rule_ids_for("const a = /[\\w&&\\d]/u;", "no-useless-set-operand").is_empty());
        // Nested-class operand is out of scope.
        assert!(rule_ids_for("const a = /[\\w&&[\\d\\s]]/v;", "no-useless-set-operand").is_empty());
    }
}

mod no_contradiction_with_assertion {
    use super::*;

    #[test]
    fn reports_unenterable_quantifier_after_boundary() {
        // `a\b` then `a*`: the `a*` can never be entered (word char right after
        // a boundary that follows a word char).
        assert_eq!(
            rule_ids_for("const a = /a\\ba*-/u;", "no-contradiction-with-assertion").as_slice(),
            &["unexpected"]
        );
        // Brace form `{0,3}` also has min zero.
        assert_eq!(
            rule_ids_for(
                "const a = /a\\ba{0,3}-/u;",
                "no-contradiction-with-assertion"
            )
            .as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_consistent_or_unsupported_shapes() {
        // Plain `a` (min 1) after the boundary — not this rule's narrow case.
        assert!(rule_ids_for("const a = /a\\ba/u;", "no-contradiction-with-assertion").is_empty());
        // Quantifier on a different-class char is consistent with the boundary.
        assert!(
            rule_ids_for("const a = /a\\b-*a/u;", "no-contradiction-with-assertion").is_empty()
        );
        // Boundary preceded by a group close — not a literal neighbour.
        assert!(
            rule_ids_for(
                "const a = /(?:x)\\ba*/u;",
                "no-contradiction-with-assertion"
            )
            .is_empty()
        );
    }
}

mod optimal_quantifier_concatenation {
    use super::*;

    #[test]
    fn reports_mergeable_same_element_quantifiers() {
        for src in [
            "const a = /aa*/u;",
            "const a = /a*a*/u;",
            "const a = /\\w\\w*/u;",
            "const a = /\\w*\\w/u;",
            "const a = /\\w+\\w/u;",
            "const a = /a+a+/u;",
        ] {
            assert_eq!(
                rule_ids_for(src, "optimal-quantifier-concatenation").as_slice(),
                &["unexpected"],
                "expected report for {src}"
            );
        }
    }

    #[test]
    fn ignores_bounded_only_or_distinct_elements() {
        for src in [
            // Bounded-only pairs upstream keeps.
            "const a = /aa?/u;",
            "const a = /\\w?\\w/u;",
            // Distinct elements.
            "const a = /\\w+\\d{4}/u;",
            "const a = /a+b+c+d+/u;",
            // Group vs shorthand — not the same single element.
            "const a = /(\\d)\\d+/u;",
        ] {
            assert!(
                rule_ids_for(src, "optimal-quantifier-concatenation").is_empty(),
                "expected no report for {src}"
            );
        }
    }
}

mod no_useless_assertions {
    use super::*;

    #[test]
    fn reports_boundary_between_same_class_literals() {
        // `\b` between two word chars: always rejects.
        assert_eq!(
            rule_ids_for("const a = /a\\bb/u;", "no-useless-assertions").as_slice(),
            &["unexpected"]
        );
        // `\B` between two word chars: always accepts.
        assert_eq!(
            rule_ids_for("const a = /a\\Bb/u;", "no-useless-assertions").as_slice(),
            &["unexpected"]
        );
        // `\b` between two non-word chars (comma): always rejects.
        assert_eq!(
            rule_ids_for("const a = /,\\b,/u;", "no-useless-assertions").as_slice(),
            &["unexpected"]
        );
        // Mixed-class neighbours (`a` word, `,` non-word) are a real boundary:
        // `\b` there is meaningful, so it must NOT be flagged.
        assert!(rule_ids_for("const a = /a\\b,/u;", "no-useless-assertions").is_empty());
    }

    #[test]
    fn ignores_decidable_or_unsupported_shapes() {
        // `\b.\b` — `.` is not a fixed-class literal; upstream-valid.
        assert!(rule_ids_for("const a = /\\b.\\b/u;", "no-useless-assertions").is_empty());
        // `\b(?:,|:)\b` — group neighbours; upstream-valid.
        assert!(rule_ids_for("const a = /\\b(?:,|:)\\b/u;", "no-useless-assertions").is_empty());
        // word/non-word transition: `\b` is genuinely meaningful, not flagged.
        assert!(rule_ids_for("const a = /a\\b /u;", "no-useless-assertions").is_empty());
        // `\b` inside a character class is a backspace, never a boundary.
        assert!(rule_ids_for("const a = /[a\\bb]/u;", "no-useless-assertions").is_empty());
    }
}

mod no_potentially_useless_backreference {
    use super::*;

    #[test]
    fn reports_backreference_to_optional_group() {
        assert_eq!(
            rule_ids_for(
                "const a = /(a)?\\1/;",
                "no-potentially-useless-backreference"
            )
            .as_slice(),
            &["potentiallyUselessBackreference"]
        );
        assert_eq!(
            rule_ids_for(
                "const a = /(a)*\\1/;",
                "no-potentially-useless-backreference"
            )
            .as_slice(),
            &["potentiallyUselessBackreference"]
        );
    }

    #[test]
    fn ignores_valid_backreferences() {
        assert!(
            rule_ids_for("const a = /()\\1/;", "no-potentially-useless-backreference").is_empty()
        );
        assert!(
            rule_ids_for(
                "const a = /(a)+\\1/;",
                "no-potentially-useless-backreference"
            )
            .is_empty()
        );
        assert!(
            rule_ids_for(
                "const a = /(a+)b|\\1/;",
                "no-potentially-useless-backreference"
            )
            .is_empty()
        );
        assert!(
            rule_ids_for(
                "const a = /(?=(a))\\1/;",
                "no-potentially-useless-backreference"
            )
            .is_empty()
        );
    }
}

mod strict {
    use super::*;

    #[test]
    fn reports_unescaped_close_bracket() {
        assert_eq!(
            rule_ids_for("const a = /]/;", "strict").as_slice(),
            &["unescapedSourceCharacter"]
        );
    }

    #[test]
    fn reports_incomplete_control_escape() {
        assert_eq!(
            rule_ids_for("const a = /\\c;/;", "strict").as_slice(),
            &["invalidControlEscape"]
        );
        assert_eq!(
            rule_ids_for(r"const a = /\c/;", "strict").as_slice(),
            &["invalidControlEscape"]
        );
    }

    #[test]
    fn reports_incomplete_unicode_escape() {
        assert_eq!(
            rule_ids_for(r"const a = /\u{42}/;", "strict").as_slice(),
            &["incompleteEscapeSequence"]
        );
        assert_eq!(
            rule_ids_for(r"const a = /\u000;/;", "strict").as_slice(),
            &["incompleteEscapeSequence"]
        );
    }

    #[test]
    fn reports_incomplete_hex_escape() {
        assert_eq!(
            rule_ids_for(r"const a = /\x4/;", "strict").as_slice(),
            &["incompleteEscapeSequence"]
        );
    }

    #[test]
    fn reports_property_escape_in_non_u_mode() {
        assert_eq!(
            rule_ids_for(r"const a = /\p/;", "strict").as_slice(),
            &["invalidPropertyEscape"]
        );
        assert_eq!(
            rule_ids_for(r"const a = /\p{H}/;", "strict").as_slice(),
            &["invalidPropertyEscape"]
        );
    }

    #[test]
    fn reports_quantified_assertion() {
        assert_eq!(
            rule_ids_for("const a = /(?!a)+/;", "strict").as_slice(),
            &["quantifiedAssertion"]
        );
    }

    #[test]
    fn ignores_valid_patterns() {
        assert!(rule_ids_for("const a = /\\p{L}/u;", "strict").is_empty());
        assert!(rule_ids_for("const a = /[A--B]/v;", "strict").is_empty());
        assert!(rule_ids_for(r"const a = /\{\}\]/;", "strict").is_empty());
        assert!(rule_ids_for(r"const a = /\x00/;", "strict").is_empty());
        assert!(rule_ids_for(r"const a = /\cA/;", "strict").is_empty());
        assert!(rule_ids_for("const a = /()\\1/;", "strict").is_empty());
        assert!(rule_ids_for("const a = /(?<foo>)\\k<foo>/;", "strict").is_empty());
    }
}

mod no_standalone_backslash {
    use super::*;

    #[test]
    fn reports_standalone_backslash() {
        // `\c` at end of pattern — no control letter follows
        assert_eq!(
            rule_ids_for(r"const a = /\c/;", "no-standalone-backslash").as_slice(),
            &["unexpected"]
        );
        // `\c` followed by a digit — not a valid control-char escape
        assert_eq!(
            rule_ids_for(r"const a = /\c1/;", "no-standalone-backslash").as_slice(),
            &["unexpected"]
        );
        // `\c` followed by `-` — not a letter
        assert_eq!(
            rule_ids_for(r"const a = /\c-/;", "no-standalone-backslash").as_slice(),
            &["unexpected"]
        );
        // `\c` inside a character class before `]`
        assert_eq!(
            rule_ids_for(r"const a = /[\c]/;", "no-standalone-backslash").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_valid_control_char_escapes() {
        // `\cX` where X is an uppercase letter — valid control-char escape
        assert!(rule_ids_for(r"const a = /\cX/;", "no-standalone-backslash").is_empty());
        // `\cA` through `\cZ` in a v-mode class
        assert!(
            rule_ids_for(r"const a = /[[\cA-\cZ]--\cX]/v;", "no-standalone-backslash").is_empty()
        );
        // Lowercase control letters are also valid
        assert!(rule_ids_for(r"const a = /\ca/;", "no-standalone-backslash").is_empty());
    }
}

mod no_useless_backreference {
    use super::*;

    #[test]
    fn reports_forward_only_backreferences() {
        // \1 appears BEFORE the capture group it references.
        assert_eq!(
            rule_ids_for("const a = /\\1(a)/u;", "no-useless-backreference").as_slice(),
            &["unexpected"]
        );
        // \2 references a non-existent second capture.
        assert_eq!(
            rule_ids_for("const a = /(a)\\2/u;", "no-useless-backreference").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_valid_backreferences_and_non_digit_escapes() {
        // Normal backref after group definition.
        assert!(rule_ids_for("const a = /(a)\\1/u;", "no-useless-backreference").is_empty());
        // \d is a class shorthand, not a backref.
        assert!(rule_ids_for("const a = /\\d/u;", "no-useless-backreference").is_empty());
        // No backrefs at all.
        assert!(rule_ids_for("const a = /abc/u;", "no-useless-backreference").is_empty());
    }
}

mod no_dupe_disjunctions {
    use super::*;

    #[test]
    fn reports_duplicate_single_literal_alternatives() {
        assert_eq!(
            rule_ids_for("const a = /(?:a|a)/u;", "no-dupe-disjunctions").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:a|b|b)/u;", "no-dupe-disjunctions").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_distinct_or_unsupported_alternatives() {
        // Distinct single-literal alts.
        assert!(rule_ids_for("const a = /(?:a|b)/u;", "no-dupe-disjunctions").is_empty());
        // No alternation.
        assert!(rule_ids_for("const a = /(?:a)/u;", "no-dupe-disjunctions").is_empty());
        // Multi-byte alt — opt out via all_alts_single_literal=false.
        assert!(rule_ids_for("const a = /(?:abc|abc)/u;", "no-dupe-disjunctions").is_empty());
        // Capturing group.
        assert!(rule_ids_for("const a = /(a|a)/u;", "no-dupe-disjunctions").is_empty());
    }
}

mod optimal_lookaround_quantifier {
    use super::*;

    #[test]
    fn reports_lookarounds_with_always_matching_body() {
        assert_eq!(
            rule_ids_for("const a = /(?=a*)/u;", "optimal-lookaround-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<=b?)/u;", "optimal-lookaround-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?!c*)/u;", "optimal-lookaround-quantifier").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_required_match_bodies_and_non_lookarounds() {
        // `+` requires a non-empty match.
        assert!(rule_ids_for("const a = /(?=a+)/u;", "optimal-lookaround-quantifier").is_empty());
        // Bare atom — no quantifier.
        assert!(rule_ids_for("const a = /(?=a)/u;", "optimal-lookaround-quantifier").is_empty());
        // Multi-byte body — deferred.
        assert!(rule_ids_for("const a = /(?=ab*)/u;", "optimal-lookaround-quantifier").is_empty());
        // Non-cap group is a different rule.
        assert!(rule_ids_for("const a = /(?:a*)/u;", "optimal-lookaround-quantifier").is_empty());
    }
}

mod prefer_predefined_assertion {
    use super::*;

    #[test]
    fn reports_lookarounds_wrapping_anchors() {
        assert_eq!(
            rule_ids_for("const a = /(?=$)/u;", "prefer-predefined-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<=^)/u;", "prefer-predefined-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?!^)/u;", "prefer-predefined-assertion").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_other_lookaround_bodies() {
        // Non-anchor single-char body.
        assert!(rule_ids_for("const a = /(?=a)/u;", "prefer-predefined-assertion").is_empty());
        // Multi-byte body.
        assert!(rule_ids_for("const a = /(?=ab)/u;", "prefer-predefined-assertion").is_empty());
        // Anchor outside a lookaround.
        assert!(rule_ids_for("const a = /^a$/u;", "prefer-predefined-assertion").is_empty());
        // Empty lookaround — owned by no-empty-lookarounds-assertion.
        assert!(rule_ids_for("const a = /(?=)/u;", "prefer-predefined-assertion").is_empty());
    }
}

mod no_trivially_nested_assertion {
    use super::*;

    #[test]
    fn reports_non_cap_wrapping_only_lookaround() {
        assert_eq!(
            rule_ids_for("const a = /(?:(?=a))/u;", "no-trivially-nested-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:(?!b))/u;", "no-trivially-nested-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:(?<=c))/u;", "no-trivially-nested-assertion").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_non_cap_with_other_content_or_unrelated_groups() {
        // Lookaround + extra content — wrapper carries that content.
        assert!(
            rule_ids_for("const a = /(?:(?=a)b)/u;", "no-trivially-nested-assertion").is_empty()
        );
        // Plain literal body — handled by other rules, not this one.
        assert!(rule_ids_for("const a = /(?:a)/u;", "no-trivially-nested-assertion").is_empty());
        // Capturing group — not in scope.
        assert!(rule_ids_for("const a = /((?=a))/u;", "no-trivially-nested-assertion").is_empty());
        // Lookaround alone at top level — not nested.
        assert!(rule_ids_for("const a = /(?=a)/u;", "no-trivially-nested-assertion").is_empty());
    }
}

mod no_extra_lookaround_assertions {
    use super::*;

    #[test]
    fn reports_lookaround_wrapping_only_another_lookaround() {
        assert_eq!(
            rule_ids_for("const a = /(?=(?=a))/u;", "no-extra-lookaround-assertions").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<!(?!b))/u;", "no-extra-lookaround-assertions").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_unrelated_shapes() {
        // Lookaround with literal body — fine.
        assert!(rule_ids_for("const a = /(?=ab)/u;", "no-extra-lookaround-assertions").is_empty());
        // Non-cap wrapping a lookaround is the other rule's job.
        assert!(
            rule_ids_for("const a = /(?:(?=a))/u;", "no-extra-lookaround-assertions").is_empty()
        );
        // Lookaround followed by more content.
        assert!(
            rule_ids_for("const a = /(?=(?=a)b)/u;", "no-extra-lookaround-assertions").is_empty()
        );
    }
}

mod sort_alternatives {
    use super::*;

    #[test]
    fn reports_out_of_order_single_literal_alts() {
        assert_eq!(
            rule_ids_for("const a = /(?:b|a)/u;", "sort-alternatives").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:c|a|b)/u;", "sort-alternatives").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:2|1)/u;", "sort-alternatives").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_sorted_or_unsupported_alts() {
        // Already sorted.
        assert!(rule_ids_for("const a = /(?:a|b)/u;", "sort-alternatives").is_empty());
        assert!(rule_ids_for("const a = /(?:a|b|c)/u;", "sort-alternatives").is_empty());
        // No alternation.
        assert!(rule_ids_for("const a = /(?:a)/u;", "sort-alternatives").is_empty());
        // Multi-byte alt — deferred.
        assert!(rule_ids_for("const a = /(?:bc|a)/u;", "sort-alternatives").is_empty());
        // Escape inside.
        assert!(rule_ids_for("const a = /(?:b|\\d)/u;", "sort-alternatives").is_empty());
        // Capturing group not in scope.
        assert!(rule_ids_for("const a = /(b|a)/u;", "sort-alternatives").is_empty());
    }
}

mod prefer_character_class {
    use super::*;

    #[test]
    fn reports_non_cap_alternation_with_three_or_more_single_literals() {
        // Upstream default minAlternatives is 3; needs ≥ 3 single-char alts.
        assert_eq!(
            rule_ids_for("const a = /(?:a|b|c)/u;", "prefer-character-class").as_slice(),
            &["unexpected"]
        );
        // Digits and letters mix is fine — both alphanumeric.
        assert_eq!(
            rule_ids_for("const a = /(?:a|1|b)/u;", "prefer-character-class").as_slice(),
            &["unexpected"]
        );
        // Four alternatives.
        assert_eq!(
            rule_ids_for("const a = /(?:a|b|c|d)/u;", "prefer-character-class").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_two_alt_non_cap_group() {
        // Exactly 2 alternatives: valid per upstream (minAlternatives default 3).
        assert!(
            rule_ids_for("const a = /(?:a|b)/u;", "prefer-character-class").is_empty(),
            "/(?:a|b)/ must not be flagged (only 2 alternatives)"
        );
    }

    #[test]
    fn ignores_alts_with_multi_byte_or_escapes_or_groups() {
        // Multi-byte alt.
        assert!(rule_ids_for("const a = /(?:a|bc)/u;", "prefer-character-class").is_empty());
        // Escape inside.
        assert!(rule_ids_for("const a = /(?:a|\\d)/u;", "prefer-character-class").is_empty());
        // Class inside.
        assert!(rule_ids_for("const a = /(?:a|[b])/u;", "prefer-character-class").is_empty());
        // No alternation.
        assert!(rule_ids_for("const a = /(?:a)/u;", "prefer-character-class").is_empty());
        // Capturing group.
        assert!(rule_ids_for("const a = /(a|b)/u;", "prefer-character-class").is_empty());
    }
}

mod no_trivially_nested_quantifier {
    use super::*;

    #[test]
    fn reports_non_cap_with_quantified_atom_body_and_outer_quantifier() {
        assert_eq!(
            rule_ids_for("const a = /(?:a+)+/u;", "no-trivially-nested-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:b*)*/u;", "no-trivially-nested-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:c?)+/u;", "no-trivially-nested-quantifier").as_slice(),
            &["unexpected"]
        );
        // Braced outer quantifier also counts.
        assert_eq!(
            rule_ids_for("const a = /(?:a+){2}/u;", "no-trivially-nested-quantifier").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_unrelated_group_shapes() {
        // No outer quantifier — handled by other rules.
        assert!(rule_ids_for("const a = /(?:a+)/u;", "no-trivially-nested-quantifier").is_empty());
        // Inner has no quantifier.
        assert!(rule_ids_for("const a = /(?:a)+/u;", "no-trivially-nested-quantifier").is_empty());
        // Multi-byte body — deferred.
        assert!(
            rule_ids_for("const a = /(?:ab+)+/u;", "no-trivially-nested-quantifier").is_empty()
        );
        // Capturing group.
        assert!(rule_ids_for("const a = /(a+)+/u;", "no-trivially-nested-quantifier").is_empty());
        // Alternation in body.
        assert!(
            rule_ids_for("const a = /(?:a+|b)+/u;", "no-trivially-nested-quantifier").is_empty()
        );
    }
}

mod no_useless_string_literal {
    use super::*;

    #[test]
    fn fires_alongside_grapheme_string_literal_on_single_char_body() {
        assert_eq!(
            rule_ids_for("const a = /[\\q{a}]/v;", "no-useless-string-literal").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_empty_and_multi_character_bodies() {
        assert!(rule_ids_for("const a = /[\\q{}]/v;", "no-useless-string-literal").is_empty());
        assert!(rule_ids_for("const a = /[\\q{ab}]/v;", "no-useless-string-literal").is_empty());
    }
}

mod sort_character_class_elements {
    use super::*;

    #[test]
    fn reports_unsorted_all_literal_classes() {
        assert_eq!(
            rule_ids_for("const a = /[ba]/u;", "sort-character-class-elements").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[cba]/u;", "sort-character-class-elements").as_slice(),
            &["unexpected"]
        );
        // Digits and letters intermixed but still unsorted.
        assert_eq!(
            rule_ids_for("const a = /[b1a]/u;", "sort-character-class-elements").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_sorted_classes_and_classes_with_escapes_or_ranges() {
        assert!(rule_ids_for("const a = /[ab]/u;", "sort-character-class-elements").is_empty());
        assert!(rule_ids_for("const a = /[abc]/u;", "sort-character-class-elements").is_empty());
        // Escape inside class — deferred.
        assert!(rule_ids_for("const a = /[a\\d]/u;", "sort-character-class-elements").is_empty());
        // Range — deferred.
        assert!(rule_ids_for("const a = /[a-z]/u;", "sort-character-class-elements").is_empty());
        // Negated class — deferred.
        assert!(rule_ids_for("const a = /[^ba]/u;", "sort-character-class-elements").is_empty());
    }
}

mod prefer_quantifier {
    use super::*;

    #[test]
    fn reports_single_body_groups_followed_by_quantifier() {
        assert_eq!(
            rule_ids_for("const a = /(?:a){3}/u;", "prefer-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:a)+/u;", "prefer-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:a)*/u;", "prefer-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?:a)?/u;", "prefer-quantifier").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_unrelated_group_shapes() {
        // No quantifier follows — no-useless-non-capturing-group's job.
        assert!(rule_ids_for("const a = /(?:a)/u;", "prefer-quantifier").is_empty());
        // Multi-byte body — `ab{3}` would change semantics, so the wrapper is needed.
        assert!(rule_ids_for("const a = /(?:ab){3}/u;", "prefer-quantifier").is_empty());
        // Capturing group is intentional and outside this rule.
        assert!(rule_ids_for("const a = /(a){3}/u;", "prefer-quantifier").is_empty());
        // Alternation body.
        assert!(rule_ids_for("const a = /(?:a|b){3}/u;", "prefer-quantifier").is_empty());
    }
}

mod no_useless_non_capturing_group {
    use super::*;

    #[test]
    fn reports_single_literal_body_without_quantifier() {
        assert_eq!(
            rule_ids_for("const a = /(?:a)/u;", "no-useless-non-capturing-group").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for(
                "const a = /pre(?:b)post/u;",
                "no-useless-non-capturing-group"
            )
            .as_slice(),
            &["unexpected"]
        );
        // Digits also collapse the same way.
        assert_eq!(
            rule_ids_for("const a = /(?:5)/u;", "no-useless-non-capturing-group").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_multi_atom_alternation_quantifier_and_capture_groups() {
        // Multi-byte body — bare equivalence is not obvious without atom analysis.
        assert!(rule_ids_for("const a = /(?:abc)/u;", "no-useless-non-capturing-group").is_empty());
        // Followed by quantifier — the group is the quantified unit.
        assert!(rule_ids_for("const a = /(?:a)+/u;", "no-useless-non-capturing-group").is_empty());
        assert!(
            rule_ids_for("const a = /(?:a){3}/u;", "no-useless-non-capturing-group").is_empty()
        );
        // Capturing group — not non-capturing.
        assert!(rule_ids_for("const a = /(a)/u;", "no-useless-non-capturing-group").is_empty());
        // Alternation present — not eligible for the narrow form.
        assert!(rule_ids_for("const a = /(?:a|b)/u;", "no-useless-non-capturing-group").is_empty());
        // Escape inside body — deferred.
        assert!(rule_ids_for("const a = /(?:\\d)/u;", "no-useless-non-capturing-group").is_empty());
    }

    #[test]
    fn ignores_groups_whose_removal_would_change_meaning() {
        // Removing (?:) would merge the inner char with preceding bytes into a
        // different token, so the group is not useless.
        for code in [
            "/\\1(?:0)/.test(str)",  // \10 backreference/octal
            "/\\0(?:1)/.test(str)",  // \01 octal
            "/{(?:2)}/.test(str)",   // {2} quantifier
            "/{2,(?:5)}/.test(str)", // {2,5} quantifier
            "/\\c(?:A)/.test(str)",  // \cA control escape
        ] {
            assert!(
                rule_ids_for(code, "no-useless-non-capturing-group").is_empty(),
                "should not flag: {code}"
            );
        }
    }
}

mod grapheme_string_literal {
    use super::*;

    #[test]
    fn reports_single_character_string_literals_in_v_classes() {
        let data = first_data("const a = /[\\q{a}]/v;", "grapheme-string-literal");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\q{a}")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("a")
        );
        // Digits collapse the same way.
        assert_eq!(
            rule_ids_for("const a = /[\\q{5}]/v;", "grapheme-string-literal").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_empty_and_multi_character_string_literals() {
        // Empty string literal is no-empty-string-literal's job.
        assert!(rule_ids_for("const a = /[\\q{}]/v;", "grapheme-string-literal").is_empty());
        // Two-character grapheme — needs grapheme analysis we don't do here.
        assert!(rule_ids_for("const a = /[\\q{ab}]/v;", "grapheme-string-literal").is_empty());
        // Plain character class.
        assert!(rule_ids_for("const a = /[ab]/v;", "grapheme-string-literal").is_empty());
    }
}

mod control_character_escape {
    use super::*;

    #[test]
    fn reports_literal_control_characters() {
        // U+0001 SOH as a literal character inside the regex pattern.
        assert_eq!(
            rule_ids_for(
                "const a = new RegExp('\\x01', 'u');",
                "control-character-escape"
            )
            .as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_escaped_control_character_references() {
        // `\\x01` in the JS source decodes to the four-character regex escape;
        // there is no literal control byte in the pattern, so this rule does
        // not fire (no-control-character still does).
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\x01', 'u');",
                "control-character-escape"
            )
            .is_empty()
        );
        // Plain ASCII pattern.
        assert!(rule_ids_for("const a = /abc/u;", "control-character-escape").is_empty());
    }

    #[test]
    fn ignores_named_escape_chars_in_constructor_args() {
        // The six characters that have well-known named regex escapes (\0 \t \n
        // \v \f \r) are valid when passed as a literal JS string escape to a
        // RegExp constructor. In JS source `'\t'` is a string with a real tab
        // byte (0x09), but the author used a JS escape so it is already
        // "named". Upstream marks all of these as valid.
        for code in ["new RegExp('\t')", "RegExp(\"\0\t\n\x0B\x0C\r\", \"i\")"] {
            assert!(
                rule_ids_for(code, "control-character-escape").is_empty(),
                "expected no diagnostic for: {code:?}"
            );
        }
    }

    #[test]
    fn still_reports_literal_tab_in_regex_literal() {
        // A literal tab inside a regex literal MUST be flagged — the author
        // should write /\t/ instead of /TAB/.
        assert_eq!(
            rule_ids_for("/\t/", "control-character-escape").as_slice(),
            &["unexpected"],
            "literal tab in regex literal must be flagged"
        );
    }
}

mod use_ignore_case {
    use super::*;

    #[test]
    fn reports_case_pair_classes_without_i_flag() {
        // Bare case-pair class: every letter in the pattern is covered by a pair.
        assert_eq!(
            rule_ids_for("const a = /[aA]/u;", "use-ignore-case").as_slice(),
            &["unexpected"]
        );
        // Multi-pair class.
        assert_eq!(
            rule_ids_for("const a = /[aAbB]/u;", "use-ignore-case").as_slice(),
            &["unexpected"]
        );
        // v-mode bare case-pair class.
        assert_eq!(
            rule_ids_for("const a = /[aA]/v;", "use-ignore-case").as_slice(),
            &["unexpected"]
        );
        // Multiple case-pair classes, no bare letters.
        assert_eq!(
            rule_ids_for("const a = /[aA][aA][aA]/u;", "use-ignore-case").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_when_i_flag_is_present_or_pair_absent() {
        // `i` flag is already on — the case pair is intentional or the rule is satisfied.
        assert!(rule_ids_for("const a = /[aA]/iu;", "use-ignore-case").is_empty());
        // Only lowercase or only uppercase — no case pair.
        assert!(rule_ids_for("const a = /[abc]/u;", "use-ignore-case").is_empty());
        // Ranges and escapes are intentionally skipped.
        assert!(rule_ids_for("const a = /[a-z]/u;", "use-ignore-case").is_empty());
        assert!(rule_ids_for("const a = /[\\w]/u;", "use-ignore-case").is_empty());
    }

    #[test]
    fn regression_does_not_fire_when_adding_i_would_change_behavior() {
        // Bare `a` outside the class: adding /i would also make the bare `a`
        // match uppercase `A`, changing the set of strings matched.
        assert!(rule_ids_for("const a = /[aA]a/u;", "use-ignore-case").is_empty());
        assert!(rule_ids_for("const a = /[aA]a/v;", "use-ignore-case").is_empty());
        assert!(rule_ids_for("const a = /[aA]a/;", "use-ignore-case").is_empty());
        // Class with unpaired `b`: adding /i would make `b` match `B` too.
        assert!(rule_ids_for("const a = /[aAb]/u;", "use-ignore-case").is_empty());
        assert!(rule_ids_for("const a = /[aAb]/v;", "use-ignore-case").is_empty());
        assert!(rule_ids_for("const a = /[aAb]/;", "use-ignore-case").is_empty());
        // `\b` (word boundary) in Unicode mode is case-variant: its matching
        // depends on whether adjacent characters are cased, so adding /i changes
        // where the boundary fires.
        assert!(rule_ids_for("const a = /\\b[aA]/u;", "use-ignore-case").is_empty());
        assert!(rule_ids_for("const a = /\\b[aA]/v;", "use-ignore-case").is_empty());
    }
}

mod prefer_escape_replacement_dollar_char {
    use super::*;

    #[test]
    fn reports_dollar_followed_by_invalid_char() {
        assert_eq!(
            rule_ids_for(
                "'str'.replace(/foo/u, 'pre $ post');",
                "prefer-escape-replacement-dollar-char"
            )
            .as_slice(),
            &["unexpected"]
        );
        // Trailing dollar.
        assert_eq!(
            rule_ids_for(
                "'str'.replace(/foo/u, 'price$');",
                "prefer-escape-replacement-dollar-char"
            )
            .as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn accepts_valid_references_and_escaped_dollars() {
        assert!(
            rule_ids_for(
                "'str'.replace(/(a)/u, '$1');",
                "prefer-escape-replacement-dollar-char"
            )
            .is_empty()
        );
        assert!(
            rule_ids_for(
                "'str'.replace(/a/u, '$$');",
                "prefer-escape-replacement-dollar-char"
            )
            .is_empty()
        );
        assert!(
            rule_ids_for(
                "'str'.replace(/a/u, '$&');",
                "prefer-escape-replacement-dollar-char"
            )
            .is_empty()
        );
        // No dollar at all.
        assert!(
            rule_ids_for(
                "'str'.replace(/a/u, 'bar');",
                "prefer-escape-replacement-dollar-char"
            )
            .is_empty()
        );
        // Non-string-literal receiver: should not report.
        assert!(
            rule_ids_for(
                "foo.replace(/./, '$');",
                "prefer-escape-replacement-dollar-char"
            )
            .is_empty()
        );
        // Non-regex first argument: should not report.
        assert!(
            rule_ids_for(
                "'abc'.replace(foo, '$');",
                "prefer-escape-replacement-dollar-char"
            )
            .is_empty()
        );
    }
}

mod no_useless_dollar_replacements {
    use super::*;

    #[test]
    fn reports_useless_dollar_zero_n_when_group_missing() {
        // $03 when pattern has only 2 groups — group 3 does not exist.
        assert_eq!(
            rule_ids_for(
                "str.replace(/(\\w+)\\s(\\w+)/u, '$03');",
                "no-useless-dollar-replacements"
            )
            .as_slice(),
            &["unexpected"]
        );
        // replaceAll variant: $09 in an 8-group pattern.
        assert_eq!(
            rule_ids_for(
                "\"abc\".replaceAll(/()()(()())()()(.)/gu, '$09');",
                "no-useless-dollar-replacements"
            )
            .as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_valid_and_unrelated_replacement_strings() {
        // $0 is always a literal in JS replace — never flag it.
        assert!(
            rule_ids_for(
                "\"abc\".replaceAll(/./gu, '$0');",
                "no-useless-dollar-replacements"
            )
            .is_empty()
        );
        // $0_ (bare $0 not followed by 1-9) — literal, never flag.
        assert!(
            rule_ids_for(
                "\"abc\".replaceAll(/./gu, '$0_');",
                "no-useless-dollar-replacements"
            )
            .is_empty()
        );
        // $09 with 9 capture groups — refers to group 9, which exists.
        assert!(
            rule_ids_for(
                "\"abc\".replaceAll(/()()(()())()()((.))/gu, '$09');",
                "no-useless-dollar-replacements"
            )
            .is_empty()
        );
        // Escaped dollar is intentional.
        assert!(
            rule_ids_for(
                "str.replace(/foo/u, '$$03');",
                "no-useless-dollar-replacements"
            )
            .is_empty()
        );
        // String first argument (not a regex literal) — cannot determine group count.
        assert!(
            rule_ids_for(
                "'abc'.replaceAll('a', '$09');",
                "no-useless-dollar-replacements"
            )
            .is_empty()
        );
        // Method without replacement is not our concern.
        assert!(rule_ids_for("str.match(/foo/u);", "no-useless-dollar-replacements").is_empty());
    }
}

mod no_lazy_ends {
    use super::*;

    // ── used-as-whole-pattern cases (should fire) ─────────────────────────

    #[test]
    fn reports_direct_lazy_quantifiers_at_end_of_pattern() {
        // Direct usage: regex literal is the receiver of a regexp method call.
        assert_eq!(
            rule_ids_for("/a*?/u.test(str)", "no-lazy-ends").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("/a+?/u.test(str)", "no-lazy-ends").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("/a??/u.test(str)", "no-lazy-ends").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("/a{2,}?/u.test(str)", "no-lazy-ends").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn reports_variable_based_lazy_quantifiers() {
        // Variable usage: regex assigned to a non-exported variable that is
        // later used as a regexp object.
        assert_eq!(
            rule_ids_for("const foo = /a*?/u;\nfoo.exec(str)", "no-lazy-ends").as_slice(),
            &["unexpected"]
        );
    }

    // ── not-used-as-whole-pattern cases (must NOT fire) ───────────────────

    #[test]
    fn ignores_bare_literal_without_usage() {
        // `/a??/` — unknown usage, must not fire (upstream valid case).
        assert!(rule_ids_for("/a??/u", "no-lazy-ends").is_empty());
        assert!(rule_ids_for("const a = /a??/u;", "no-lazy-ends").is_empty());
        assert!(rule_ids_for("const a = /a*?/u;", "no-lazy-ends").is_empty());
    }

    #[test]
    fn ignores_fixed_braced_quantifier() {
        // `/a{3}?/.test(str)` — upstream treats `{n}?` (min == max) as
        // "uselessly lazy" but NOT as a `no-lazy-ends` violation.
        assert!(rule_ids_for("/a{3}?/u.test(str)", "no-lazy-ends").is_empty());
    }

    #[test]
    fn ignores_lazy_quantifiers_followed_by_something() {
        // Lazy quantifier not at the end — not a lazy-end.
        assert!(rule_ids_for("/a*?b/u.test(str)", "no-lazy-ends").is_empty());
        // Anchored: `$` follows the lazy quantifier.
        assert!(rule_ids_for("/a*?$/u.test(str)", "no-lazy-ends").is_empty());
        // Greedy quantifier at the end is fine.
        assert!(rule_ids_for("/a*/u.test(str)", "no-lazy-ends").is_empty());
        // Plain greedy braced quantifier.
        assert!(rule_ids_for("/a{2,}/u.test(str)", "no-lazy-ends").is_empty());
    }

    #[test]
    fn regression_bare_vs_direct_usage() {
        // The key regression: same pattern `/a??/` must NOT fire when bare,
        // but MUST fire when used via `.test()`.
        assert!(rule_ids_for("/a??/u", "no-lazy-ends").is_empty());
        assert_eq!(
            rule_ids_for("/a??/u.test(str)", "no-lazy-ends").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_exported_variable_upstream_valid_cases() {
        // Upstream valid case: `/* exported a */` makes the binding exported;
        // even though `a.test(str)` is a whole-pattern usage, the regex should
        // not be flagged because it may be used externally as a partial pattern.
        assert!(
            rule_ids_for(
                "\n/* exported a */\nconst a = /a??/\na.test(str)",
                "no-lazy-ends"
            )
            .is_empty()
        );
    }

    #[test]
    fn upstream_valid_lazy_not_at_end() {
        // From upstream fixture valid[]: lazy quantifier is NOT the last element.
        assert!(rule_ids_for("/a+?b*/.test(str)", "no-lazy-ends").is_empty());
        assert!(rule_ids_for("/a??(?:ba+?|c)*/.test(str)", "no-lazy-ends").is_empty());
        // `$` follows the lazy quantifier — not at effective end.
        assert!(rule_ids_for("/ba*?$/.test(str)", "no-lazy-ends").is_empty());
    }
}

mod no_useless_flag {
    use super::*;

    #[test]
    fn reports_s_flag_on_dotless_patterns() {
        let data = first_data("const a = new RegExp('abc', 's');", "no-useless-flag");
        assert_eq!(data.flag.as_ref().map(CompactString::as_str), Some("s"));
    }

    #[test]
    fn reports_m_flag_on_anchorless_patterns() {
        let data = first_data("const a = new RegExp('abc', 'm');", "no-useless-flag");
        assert_eq!(data.flag.as_ref().map(CompactString::as_str), Some("m"));
    }

    #[test]
    fn accepts_flag_when_pattern_uses_the_corresponding_syntax() {
        // `.` makes the `s` flag meaningful.
        assert!(rule_ids_for("const a = new RegExp('a.b', 's');", "no-useless-flag").is_empty());
        // `^` makes the `m` flag meaningful.
        assert!(rule_ids_for("const a = new RegExp('^abc', 'm');", "no-useless-flag").is_empty());
        // `$` at end activates the `m` flag.
        assert!(rule_ids_for("const a = new RegExp('abc$', 'm');", "no-useless-flag").is_empty());
    }
}

mod no_optional_assertion {
    use super::*;

    #[test]
    fn reports_question_after_each_lookaround_shape() {
        assert_eq!(
            rule_ids_for("const a = /(?=a)?/u;", "no-optional-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?!a)?/u;", "no-optional-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<=a)?/u;", "no-optional-assertion").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /(?<!a)?/u;", "no-optional-assertion").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_assertions_without_quantifier_and_non_assertion_optionals() {
        assert!(rule_ids_for("const a = /(?=a)/u;", "no-optional-assertion").is_empty());
        // `?` after a non-lookaround group must not fire this rule.
        assert!(rule_ids_for("const a = /(?:a)?/u;", "no-optional-assertion").is_empty());
        assert!(rule_ids_for("const a = /(a)?/u;", "no-optional-assertion").is_empty());
    }
}

mod require_unicode_sets_regexp {
    use super::*;

    #[test]
    fn reports_missing_v_flag() {
        assert_eq!(
            rule_ids_for("const a = /a/u;", "require-unicode-sets-regexp").as_slice(),
            &["require"]
        );
        assert_eq!(
            rule_ids_for("const a = /a/;", "require-unicode-sets-regexp").as_slice(),
            &["require"]
        );
        assert_eq!(
            rule_ids_for(
                "const a = new RegExp('a', 'gimsu');",
                "require-unicode-sets-regexp"
            )
            .as_slice(),
            &["require"]
        );
    }

    #[test]
    fn accepts_patterns_with_v_flag() {
        assert!(rule_ids_for("const a = /a/v;", "require-unicode-sets-regexp").is_empty());
        assert!(rule_ids_for("const a = /a/gv;", "require-unicode-sets-regexp").is_empty());
        assert!(
            rule_ids_for(
                "const a = new RegExp('a', 'v');",
                "require-unicode-sets-regexp"
            )
            .is_empty()
        );
    }
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
    // When the flags argument is a non-literal expression the flags cannot be
    // statically determined, so the `u`/`v`-flag rules must stay silent (they
    // would otherwise false-positive on `new RegExp('a', flags)`); upstream
    // skips them in this case.
    assert!(rule_names_for("const a = new RegExp('a', flags);").is_empty());
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

    #[test]
    fn suppresses_named_escape_chars_in_constructor_false_positive() {
        // Regression: new RegExp('\n') delivers a literal newline (U+000A) to
        // the pattern via the JS string escape. Upstream marks this valid because
        // the author expressed it as a named escape, so we must NOT fire here.
        assert!(
            rule_ids_for("const a = new RegExp('\n', 'u');", "no-control-character").is_empty(),
            "constructor newline (U+000A named escape) must not fire"
        );
        // Same for the other named-escape control characters.
        assert!(
            rule_ids_for("const a = new RegExp('\t', 'u');", "no-control-character").is_empty(),
            "constructor tab (U+0009 named escape) must not fire"
        );
        assert!(
            rule_ids_for("const a = new RegExp('\r', 'u');", "no-control-character").is_empty(),
            "constructor carriage-return (U+000D named escape) must not fire"
        );
        // Regex literals with a raw control character MUST still fire.
        assert!(
            rule_ids_for("const a = /a\x01b/u;", "no-control-character").contains(&"unexpected"),
            "regex literal raw SOH (U+0001) must still fire"
        );
        // Constructor with a hex-escaped non-named control char still fires.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\x01', 'u');",
                "no-control-character"
            )
            .contains(&"unexpected"),
            "constructor hex escape \\x01 must still fire"
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

    #[test]
    fn skips_constructor_with_non_literal_flags_arg() {
        // When the flags argument is a non-literal expression (identifier,
        // binary expression, member expression, etc.) we cannot statically
        // determine the flags, so the rule must not fire.
        assert!(
            rule_ids_for("new RegExp('', flags)", "require-unicode-regexp").is_empty(),
            "free variable flags should not be flagged"
        );
        assert!(
            rule_ids_for("new RegExp('', flags + 'u')", "require-unicode-regexp").is_empty(),
            "binary expression flags should not be flagged"
        );
        assert!(
            rule_ids_for("new RegExp('foo', flags[3])", "require-unicode-regexp").is_empty(),
            "member expression flags should not be flagged"
        );
        assert!(
            rule_ids_for(
                "function f(flags) { return new RegExp('', flags) }",
                "require-unicode-regexp"
            )
            .is_empty(),
            "parameter variable flags should not be flagged"
        );
        // RegExp call (non-new) with non-literal flags also skipped.
        assert!(
            rule_ids_for("RegExp('foo', flags)", "require-unicode-regexp").is_empty(),
            "call expression with non-literal flags should not be flagged"
        );
    }

    #[test]
    fn ignores_shadowed_regexp_identifier() {
        // When `RegExp` is shadowed by a local binding (e.g. a function
        // parameter), the constructor call no longer refers to the global
        // `RegExp` and must NOT trigger require-unicode-regexp or
        // require-unicode-sets-regexp.
        assert!(
            rule_ids_for(
                "function f(RegExp) { return new RegExp('foo') }",
                "require-unicode-regexp"
            )
            .is_empty(),
            "shadowed RegExp parameter must not fire require-unicode-regexp"
        );
        assert!(
            rule_ids_for(
                "function f(RegExp) { return new RegExp('foo') }",
                "require-unicode-sets-regexp"
            )
            .is_empty(),
            "shadowed RegExp parameter must not fire require-unicode-sets-regexp"
        );
        // But the global RegExp (no shadowing) still fires.
        assert!(
            !rule_ids_for("new RegExp('foo')", "require-unicode-regexp").is_empty(),
            "global RegExp without flags must still fire require-unicode-regexp"
        );
        assert!(
            !rule_ids_for("new RegExp('foo')", "require-unicode-sets-regexp").is_empty(),
            "global RegExp without flags must still fire require-unicode-sets-regexp"
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
        // `[\s\S]` is the canonical recommended form — it is valid.
        assert!(rule_ids_for("const a = /[\\s\\S]/u;", "match-any").is_empty());
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
        // The canonical `[\s\S]` is the recommended form; only the reverse and other families are flagged.
        assert!(rule_ids_for("const a = /[\\s\\S]/u;", "match-any").is_empty());
    }

    #[test]
    fn reversed_order_still_reports() {
        assert_eq!(
            rule_ids_for("const a = /[\\S\\s]/u;", "match-any").as_slice(),
            &["unexpected"]
        );
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

    #[test]
    fn suppresses_named_escape_chars_in_constructor_false_positive() {
        // Regression: new RegExp('\t') delivers a literal tab (U+0009) to the
        // pattern via the JS string escape. Upstream marks this valid because the
        // author expressed it as a named escape, so we must NOT fire here.
        assert!(
            rule_ids_for("const a = new RegExp('\t', 'u');", "no-invisible-character").is_empty(),
            "constructor tab (U+0009 named escape) must not fire"
        );
        // Other named-escape characters in the invisible set (\v U+000B, \f U+000C).
        assert!(
            rule_ids_for(
                "const a = new RegExp('\x0b', 'u');",
                "no-invisible-character"
            )
            .is_empty(),
            "constructor vertical-tab (U+000B named escape) must not fire"
        );
        assert!(
            rule_ids_for(
                "const a = new RegExp('\x0c', 'u');",
                "no-invisible-character"
            )
            .is_empty(),
            "constructor form-feed (U+000C named escape) must not fire"
        );
        // Regex literals with a raw (literal) tab MUST still fire.
        assert!(
            rule_ids_for("const a = /a\tb/u;", "no-invisible-character").contains(&"unexpected"),
            "regex literal with raw tab (U+0009) must still fire"
        );
        // The suppression only covers named escapes; a literal NBSP (U+00A0) in a
        // constructor pattern string still fires (NBSP is not in the named-escape set).
        assert!(
            rule_ids_for("const a = new RegExp('a', 'u');", "no-invisible-character").is_empty(),
            "plain constructor with no invisible char must not fire"
        );
    }
}

mod hexadecimal_escape {
    use super::*;

    #[test]
    fn reports_unicode_escapes_with_hex_replacement() {
        // \uHHHH form whose code point is ≤ 0xFF is flagged and suggests \xHH.
        let data = first_data("const a = /\\u000a/u;", "hexadecimal-escape");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\u000a")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\x0a")
        );

        // \u{H+} form with code point ≤ 0xFF is also flagged.
        let data = first_data(
            "const a = new RegExp('\\\\u{00000a}', 'u');",
            "hexadecimal-escape",
        );
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\u{00000a}")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\x0a")
        );

        // «» (U+00AB, ≤ 0xFF) represented as \uHHHH is flagged → \xab.
        let data = first_data("const a = /\\u00ab/u;", "hexadecimal-escape");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\u00ab")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\xab")
        );
    }

    #[test]
    fn ignores_hex_x_escapes_and_high_code_points_and_unrelated_escapes() {
        // \xHH is already in the correct form — not flagged.
        assert!(rule_ids_for("const a = /\\xab/u;", "hexadecimal-escape").is_empty());
        // \uHHHH with code point > 0xFF — not representable as \xHH, not flagged.
        assert!(rule_ids_for("const a = /\\uabcd/u;", "hexadecimal-escape").is_empty());
        // \u{100} is code point 256 > 0xFF — not flagged.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\u{100}', 'u');",
                "hexadecimal-escape"
            )
            .is_empty()
        );
        // Non-escape characters are never flagged.
        assert!(rule_ids_for("const a = /\\d/u;", "hexadecimal-escape").is_empty());
        // \xHH pattern from the upstream valid fixture — no diagnostic.
        assert!(
            rule_ids_for(
                "const a = /a \\x0a \\cM \\0 \u{0100} \\u{100}/u;",
                "hexadecimal-escape"
            )
            .is_empty()
        );
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

    #[test]
    fn ignores_surrogate_halves() {
        // Surrogate halves belong to `prefer-unicode-codepoint-escapes`, not here.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\ud83d\\\\ude00', 'u');",
                "unicode-escape"
            )
            .is_empty()
        );
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

mod prefer_regexp_test {
    use super::*;

    #[test]
    fn reports_exec_in_if_test() {
        // `pattern.exec(text)` directly in `if (...)` — boolean context.
        assert_eq!(
            rule_ids_for(
                "const pattern = /thing/; const text = 'something'; if (pattern.exec(text)) {}",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
    }

    #[test]
    fn reports_match_with_regexp_literal_in_if_test() {
        // `text.match(/pattern/)` directly in `if (...)` — boolean context.
        assert_eq!(
            rule_ids_for(
                "const text = 'something'; if (text.match(/thing/)) {}",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
    }

    #[test]
    fn reports_exec_under_negation() {
        // `!re.exec(str)` — `!` places operand in boolean context.
        assert_eq!(
            rule_ids_for(
                "const re = /a/; const s = 'abc'; const b = !re.exec(s);",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
    }

    #[test]
    fn reports_exec_under_boolean_call() {
        assert_eq!(
            rule_ids_for(
                "const re = /a/; const s = 'abc'; const b = Boolean(re.exec(s));",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
    }

    #[test]
    fn reports_exec_in_while_and_ternary() {
        // `while (re.exec(s)) {}`
        assert_eq!(
            rule_ids_for(
                "const re = /a/; const s = 'abc'; while (re.exec(s)) {}",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
        // Ternary test position.
        assert_eq!(
            rule_ids_for(
                "const re = /a/; const s = 'abc'; const x = re.exec(s) ? 1 : 0;",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
    }

    #[test]
    fn ignores_exec_when_result_is_used_as_value() {
        // Result stored in a variable — not a boolean context.
        assert!(
            rule_ids_for(
                "const re = /a/u; const s = 'abc'; const m = re.exec(s);",
                "prefer-regexp-test",
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_string_receiver_calling_exec() {
        // `text.exec(pattern)` — receiver is a known string; skip (wrong receiver type).
        assert!(
            rule_ids_for(
                "const text = 'something'; const pattern = /thing/; if (text.exec(pattern)) {}",
                "prefer-regexp-test",
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_match_with_global_flag() {
        // `.match(/g/)` with `g` flag returns array — not equivalent to `.test()`.
        assert!(
            rule_ids_for(
                "const text = 'something'; if (text.match(/thing/g)) {}",
                "prefer-regexp-test",
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_match_when_receiver_is_not_known_string() {
        // `pattern.match(text)` — receiver is not a known string (it is a regexp
        // literal variable), so we conservatively skip.
        assert!(
            rule_ids_for(
                "const pattern = /thing/; const text = 'something'; if (pattern.match(text)) {}",
                "prefer-regexp-test",
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_loose_null_comparison() {
        // `!= null` (loose) is NOT a strict boolean context this rule detects.
        assert!(rule_ids_for(
            "const text = 'something'; const pattern = /thing/; const d = text.match(pattern) != null;",
            "prefer-regexp-test",
        )
        .is_empty());
    }

    #[test]
    fn reports_strict_null_comparison() {
        // `=== null` and `!== null` are boolean contexts.
        assert_eq!(
            rule_ids_for(
                "const text = 'something'; const a = text.match(/thing/) === null;",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
        assert_eq!(
            rule_ids_for(
                "const text = 'something'; const b = text.match(/thing/) !== null;",
                "prefer-regexp-test",
            )
            .as_slice(),
            &["disallow"]
        );
    }
}

mod no_missing_g_flag {
    use super::*;

    #[test]
    fn reports_match_all_and_replace_all_without_g() {
        // String-literal receiver → must report.
        let data = first_data("'abc'.matchAll(/foo/u);", "no-missing-g-flag");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("matchAll")
        );
        let data = first_data("'abc'.replaceAll(/foo/, 'bar');", "no-missing-g-flag");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("replaceAll")
        );
        // `const s = 'foo'` → s is a known string → must report.
        let data = first_data(
            "const s = 'foo'; s.replaceAll(/foo/, 'bar');",
            "no-missing-g-flag",
        );
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("replaceAll")
        );
        // `const s = 'foo'` → s is a known string → must report for matchAll.
        let data = first_data("const s = 'foo'; s.matchAll(/foo/);", "no-missing-g-flag");
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("matchAll")
        );
    }

    #[test]
    fn ignores_global_regexps_and_unrelated_calls() {
        // Global flag already present.
        assert!(rule_ids_for("'abc'.matchAll(/foo/g);", "no-missing-g-flag").is_empty());
        assert!(rule_ids_for("'abc'.replaceAll(/foo/gu, 'bar');", "no-missing-g-flag").is_empty());
        // Non-literal argument — flags cannot be determined statically.
        assert!(rule_ids_for("'abc'.matchAll(pattern);", "no-missing-g-flag").is_empty());
        // Unrelated method.
        assert!(rule_ids_for("'abc'.match(/foo/u);", "no-missing-g-flag").is_empty());
        // `replaceAll` accepts a string as its first argument; we must not
        // false-positive when the call is not regex-based.
        assert!(rule_ids_for("'abc'.replaceAll('foo', 'bar');", "no-missing-g-flag").is_empty());
    }

    #[test]
    fn ignores_unknown_receiver() {
        // A free/unresolved variable is not a known string — must NOT report.
        assert!(rule_ids_for("unknown.replaceAll(/foo/, 'bar');", "no-missing-g-flag").is_empty());
        assert!(rule_ids_for("unknown.matchAll(/foo/);", "no-missing-g-flag").is_empty());
        // A call result is also not a known string.
        assert!(rule_ids_for("getStr().replaceAll(/foo/, 'bar');", "no-missing-g-flag").is_empty());
        // A member expression receiver is not a known string.
        assert!(rule_ids_for("obj.str.replaceAll(/foo/, 'bar');", "no-missing-g-flag").is_empty());
    }

    #[test]
    fn reports_string_literal_and_const_string_receivers() {
        // Regression: string-literal receiver fires.
        assert!(!rule_ids_for("'abc'.replaceAll(/foo/, 'bar');", "no-missing-g-flag").is_empty());
        // Regression: const-string receiver fires.
        assert!(
            !rule_ids_for("const s = 'abc'; s.matchAll(/foo/);", "no-missing-g-flag").is_empty()
        );
        // Regression: free receiver does NOT fire.
        assert!(rule_ids_for("unknown.replaceAll(/foo/, 'bar');", "no-missing-g-flag").is_empty());
    }
}

mod no_useless_character_class {
    use super::*;

    #[test]
    fn reports_single_literal_classes() {
        let data = first_data("const a = /[a]/u;", "no-useless-character-class");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("[a]"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("a")
        );
        assert_eq!(
            rule_ids_for("const a = /[5]/u;", "no-useless-character-class").as_slice(),
            &["unexpected"]
        );
        // Even followed by a quantifier the class is equivalent to the bare char.
        assert_eq!(
            rule_ids_for("const a = /[a]+/u;", "no-useless-character-class").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_negation_escape_and_multi_element_classes() {
        assert!(rule_ids_for("const a = /[^a]/u;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /[\\d]/u;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /[ab]/u;", "no-useless-character-class").is_empty());
        // `[-]` is technically one literal but `-` carries range meaning;
        // we intentionally skip it.
        assert!(rule_ids_for("const a = /[-]/u;", "no-useless-character-class").is_empty());
    }

    #[test]
    fn ignores_equals_sign_class() {
        // `[=]` is exempted upstream to avoid confusion with POSIX equivalence classes.
        assert!(rule_ids_for("const a = /[=]/;", "no-useless-character-class").is_empty());
    }

    #[test]
    fn ignores_classes_that_disambiguate_adjacent_tokens() {
        // `\1[0]` — removing brackets makes `\10` (different back-reference).
        assert!(rule_ids_for("const a = /\\1[0]/;", "no-useless-character-class").is_empty());
        // `\0[1]` — removing brackets makes `\01` (octal escape).
        assert!(rule_ids_for("const a = /\\0[1]/;", "no-useless-character-class").is_empty());
        // `{[0]}` / `{123[0]}` — bracket is inside a quantifier body.
        assert!(rule_ids_for("const a = /a{[0]}/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /a{123[0]}/;", "no-useless-character-class").is_empty());
        // `\c[M]` — removing brackets produces `\cM` (control-M escape).
        assert!(rule_ids_for("const a = /\\c[M]/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\c[A]/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\c[Z]/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\c[m]/;", "no-useless-character-class").is_empty());
        // `\xF[F]` — removing brackets completes `\xFF` hex escape.
        assert!(rule_ids_for("const a = /\\xF[F]/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\xf[f]/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\x4[4]/;", "no-useless-character-class").is_empty());
        // `\uF[F]FF` — removing brackets supplies the second digit of `￿`.
        assert!(rule_ids_for("const a = /\\uF[F]FF/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\uf[f]ff/;", "no-useless-character-class").is_empty());
        assert!(rule_ids_for("const a = /\\u4[4]44/;", "no-useless-character-class").is_empty());
    }
}

mod no_empty_string_literal {
    use super::*;

    #[test]
    fn reports_empty_v_mode_string_literal() {
        assert_eq!(
            rule_ids_for("const a = /[\\q{}]/v;", "no-empty-string-literal").as_slice(),
            &["unexpected"]
        );
        // Non-empty string literals are not flagged.
        assert!(rule_ids_for("const a = /[\\q{ab}]/v;", "no-empty-string-literal").is_empty());
    }

    #[test]
    fn ignores_unrelated_braced_constructs() {
        // `\u{...}` is unrelated.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\u{41}', 'u');",
                "no-empty-string-literal"
            )
            .is_empty()
        );
        // `{}` outside a `\q` context is not the empty string literal.
        assert!(rule_ids_for("const a = /a{}/u;", "no-empty-string-literal").is_empty());
    }
}

mod confusing_quantifier {
    use super::*;

    #[test]
    fn reports_lazy_zero_min_quantifiers() {
        assert_eq!(
            rule_ids_for("const a = /a*?/u;", "confusing-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /a??/u;", "confusing-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /a{0,}?/u;", "confusing-quantifier").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /a{0,1}?/u;", "confusing-quantifier").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_greedy_quantifiers_and_lazy_one_or_more() {
        assert!(rule_ids_for("const a = /a*/u;", "confusing-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a+/u;", "confusing-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a?/u;", "confusing-quantifier").is_empty());
        // Lazy with non-zero min — not flagged.
        assert!(rule_ids_for("const a = /a+?/u;", "confusing-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{1,}?/u;", "confusing-quantifier").is_empty());
    }
}

mod prefer_named_replacement {
    use super::*;

    #[test]
    fn reports_numbered_backreference_with_named_capture_pattern() {
        // String-literal receiver + $N referring to a named group → fires.
        assert_eq!(
            rule_ids_for(
                "\"s\".replace(/(?<year>\\d{4})/u, '$1');",
                "prefer-named-replacement"
            )
            .as_slice(),
            &["unexpected"]
        );
        // `replaceAll` shares the same shape.
        assert_eq!(
            rule_ids_for(
                "\"s\".replaceAll(/(?<year>\\d{4})/gu, 'year: $1');",
                "prefer-named-replacement"
            )
            .as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_named_replacement_unnamed_regex_and_unrelated_calls() {
        // Named replacement form — no diagnostic.
        assert!(
            rule_ids_for(
                "\"s\".replace(/(?<year>\\d{4})/u, '$<year>');",
                "prefer-named-replacement"
            )
            .is_empty()
        );
        // Regex has no named capture, so `$1` is the only way to refer back.
        assert!(
            rule_ids_for(
                "\"s\".replace(/(\\d{4})/u, '$1');",
                "prefer-named-replacement"
            )
            .is_empty()
        );
        // Escaped dollar must not count as a numeric backreference.
        assert!(
            rule_ids_for(
                "\"s\".replace(/(?<year>\\d{4})/u, '$$1');",
                "prefer-named-replacement"
            )
            .is_empty()
        );
        // Unrelated method.
        assert!(
            rule_ids_for(
                "\"s\".match(/(?<year>\\d{4})/u);",
                "prefer-named-replacement"
            )
            .is_empty()
        );
    }

    #[test]
    fn regression_unnamed_group_not_flagged() {
        // $1 refers to group 1 = (a) which is UNNAMED → must NOT fire.
        assert!(
            rule_ids_for(
                "\"str\".replace(/(a)(?<foo>b)c/, '_$1_');",
                "prefer-named-replacement"
            )
            .is_empty()
        );
        // Free/unknown receiver → must NOT fire regardless of group type.
        assert!(
            rule_ids_for(
                "unknown.replace(/a(?<foo>b)c/, '_$1_');",
                "prefer-named-replacement"
            )
            .is_empty()
        );
        assert!(
            rule_ids_for(
                "unknown.replaceAll(/a(?<foo>b)c/, '_$1_');",
                "prefer-named-replacement"
            )
            .is_empty()
        );
        // String-literal receiver + $N pointing at a NAMED group → fires.
        assert_eq!(
            rule_ids_for(
                "\"s\".replace(/(?<foo>b)/, '$1');",
                "prefer-named-replacement"
            )
            .as_slice(),
            &["unexpected"]
        );
    }
}

mod no_obscure_range {
    use super::*;

    #[test]
    fn reports_boundary_crossing_ranges() {
        let data = first_data("const a = /[A-z]/u;", "no-obscure-range");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("A-z"));
        assert_eq!(
            rule_ids_for("const a = /[0-A]/u;", "no-obscure-range").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_canonical_within_category_ranges() {
        assert!(rule_ids_for("const a = /[a-z]/u;", "no-obscure-range").is_empty());
        assert!(rule_ids_for("const a = /[A-Z]/u;", "no-obscure-range").is_empty());
        assert!(rule_ids_for("const a = /[0-9]/u;", "no-obscure-range").is_empty());
        // Escaped endpoints are skipped — we cannot easily reason about them.
        assert!(rule_ids_for("const a = /[\\x41-z]/u;", "no-obscure-range").is_empty());
    }
}

mod prefer_unicode_codepoint_escapes {
    use super::*;

    #[test]
    fn reports_surrogate_pairs() {
        let data = first_data(
            "const a = new RegExp('\\\\uD83D\\\\uDE00', 'u');",
            "prefer-unicode-codepoint-escapes",
        );
        assert_eq!(
            data.expr.as_ref().map(CompactString::as_str),
            Some("\\uD83D\\uDE00")
        );
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("\\u{1f600}")
        );
    }

    #[test]
    fn ignores_non_surrogate_pairs_and_codepoint_escapes() {
        // Two BMP unicode escapes are unrelated.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\u0041\\\\u0042', 'u');",
                "prefer-unicode-codepoint-escapes"
            )
            .is_empty()
        );
        // Already the codepoint form.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\u{1F600}', 'u');",
                "prefer-unicode-codepoint-escapes"
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_surrogate_pair_without_unicode_flag() {
        // Without u/v flag, \uHHHH pairs are two separate code units, not a
        // surrogate pair — upstream does not flag them.
        assert!(
            rule_ids_for(
                "const a = new RegExp('\\\\uD83D\\\\uDE00');",
                "prefer-unicode-codepoint-escapes"
            )
            .is_empty()
        );
    }

    #[test]
    fn still_reports_with_u_flag() {
        // With the u flag the pair IS a surrogate pair and must be flagged.
        assert_eq!(
            rule_ids_for(
                "const a = new RegExp('\\\\uD83D\\\\uDE00', 'u');",
                "prefer-unicode-codepoint-escapes"
            )
            .as_slice(),
            &["unexpected"]
        );
    }
}

mod no_dupe_characters_character_class {
    use super::*;

    #[test]
    fn reports_duplicate_literal_characters() {
        let data = first_data("const a = /[aab]/u;", "no-dupe-characters-character-class");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("a"));
        // Reordered or with surrounding chars still reports.
        assert_eq!(
            rule_ids_for("const a = /[xaya]/u;", "no-dupe-characters-character-class").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_unique_literals_ranges_and_escapes() {
        assert!(
            rule_ids_for("const a = /[abc]/u;", "no-dupe-characters-character-class").is_empty()
        );
        // Range `a-c` is not a duplicate of `a`.
        assert!(
            rule_ids_for("const a = /[a-c]/u;", "no-dupe-characters-character-class").is_empty()
        );
        // Escapes are skipped; `\\d\\d` is not flagged.
        assert!(
            rule_ids_for(
                "const a = /[\\d\\d]/u;",
                "no-dupe-characters-character-class"
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_v_mode_string_disjunctions() {
        // Multiple \q{...} constructs must not be seen as duplicate `{` literals.
        assert!(
            rule_ids_for(
                "const a = /[\\q{a}\\q{ab}\\q{abc}]/v;",
                "no-dupe-characters-character-class"
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_v_mode_nested_class_set_operations() {
        // Nested [...] in v-mode are set operands; their inner bytes must not be
        // treated as outer-class members (false-positive regression).
        assert!(
            rule_ids_for(
                "const a = /[\\q{a}\\q{ab}\\q{abc}[\\w--[ab]][\\w&&b]]/v;",
                "no-dupe-characters-character-class"
            )
            .is_empty(),
            "nested v-mode classes must not produce a false duplicate"
        );
        // Flat classes with real duplicates must still be reported.
        assert!(
            !rule_ids_for("const a = /[aab]/u;", "no-dupe-characters-character-class").is_empty(),
            "flat /[aab]/u must still report a duplicate"
        );
    }

    #[test]
    fn non_v_literal_open_bracket_parsed_flat() {
        // In non-v mode an unescaped `[` inside a class is a literal character,
        // not the start of a nested class. The class `[[]` (pattern `[[]`) ends
        // at the first unescaped `]` and must not produce a false positive for
        // `no-empty-character-class` (regression: `find_class_end_nested` would
        // see the inner `[` as opening a nested class and return `None` for the
        // supposed outer class, causing the scanner to advance byte-by-byte
        // past the `]` and treat the remaining `]` as an empty class).
        assert!(
            rule_ids_for("const a = /[[]/u;", "no-empty-character-class").is_empty(),
            "/[[]/u must not trigger no-empty-character-class"
        );
        // `/[a[b]/u` — class contains `a`, `[`, `b`; no duplicates.
        assert!(
            rule_ids_for("const a = /[a[b]/u;", "no-dupe-characters-character-class").is_empty(),
            "/[a[b]/u must not trigger no-dupe-characters-character-class"
        );
        // Sanity: a real duplicate in non-v mode is still caught.
        assert!(
            !rule_ids_for("const a = /[aab]/u;", "no-dupe-characters-character-class").is_empty(),
            "/[aab]/u must still report a duplicate"
        );
    }
}

mod prefer_range {
    use super::*;

    #[test]
    fn reports_four_or_more_consecutive_literals() {
        let data = first_data("const a = /[abcd]/u;", "prefer-range");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("abcd"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some("a-d")
        );
        // Digits collapse just like letters.
        assert_eq!(
            rule_ids_for("const a = /[12345]/u;", "prefer-range").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_short_runs_and_existing_ranges() {
        assert!(rule_ids_for("const a = /[ab]/u;", "prefer-range").is_empty());
        // A run of three is left alone — upstream only collapses runs of four
        // or more (`[abc]` is valid, `[abcd]` is not).
        assert!(rule_ids_for("const a = /[abc]/u;", "prefer-range").is_empty());
        // A range already covers the chars; no further reduction needed.
        assert!(rule_ids_for("const a = /[a-c]/u;", "prefer-range").is_empty());
        // Non-consecutive bytes break the run.
        assert!(rule_ids_for("const a = /[acd]/u;", "prefer-range").is_empty());
    }
}

mod no_useless_escape {
    use super::*;

    #[test]
    fn reports_pointless_escapes_outside_classes() {
        let data = first_data("const a = /\\:/u;", "no-useless-escape");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("\\:"));
        assert_eq!(
            data.replacement.as_ref().map(CompactString::as_str),
            Some(":")
        );
        // Other punctuation variants.
        assert_eq!(
            rule_ids_for("const a = /a\\@b/u;", "no-useless-escape").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /\\#/u;", "no-useless-escape").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_known_escape_sequences_and_class_contents() {
        // Real escapes are untouched.
        assert!(rule_ids_for("const a = /\\d/u;", "no-useless-escape").is_empty());
        assert!(rule_ids_for("const a = /\\b/u;", "no-useless-escape").is_empty());
        assert!(rule_ids_for("const a = /\\./u;", "no-useless-escape").is_empty());
        // Inside a character class — deferred to keep the check sound.
        assert!(rule_ids_for("const a = /[\\:]/u;", "no-useless-escape").is_empty());
        // Escaping the literal delimiter is required, not useless.
        assert!(rule_ids_for("const a = /\\//u;", "no-useless-escape").is_empty());
    }
}

mod no_useless_quantifier {
    use super::*;

    #[test]
    fn reports_one_braced_quantifiers() {
        let data = first_data("const a = /a{1}/u;", "no-useless-quantifier");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("{1}"));
        let data = first_data("const a = /a{1,1}/u;", "no-useless-quantifier");
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("{1,1}"));
    }

    #[test]
    fn ignores_other_quantifiers_and_class_contexts() {
        assert!(rule_ids_for("const a = /a{2}/u;", "no-useless-quantifier").is_empty());
        assert!(rule_ids_for("const a = /a{1,3}/u;", "no-useless-quantifier").is_empty());
        // `{1}` inside a class is literal characters, not a quantifier.
        assert!(rule_ids_for("const a = /[{1}]/u;", "no-useless-quantifier").is_empty());
    }
}

mod prefer_named_backreference {
    use super::*;

    #[test]
    fn reports_numbered_backreference_alongside_named_capture() {
        let data = first_data(
            "const a = /(?<year>\\d{4})-\\1/u;",
            "prefer-named-backreference",
        );
        assert_eq!(data.expr.as_ref().map(CompactString::as_str), Some("\\1"));
    }

    #[test]
    fn ignores_numbered_backref_without_named_group_and_class_contents() {
        // No named capture in the pattern — \1 is the only way to refer back.
        assert!(
            rule_ids_for("const a = /(\\d{4})-\\1/u;", "prefer-named-backreference").is_empty()
        );
        // \1 inside a character class is literal, not a backreference.
        assert!(
            rule_ids_for("const a = /(?<n>a)[\\1b]/u;", "prefer-named-backreference").is_empty()
        );
        // \1 refers to an unnamed group (group 1); group 2 is named — must NOT flag.
        assert!(
            rule_ids_for(
                "const a = /(a)\\1 (?<foo>a)\\k<foo>/;",
                "prefer-named-backreference"
            )
            .is_empty()
        );
    }
}

mod negation {
    use super::*;

    #[test]
    fn reports_negated_shorthand_classes() {
        // All six shorthand letters: one explicit assertion per case to keep
        // the test loop free of `format!` (disallowed by the perf policy).
        assert_eq!(
            rule_ids_for("const a = /[^\\d]/u;", "negation").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[^\\D]/u;", "negation").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[^\\s]/u;", "negation").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[^\\S]/u;", "negation").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[^\\w]/u;", "negation").as_slice(),
            &["unexpected"]
        );
        assert_eq!(
            rule_ids_for("const a = /[^\\W]/u;", "negation").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_unsupported_classes() {
        // Non-negated shorthand stays silent (`[\\d]` is the useless-class case).
        assert!(rule_ids_for("const a = /[\\d]/u;", "negation").is_empty());
        // Negated body is a literal, not a shorthand.
        assert!(rule_ids_for("const a = /[^a]/u;", "negation").is_empty());
        // Multi-element body — not a clean single replacement.
        assert!(rule_ids_for("const a = /[^\\d\\s]/u;", "negation").is_empty());
        // Unrelated escape letter inside negated class.
        assert!(rule_ids_for("const a = /[^\\b]/u;", "negation").is_empty());
        // No character class at all.
        assert!(rule_ids_for("const a = /abc/u;", "negation").is_empty());
    }
}

mod no_useless_lazy {
    use super::*;

    #[test]
    fn reports_fixed_count_lazy_quantifiers() {
        // `{n}?` form.
        assert_eq!(
            rule_ids_for("const a = /a{3}?/u;", "no-useless-lazy").as_slice(),
            &["unexpected"]
        );
        // `{n,n}?` form with the same min and max.
        assert_eq!(
            rule_ids_for("const a = /a{2,2}?/u;", "no-useless-lazy").as_slice(),
            &["unexpected"]
        );
        // `{0}?` — degenerate fixed count but still a useless lazy
        // modifier on a fixed-count quantifier.
        assert_eq!(
            rule_ids_for("const a = /a{0}?/u;", "no-useless-lazy").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_non_fixed_count_lazy_quantifiers() {
        // Different min and max — lazy is meaningful.
        assert!(rule_ids_for("const a = /a{2,5}?/u;", "no-useless-lazy").is_empty());
        // `{0,}?` is a lazy star — not fixed-count.
        assert!(rule_ids_for("const a = /a{0,}?/u;", "no-useless-lazy").is_empty());
        // Plain greedy fixed-count quantifier — no `?`.
        assert!(rule_ids_for("const a = /a{3}/u;", "no-useless-lazy").is_empty());
        // `*?`, `+?`, `??` are non-brace lazy quantifiers and are
        // intentionally not flagged by the narrow port.
        assert!(rule_ids_for("const a = /a*?/u;", "no-useless-lazy").is_empty());
        assert!(rule_ids_for("const a = /a+?/u;", "no-useless-lazy").is_empty());
    }
}

mod no_misleading_unicode_character {
    use super::*;

    #[test]
    fn reports_classes_containing_zwj() {
        // ZWJ-joined family emoji inside a character class. The U+200D bytes
        // `0xE2 0x80 0x8D` appear inside the class body, so the engine
        // matches the ZWJ as a separate atom and the grapheme cluster cannot
        // be matched as a unit.
        assert_eq!(
            rule_ids_for("const a = /[👨‍👩‍👦]/u;", "no-misleading-unicode-character").as_slice(),
            &["unexpected"]
        );
        // Bare ZWJ in a class is also misleading.
        assert_eq!(
            rule_ids_for("const a = /[‍]/u;", "no-misleading-unicode-character").as_slice(),
            &["unexpected"]
        );
    }

    #[test]
    fn ignores_classes_without_zwj_bytes() {
        // ASCII-only class.
        assert!(rule_ids_for("const a = /[abc]/u;", "no-misleading-unicode-character").is_empty());
        // Single-codepoint emoji without ZWJ.
        assert!(rule_ids_for("const a = /[😀]/u;", "no-misleading-unicode-character").is_empty());
        // ZWJ outside any class — not flagged by the narrow port.
        assert!(rule_ids_for("const a = /a‍b/u;", "no-misleading-unicode-character").is_empty());
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
    // The `u`-only flags trigger `require-unicode-sets-regexp` once per literal
    // alongside the pattern-specific diagnostics.
    assert_eq!(
        rule_names_for("const a = /[]/u; const b = /a|/u;").as_slice(),
        &[
            "require-unicode-sets-regexp",
            "no-empty-character-class",
            "require-unicode-sets-regexp",
            "no-empty-alternative",
        ]
    );
}
