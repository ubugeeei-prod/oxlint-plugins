//! Rule `bool-param-default` (SonarJS key S4798).
//!
//! Clean-room port. An optional boolean parameter that has no default value is
//! ambiguous: the function body then has to distinguish three states — `true`,
//! `false`, and "argument not provided" (`undefined`). Giving the parameter a
//! default value (`param: boolean = false`) collapses that to two states and
//! removes the ambiguity at every call site.
//!
//! This is a purely syntactic, zero-false-positive TypeScript-AST check. A
//! [`FormalParameter`] is flagged only when ALL of the following hold:
//! - Its binding is a plain `BindingIdentifier` (NOT a destructuring or rest
//!   pattern).
//! - It is declared optional (the `?` marker is present, `optional == true`).
//! - It has no initializer (guaranteed by `optional` in valid TS, but checked
//!   defensively).
//! - Its type annotation is present and is exactly the bare `boolean` keyword
//!   (`TSType::TSBooleanKeyword`) — NOT a union such as `boolean | undefined`,
//!   NOT an array `boolean[]`, NOT a type reference.
//!
//! The check is driven only from function-bearing nodes that can actually
//! carry a default value: concrete [`Function`]s (`body.is_some()`) and arrow
//! functions. This deliberately excludes parameters of *type-only* signatures,
//! where a default value is syntactically impossible and the suggested fix
//! ("give it a default") could never be applied — flagging them would be a
//! pure false positive. Excluded constructs are:
//! - interface method signatures (`TSMethodSignature`),
//! - type-alias function types (`TSFunctionType`) and type-literal methods,
//! - call/construct signatures (`TSCallSignatureDeclaration`, etc.),
//! - `abstract` methods (a `Function` with `body == None`),
//! - function overload signatures (a `Function` with `body == None`),
//! - `declare`d/ambient functions (a `Function` with `body == None`).
//!
//! These are skipped automatically: type signatures are distinct AST nodes that
//! are never iterated here, and bodiless `Function`s are gated out by the
//! `body.is_some()` check at the call sites.
//!
//! Behaviour is reproduced from the public SonarSource rule documentation
//! (S4798) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! ## Flagged
//! - `function f(flag?: boolean) {}`
//! - `const g = (flag?: boolean) => {};`
//! - `class C { m(flag?: boolean) {} }`
//!
//! ## Not flagged
//! - `function f(flag: boolean) {}` — required, unambiguous
//! - `function f(flag: boolean = false) {}` — already has a default
//! - `function f(flag?: boolean | undefined) {}` — union annotation
//! - `function f(flag?: SomeBool) {}` — type reference, not the keyword
//! - `function f(flag?) {}` — untyped optional parameter
//! - `function f({ flag }?: { flag: boolean }) {}` — destructuring pattern
//! - `interface I { m(flag?: boolean): void; }` — type-only signature
//! - `type T = (flag?: boolean) => void;` — function type, no body possible
//! - `type O = { m(flag?: boolean): void };` — type-literal method signature
//! - `abstract class C { abstract m(flag?: boolean): void; }` — abstract method
//! - `function f(flag?: boolean): void;` (overload signature, no body)
//! - `declare function f(flag?: boolean): void;` — ambient declaration

use oxc_ast::ast::{BindingPattern, FormalParameter, FormalParameters, TSType};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "bool-param-default";

impl Scanner<'_> {
    /// Checks every parameter of a concrete function/method that can take a
    /// default value. Call sites must ensure a body is present (i.e. this is
    /// NOT an overload/abstract/ambient signature, and not a type-only node).
    pub(crate) fn check_bool_param_default(&mut self, params: &FormalParameters<'_>) {
        for param in &params.items {
            self.check_bool_param_default_param(param);
        }
    }

    /// Flags an optional, bare-`boolean`-typed parameter that has no default.
    fn check_bool_param_default_param(&mut self, param: &FormalParameter<'_>) {
        if !param.optional {
            return;
        }
        // Defensive: an optional parameter cannot carry a default in valid TS,
        // but never flag a parameter that already has one.
        if param.initializer.is_some() {
            return;
        }
        // Only plain identifier bindings; destructuring/rest patterns are out
        // of scope.
        if !matches!(param.pattern, BindingPattern::BindingIdentifier(_)) {
            return;
        }
        let Some(annotation) = &param.type_annotation else {
            return;
        };
        if !matches!(annotation.type_annotation, TSType::TSBooleanKeyword(_)) {
            return;
        }
        self.report(RULE_NAME, "boolParamDefault", param.span);
    }
}
