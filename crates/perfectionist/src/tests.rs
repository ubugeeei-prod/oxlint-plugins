use oxlint_plugins_carton::SmallVec;

use crate::{RULE_NAMES, implemented_perfectionist_rule_names, scan_perfectionist};

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
