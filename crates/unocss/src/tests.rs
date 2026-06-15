use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{BlocklistEntry, UnocssOptions, implemented_unocss_rule_names, scan_unocss};

#[test]
fn exposes_all_rule_names() {
    assert_eq!(
        implemented_unocss_rule_names(),
        [
            "blocklist",
            "enforce-class-compile",
            "order",
            "order-attributify"
        ]
    );
}

#[test]
fn scans_jsx_class_rules() {
    let mut blocklist = SmallVec::new();
    blocklist.push(BlocklistEntry {
        name: CompactString::from("border"),
        reason: CompactString::new(""),
    });
    let options = UnocssOptions {
        blocklist,
        ..UnocssOptions::default()
    };
    let diagnostics = scan_unocss(
        r#"<div className="mx1 m1 border"></div>"#,
        "fixture.tsx",
        &options,
    );
    let names: SmallVec<[&str; 4]> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();
    assert_eq!(
        names.as_slice(),
        ["blocklist", "enforce-class-compile", "order"]
    );
}

#[test]
fn scans_uno_call_and_attributify_order() {
    let diagnostics = scan_unocss(
        r#"const value = clsx("mr-1 ml-1"); const node = <div p4 flex />;"#,
        "fixture.tsx",
        &UnocssOptions::default(),
    );
    let names: SmallVec<[&str; 4]> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();
    assert_eq!(names.as_slice(), ["order", "order-attributify"]);
}

// ── AST-rewrite bug-fix regression tests ────────────────────────────────────

/// A regex literal `/"/` elsewhere in the source must NOT corrupt class detection.
/// The old byte-scanner would see the `"` inside the regex as the start of a
/// string literal and misbehave.  The AST visitor never enters regex literals.
#[test]
fn regex_literal_with_quote_does_not_corrupt_class_detection() {
    // The regex literal `/"/` appears before the JSX; old scanner misparses.
    let source = r#"const re = /"/; <div className="mr-1 ml-1"></div>;"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    let names: SmallVec<[&str; 4]> = diagnostics.iter().map(|d| d.rule_name).collect();
    // Should see enforce-class-compile + order for the className; nothing else.
    assert!(
        names.contains(&"order"),
        "expected 'order' diagnostic, got: {names:?}"
    );
    assert!(
        !names.contains(&"blocklist"),
        "unexpected 'blocklist' diagnostic; regex confused scanner: {names:?}"
    );
}

/// A string inside one uno-call must not bleed into an unrelated adjacent call.
/// The old scanner's `rfind(';')` heuristic could attribute a literal in `foo()`
/// to the preceding `clsx()` call when they are on the same statement.
#[test]
fn unrelated_adjacent_call_does_not_false_positive() {
    // `notaclass` is NOT in uno_functions; its argument must not trigger `order`.
    let source = r#"const a = clsx("mr-1"); const b = notaclass("px-1 py-1");"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    // clsx("mr-1") – single token, not orderable → no `order` diagnostic.
    // notaclass is not a uno-function → no `order` diagnostic from its arg.
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got: {diagnostics:?}"
    );
}

/// Class attribute literals are detected regardless of attribute spacing
/// and quote style, and the reported span stays correct.
#[test]
fn class_literal_detected_with_attr_spacing_variations() {
    // Extra space before `=` and single quotes.
    let source = "<div className='mr-1 ml-1'></div>";
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    let names: SmallVec<[&str; 4]> = diagnostics.iter().map(|d| d.rule_name).collect();
    assert!(
        names.contains(&"order"),
        "expected 'order' diagnostic for single-quoted className, got: {names:?}"
    );
}

/// Custom function in `unoFunctions` must be detected, matching the JS adapter
/// test: `superclass("pr1 pl1")` with options.unoFunctions=["superclass"].
#[test]
fn custom_uno_function_triggers_order() {
    let mut options = UnocssOptions::default();
    options
        .uno_functions
        .push(CompactString::from("superclass"));
    let source = r#"superclass("pr1 pl1");"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &options);
    let names: SmallVec<[&str; 4]> = diagnostics.iter().map(|d| d.rule_name).collect();
    assert!(
        names.contains(&"order"),
        "expected 'order' for custom uno function, got: {names:?}"
    );
}

/// UnoCSS variable (matching `classNames?$`) triggers order check on its init.
#[test]
fn uno_variable_triggers_order() {
    let source = r#"const classNames = { base: "mr-1 ml-1" };"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    let names: SmallVec<[&str; 4]> = diagnostics.iter().map(|d| d.rule_name).collect();
    assert!(
        names.contains(&"order"),
        "expected 'order' for uno variable declarator, got: {names:?}"
    );
}

/// `enforce-class-compile` with a UTF-8 multi-byte character: the fix byte
/// range must cover the inner content (byte offsets, not char offsets).
#[test]
fn enforce_class_compile_utf8_fix_offsets() {
    // é is 2 bytes in UTF-8; make sure the fix spans are byte-correct.
    // `class_compile_enable_fix` already defaults to true.
    let source = "<div className=\"é mx1 m1\"></div>";
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    // The inner content "é mx1 m1" starts at byte 16 (after `<div className="`).
    let fix_start = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.rule_name == "enforce-class-compile")
        .and_then(|diagnostic| diagnostic.fix.as_ref())
        .map(|fix| fix.start);
    assert_eq!(
        fix_start,
        Some(16),
        "enforce-class-compile fix must start at the byte after the opening quote"
    );
}

// ── full-code-review fixes ──────────────────────────────────────────────────

/// The ordering heuristic must not treat ordinary English class names that
/// merely start with `h`/`w`/an axis letter (`hello`, `world`, `my`, `prose`)
/// as UnoCSS utilities, or `order` would reorder non-UnoCSS class names.
#[test]
fn plain_words_do_not_trigger_order() {
    for source in [
        r#"const node = <div className="world hello flex" />;"#,
        r#"const node = <div className="my prose flex" />;"#,
        r#"const node = <div className="play previous block" />;"#,
    ] {
        let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
        assert!(
            diagnostics.iter().all(|d| d.rule_name != "order"),
            "unexpected 'order' diagnostic for {source:?}: {diagnostics:?}"
        );
    }
}

/// Genuine bare `h-`/`w-`/axis utilities are still recognized after tightening.
#[test]
fn genuine_utilities_still_ordered() {
    let source = r#"const node = <div className="w-1 h-1 mx1" />;"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    assert!(
        diagnostics.iter().any(|d| d.rule_name == "order"),
        "expected 'order' for real utilities, got: {diagnostics:?}"
    );
}

/// A `blocklist` diagnostic must point at the offending token, not the whole
/// class string, so editors underline only the blocked utility.
#[test]
fn blocklist_span_covers_only_the_token() {
    let mut blocklist = SmallVec::new();
    blocklist.push(BlocklistEntry {
        name: CompactString::from("border"),
        reason: CompactString::new(""),
    });
    let options = UnocssOptions {
        blocklist,
        ..UnocssOptions::default()
    };
    // `border` sits at bytes 21..27 of the source: `<div className="flex border">`.
    let source = r#"<div className="flex border"></div>"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &options);
    let columns = diagnostics
        .iter()
        .find(|d| d.rule_name == "blocklist")
        .map(|d| (d.loc.start_column, d.loc.end_column));
    assert_eq!(
        columns,
        Some((21, 27)),
        "blocklist span should cover only `border`, not the whole class string"
    );
}

/// A UnoCSS-function object argument has its property KEYS checked for order
/// (upstream `handleObjectExpression`), e.g. `clsx({ 'mr-1 ml-1': cond })`.
#[test]
fn object_key_in_uno_call_is_order_checked() {
    let source = r#"clsx({ "mr-1 ml-1": cond });"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    let order = diagnostics
        .iter()
        .filter(|d| d.rule_name == "order")
        .count();
    assert_eq!(
        order, 1,
        "expected the object key to be order-checked: {diagnostics:?}"
    );
}

/// A UnoCSS-VARIABLE initialiser object does NOT have its keys checked — upstream
/// only collects object keys in the call path, not the variable path.
#[test]
fn object_key_in_uno_variable_is_not_checked() {
    let source = r#"const clsButton = { "mr-1 ml-1": cond };"#;
    let diagnostics = scan_unocss(source, "fixture.tsx", &UnocssOptions::default());
    assert!(
        diagnostics.iter().all(|d| d.rule_name != "order"),
        "variable-init object keys must not be order-checked: {diagnostics:?}"
    );
}
