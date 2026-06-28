//! Rule `prefer-read-only-props` (SonarJS key S6759).
//!
//! Clean-room port. The props object passed to a React function component is
//! owned by React and must never be mutated by the component; declaring the
//! props type as read-only makes that contract explicit and lets the compiler
//! reject any accidental write. SonarJS therefore flags a React function
//! component whose props parameter is typed with a *mutable* type.
//!
//! ## Narrow form
//!
//! Faithfully reproducing the full rule requires the TypeScript type checker —
//! resolving an arbitrary props type (a named `interface`, a generic alias, an
//! intersection, an imported type, …) to decide whether *every* member is
//! read-only is impossible from syntax alone. This port implements the
//! unambiguous, configuration- and type-checker-independent core, which
//! guarantees no false positives:
//!
//! A function is treated as a React component only when its *own* body (not the
//! body of any nested function/arrow) syntactically contains JSX — a
//! `JSXElement` or `JSXFragment`. For such a component, the FIRST parameter's
//! type annotation is examined, and the component is flagged only when that
//! annotation is an **inline object type literal** (`{ … }`) that contains at
//! least one property signature WITHOUT the `readonly` modifier.
//!
//! ```tsx
//! function Welcome(props: { name: string }) {            // Noncompliant
//!   return <h1>Hello, {props.name}</h1>;
//! }
//! function Welcome(props: { readonly name: string }) {   // Compliant
//!   return <h1>Hello, {props.name}</h1>;
//! }
//! function Welcome(props: Readonly<{ name: string }>) {  // Compliant
//!   return <h1>Hello, {props.name}</h1>;
//! }
//! ```
//!
//! Deliberately OUT of scope (the port under-reports rather than risk a false
//! positive):
//! - props typed by a named reference (`props: MyProps`) or any non-literal
//!   type (intersection, mapped, `Readonly<…>`, …) — these need type
//!   resolution and are never flagged;
//! - JSX produced only by a nested callback or nested component — the JSX
//!   search stops at every nested function/arrow boundary, so a factory
//!   function that merely *returns* an inner component is not mistaken for a
//!   component itself;
//! - functions that are not components (no JSX in their own body).
//!
//! Both concrete functions/methods (`visit_function`) and arrow functions
//! (`visit_arrow_function_expression`) are covered; bodiless functions
//! (overloads, abstract/ambient signatures) have no body and are skipped.
//!
//! Behaviour is reproduced from the public RSPEC description (S6759) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{
    ArrowFunctionExpression, FormalParameters, Function, FunctionBody, JSXElement, JSXFragment,
    TSSignature, TSType,
};
use oxc_ast_visit::Visit;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-read-only-props";

/// Returns the span of an inline object type literal that has at least one
/// property signature lacking the `readonly` modifier, or `None` for any other
/// type (named reference, `Readonly<…>`, intersection, all-readonly literal,
/// literal with no property signatures, …).
fn mutable_object_literal_span(ty: &TSType) -> Option<Span> {
    let TSType::TSTypeLiteral(lit) = ty else {
        return None;
    };
    let has_mutable_property = lit
        .members
        .iter()
        .any(|member| matches!(member, TSSignature::TSPropertySignature(prop) if !prop.readonly));
    has_mutable_property.then_some(lit.span)
}

/// Visitor that records whether JSX appears in the immediate function scope. It
/// descends through statements and expressions but stops at every nested
/// function/arrow boundary, so only JSX belonging to the visited body itself is
/// detected.
struct JsxFinder {
    found: bool,
}

impl<'a> Visit<'a> for JsxFinder {
    fn visit_jsx_element(&mut self, _it: &JSXElement<'a>) {
        self.found = true;
    }

    fn visit_jsx_fragment(&mut self, _it: &JSXFragment<'a>) {
        self.found = true;
    }

    // Do not descend into nested functions/arrows: their JSX belongs to a
    // different (possibly inner) component, not to the body being inspected.
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {}
}

fn body_contains_jsx<'a>(body: &FunctionBody<'a>) -> bool {
    let mut finder = JsxFinder { found: false };
    finder.visit_function_body(body);
    finder.found
}

impl<'a> Scanner<'a> {
    /// Entry for concrete functions and methods. Bodiless nodes (overloads,
    /// abstract/ambient signatures) have no body and are skipped.
    pub(crate) fn check_prefer_read_only_props(&mut self, func: &Function<'a>) {
        let Some(body) = &func.body else {
            return;
        };
        self.report_prefer_read_only_props(&func.params, body);
    }

    /// Entry for arrow functions, which always have a body (block or
    /// expression).
    pub(crate) fn check_prefer_read_only_props_arrow(
        &mut self,
        arrow: &ArrowFunctionExpression<'a>,
    ) {
        self.report_prefer_read_only_props(&arrow.params, &arrow.body);
    }

    fn report_prefer_read_only_props(
        &mut self,
        params: &FormalParameters<'a>,
        body: &FunctionBody<'a>,
    ) {
        // Inspect only the first parameter — the props object of a component.
        let Some(first) = params.items.first() else {
            return;
        };
        let Some(annotation) = &first.type_annotation else {
            return;
        };
        let Some(span) = mutable_object_literal_span(&annotation.type_annotation) else {
            return;
        };
        // Only treat the function as a component if its own body renders JSX.
        if !body_contains_jsx(body) {
            return;
        }
        self.report(RULE_NAME, "readonlyProps", span);
    }
}
