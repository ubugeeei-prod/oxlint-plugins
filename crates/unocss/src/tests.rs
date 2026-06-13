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
