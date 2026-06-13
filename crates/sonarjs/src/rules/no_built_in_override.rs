//! Rule `no-built-in-override` (SonarJS key S2424).
//!
//! Clean-room port. Overriding or shadowing a standard ECMAScript built-in
//! global object or function is dangerous and confusing. Code that relies on a
//! built-in being available (e.g. `Array.isArray`, `Object.keys`) will break
//! silently if an outer scope has rebound the name to something else.
//!
//! ## Standard built-in globals
//!
//! The rule targets the following standard ECMAScript built-in global names:
//!
//! `Object`, `Function`, `Boolean`, `Symbol`, `Error`, `EvalError`,
//! `RangeError`, `ReferenceError`, `SyntaxError`, `TypeError`, `URIError`,
//! `Number`, `BigInt`, `Math`, `Date`, `String`, `RegExp`, `Array`, `Map`,
//! `Set`, `WeakMap`, `WeakSet`, `Promise`, `Proxy`, `Reflect`, `JSON`,
//! `ArrayBuffer`, `DataView`, `Infinity`, `NaN`, `undefined`, `globalThis`,
//! `parseInt`, `parseFloat`, `isNaN`, `isFinite`, `decodeURI`,
//! `decodeURIComponent`, `encodeURI`, `encodeURIComponent`.
//!
//! ## Detection strategy
//!
//! Two paths are covered:
//!
//! 1. **Binding declarations** (`visit_binding_identifier`): catches `let`,
//!    `const`, `var`, function declarations, class declarations, function
//!    parameters, and destructuring bindings whose name matches a built-in.
//!    E.g. `let Object = 1`, `function Array() {}`, `class Map {}`,
//!    `function f(Promise) {}`.
//!
//! 2. **Simple assignments** (`visit_assignment_expression`): catches bare
//!    assignments whose left-hand side is a plain identifier that matches a
//!    built-in.  E.g. `Array = 2`. Member-expression targets such as
//!    `Math.PI = 3` are intentionally **not** reported — only bare identifier
//!    targets are flagged.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{AssignmentExpression, AssignmentTarget, BindingIdentifier};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-built-in-override";

const BUILTINS: [&str; 40] = [
    "Array",
    "ArrayBuffer",
    "BigInt",
    "Boolean",
    "DataView",
    "Date",
    "Error",
    "EvalError",
    "Function",
    "Infinity",
    "JSON",
    "Map",
    "Math",
    "NaN",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "Reflect",
    "RegExp",
    "Set",
    "String",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "URIError",
    "WeakMap",
    "WeakSet",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "globalThis",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "undefined",
];

fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

impl Scanner<'_> {
    pub(crate) fn check_no_built_in_override_binding(&mut self, id: &BindingIdentifier<'_>) {
        if !is_builtin(id.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noBuiltInOverride", id.span);
    }

    pub(crate) fn check_no_built_in_override_assignment(
        &mut self,
        assign: &AssignmentExpression<'_>,
    ) {
        let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left else {
            return;
        };
        if !is_builtin(id.name.as_str()) {
            return;
        }
        self.report(RULE_NAME, "noBuiltInOverride", id.span);
    }
}
