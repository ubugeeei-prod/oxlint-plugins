use crate::{TestingLibraryOptions, implemented_testing_library_rule_names, scan_testing_library};

#[test]
fn exposes_all_rule_names() {
    assert_eq!(implemented_testing_library_rule_names().len(), 29);
    assert!(implemented_testing_library_rule_names().contains(&"await-async-events"));
    assert!(implemented_testing_library_rule_names().contains(&"render-result-naming-convention"));
}

#[test]
fn scans_representative_rules() {
    let diagnostics = scan_testing_library(
        r#"
import { fireEvent } from '@testing-library/dom';
const { getByText } = render(<Button data-testid="BadId" />);
userEvent.click(button);
await fireEvent.click(button);
screen.getByText(/Save/g);
cleanup();
container.querySelector('.button');
waitFor(() => { expect(a).toBe(1); expect(b).toBe(2); fireEvent.click(button); expect(screen.getByText('x')).toBeInTheDocument(); });
waitForElementToBeRemoved(() => screen.getByText('gone'));
const result = render(<Button />);
"#,
        "fixture.test.tsx",
        &TestingLibraryOptions::default(),
    );
    let rules: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();
    assert!(rules.contains(&"await-async-events"));
    // `consistent-data-testid` is a no-op unless a `testIdPattern` is configured
    // (upstream default is the empty pattern), so it must not fire here.
    assert!(!rules.contains(&"consistent-data-testid"));
    assert!(rules.contains(&"no-await-sync-events"));
    assert!(rules.contains(&"no-dom-import"));
    assert!(rules.contains(&"no-global-regexp-flag-in-query"));
    assert!(rules.contains(&"no-manual-cleanup"));
    assert!(rules.contains(&"no-container"));
    assert!(rules.contains(&"no-node-access"));
    assert!(rules.contains(&"no-wait-for-multiple-assertions"));
    assert!(rules.contains(&"no-wait-for-side-effects"));
    assert!(rules.contains(&"prefer-find-by"));
    assert!(rules.contains(&"prefer-query-by-disappearance"));
    assert!(rules.contains(&"prefer-user-event"));
    assert!(rules.contains(&"prefer-screen-queries"));
    assert!(rules.contains(&"render-result-naming-convention"));
}

fn consistent_data_testid_options(pattern: &str) -> TestingLibraryOptions {
    TestingLibraryOptions {
        rule_names: ["consistent-data-testid".into()].into_iter().collect(),
        test_id_pattern: pattern.into(),
        ..TestingLibraryOptions::default()
    }
}

#[test]
fn consistent_data_testid_honors_pattern() {
    let source = r#"const a = <Button data-testid="kebab-id" />;"#;

    // Matching value: no diagnostic.
    assert!(
        scan_testing_library(
            source,
            "fixture.test.tsx",
            &consistent_data_testid_options("^[a-z-]+$"),
        )
        .is_empty()
    );

    // Non-matching value: one diagnostic, message echoes the resolved regex.
    let diagnostics = scan_testing_library(
        r#"const a = <Button data-testid="BadId" />;"#,
        "fixture.test.tsx",
        &consistent_data_testid_options("^[a-z-]+$"),
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "consistent-data-testid");
    assert!(diagnostics[0].message.contains("BadId"));
    assert!(diagnostics[0].message.contains("/^[a-z-]+$/"));
}

#[test]
fn consistent_data_testid_default_is_noop() {
    let diagnostics = scan_testing_library(
        r#"const a = <Button data-testid="BadId" />;"#,
        "fixture.test.tsx",
        &TestingLibraryOptions {
            rule_names: ["consistent-data-testid".into()].into_iter().collect(),
            ..TestingLibraryOptions::default()
        },
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn consistent_data_testid_substitutes_filename() {
    // `{fileName}` is replaced with the derived file name (`Button`), so a
    // matching id passes and a mismatching one reports.
    let options = consistent_data_testid_options("^{fileName}__[a-z]+$");
    assert!(
        scan_testing_library(
            r#"const a = <Button data-testid="Button__primary" />;"#,
            "src/Button.test.tsx",
            &options,
        )
        .is_empty()
    );
    assert_eq!(
        scan_testing_library(
            r#"const a = <Button data-testid="Other__primary" />;"#,
            "src/Button.test.tsx",
            &options,
        )
        .len(),
        1
    );
}

#[test]
fn consistent_data_testid_custom_message_and_attribute() {
    let options = TestingLibraryOptions {
        rule_names: ["consistent-data-testid".into()].into_iter().collect(),
        test_id_pattern: "^[a-z-]+$".into(),
        test_id_attribute: ["data-test-id".into()].into_iter().collect(),
        custom_message: Some("use kebab-case".into()),
    };
    let diagnostics = scan_testing_library(
        r#"const a = <Button data-test-id="BadId" data-testid="ignored" />;"#,
        "fixture.test.tsx",
        &options,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message.as_str(), "use kebab-case");
}

#[test]
fn accepts_awaited_async_interactions() {
    let options = TestingLibraryOptions {
        rule_names: ["await-async-events".into()].into_iter().collect(),
        ..TestingLibraryOptions::default()
    };
    assert!(
        scan_testing_library(
            "await userEvent.click(button);",
            "fixture.test.ts",
            &options
        )
        .is_empty()
    );
}
