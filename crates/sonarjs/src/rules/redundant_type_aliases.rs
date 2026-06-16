//! Rule `redundant-type-aliases` (SonarJS key S6564).
//!
//! Clean-room port. Reports a TypeScript type alias declaration whose
//! right-hand side merely renames an existing type without adding any new
//! information, because such an alias provides no extra meaning and only adds
//! an indirection that readers must resolve.
//!
//! An alias is considered redundant when its right-hand side is EITHER:
//!   (a) a primitive / built-in keyword type (e.g. `string`, `number`,
//!       `boolean`, `bigint`, `symbol`, `object`, `any`, `unknown`, `never`,
//!       `void`, `null`, `undefined`); OR
//!   (b) a bare type reference with no type arguments (e.g. `type X = Y`),
//!       which simply renames another named type or alias.
//!
//! To stay zero-false-positive the rule never flags an alias that declares its
//! own type parameters (e.g. `type Box<T> = T`), since such an alias is a
//! genuine generic abstraction rather than a plain rename. Composite types
//! (unions, intersections), object / tuple / function / array / literal types,
//! and type references that carry type arguments (`Array<string>`, `Foo<T>`)
//! are likewise never flagged because they add information beyond a rename.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{TSType, TSTypeAliasDeclaration};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "redundant-type-aliases";

impl Scanner<'_> {
    pub(crate) fn check_redundant_type_aliases(&mut self, it: &TSTypeAliasDeclaration<'_>) {
        // A generic alias (one that declares its own type parameters) is a real
        // abstraction, not a plain rename, so it is never redundant.
        if it.type_parameters.is_some() {
            return;
        }

        let redundant = match &it.type_annotation {
            // (a) primitive / built-in keyword types.
            TSType::TSStringKeyword(_)
            | TSType::TSNumberKeyword(_)
            | TSType::TSBooleanKeyword(_)
            | TSType::TSBigIntKeyword(_)
            | TSType::TSSymbolKeyword(_)
            | TSType::TSObjectKeyword(_)
            | TSType::TSAnyKeyword(_)
            | TSType::TSUnknownKeyword(_)
            | TSType::TSNeverKeyword(_)
            | TSType::TSVoidKeyword(_)
            | TSType::TSNullKeyword(_)
            | TSType::TSUndefinedKeyword(_) => true,
            // (b) a bare type reference (alias-to-alias / alias-to-named-type)
            //     with no type arguments.
            TSType::TSTypeReference(reference) => reference.type_arguments.is_none(),
            _ => false,
        };

        if redundant {
            self.report(RULE_NAME, "redundantTypeAlias", it.span);
        }
    }
}
