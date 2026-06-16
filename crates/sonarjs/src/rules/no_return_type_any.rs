//! Rule `no-return-type-any` (SonarJS key S4324).
//!
//! Clean-room port. Reports a function, method, or arrow function whose
//! EXPLICIT return type annotation is exactly the `any` type. Declaring `any`
//! as the return type discards type safety for every caller of the function,
//! so a more specific type (or `unknown`) should be declared instead.
//!
//! Zero-false-positive syntactic subset: the rule only flags a return-type
//! annotation that is the bare `any` keyword (`: any`). It deliberately does
//! NOT flag:
//!   - composite types that merely contain `any`, such as `any[]`,
//!     `Promise<any>`, `Record<string, any>`, or a union like `any | string`
//!     (these are distinct annotations and flagging them would over-report);
//!   - functions with no return-type annotation at all (a function whose
//!     return type is inferred is a different concern handled elsewhere).
//!
//! Flagged:
//!   - `function f(): any { return x; }`
//!   - `const f = (): any => x;`
//!   - `class C { m(): any { return x; } }`
//!
//! Not flagged:
//!   - `function f(): number { return 1; }`
//!   - `function f(): any[] { return []; }`
//!   - `function f(): Promise<any> { return p; }`
//!   - `function f() { return x; }` (no annotation)
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{ArrowFunctionExpression, Function, TSType, TSTypeAnnotation};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-return-type-any";

impl Scanner<'_> {
    pub(crate) fn check_no_return_type_any_function(&mut self, it: &Function<'_>) {
        self.report_if_any_return_type(it.return_type.as_deref());
    }

    pub(crate) fn check_no_return_type_any_arrow(&mut self, it: &ArrowFunctionExpression<'_>) {
        self.report_if_any_return_type(it.return_type.as_deref());
    }

    /// Reports the annotation span when the return-type annotation is exactly
    /// the bare `any` keyword. Composite types containing `any` and absent
    /// annotations are intentionally left untouched.
    fn report_if_any_return_type(&mut self, return_type: Option<&TSTypeAnnotation<'_>>) {
        let Some(annotation) = return_type else {
            return;
        };
        if matches!(annotation.type_annotation, TSType::TSAnyKeyword(_)) {
            self.report(RULE_NAME, "noReturnTypeAny", annotation.span);
        }
    }
}
