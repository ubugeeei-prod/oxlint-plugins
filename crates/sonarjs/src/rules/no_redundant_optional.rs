//! Rule `no-redundant-optional` (SonarJS key S4782).
//!
//! Clean-room port. A TypeScript property declared optional with `?` AND whose
//! type annotation already includes the `undefined` keyword is redundant — the
//! `?` marker already permits `undefined`. Flag the property signature so the
//! developer can remove the explicit `undefined` from the type.
//!
//! **Scope**: `TSPropertySignature` nodes only — property signatures inside
//! interfaces and object type literals. Optional function parameters
//! (`x?: T | undefined`) and optional class fields are **out of scope** for
//! this rule; those shapes require type-aware analysis and are left for a
//! follow-up.
//!
//! **Trigger**: a `TSPropertySignature` where ALL of:
//! 1. `optional == true` (the `?` marker is present).
//! 2. The property has a `type_annotation`.
//! 3. That annotation's type IS `undefined` directly, OR is a union that
//!    contains `undefined` as one of its members.
//!
//! **DO flag**: `interface I { a?: string | undefined; }`,
//! `interface I { b?: undefined; }`,
//! `interface I { c?: number | string | undefined; }`.
//!
//! **DON'T flag**: `interface I { a?: string; }` (no `undefined` in type),
//! `interface I { b: string | undefined; }` (not optional — no `?`),
//! `interface I { c?: string | null; }` (`null`, not `undefined`).
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{TSPropertySignature, TSType};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-redundant-optional";

/// Returns `true` when `ty` is `undefined` directly or is a union that
/// contains `undefined` as one of its member types.
fn type_has_undefined(ty: &TSType) -> bool {
    match ty {
        TSType::TSUndefinedKeyword(_) => true,
        TSType::TSUnionType(union) => union
            .types
            .iter()
            .any(|t| matches!(t, TSType::TSUndefinedKeyword(_))),
        _ => false,
    }
}

impl Scanner<'_> {
    pub(crate) fn check_no_redundant_optional(&mut self, prop: &TSPropertySignature<'_>) {
        if !prop.optional {
            return;
        }
        let Some(annotation) = &prop.type_annotation else {
            return;
        };
        if !type_has_undefined(&annotation.type_annotation) {
            return;
        }
        self.report(RULE_NAME, "redundantOptional", prop.span);
    }
}
