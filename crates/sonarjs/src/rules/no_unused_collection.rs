//! Rule `no-unused-collection` (SonarJS key S4030).
//!
//! Clean-room port. A collection (array, object, Map, Set, etc.) that is only
//! ever written to but whose contents are never read is likely a bug: data is
//! accumulated but never consumed.
//!
//! Only `const`/`let` bindings with a plain `BindingIdentifier` and a
//! recognisable collection initializer (`[]`, `{}`, `new Map()`, `new Set()`,
//! `new Array()`, `new WeakMap()`, `new WeakSet()`) are considered. Every
//! resolved reference to the binding is classified:
//! - **Write-only**: the binding is the object of a mutating method call
//!   (`push`, `pop`, `set`, `add`, etc.) whose result is discarded (the call
//!   is a bare `ExpressionStatement`), or it is the object of a plain `=`
//!   static or computed member assignment target.
//! - **Read/expose**: anything else (passed as an argument, returned, read via
//!   `.length`/`.get`/`.has`, iterated, etc.).
//!
//! The declaration is flagged only when there is at least one write-only
//! reference and zero read/expose references.  When semantic information is
//! unavailable nothing is reported (conservative — no false positives).
//!
//! Behaviour is reproduced from the public SonarSource rule documentation
//! (S4030) only; no upstream source, tests, fixtures, or message strings were
//! consulted or copied.
//!
//! ## Flagged
//! - `const a = []; a.push(1); a.push(2);` — only written to
//! - `const m = new Map(); m.set('k', 1);` — only written to
//!
//! ## Not flagged
//! - `const a = []; a.push(1); return a;` — returned (read)
//! - `const a = []; a.push(1); console.log(a);` — passed to a function (read)
//! - `const a = []; a.push(1); const b = a.length;` — `.length` is read
//! - `const a = [1, 2]; foo(a);` — passed to a function

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AssignmentTarget, BindingPattern, Expression, VariableDeclarationKind, VariableDeclarator,
};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::GetSpan;
use oxc_syntax::operator::AssignmentOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-unused-collection";

/// Returns `true` when `name` is a collection-mutating method that does not
/// expose the collection's contents when its return value is discarded.
fn is_write_method(name: &str) -> bool {
    matches!(
        name,
        "push"
            | "pop"
            | "shift"
            | "unshift"
            | "splice"
            | "fill"
            | "copyWithin"
            | "sort"
            | "reverse"
            | "set"
            | "add"
            | "delete"
            | "clear"
    )
}

/// Returns `true` when `init` is a recognised collection initializer.
fn is_collection_init(init: &Expression<'_>) -> bool {
    match init.get_inner_expression() {
        Expression::ArrayExpression(_) | Expression::ObjectExpression(_) => true,
        Expression::NewExpression(new_expr) => {
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

/// Returns `true` when the reference identified by `ref_node_id` is
/// **write-only** — it does not expose the collection's contents.
///
/// Conservative: any reference that cannot be confidently classified as
/// write-only is treated as a read, suppressing the report.
fn is_write_ref(nodes: &AstNodes<'_>, ref_node_id: NodeId) -> bool {
    match nodes.parent_kind(ref_node_id) {
        AstKind::StaticMemberExpression(sme) => {
            let parent_node_id = nodes.parent_id(ref_node_id);
            match nodes.parent_kind(parent_node_id) {
                AstKind::CallExpression(call) => {
                    // Confirm the SME is the callee, not an argument position.
                    if call.callee.span() != sme.span {
                        return false;
                    }
                    // Require the call result to be discarded: if the return
                    // value is consumed (chaining, assignment, argument) the
                    // collection contents may be exposed indirectly.
                    let gp_node_id = nodes.parent_id(parent_node_id);
                    if !matches!(
                        nodes.parent_kind(gp_node_id),
                        AstKind::ExpressionStatement(_)
                    ) {
                        return false;
                    }
                    is_write_method(sme.property.name.as_str())
                }
                AstKind::AssignmentExpression(asgn) => {
                    // X.prop = val — only a plain `=` is a pure write; compound
                    // operators (+=, etc.) read the element first.
                    if asgn.operator != AssignmentOperator::Assign {
                        return false;
                    }
                    if let AssignmentTarget::StaticMemberExpression(lhs) = &asgn.left {
                        lhs.span == sme.span
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        AstKind::ComputedMemberExpression(cme) => {
            let parent_node_id = nodes.parent_id(ref_node_id);
            match nodes.parent_kind(parent_node_id) {
                AstKind::AssignmentExpression(asgn) => {
                    // X[i] = val — only a plain `=` is a pure write.
                    if asgn.operator != AssignmentOperator::Assign {
                        return false;
                    }
                    if let AssignmentTarget::ComputedMemberExpression(lhs) = &asgn.left {
                        lhs.span == cme.span
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

impl<'a> Scanner<'a> {
    /// Checks whether a `const`/`let` collection binding is populated but
    /// never read.  Reports at the `VariableDeclarator` span.
    pub(crate) fn check_unused_collection(&mut self, it: &VariableDeclarator<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            return;
        }
        let BindingPattern::BindingIdentifier(ident) = &it.id else {
            return;
        };
        let Some(init) = &it.init else {
            return;
        };
        if !is_collection_init(init) {
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

        let mut write_count: u32 = 0;
        for reference in scoping.get_resolved_references(symbol_id) {
            if !reference.is_value() {
                continue;
            }
            if is_write_ref(nodes, reference.node_id()) {
                write_count += 1;
            } else {
                // Any read reference means the collection is used.
                return;
            }
        }

        if write_count > 0 {
            self.report(RULE_NAME, "unusedCollection", it.span);
        }
    }
}
