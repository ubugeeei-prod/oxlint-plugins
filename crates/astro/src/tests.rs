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

fn apply_fix(source: &str, diagnostic: &Diagnostic) -> CompactString {
    let fix = diagnostic.fix.as_ref().expect("diagnostic fix");
    let mut output = CompactString::new(&source[..fix.start as usize]);
    output.push_str(&fix.replacement);
    output.push_str(&source[fix.end as usize..]);
    output
}

#[test]
fn exposes_the_slice_rule_names() {
    assert_eq!(
        implemented_astro_rule_names(),
        [
            "no-deprecated-astro-canonicalurl",
            "no-deprecated-astro-fetchcontent",
            "no-deprecated-astro-resolve",
            "no-deprecated-getentrybyslug",
            "no-set-html-directive",
            "no-set-text-directive",
            "prefer-class-list-directive",
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
            replacement: "glob".into(),
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
fn replays_upstream_resolve_template_fixture() {
    let source = "---\nconst { animal } = Astro.props;\n---\n\n{/* ✗ BAD */}\n<img src={Astro.resolve(`../images/${animal}.png`)} />";
    let diagnostics = scan_rule(source, "no-deprecated-astro-resolve");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "deprecated");
    assert_eq!(diagnostics[0].loc.start_line, 6);
    assert_eq!(diagnostics[0].loc.start_column, 10);
}

#[test]
fn accepts_upstream_resolve_replacement() {
    assert!(
        scan_rule(
            "---\nconst { animal } = Astro.props;\n---\n\n{/* ✓ GOOD */}\n<img src={await import(`../images/${animal}.png`)} />",
            "no-deprecated-astro-resolve",
        )
        .is_empty()
    );
}

#[test]
fn frontmatter_astro_binding_shadows_template_references() {
    assert!(
        scan_rule(
            "---\nconst Astro = { resolve() {} }\n---\n<div>{Astro.resolve(\"local\")}</div>",
            "no-deprecated-astro-resolve",
        )
        .is_empty()
    );
}

#[test]
fn reports_set_html_names_for_expression_and_template_attributes() {
    let diagnostics = scan_rule(
        "<p set:html={html}></p>\n<p set:html=`<strong>${html}</strong>`></p>",
        "no-set-html-directive",
    );
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.message_id == "unexpected" && diagnostic.loc.start_column == 3
    }));
}

#[test]
fn replays_set_text_normal_and_self_closing_fixes() {
    let normal = "---\nlet string = `text`\n---\n\n<p set:text={string}></p>\n";
    let normal_diagnostic = scan_rule(normal, "no-set-text-directive");
    assert_eq!(
        apply_fix(normal, &normal_diagnostic[0]),
        "---\nlet string = `text`\n---\n\n<p >{string}</p>\n"
    );

    let self_closing = "---\nlet string = `text`\n---\n\n<p set:text={string} />\n";
    let self_closing_diagnostic = scan_rule(self_closing, "no-set-text-directive");
    assert_eq!(
        apply_fix(self_closing, &self_closing_diagnostic[0]),
        "---\nlet string = `text`\n---\n\n<p  >{string}</p>\n"
    );
}

#[test]
fn replays_set_text_template_attribute_fix() {
    let source = "<p set:text=`text`></p>\n";
    let diagnostics = scan_rule(source, "no-set-text-directive");
    assert_eq!(apply_fix(source, &diagnostics[0]), "<p >{`text`}</p>\n");
}

#[test]
fn set_text_reports_without_fix_for_boolean_void_or_nonempty_children() {
    for source in [
        "<p set:text ></p>",
        "<input set:text={text}>",
        "<div set:text={text}>child</div>",
        "<div set:text={text}><!-- comment --></div>",
    ] {
        let diagnostics = scan_rule(source, "no-set-text-directive");
        assert_eq!(diagnostics.len(), 1, "{source}");
        assert!(diagnostics[0].fix.is_none(), "{source}");
    }
}

#[test]
fn set_text_whitespace_body_fix_is_idempotent() {
    let source = "<div set:text={text}>\n  \n</div>";
    let diagnostics = scan_rule(source, "no-set-text-directive");
    let fixed = apply_fix(source, &diagnostics[0]);
    assert_eq!(fixed, "<div >{text}</div>");
    assert!(scan_rule(&fixed, "no-set-text-directive").is_empty());
}

#[test]
fn replays_prefer_class_list_expression_and_template_fixes() {
    for (source, expected) in [
        ("<div class={foo}></div>", "<div class:list={foo}></div>"),
        (
            "<div class=`${foo}`></div>",
            "<div class:list=`${foo}`></div>",
        ),
        ("<div class=`foo`></div>", "<div class:list=`foo`></div>"),
        ("<div {class}></div>", "<div class:list={class}></div>"),
    ] {
        let diagnostics = scan_rule(source, "prefer-class-list-directive");
        assert_eq!(diagnostics.len(), 1, "{source}");
        let fixed = apply_fix(source, &diagnostics[0]);
        assert_eq!(fixed, expected, "{source}");
        assert!(
            scan_rule(&fixed, "prefer-class-list-directive").is_empty(),
            "{source}"
        );
    }
}

#[test]
fn prefer_class_list_ignores_static_and_existing_directive_attributes() {
    assert!(
        scan_rule(
            "<div class=\"foo\" class:list={foo}></div>",
            "prefer-class-list-directive",
        )
        .is_empty()
    );
}

#[test]
fn template_expression_segmenter_handles_nested_braces_and_template_literals() {
    let source = "<div>{condition ? { image: Astro.resolve(`./${name}.png`) } : null}</div>";
    let diagnostics = scan_rule(source, "no-deprecated-astro-resolve");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        &source[diagnostics[0].start as usize..diagnostics[0].end as usize],
        "Astro.resolve"
    );
}

#[test]
fn template_expression_segmenter_ignores_comments_strings_and_plain_text() {
    for source in [
        "<!-- {Astro.resolve('comment')} -->",
        "{/* Astro.resolve('comment') */}",
        "<p>Astro.resolve('text')</p>",
        "<p>{'Astro.resolve(\\'string\\')'}</p>",
    ] {
        assert!(
            scan_rule(source, "no-deprecated-astro-resolve").is_empty(),
            "{source}"
        );
    }
}

#[test]
fn template_expression_semantics_ignore_a_local_shadow() {
    assert!(
        scan_rule(
            "<div>{items.map((Astro) => Astro.resolve('local'))}</div>",
            "no-deprecated-astro-resolve",
        )
        .is_empty()
    );
}

#[test]
fn template_expression_reports_computed_resolve_access() {
    let diagnostics = scan_rule(
        "<img src={Astro['resolve']('./image.png')} />",
        "no-deprecated-astro-resolve",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn template_attribute_parser_handles_delimiters_inside_values() {
    let diagnostics = scan_rule(
        "<p title=\">\" data={{ nested: '>' }} set:html=`<strong>${html}</strong>`></p>",
        "no-set-html-directive",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].loc.start_column, 36);
}

#[test]
fn template_locations_remain_utf16_after_non_bmp_text() {
    let diagnostics = scan_rule(
        "<p title=\"😀\" set:html={html}></p>",
        "no-set-html-directive",
    );
    assert_eq!(diagnostics[0].loc.start_column, 14);
}

#[test]
fn template_fixes_keep_utf8_byte_ranges_after_non_ascii_text() {
    let source = "<p title=\"日本語\" class={klass}></p>";
    let diagnostics = scan_rule(source, "prefer-class-list-directive");
    let insertion = source.find("class").expect("class attribute") + "class".len();
    assert_eq!(
        diagnostics[0].fix.as_ref().expect("class fix").start,
        source[..insertion].len() as u32
    );
    assert_eq!(
        apply_fix(source, &diagnostics[0]),
        "<p title=\"日本語\" class:list={klass}></p>"
    );
}

#[test]
fn set_text_fix_preserves_crlf_around_the_element() {
    let source = "<section>\r\n<p set:text={text}>\r\n\t</p>\r\n</section>";
    let diagnostics = scan_rule(source, "no-set-text-directive");
    assert_eq!(
        apply_fix(source, &diagnostics[0]),
        "<section>\r\n<p >{text}</p>\r\n</section>"
    );
}

#[test]
fn template_diagnostics_remain_in_source_order_across_elements() {
    let diagnostics = scan(
        "<p set:text={text}></p>\n<div class={klass}></div>\n<section set:html={html}></section>",
    );
    assert_eq!(
        names(&diagnostics).as_slice(),
        [
            "no-set-text-directive",
            "prefer-class-list-directive",
            "no-set-html-directive",
        ]
    );
}

#[test]
fn malformed_template_segments_fail_closed_without_hiding_prior_elements() {
    let diagnostics = scan_rule(
        "<p set:html={html}></p>\n<div>{Astro.resolve({ broken: true)</div>",
        "no-set-html-directive",
    );
    assert_eq!(diagnostics.len(), 1);
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
    assert_eq!(
        diagnostics[0].fix.as_ref().expect("fix").start,
        property as u32
    );
}

#[test]
fn applied_fetchcontent_fix_is_idempotent() {
    let source = "---\nconst emoji = \"😀\"; Astro.fetchContent(\"*.md\")\n---";
    let first = scan(source);
    let fix = first[0].fix.as_ref().expect("first scan fix");
    let mut fixed = CompactString::new(&source[..fix.start as usize]);
    fixed.push_str(&fix.replacement);
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
fn scans_template_expressions_without_frontmatter() {
    assert_eq!(
        names(&scan("<p>{Astro.canonicalURL}</p>")).as_slice(),
        ["no-deprecated-astro-canonicalurl"]
    );
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
    assert_eq!(
        diagnostics[0].fix.as_ref().expect("fix").start,
        property as u32
    );
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
    let source = "---\nimport { getEntryBySlug } from \"astro:content\"\nAstro.fetchContent(\"*.md\")\nAstro.canonicalURL\n---\n<div set:html={html} set:text={text} class={klass}>{Astro.resolve(\"asset\")}</div>";
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
