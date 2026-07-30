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

fn configured_exports(source: &str, options: serde_json::Value) -> SmallVec<[RuleDiagnostic; 8]> {
    scan_perfectionist_rule(source, "fixture.ts", "sort-named-exports", &options)
}

fn configured_export_declarations(
    source: &str,
    options: serde_json::Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    scan_perfectionist_rule(source, "fixture.ts", "sort-exports", &options)
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

#[test]
fn sorts_named_exports_by_exported_alias_and_original_name() {
    let by_alias = configured_exports("export { a as C, b as B, c as A } from 'pkg';", json!([]));
    assert_eq!(by_alias.len(), 2);
    assert_eq!(by_alias[0].data.left, "C");
    assert_eq!(by_alias[0].data.right, "B");
    assert_eq!(by_alias[0].fix.replacement, "c as A, b as B, a as C");

    let by_original = configured_exports(
        "export { c as A, b as B, a as C } from 'pkg';",
        json!([{ "ignoreAlias": true }]),
    );
    assert_eq!(by_original.len(), 2);
    assert_eq!(by_original[0].data.left, "c");
    assert_eq!(by_original[0].data.right, "b");
    assert_eq!(by_original[0].fix.replacement, "a as C, b as B, c as A");
}

#[test]
fn sorts_named_export_predefined_and_custom_groups() {
    let diagnostics = configured_exports(
        "export { value, type ApiType, type Other } from 'pkg';",
        json!([{
            "customGroups": [{
                "groupName": "api",
                "modifiers": ["type"],
                "selector": "export",
                "elementNamePattern": "^Api"
            }],
            "groups": ["api", "type-export", "unknown"]
        }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message_id,
        "unexpectedNamedExportsGroupOrder"
    );
    assert_eq!(diagnostics[0].data.right, "ApiType");
    assert_eq!(diagnostics[0].data.right_group.as_deref(), Some("api"));
    assert_eq!(
        diagnostics[0].fix.replacement,
        "type ApiType, type Other, value"
    );
}

#[test]
fn applies_named_export_partition_and_newline_policies() {
    let diagnostics = configured_exports(
        "export {\n  D,\n  A,\n\n  C,\n  // Part\n  B,\n  A2,\n} from 'pkg';",
        json!([{
            "partitionByNewLine": true,
            "partitionByComment": "^Part",
            "newlinesInside": 0
        }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].data.left, "D");
    assert_eq!(diagnostics[0].data.right, "A");
    assert_eq!(diagnostics[1].data.left, "B");
    assert_eq!(diagnostics[1].data.right, "A2");
}

#[test]
fn selects_named_export_conditional_configuration() {
    let diagnostics = configured_exports(
        "export { b, g, r } from 'pkg';",
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
                    "matchesAstSelector": "ExportNamedDeclaration",
                    "allNamesMatchPattern": "^[rgb]$"
                }
            }
        ]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "unexpectedNamedExportsGroupOrder")
    );
    assert_eq!(diagnostics[0].fix.replacement, "r, g, b");
}

#[test]
fn keeps_utf16_offsets_for_unicode_named_export_fixes() {
    let source = "'😀';\nexport { 世界, 你好, api } from '模块';";
    let diagnostics = configured_exports(
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

#[test]
fn named_export_rule_selection_and_malformed_sources_fail_closed() {
    assert!(
        scan_perfectionist_rule(
            "export { b, a };",
            "fixture.ts",
            "sort-named-imports",
            &json!([])
        )
        .is_empty()
    );
    assert!(
        scan_perfectionist_rule(
            "export { b,",
            "fixture.ts",
            "sort-named-exports",
            &json!([])
        )
        .is_empty()
    );
    assert!(
        scan_perfectionist_rule(
            "export { a };",
            "fixture.ts",
            "sort-named-exports",
            &json!([])
        )
        .is_empty()
    );
}

#[test]
fn sorts_export_declarations_and_preserves_trailing_comments() {
    let source = "export * from 'z'; // z docs\nexport { a } from 'a'; // a docs\n";
    let diagnostics = configured_export_declarations(source, json!([]));

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "unexpectedExportsOrder");
    assert_eq!(diagnostics[0].data.left, "z");
    assert_eq!(diagnostics[0].data.right, "a");
    assert_eq!(
        diagnostics[0].fix.replacement,
        "export { a } from 'a'; // a docs\nexport * from 'z'; // z docs"
    );
}

#[test]
fn sorts_export_declarations_by_all_predefined_modifiers() {
    let diagnostics = configured_export_declarations(
        "export type {\n  A,\n} from 'types';\nexport * from 'runtime';\n",
        json!([{
            "groups": [
                "value-wildcard-singleline-export",
                "type-named-multiline-export"
            ]
        }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "unexpectedExportsGroupOrder");
    assert_eq!(
        diagnostics[0].data.left_group.as_deref(),
        Some("type-named-multiline-export")
    );
    assert_eq!(
        diagnostics[0].data.right_group.as_deref(),
        Some("value-wildcard-singleline-export")
    );
}

#[test]
fn matches_export_custom_groups_by_multiple_modifiers_and_any_of() {
    let diagnostics = configured_export_declarations(
        "export { a } from 'other';\nexport * from 'api-runtime';\n",
        json!([{
            "customGroups": [{
                "groupName": "api-wildcard",
                "anyOf": [
                    {
                        "elementNamePattern": "^api",
                        "modifiers": ["value", "wildcard", "singleline"],
                        "selector": "export"
                    },
                    { "elementNamePattern": "^never" }
                ]
            }],
            "groups": ["api-wildcard", "unknown"]
        }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "unexpectedExportsGroupOrder");
    assert_eq!(
        diagnostics[0].data.right_group.as_deref(),
        Some("api-wildcard")
    );
}

#[test]
fn keeps_disabled_export_declarations_in_place() {
    let diagnostics = configured_export_declarations(
        "export { c } from './c'\nexport { b } from './b'\n// eslint-disable-next-line\nexport { a } from './a'",
        json!([{}]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].data.left, "./c");
    assert_eq!(diagnostics[0].data.right, "./b");
    assert_eq!(diagnostics[0].fix.start, 0);
    assert_eq!(diagnostics[0].fix.end, 47);
    assert_eq!(
        diagnostics[0].fix.replacement,
        "export { b } from './b'\nexport { c } from './c'"
    );
}

#[test]
fn reports_and_fixes_missing_export_group_comments() {
    let diagnostics = configured_export_declarations(
        "export type { a } from './a';\n\nexport { b } from './b';",
        json!([{
            "groups": [
                { "commentAbove": "Types", "group": "type-export" },
                { "commentAbove": "Values", "group": "unknown" }
            ]
        }]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "missedCommentAboveExport")
    );
    assert_eq!(
        diagnostics[0].data.missed_comment_above.as_deref(),
        Some("Types")
    );
    assert_eq!(
        diagnostics[1].data.missed_comment_above.as_deref(),
        Some("Values")
    );
    assert_eq!(diagnostics[0].fix, diagnostics[1].fix);
    assert_eq!(diagnostics[0].fix.start, 0);
    assert_eq!(diagnostics[0].fix.end, 31);
}

#[test]
fn keeps_utf16_offsets_for_unicode_export_declaration_fixes() {
    let source = "'😀';\nexport { 世界 } from '世界';\nexport { api } from 'api';";
    let diagnostics = configured_export_declarations(
        source,
        json!([{
            "customGroups": [{ "groupName": "api", "elementNamePattern": "^api$" }],
            "groups": ["api", "unknown"],
            "locales": "zh-CN"
        }]),
    );

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].fix.start, 6);
    assert_eq!(diagnostics[0].fix.end, 57);
    assert_eq!(
        diagnostics[0].fix.replacement,
        "export { api } from 'api';\nexport { 世界 } from '世界';"
    );
}

#[test]
fn export_declaration_rule_ignores_local_exports_and_other_rules() {
    assert!(
        configured_export_declarations(
            "export const z = 1;\nexport function a() {}\nexport { z, a };",
            json!([])
        )
        .is_empty()
    );
    assert!(
        scan_perfectionist_rule(
            "export { z } from 'z';\nexport { a } from 'a';",
            "fixture.ts",
            "sort-named-exports",
            &json!([])
        )
        .is_empty()
    );
    assert!(
        scan_perfectionist_rule(
            "export { z } from 'z';\nexport { a } from 'a';",
            "fixture.ts",
            "sort-named-imports",
            &json!([])
        )
        .is_empty()
    );
}

#[test]
fn export_declaration_rule_fails_closed_for_malformed_sources_and_options() {
    assert!(configured_export_declarations("export { a } from", json!([])).is_empty());
    assert!(
        configured_export_declarations(
            "export { b } from 'b';\nexport { a } from 'a';",
            json!("not-an-option-object")
        )
        .len()
            == 1
    );
    assert!(
        configured_export_declarations(
            "export { b } from 'b';\nexport { a } from 'a';",
            json!([{ "type": "not-a-sort", "groups": [null, 1, false] }])
        )
        .len()
            == 1
    );
}
