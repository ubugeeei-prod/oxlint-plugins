//! Rule `no-empty-collection` (SonarJS key S4158).
//!
//! Clean-room port. This is the inverse of `no-unused-collection`: a collection
//! that is declared, read from (queried, iterated, indexed, `.length`/`.size`
//! accessed, etc.) but is **never populated** with any element is dead/buggy
//! code — every read observes an empty collection.
//!
//! Only `const`/`let` bindings with a plain `BindingIdentifier` and a
//! recognisable **empty** collection initializer are considered:
//! - `[]`  (an `ArrayExpression` with zero elements)
//! - `new Map()` / `new Set()` / `new Array()` / `new WeakMap()` /
//!   `new WeakSet()` with zero arguments
//!
//! If the initializer has any initial element/argument (`[1]`, `new Array(5)`)
//! it is not empty and is skipped.
//!
//! ## Handling of `{}` (object literals) — EXCLUDED
//! Object literals are deliberately **not** considered. Object literals are
//! pervasively used as plain records, namespaces, and config bags that are
//! populated in many hard-to-track ways (`Object.assign`, spread merges,
//! dynamic/computed keys, helper functions). Including `{}` would risk false
//! positives, so to honour the zero-false-positive mandate we restrict this
//! rule to array-like collections only. Correctness is preferred over coverage.
//!
//! Every resolved value-reference to the binding is classified:
//! - **Write (populates)**: object of a populating mutating method call
//!   (`push`, `unshift`, `splice`, `set`, `add`, `fill`, `copyWithin`), or the
//!   object of a plain `=` static/computed member assignment target
//!   (`a[i] = x`, `a.prop = x`).
//! - **Read**: a curated set of pure query/iteration member accesses
//!   (`.length`, `.size`, `.get`, `.has`, `.includes`, iteration via `for..of`
//!   and spread, an index/property read, etc.).
//! - **Ambiguous**: anything else (passed as a call argument, object of an
//!   unknown method, aliased via assignment, returned, captured by a closure,
//!   compound assignment, etc.) — a function could populate it.
//!
//! The declaration is flagged only when there is **at least one read** AND
//! **zero writes** AND **zero ambiguous** references. Any ambiguous reference
//! is treated as a potential populate and suppresses the report. When semantic
//! information is unavailable nothing is reported. This guarantees
//! under-reporting / zero false positives.
//!
//! Behaviour is reproduced from the public SonarSource rule documentation
//! (S4158) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! ## Flagged
//! - `const a = []; return a.length;` — read but never populated
//! - `const m = new Map(); if (m.has(k)) {}` — queried but never populated
//!
//! ## Not flagged
//! - `const a = []; a.push(1); return a.length;` — populated via `push`
//! - `const a = []; fill(a); return a.length;` — passed somewhere (ambiguous)
//! - `const a = [1]; return a.length;` — not initially empty

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AssignmentTarget, BindingPattern, Expression, VariableDeclarationKind, VariableDeclarator,
};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-empty-collection";

/// How a single reference to the collection binding is interpreted.
enum RefKind {
    /// Populates the collection with elements.
    Write,
    /// Reads / queries / iterates the collection without populating it.
    Read,
    /// Cannot be confidently classified; could populate the collection.
    Ambiguous,
}

/// Returns `true` when `name` is a method that populates the collection with
/// new elements when called.
fn is_populating_method(name: &str) -> bool {
    matches!(
        name,
        "push" | "unshift" | "splice" | "set" | "add" | "fill" | "copyWithin"
    )
}

/// Returns `true` when `name` is a pure query/read member (property or method)
/// that observes the collection's contents without populating it.
fn is_reading_member(name: &str) -> bool {
    matches!(
        name,
        "length"
            | "size"
            | "get"
            | "has"
            | "includes"
            | "indexOf"
            | "lastIndexOf"
            | "find"
            | "findIndex"
            | "findLast"
            | "findLastIndex"
            | "forEach"
            | "map"
            | "filter"
            | "reduce"
            | "reduceRight"
            | "some"
            | "every"
            | "join"
            | "slice"
            | "at"
            | "keys"
            | "values"
            | "entries"
            | "flat"
            | "flatMap"
            | "concat"
            | "toString"
    )
}

/// Returns `true` when `init` is a recognised **empty** array-like collection
/// initializer (object literals are intentionally excluded — see module docs).
fn is_empty_collection_init(init: &Expression<'_>) -> bool {
    match init.get_inner_expression() {
        Expression::ArrayExpression(arr) => arr.elements.is_empty(),
        Expression::NewExpression(new_expr) => {
            if !new_expr.arguments.is_empty() {
                return false;
            }
            let Expression::Identifier(callee) = new_expr.callee.get_inner_expression() else {
                return false;
            };
            matches!(
                callee.name.as_str(),
                "Map" | "Set" | "Array" | "WeakMap" | "WeakSet"
            )
        }
        _ => false,
    }
}

/// Classifies the reference identified by `ref_node_id` as a write (populate),
/// a read, or ambiguous. Anything that cannot be confidently proven to be a
/// pure read or a pure populate is treated as ambiguous (suppresses report).
fn classify_ref(nodes: &AstNodes<'_>, ref_node_id: NodeId) -> RefKind {
    let ref_span = nodes.get_node(ref_node_id).kind().span();
    match nodes.parent_kind(ref_node_id) {
        AstKind::StaticMemberExpression(sme) => {
            // Only the object position refers to the collection itself.
            if sme.object.span() != ref_span {
                return RefKind::Ambiguous;
            }
            let prop = sme.property.name.as_str();
            let parent_id = nodes.parent_id(ref_node_id);

            if is_populating_method(prop) {
                // Treat as a populate only when actually invoked as the callee.
                if let AstKind::CallExpression(call) = nodes.parent_kind(parent_id)
                    && call.callee.span() == sme.span
                {
                    return RefKind::Write;
                }
                return RefKind::Ambiguous;
            }

            // Plain `=` assignment to a static member populates the collection.
            if let AstKind::AssignmentExpression(asgn) = nodes.parent_kind(parent_id) {
                if asgn.operator == AssignmentOperator::Assign
                    && let AssignmentTarget::StaticMemberExpression(lhs) = &asgn.left
                    && lhs.span == sme.span
                {
                    return RefKind::Write;
                }
                // Compound assignment reads then writes — ambiguous.
                return RefKind::Ambiguous;
            }

            if is_reading_member(prop) {
                return RefKind::Read;
            }

            RefKind::Ambiguous
        }
        AstKind::ComputedMemberExpression(cme) => {
            // Only the object position refers to the collection itself
            // (the reference could instead be the index expression `b[a]`).
            if cme.object.span() != ref_span {
                return RefKind::Ambiguous;
            }
            let parent_id = nodes.parent_id(ref_node_id);
            if let AstKind::AssignmentExpression(asgn) = nodes.parent_kind(parent_id) {
                if asgn.operator == AssignmentOperator::Assign
                    && let AssignmentTarget::ComputedMemberExpression(lhs) = &asgn.left
                    && lhs.span == cme.span
                {
                    return RefKind::Write;
                }
                return RefKind::Ambiguous;
            }
            // Reading `a[i]`.
            RefKind::Read
        }
        AstKind::ForOfStatement(stmt) => {
            // `for (... of a)` iterates the collection (a read).
            if stmt.right.span() == ref_span {
                RefKind::Read
            } else {
                RefKind::Ambiguous
            }
        }
        AstKind::SpreadElement(spread) => {
            // `[...a]` / `f(...a)` iterates the collection (a read).
            if spread.argument.span() == ref_span {
                RefKind::Read
            } else {
                RefKind::Ambiguous
            }
        }
        _ => RefKind::Ambiguous,
    }
}

impl<'a> Scanner<'a> {
    /// Checks whether a `const`/`let` collection binding is read from but never
    /// populated. Reports at the `VariableDeclarator` span.
    pub(crate) fn check_empty_collection(&mut self, it: &VariableDeclarator<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            return;
        }
        let BindingPattern::BindingIdentifier(ident) = &it.id else {
            return;
        };
        let Some(init) = &it.init else {
            return;
        };
        if !is_empty_collection_init(init) {
            return;
        }
        let Some(scoping) = self.scoping else {
            return;
        };
        let Some(nodes) = self.nodes else {
            return;
        };
        let Some(symbol_id) = ident.symbol_id.get() else {
            return;
        };

        let mut read_count: u32 = 0;
        for reference in scoping.get_resolved_references(symbol_id) {
            if !reference.is_value() {
                continue;
            }
            match classify_ref(nodes, reference.node_id()) {
                // Populated, or possibly populated → never report.
                RefKind::Write | RefKind::Ambiguous => return,
                RefKind::Read => read_count += 1,
            }
        }

        if read_count > 0 {
            self.report(RULE_NAME, "emptyCollection", it.span);
        }
    }
}
