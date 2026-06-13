//! Rust unit tests for the sonarjs core. All test inputs are independently
//! authored (clean-room); no upstream SonarJS fixtures or expectations are used.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Diagnostic, SonarjsOptions, scan_sonarjs};

fn scan(rule_name: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
    let options = SonarjsOptions {
        rule_names: [CompactString::from(rule_name)].into_iter().collect(),
    };
    scan_sonarjs(source, "sample.ts", &options)
}

#[test]
fn reports_template_literal_nested_in_another() {
    let diagnostics = scan(
        "no-nested-template-literals",
        "const x = `outer ${`inner`} end`;",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-template-literals");
    assert_eq!(diagnostics[0].message_id, "nestedTemplateLiteral");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_flat_template_literal() {
    let diagnostics = scan("no-nested-template-literals", "const x = `value ${y}`;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_adjacent_template_literals() {
    let diagnostics = scan(
        "no-nested-template-literals",
        "const a = `x ${y}`;\nconst b = `z ${w}`;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_each_nested_level() {
    let diagnostics = scan(
        "no-nested-template-literals",
        "const x = `a ${`b ${`c`}`}`;",
    );
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn disabled_rule_reports_nothing() {
    let options = SonarjsOptions {
        rule_names: SmallVec::new(),
    };
    let diagnostics = scan_sonarjs("const x = `outer ${`inner`}`;", "sample.ts", &options);
    assert!(diagnostics.is_empty());
}
