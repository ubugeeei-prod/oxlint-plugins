use super::{
    EnforceParameterCount, FunctionalOptions, implemented_functional_rule_names, scan_functional,
};

#[test]
fn functional_parameters_rest_param() {
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        enforce_parameter_count: EnforceParameterCount::Off,
        ..FunctionalOptions::default()
    };
    let count =
        |source: &str, opts: &FunctionalOptions| scan_functional(source, "fixture.ts", opts).len();

    // Rest parameter is reported by default.
    assert_eq!(count("function f(...args) {}", &options), 1);

    // allowRestParameter suppresses the report.
    let allow_rest = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        allow_rest_parameter: true,
        enforce_parameter_count: EnforceParameterCount::Off,
        ..FunctionalOptions::default()
    };
    assert_eq!(count("function f(...args) {}", &allow_rest), 0);
}

#[test]
fn functional_parameters_arguments_keyword() {
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        enforce_parameter_count: EnforceParameterCount::Off,
        ..FunctionalOptions::default()
    };
    let count =
        |source: &str, opts: &FunctionalOptions| scan_functional(source, "fixture.ts", opts).len();

    // `arguments` reference is reported.
    assert_eq!(count("function f(x) { return arguments; }", &options), 1);

    // allowArgumentsKeyword suppresses it.
    let allow_args = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        allow_arguments_keyword: true,
        enforce_parameter_count: EnforceParameterCount::Off,
        ..FunctionalOptions::default()
    };
    assert_eq!(count("function f(x) { return arguments; }", &allow_args), 0);
}

#[test]
fn functional_parameters_count_at_least_one_default() {
    // Default options: atLeastOne, ignoreIIFE=true, ignoreGettersAndSetters=true.
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("function f() {}", "fixture.ts", &options);
    let ids: Vec<&str> = diagnostics.iter().map(|d| d.message_id).collect();
    assert!(ids.contains(&"paramCountAtLeastOne"));
}

#[test]
fn functional_parameters_iife_ignored_by_default() {
    // By default ignoreIIFE=true, so an IIFE with no params should NOT be reported.
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("(function() {})()", "fixture.ts", &options);
    let ids: Vec<&str> = diagnostics.iter().map(|d| d.message_id).collect();
    assert!(!ids.contains(&"paramCountAtLeastOne"));
}

#[test]
fn functional_parameters_iife_reported_when_ignore_iife_false() {
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        enforce_count_ignore_iife: false,
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("(function() {})()", "fixture.ts", &options);
    let ids: Vec<&str> = diagnostics.iter().map(|d| d.message_id).collect();
    assert!(ids.contains(&"paramCountAtLeastOne"));
}

#[test]
fn functional_parameters_exactly_one_two_params() {
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        enforce_parameter_count: EnforceParameterCount::ExactlyOne,
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("function f(a, b) {}", "fixture.ts", &options);
    let ids: Vec<&str> = diagnostics.iter().map(|d| d.message_id).collect();
    assert!(ids.contains(&"paramCountExactlyOne"));
}

#[test]
fn functional_parameters_ignore_identifier_pattern_suppresses() {
    let options = FunctionalOptions {
        rule_names: ["functional-parameters".into()].into_iter().collect(),
        ignore_identifier_pattern: ["^foo$".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    // `foo` matches the ignore pattern — rest param and count should not be reported.
    let diagnostics = scan_functional("function foo(...args) {}", "fixture.ts", &options);
    assert!(diagnostics.is_empty());

    // `bar` does NOT match — rest param is reported.
    let diagnostics2 = scan_functional("function bar(...args) {}", "fixture.ts", &options);
    let ids: Vec<&str> = diagnostics2.iter().map(|d| d.message_id).collect();
    assert!(ids.contains(&"restParam"));
}

#[test]
fn no_let_honors_options() {
    let base = || FunctionalOptions {
        rule_names: ["no-let".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let count = |source: &str, options: &FunctionalOptions| {
        scan_functional(source, "fixture.ts", options).len()
    };

    // Plain let reports 1 diagnostic.
    assert_eq!(count("let x;", &base()), 1);

    // With allow_in_functions, let inside a function body is allowed.
    let allow_fn = FunctionalOptions {
        rule_names: ["no-let".into()].into_iter().collect(),
        allow_in_functions: true,
        ..FunctionalOptions::default()
    };
    assert_eq!(count("function f() { let x; }", &allow_fn), 0);
    // But top-level let is still reported.
    assert_eq!(count("let x;", &allow_fn), 1);

    // With ignore_identifier_pattern, matching names are allowed.
    let ignore_mutable = FunctionalOptions {
        rule_names: ["no-let".into()].into_iter().collect(),
        ignore_identifier_pattern: ["^mutable".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    assert_eq!(count("let mutable;", &ignore_mutable), 0);
    assert_eq!(count("let immutable;", &ignore_mutable), 1);
}

#[test]
fn prefer_property_signatures_honors_ignore_if_readonly_wrapped() {
    let base = || FunctionalOptions {
        rule_names: ["prefer-property-signatures".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let count = |source: &str, options: &FunctionalOptions| {
        scan_functional(source, "fixture.ts", options).len()
    };

    // A bare method signature is always reported.
    assert_eq!(count("type Foo = { bar(): number };", &base()), 1);

    // Wrapped in Readonly<...> with the option enabled, it is ignored; without
    // the option it is still reported.
    let wrapped = "type Foo = Readonly<{ bar(): number }>;";
    assert_eq!(count(wrapped, &base()), 1);
    let mut ignoring = base();
    ignoring.ignore_if_readonly_wrapped = true;
    assert_eq!(count(wrapped, &ignoring), 0);

    // Property signatures are never reported.
    assert_eq!(count("type Foo = { bar: () => number };", &base()), 0);
}

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
fn no_classes_honors_ignore_patterns() {
    let base_options = FunctionalOptions {
        rule_names: ["no-classes".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };

    // Plain class reports 1 diagnostic.
    let diagnostics = scan_functional("class Foo {}", "fixture.ts", &base_options);
    assert_eq!(diagnostics.len(), 1);

    // ignoreIdentifierPattern matching the class name suppresses the report.
    let id_pattern_options = FunctionalOptions {
        rule_names: ["no-classes".into()].into_iter().collect(),
        ignore_identifier_pattern: ["^Foo$".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("class Foo {}", "fixture.ts", &id_pattern_options);
    assert_eq!(diagnostics.len(), 0);

    // ignoreCodePattern matching the class source text suppresses the report.
    let code_pattern_options = FunctionalOptions {
        rule_names: ["no-classes".into()].into_iter().collect(),
        ignore_code_pattern: ["class Foo".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("class Foo {}", "fixture.ts", &code_pattern_options);
    assert_eq!(diagnostics.len(), 0);

    // A non-matching identifier pattern still reports 1 diagnostic.
    let non_matching_options = FunctionalOptions {
        rule_names: ["no-classes".into()].into_iter().collect(),
        ignore_identifier_pattern: ["^Bar$".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let diagnostics = scan_functional("class Foo {}", "fixture.ts", &non_matching_options);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn no_class_inheritance_reports_abstract_and_extends() {
    let options = FunctionalOptions {
        rule_names: ["no-class-inheritance".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let message_ids = |source: &str| -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = scan_functional(source, "fixture.ts", &options)
            .iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect();
        ids.sort_unstable();
        ids
    };

    assert!(message_ids("class Foo {}").is_empty());
    assert_eq!(message_ids("class Foo extends Bar {}"), ["extends"]);
    assert_eq!(message_ids("abstract class Foo {}"), ["abstract"]);
    assert_eq!(
        message_ids("abstract class Foo extends Bar {}"),
        ["abstract", "extends"]
    );

    // Ignore patterns suppress both reports.
    let ignoring = FunctionalOptions {
        rule_names: ["no-class-inheritance".into()].into_iter().collect(),
        ignore_identifier_pattern: ["^Foo$".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    assert!(
        scan_functional("abstract class Foo extends Bar {}", "fixture.ts", &ignoring).is_empty()
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

#[test]
fn reports_expressions_in_argument_position() {
    // no-this-expressions fires for `this` passed as a call argument.
    let this_options = FunctionalOptions {
        rule_names: ["no-this-expressions".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    assert_eq!(
        scan_functional("foo(this);", "fixture.ts", &this_options).len(),
        1
    );

    // no-promise-reject fires for a rejecting `new Promise` in argument position.
    let reject_options = FunctionalOptions {
        rule_names: ["no-promise-reject".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let source = "foo(new Promise((resolve, reject) => reject('e')));";
    assert_eq!(
        scan_functional(source, "fixture.ts", &reject_options).len(),
        1
    );
}

#[test]
fn no_mixed_types_reports_once_and_honors_options() {
    let options = FunctionalOptions {
        rule_names: ["no-mixed-types".into()].into_iter().collect(),
        ..FunctionalOptions::default()
    };
    let count =
        |source: &str, opts: &FunctionalOptions| scan_functional(source, "fixture.ts", opts).len();

    // A type alias mixing a property and a method is reported exactly once
    // (not double-counted by both the alias and the type-literal visitor).
    let alias = count("type Foo = { bar: string; baz(): number };", &options);
    assert_eq!(alias, 1);
    let iface = count("interface Foo { bar: string; baz(): number }", &options);
    assert_eq!(iface, 1);
    // Same-kind members are not reported.
    let same = count("type Foo = { bar: string; baz: number };", &options);
    assert_eq!(same, 0);

    // checkTypeLiterals / checkInterfaces: false skip their declaration kind.
    let no_literals = FunctionalOptions {
        rule_names: ["no-mixed-types".into()].into_iter().collect(),
        check_type_literals: false,
        ..FunctionalOptions::default()
    };
    assert_eq!(count("type Foo = { bar: string; baz(): number };", &no_literals), 0);
    let no_ifaces = FunctionalOptions {
        rule_names: ["no-mixed-types".into()].into_iter().collect(),
        check_interfaces: false,
        ..FunctionalOptions::default()
    };
    assert_eq!(count("interface Foo { bar: string; baz(): number }", &no_ifaces), 0);
}
