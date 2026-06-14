//! Small accessors shared by rules, mirroring upstream `src/utils/ast.ts`.
//!
//! Rules walk the enriched libpg_query JSON directly, so these helpers are thin
//! wrappers over `serde_json` lookups that keep the rule code reading like the
//! original TypeScript (`isColumnRef(node)`, `node.fields`, …).

use serde_json::Value;

/// The node's `type` tag (e.g. `"SelectStmt"`), if it is a tagged node.
pub fn node_type(node: &Value) -> Option<&str> {
    node.get("type").and_then(Value::as_str)
}

/// Whether `node` is a tagged node of the given type.
pub fn is_type(node: &Value, ty: &str) -> bool {
    node_type(node) == Some(ty)
}

/// Borrow a child field by key.
pub fn field<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    node.get(key)
}

/// Borrow a field expected to be an array.
pub fn array_field<'a>(node: &'a Value, key: &str) -> Option<&'a [Value]> {
    node.get(key)
        .and_then(Value::as_array)
        .map(|values| &values[..])
}

/// Borrow a field expected to be a string.
#[allow(dead_code, reason = "shared accessor used by rules added in later PRs")]
pub fn str_field<'a>(node: &'a Value, key: &str) -> Option<&'a str> {
    node.get(key).and_then(Value::as_str)
}

/// The name of a `TypeName` node, mirroring upstream `getTypeName`: the enriched
/// AST keys the `names` list as `"0"`, `"1"`, … of `String` nodes; the type is
/// the last segment's `sval` (`"1"` if present, else `"0"`), so `varchar`'s
/// `pg_catalog.varchar` resolves to `varchar` and a bare `money` to `money`.
#[allow(dead_code, reason = "shared accessor used by rules added in later PRs")]
pub fn type_name(type_name: Option<&Value>) -> Option<&str> {
    let names = type_name?;
    let sval = |key: &str| {
        names
            .get(key)
            .and_then(|segment| segment.get("sval"))
            .and_then(Value::as_str)
    };
    sval("1").or_else(|| sval("0"))
}
