use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{BanDependency, Diagnostic, E18eOptions, scan_e18e};

fn scan(rule: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
    scan_e18e(
        source,
        "sample.ts",
        &E18eOptions {
            rule_names: [CompactString::from(rule)].into_iter().collect(),
            banned_dependencies: SmallVec::new(),
        },
    )
}

#[test]
fn modern_array_rules_report_and_fix() {
    let diagnostics = scan(
        "prefer-array-from-map",
        "const out = [...items].map(item => item.id);",
    );
    assert_eq!(diagnostics[0].message_id, "preferArrayFrom");
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("diagnostic should include a fix")
            .replacement,
        "Array.from(items, item => item.id)"
    );

    let diagnostics = scan("prefer-array-at", "const last = items[items.length - 1];");
    assert_eq!(diagnostics[0].message_id, "preferAt");
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("diagnostic should include a fix")
            .replacement,
        "items.at(-1)"
    );
}

#[test]
fn performance_rules_report_and_fix() {
    let diagnostics = scan(
        "prefer-exponentiation-operator",
        "const x = Math.pow(a, 2);",
    );
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("diagnostic should include a fix")
            .replacement,
        "(a) ** (2)"
    );

    let diagnostics = scan(
        "prefer-string-fromcharcode",
        "String.fromCodePoint(65, 66);",
    );
    assert_eq!(diagnostics[0].loc.start_column, 7);
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("diagnostic should include a fix")
            .replacement,
        "fromCharCode"
    );
}

#[test]
fn boolean_rules_report_and_fix() {
    let diagnostics = scan("prefer-includes", "if (items.indexOf(id) !== -1) ok();");
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("diagnostic should include a fix")
            .replacement,
        "items.includes(id)"
    );

    let diagnostics = scan(
        "prefer-array-some",
        "if (items.filter(fn).length > 0) ok();",
    );
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("diagnostic should include a fix")
            .replacement,
        "items.some(fn)"
    );
}

#[test]
fn ban_dependencies_uses_options() {
    let diagnostics = scan_e18e(
        "import merge from 'lodash.merge';",
        "sample.js",
        &E18eOptions {
            rule_names: [CompactString::from("ban-dependencies")]
                .into_iter()
                .collect(),
            banned_dependencies: [BanDependency {
                module_name: CompactString::from("lodash.merge"),
                message_id: CompactString::from("documentedReplacement"),
                replacement: Some(CompactString::from("deepmerge-ts")),
                url: Some(CompactString::from("https://example.com")),
                description: None,
            }]
            .into_iter()
            .collect(),
        },
    );
    assert_eq!(diagnostics[0].message_id, "documentedReplacement");
    assert_eq!(diagnostics[0].data.name.as_deref(), Some("lodash.merge"));
}
