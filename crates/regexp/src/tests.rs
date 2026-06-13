use oxlint_plugins_carton::SmallVec;

use crate::{implemented_regexp_rule_names, scan_regexp};

fn ids(source: &str) -> SmallVec<[(&'static str, &'static str); 8]> {
    scan_regexp(source, "fixture.js")
        .into_iter()
        .map(|diagnostic| (diagnostic.rule_name, diagnostic.message_id))
        .collect()
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
fn scans_literal_pattern_rules() {
    assert_eq!(
        ids("const a = /[]/u;").as_slice(),
        &[("no-empty-character-class", "empty")]
    );
    assert_eq!(ids("const a = /()/u;").len(), 2);
    assert_eq!(
        ids("const a = /a|/u;").as_slice(),
        &[("no-empty-alternative", "empty")]
    );
    assert_eq!(
        ids("const a = /a{0}/u;").as_slice(),
        &[("no-zero-quantifier", "unexpected")]
    );
}

#[test]
fn scans_constructor_patterns_and_flags() {
    assert_eq!(
        ids("const a = new RegExp('[]', 'u');").as_slice(),
        &[("no-empty-character-class", "empty")]
    );
    assert_eq!(
        ids("const a = new RegExp('[', 'u');").as_slice(),
        &[("no-invalid-regexp", "error")]
    );
    assert_eq!(
        ids("const a = new RegExp('a', 'gg');").as_slice(),
        &[("no-invalid-regexp", "duplicateFlag")]
    );
    assert_eq!(
        ids("const a = RegExp('a', 'vu');").as_slice(),
        &[("no-invalid-regexp", "uvFlag")]
    );
}

#[test]
fn scans_style_and_legacy_rules() {
    assert_eq!(
        ids("const a = /a/mi;").as_slice(),
        &[
            ("sort-flags", "sortFlags"),
            ("require-unicode-regexp", "require"),
        ]
    );
    assert_eq!(
        ids("const a = /\\07/u;").as_slice(),
        &[("no-octal", "unexpected")]
    );
    assert_eq!(
        ids("const a = new RegExp('\\u{1}');").as_slice(),
        &[
            ("require-unicode-regexp", "require"),
            ("no-control-character", "unexpected"),
        ]
    );
}
