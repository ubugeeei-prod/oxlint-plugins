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
