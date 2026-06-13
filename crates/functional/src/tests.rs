use super::{FunctionalOptions, implemented_functional_rule_names, scan_functional};

#[test]
fn exposes_all_rule_names() {
    assert_eq!(implemented_functional_rule_names().len(), 20);
    assert!(implemented_functional_rule_names().contains(&"no-let"));
    assert!(implemented_functional_rule_names().contains(&"prefer-readonly-type"));
}

#[test]
fn scans_core_syntax_rules() {
    let diagnostics = scan_functional(
        r#"
let value = 1;
class Derived extends Base { method() { this.x = 1; } }
if (value) { value += 1; }
for (let i = 0; i < 1; i++) {}
try { throw new Error('x'); } catch (err) {}
Promise.reject(err);
const f = () => effect();
"#,
        "fixture.ts",
        &FunctionalOptions::default(),
    );
    let rules: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();
    assert!(rules.contains(&"no-let"));
    assert!(rules.contains(&"no-classes"));
    assert!(rules.contains(&"no-class-inheritance"));
    assert!(rules.contains(&"no-conditional-statements"));
    assert!(rules.contains(&"no-loop-statements"));
    assert!(rules.contains(&"no-try-statements"));
    assert!(rules.contains(&"no-throw-statements"));
    assert!(rules.contains(&"no-promise-reject"));
    assert!(rules.contains(&"no-this-expressions"));
    assert!(rules.contains(&"functional-parameters"));
}

#[test]
fn scans_type_rules() {
    let diagnostics = scan_functional(
        r#"
interface Mixed {
  readonly items: string[];
  run(): void;
}
type Bag = { value: Array<string> };
const takes = (items: string[]): void => {};
"#,
        "fixture.ts",
        &FunctionalOptions::default(),
    );
    let rules: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();
    assert!(rules.contains(&"no-mixed-types"));
    assert!(rules.contains(&"prefer-property-signatures"));
    assert!(rules.contains(&"prefer-readonly-type"));
    assert!(rules.contains(&"prefer-immutable-types"));
    assert!(rules.contains(&"readonly-type"));
    assert!(rules.contains(&"type-declaration-immutability"));
    assert!(rules.contains(&"no-return-void"));
}

#[test]
fn no_loop_statements_reports_every_loop_form_with_generic_message_id() {
    let options = FunctionalOptions {
        rule_names: ["no-loop-statements".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let source = r#"
for (let i = 0; i < 1; i++) {}
for (const k in obj) {}
for (const v of list) {}
while (cond) {}
do {} while (cond);
"#;
    let diagnostics = scan_functional(source, "fixture.ts", &options);
    assert_eq!(diagnostics.len(), 5);
    for diagnostic in &diagnostics {
        assert_eq!(diagnostic.rule_name, "no-loop-statements");
        assert_eq!(diagnostic.message_id, "generic");
    }

    // `if` is not a loop, so nothing is reported.
    assert!(scan_functional("if (cond) {}", "fixture.ts", &options).is_empty());
}

#[test]
fn no_promise_reject_matches_upstream_syntactic_behavior() {
    let options = FunctionalOptions {
        rule_names: ["no-promise-reject".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let count = |source: &str| scan_functional(source, "fixture.ts", &options).len();

    // Reports `Promise.reject`, `new Promise` with a `reject` param, and async
    // throws that escape (no enclosing catch).
    assert_eq!(count("function f() { return Promise.reject('e'); }"), 1);
    assert_eq!(
        count("function f() { return new Promise((resolve, reject) => { reject('e'); }); }"),
        1
    );
    assert_eq!(count("async function f() { throw new Error('e'); }"), 1);
    assert_eq!(
        count("async function f() { try { throw new Error('e'); } finally { g(); } }"),
        1
    );

    // Does not report resolves, executors without a reject param, or async
    // throws caught by an enclosing try/catch.
    assert_eq!(count("function f() { return Promise.resolve('x'); }"), 0);
    assert_eq!(
        count("function f() { return new Promise((resolve) => resolve('x')); }"),
        0
    );
    assert_eq!(
        count("async function f() { try { throw new Error('e'); } catch (e) { g(e); } }"),
        0
    );
}

#[test]
fn honors_core_options() {
    let mut options = FunctionalOptions {
        rule_names: ["no-let".into()].into_iter().collect(),
        allow_let_in_for_loop_init: true,
        ..FunctionalOptions::default()
    };
    assert!(scan_functional("for (let i = 0; i < 1; i++) {}", "fixture.ts", &options).is_empty());

    options = FunctionalOptions {
        rule_names: ["no-try-statements".into()].into_iter().collect(),
        allow_try_catch: true,
        allow_try_finally: true,
        ..FunctionalOptions::default()
    };
    assert!(
        scan_functional(
            "try { work(); } catch (error) {} finally { cleanup(); }",
            "fixture.ts",
            &options,
        )
        .is_empty()
    );

    options = FunctionalOptions {
        rule_names: ["no-throw-statements".into()].into_iter().collect(),
        allow_throw_to_reject_promises: true,
        ..FunctionalOptions::default()
    };
    assert!(
        scan_functional(
            "async function f(error) { throw error; }",
            "fixture.ts",
            &options
        )
        .is_empty()
    );
}
