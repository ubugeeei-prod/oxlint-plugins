#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "serde_json::json! intentionally constructs public option payloads in tests."
)]

use oxlint_plugins_carton::SmallVec;

use serde_json::json;

use crate::{
    RULE_NAMES, RuleDiagnostic, implemented_perfectionist_rule_names, scan_perfectionist,
    scan_perfectionist_rule,
};

#[test]
fn exposes_all_rule_names() {
    assert_eq!(implemented_perfectionist_rule_names(), RULE_NAMES);
}

#[test]
fn scans_representative_rules() {
    let source = r#"
import { b, a } from "pkg";
export { b, a };
import z from "z";
import a from "a";
export { z } from "z";
export { a } from "a";
import data from "./data.json" with { type: "json", foo: "bar" };
export { data } from "./data.json" with { type: "json", foo: "bar" };
@Z @A class Decorated {}
class Derived implements Z, A {}
const array = ["b", "a"];
["b", "a"].includes(value);
const set = new Set(["b", "a"]);
const map = new Map([["b", 1], ["a", 2]]);
const object = { b: 1, a: 2 };
type ObjectType = { b: string; a: string };
interface Interface { b: string; a: string }
enum Enum { B, A }
class Class { b() {} a() {} }
const jsx = <Component b={1} a={2} />;
const b = 1, a = 2;
type Union = B | A;
type Intersection = B & A;
switch (value) { case "b": break; case "a": break; }
const z = 1;
function a() {}
"#;
    let diagnostics = scan_perfectionist(source, "fixture.tsx");
    let names: SmallVec<[&str; 24]> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect();

    assert_eq!(names.len(), RULE_NAMES.len());
}

fn configured(source: &str, options: serde_json::Value) -> SmallVec<[RuleDiagnostic; 8]> {
    scan_perfectionist_rule(source, "fixture.ts", "sort-named-imports", &options)
}

#[test]
fn sorts_predefined_type_and_value_groups() {
    let diagnostics = configured(
        "import { value, type Type } from 'pkg';",
        json!([{ "groups": ["type-import", "unknown"] }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message_id,
        "unexpectedNamedImportsGroupOrder"
    );
    assert_eq!(diagnostics[0].data.left_group.as_deref(), Some("unknown"));
    assert_eq!(
        diagnostics[0].data.right_group.as_deref(),
        Some("type-import")
    );
}

#[test]
fn matches_custom_groups_by_regex_modifier_and_any_of() {
    let diagnostics = configured(
        "import { type other, type FooType, fooValue } from 'pkg';",
        json!([{
            "customGroups": [{
                "groupName": "foo",
                "anyOf": [
                    { "modifiers": ["type"], "elementNamePattern": "Foo" },
                    { "elementNamePattern": "^foo" }
                ]
            }],
            "groups": ["foo", "unknown"]
        }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].data.right, "FooType");
    assert_eq!(diagnostics[0].data.right_group.as_deref(), Some("foo"));
}

#[test]
fn applies_custom_group_sort_overrides() {
    let diagnostics = configured(
        "import { type a, type bb, type cccc, value } from 'pkg';",
        json!([{
            "customGroups": [{
                "groupName": "types",
                "modifiers": ["type"],
                "type": "line-length",
                "order": "desc"
            }],
            "groups": ["types", "unknown"]
        }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "unexpectedNamedImportsOrder")
    );
    assert_eq!(diagnostics[0].fix.replacement, "type cccc, type bb, type a");
}

#[test]
fn partitions_by_newline_and_matching_comments() {
    let diagnostics = configured(
        "import {\n  D,\n  A,\n\n  C,\n  // Part\n  B,\n  A2,\n} from 'pkg';",
        json!([{
            "partitionByNewLine": true,
            "partitionByComment": "^Part"
        }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].data.left, "D");
    assert_eq!(diagnostics[0].data.right, "A");
    assert_eq!(diagnostics[1].data.left, "B");
    assert_eq!(diagnostics[1].data.right, "A2");
}

#[test]
fn reports_order_and_spacing_with_one_shared_fix() {
    let diagnostics = configured(
        "import {\n  beta,\n\n  alpha,\n} from 'pkg';",
        json!([{ "newlinesInside": 0 }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].message_id, "unexpectedNamedImportsOrder");
    assert_eq!(diagnostics[1].message_id, "extraSpacingBetweenNamedImports");
    assert_eq!(diagnostics[0].fix, diagnostics[1].fix);
    assert_eq!(diagnostics[0].fix.replacement, "alpha,\n  beta");
}

#[test]
fn enforces_inline_newline_directives() {
    let diagnostics = configured(
        "import {\n  alpha,\n\n  beta,\n  charlie,\n} from 'pkg';",
        json!([{
            "customGroups": [
                { "groupName": "a", "elementNamePattern": "^alpha$" },
                { "groupName": "b", "elementNamePattern": "^beta$" },
                { "groupName": "c", "elementNamePattern": "^charlie$" }
            ],
            "groups": [
                "a",
                { "newlinesBetween": 0 },
                "b",
                { "newlinesBetween": 1 },
                "c"
            ],
            "newlinesBetween": 2
        }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].message_id, "extraSpacingBetweenNamedImports");
    assert_eq!(
        diagnostics[1].message_id,
        "missedSpacingBetweenNamedImports"
    );
}

#[test]
fn keeps_the_strongest_newline_policy_across_unused_groups() {
    let diagnostics = configured(
        "import {\n  alpha,\n  beta,\n} from 'pkg';",
        json!([{
            "customGroups": [
                { "groupName": "a", "elementNamePattern": "^alpha$" },
                { "groupName": "unused", "elementNamePattern": "^never$" },
                { "groupName": "b", "elementNamePattern": "^beta$" }
            ],
            "groups": [
                "a",
                "unused",
                { "newlinesBetween": 0 },
                "b"
            ],
            "newlinesBetween": 2
        }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message_id,
        "missedSpacingBetweenNamedImports"
    );
    assert_eq!(diagnostics[0].fix.replacement, ",\n\n\n  ");
}

#[test]
fn selects_the_first_matching_conditional_configuration() {
    let diagnostics = configured(
        "import { b, g, r } from 'pkg';",
        json!([
            {
                "type": "unsorted",
                "useConfigurationIf": { "allNamesMatchPattern": "^foo" }
            },
            {
                "customGroups": [
                    { "groupName": "r", "elementNamePattern": "^r$" },
                    { "groupName": "g", "elementNamePattern": "^g$" },
                    { "groupName": "b", "elementNamePattern": "^b$" }
                ],
                "groups": ["r", "g", "b"],
                "useConfigurationIf": {
                    "matchesAstSelector": "ImportDeclaration",
                    "allNamesMatchPattern": "^[rgb]$"
                }
            }
        ]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "unexpectedNamedImportsGroupOrder")
    );
    assert_eq!(diagnostics[0].fix.replacement, "r, g, b");
}

#[test]
fn keeps_utf16_offsets_for_unicode_group_fixes() {
    let source = "'😀';\nimport { 世界, 你好, api } from '模块';";
    let diagnostics = configured(
        source,
        json!([{
            "locales": "zh-CN",
            "customGroups": [{ "groupName": "api", "elementNamePattern": "^api$" }],
            "groups": ["api", "unknown"]
        }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].fix.start, 15);
    assert_eq!(diagnostics[0].fix.end, 26);
    assert_eq!(diagnostics[0].fix.replacement, "api, 你好, 世界");
}
