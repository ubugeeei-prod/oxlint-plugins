use super::{MochaOptions, scan_mocha};

fn rule_names(source_text: &str) -> oxlint_plugins_carton::SmallVec<[&'static str; 16]> {
    scan_mocha(source_text, "fixture.test.js", &MochaOptions::default())
        .into_iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect()
}

#[test]
fn scans_core_mocha_rules() {
    let rules = rule_names(
        r#"
            beforeEach(function () {});
            it("global", function () {});
            describe.only("", async function () {
              before(function (done) {});
              before(function () {});
              it("works", function (done) { return fetch("/"); });
              it("works", () => {});
              it.skip("later");
              it("async return", async function () { return fetch("/"); });
              it("nested", function () { it("bad", function () {}); });
              helper();
            });
            describe("single", function () {
              before(function () {});
              it("one", function (done) { done(); });
            });
            suite("tdd", function () { test("bad", function () {}); });
            export const value = 1;
            "#,
    );

    assert!(rules.contains(&"consistent-spacing-between-blocks"));
    assert!(rules.contains(&"no-top-level-hooks"));
    assert!(rules.contains(&"no-hooks"));
    assert!(rules.contains(&"no-exclusive-tests"));
    assert!(rules.contains(&"no-empty-title"));
    assert!(rules.contains(&"no-async-suite"));
    assert!(rules.contains(&"handle-done-callback"));
    assert!(rules.contains(&"no-sibling-hooks"));
    assert!(rules.contains(&"no-return-and-callback"));
    assert!(rules.contains(&"no-return-from-async"));
    assert!(rules.contains(&"no-mocha-arrows"));
    assert!(rules.contains(&"no-pending-tests"));
    assert!(rules.contains(&"no-global-tests"));
    assert!(rules.contains(&"no-hooks-for-single-case"));
    assert!(rules.contains(&"no-nested-tests"));
    assert!(rules.contains(&"no-setup-in-describe"));
    assert!(rules.contains(&"no-synchronous-tests"));
    assert!(rules.contains(&"consistent-interface"));
    assert!(rules.contains(&"no-exports"));
}

#[test]
fn scans_title_rules_with_options() {
    let options = MochaOptions {
        valid_suite_title_pattern: Some("^Suite".into()),
        valid_test_title_pattern: Some("^should".into()),
        ..MochaOptions::default()
    };
    let rules = scan_mocha(
        r#"
            describe("bad suite", function () {
              it("bad test", function () {});
            });
            "#,
        "fixture.test.js",
        &options,
    )
    .into_iter()
    .map(|diagnostic| diagnostic.rule_name)
    .collect::<oxlint_plugins_carton::SmallVec<[&'static str; 8]>>();

    assert!(rules.contains(&"valid-suite-title"));
    assert!(rules.contains(&"valid-test-title"));
}
