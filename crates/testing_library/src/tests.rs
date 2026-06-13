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
    assert!(rules.contains(&"consistent-data-testid"));
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
