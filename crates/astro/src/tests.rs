use oxlint_plugins_carton::{CompactString, SmallVec};

use super::{
    AstroOptions, Diagnostic, DiagnosticFix, DiagnosticLoc, implemented_astro_rule_names,
    scan_astro,
};

fn scan(source: &str) -> SmallVec<[Diagnostic; 8]> {
    scan_astro(source, "fixture.astro", &AstroOptions::default())
}

fn scan_rule(source: &str, rule_name: &str) -> SmallVec<[Diagnostic; 8]> {
    scan_astro(
        source,
        "fixture.astro",
        &AstroOptions {
            rule_names: [CompactString::from(rule_name)].into_iter().collect(),
            frontmatter_only: false,
        },
    )
}

fn names(diagnostics: &[Diagnostic]) -> SmallVec<[&str; 8]> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect()
}

#[test]
fn exposes_the_slice_rule_names() {
    assert_eq!(
        implemented_astro_rule_names(),
        [
            "no-deprecated-astro-canonicalurl",
            "no-deprecated-astro-fetchcontent",
            "no-deprecated-getentrybyslug",
        ]
    );
}

#[test]
fn replays_upstream_canonicalurl_invalid_fixture() {
    let diagnostics = scan(
        "---\n/* ✗ BAD */\nconst canonicalURL = Astro.canonicalURL\nconsole.log(canonicalURL)\n---",
    );
    assert_eq!(
        names(&diagnostics).as_slice(),
        ["no-deprecated-astro-canonicalurl"]
    );
    assert_eq!(
        diagnostics[0].loc,
        DiagnosticLoc {
            start_line: 3,
            start_column: 21,
            end_line: 3,
            end_column: 39,
        }
    );
}

#[test]
fn replays_upstream_canonicalurl_valid_fixture() {
    assert!(
        scan("---\n/* ✓ GOOD */\nconst canonicalURL = new URL(Astro.url.pathname, Astro.site)\nconsole.log(canonicalURL)\n---")
            .is_empty()
    );
}

#[test]
fn replays_upstream_fetchcontent_invalid_fixture_and_fix() {
    let source =
        "---\n/* ✗ BAD */\nconst posts = await Astro.fetchContent(\"../pages/post/*.md\")\n---";
    let diagnostics = scan(source);
    assert_eq!(
        names(&diagnostics).as_slice(),
        ["no-deprecated-astro-fetchcontent"]
    );
    assert_eq!(
        diagnostics[0].loc,
        DiagnosticLoc {
            start_line: 3,
            start_column: 20,
            end_line: 3,
            end_column: 38,
        }
    );
    let property = source.find("fetchContent").expect("fixture property");
    assert_eq!(
        diagnostics[0].fix,
        Some(DiagnosticFix {
            start: property as u32,
            end: (property + "fetchContent".len()) as u32,
            replacement: "glob",
        })
    );
}

#[test]
fn replays_upstream_fetchcontent_valid_fixture() {
    assert!(
        scan("---\n/* ✓ GOOD */\nconst posts = await Astro.glob(\"../pages/post/*.md\")\n---")
            .is_empty()
    );
}

#[test]
fn replays_upstream_getentrybyslug_invalid_fixture() {
    let diagnostics =
        scan("---\n/* ✗ BAD */\nimport { getEntryBySlug } from \"astro:content\"\n---");
    assert_eq!(
        names(&diagnostics).as_slice(),
        ["no-deprecated-getentrybyslug"]
    );
    assert_eq!(
        diagnostics[0].loc,
        DiagnosticLoc {
            start_line: 3,
            start_column: 9,
            end_line: 3,
            end_column: 23,
        }
    );
}

#[test]
fn replays_upstream_getentry_valid_fixture() {
    assert!(scan("---\n/* ✓ GOOD */\nimport { getEntry } from \"astro:content\"\n---").is_empty());
}

#[test]
fn reports_all_three_rules_in_source_order() {
    let diagnostics = scan(
        "---\nimport { getEntryBySlug } from \"astro:content\"\nAstro.fetchContent(\"*.md\")\nAstro.canonicalURL\n---",
    );
    assert_eq!(
        names(&diagnostics).as_slice(),
        [
            "no-deprecated-getentrybyslug",
            "no-deprecated-astro-fetchcontent",
            "no-deprecated-astro-canonicalurl",
        ]
    );
}

#[test]
fn reports_every_global_reference() {
    let diagnostics =
        scan("---\nAstro.canonicalURL\nAstro.canonicalURL\nAstro.fetchContent(\"*.md\")\n---");
    assert_eq!(diagnostics.len(), 3);
}

#[test]
fn ignores_a_shadowing_const() {
    assert!(
        scan("---\nconst Astro = { canonicalURL: \"local\", fetchContent() {} }\nAstro.canonicalURL\nAstro.fetchContent()\n---")
            .is_empty()
    );
}

#[test]
fn ignores_a_shadowing_import() {
    assert!(
        scan("---\nimport Astro from \"./astro\"\nAstro.canonicalURL\nAstro.fetchContent()\n---")
            .is_empty()
    );
}

#[test]
fn ignores_a_shadowing_parameter() {
    assert!(scan("---\nfunction render(Astro: any) { return Astro.canonicalURL }\n---").is_empty());
}

#[test]
fn reports_global_astro_in_a_nested_scope() {
    assert_eq!(
        names(&scan(
            "---\nfunction render() { return Astro.canonicalURL }\n---"
        ))
        .as_slice(),
        ["no-deprecated-astro-canonicalurl"]
    );
}

#[test]
fn supports_computed_string_properties_without_a_fix() {
    let diagnostics = scan("---\nAstro[\"canonicalURL\"]\nAstro['fetchContent'](\"*.md\")\n---");
    assert_eq!(
        names(&diagnostics).as_slice(),
        [
            "no-deprecated-astro-canonicalurl",
            "no-deprecated-astro-fetchcontent",
        ]
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.fix.is_none())
    );
}

#[test]
fn ignores_dynamic_computed_properties() {
    assert!(scan("---\nconst key = \"canonicalURL\"\nAstro[key]\n---").is_empty());
}

#[test]
fn reports_optional_static_access() {
    assert_eq!(
        names(&scan("---\nAstro?.fetchContent(\"*.md\")\n---")).as_slice(),
        ["no-deprecated-astro-fetchcontent"]
    );
}

#[test]
fn reports_parenthesized_global_access() {
    assert_eq!(
        names(&scan("---\n(Astro).canonicalURL\n---")).as_slice(),
        ["no-deprecated-astro-canonicalurl"]
    );
}

#[test]
fn maps_utf16_columns_after_non_bmp_text() {
    let diagnostics = scan("---\nconst label = \"😀\"; Astro.canonicalURL\n---");
    assert_eq!(diagnostics[0].loc.start_line, 2);
    assert_eq!(diagnostics[0].loc.start_column, 20);
}

#[test]
fn maps_utf16_columns_after_cjk_text() {
    let diagnostics = scan("---\nconst 日本語 = 1; Astro.canonicalURL\n---");
    assert_eq!(diagnostics[0].loc.start_column, 15);
}

#[test]
fn keeps_native_fix_ranges_as_utf8_bytes() {
    let source = "---\nconst emoji = \"😀\"; Astro.fetchContent(\"*.md\")\n---";
    let diagnostics = scan(source);
    let property = source.find("fetchContent").expect("fixture property");
    assert_eq!(diagnostics[0].fix.expect("fix").start, property as u32);
}

#[test]
fn applied_fetchcontent_fix_is_idempotent() {
    let source = "---\nconst emoji = \"😀\"; Astro.fetchContent(\"*.md\")\n---";
    let first = scan(source);
    let fix = first[0].fix.expect("first scan fix");
    let mut fixed = CompactString::new(&source[..fix.start as usize]);
    fixed.push_str(fix.replacement);
    fixed.push_str(&source[fix.end as usize..]);
    assert_eq!(
        fixed,
        "---\nconst emoji = \"😀\"; Astro.glob(\"*.md\")\n---"
    );
    assert!(scan(&fixed).is_empty());
}

#[test]
fn accepts_a_utf8_bom() {
    let diagnostics = scan("\u{feff}---\nAstro.canonicalURL\n---");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn accepts_crlf_delimiters_and_locations() {
    let diagnostics = scan("---\r\nAstro.canonicalURL\r\n---\r\n");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn accepts_cr_delimiters_and_locations() {
    let diagnostics = scan("---\rAstro.canonicalURL\r---\r");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn accepts_unicode_line_separators() {
    for separator in ['\u{2028}', '\u{2029}'] {
        let mut source = CompactString::new("---");
        source.push(separator);
        source.push_str("Astro.canonicalURL");
        source.push(separator);
        source.push_str("---");
        let diagnostics = scan(&source);
        assert_eq!(diagnostics[0].loc.start_line, 2);
    }
}

#[test]
fn accepts_trailing_horizontal_space_on_delimiters() {
    assert_eq!(scan("--- \nAstro.canonicalURL\n---\t").len(), 1);
}

#[test]
fn ignores_source_without_frontmatter() {
    assert!(scan("<p>{Astro.canonicalURL}</p>").is_empty());
}

#[test]
fn accepts_an_empty_frontmatter_block() {
    assert!(scan("---\n---\n<p>Hello</p>").is_empty());
}

#[test]
fn scans_an_already_extracted_frontmatter_segment_for_oxlint() {
    let diagnostics = scan_astro(
        "Astro.canonicalURL\n",
        "fixture.astro",
        &AstroOptions {
            rule_names: SmallVec::new(),
            frontmatter_only: true,
        },
    );
    assert_eq!(
        names(&diagnostics).as_slice(),
        ["no-deprecated-astro-canonicalurl"]
    );
    assert_eq!(diagnostics[0].loc.start_line, 1);
    assert_eq!(diagnostics[0].loc.start_column, 0);
}

#[test]
fn extracted_frontmatter_fix_ranges_start_at_the_virtual_source() {
    let source = "const emoji = \"😀\"; Astro.fetchContent(\"*.md\")\n";
    let diagnostics = scan_astro(
        source,
        "fixture.astro",
        &AstroOptions {
            rule_names: SmallVec::new(),
            frontmatter_only: true,
        },
    );
    let property = source.find("fetchContent").expect("fixture property");
    assert_eq!(diagnostics[0].fix.expect("fix").start, property as u32);
}

#[test]
fn reports_multiple_diagnostics_at_the_frontmatter_boundaries() {
    let diagnostics = scan(
        "---\nAstro.fetchContent(\"*.md\"); Astro.canonicalURL\nAstro.fetchContent(\"*.md\")\n---\n<p>template</p>",
    );
    assert_eq!(
        names(&diagnostics).as_slice(),
        [
            "no-deprecated-astro-fetchcontent",
            "no-deprecated-astro-canonicalurl",
            "no-deprecated-astro-fetchcontent",
        ]
    );
    assert_eq!(diagnostics[0].loc.start_line, 2);
    assert_eq!(diagnostics[2].loc.start_line, 3);
}

#[test]
fn ignores_a_nonleading_delimiter() {
    assert!(scan("\n---\nAstro.canonicalURL\n---").is_empty());
}

#[test]
fn ignores_an_unterminated_frontmatter_block() {
    assert!(scan("---\nAstro.canonicalURL\n").is_empty());
}

#[test]
fn ignores_an_opening_delimiter_without_a_line_break() {
    assert!(scan("---").is_empty());
}

#[test]
fn ignores_malformed_typescript() {
    assert!(scan("---\nconst = Astro.canonicalURL\n---").is_empty());
}

#[test]
fn ignores_non_astro_extensions_case_insensitively_except_astro() {
    let options = AstroOptions::default();
    assert!(scan_astro("---\nAstro.canonicalURL\n---", "fixture.ts", &options).is_empty());
    assert_eq!(
        scan_astro("---\nAstro.canonicalURL\n---", "fixture.ASTRO", &options).len(),
        1
    );
}

#[test]
fn isolates_each_selected_rule() {
    let source = "---\nimport { getEntryBySlug } from \"astro:content\"\nAstro.fetchContent(\"*.md\")\nAstro.canonicalURL\n---";
    for rule_name in implemented_astro_rule_names() {
        assert_eq!(
            names(&scan_rule(source, rule_name)).as_slice(),
            [*rule_name]
        );
    }
}

#[test]
fn unknown_rule_selection_reports_nothing() {
    assert!(scan_rule("---\nAstro.canonicalURL\n---", "not-an-astro-rule").is_empty());
}

#[test]
fn supports_aliased_getentrybyslug_imports() {
    assert_eq!(
        names(&scan(
            "---\nimport { getEntryBySlug as legacy } from \"astro:content\"\n---"
        ))
        .as_slice(),
        ["no-deprecated-getentrybyslug"]
    );
}

#[test]
fn reports_type_only_getentrybyslug_like_upstream() {
    assert_eq!(
        names(&scan(
            "---\nimport { type getEntryBySlug } from \"astro:content\"\n---"
        ))
        .as_slice(),
        ["no-deprecated-getentrybyslug"]
    );
}

#[test]
fn ignores_getentrybyslug_from_another_module() {
    assert!(scan("---\nimport { getEntryBySlug } from \"./content\"\n---").is_empty());
}

#[test]
fn ignores_default_and_namespace_imports() {
    assert!(
        scan("---\nimport getEntryBySlug from \"astro:content\"\nimport * as getEntryBySlug2 from \"astro:content\"\n---")
            .is_empty()
    );
}

#[test]
fn ignores_string_named_imports_like_upstream() {
    assert!(
        scan("---\nimport { \"getEntryBySlug\" as legacy } from \"astro:content\"\n---").is_empty()
    );
}

#[test]
fn scans_only_the_first_frontmatter_block() {
    let diagnostics = scan("---\nAstro.canonicalURL\n---\n---\nAstro.fetchContent(\"*.md\")\n---");
    assert_eq!(
        names(&diagnostics).as_slice(),
        ["no-deprecated-astro-canonicalurl"]
    );
}
