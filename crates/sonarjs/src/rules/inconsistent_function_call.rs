//! Rule `inconsistent-function-call` (SonarJS key S3686).
//!
//! Clean-room port. Flags a function that is invoked both as a plain function
//! call (`f(...)`) and as a constructor (`new f(...)`), indicating confusion
//! about whether the function is meant to be a constructor.
//!
//! Only identifier callees that resolve to a locally-declared function or
//! arrow function symbol are tracked. Unresolved references and
//! member-expression callees are ignored.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//! - `function f() {} f(); new f();` — called as both plain and constructor
//! - `const f = () => {}; f(); new f();` — arrow called both ways
//!
//! ## Not flagged
//! - `function f() {} f(); f();` — only called as plain function
//! - `function f() {} new f(); new f();` — only called as constructor
//! - `obj.method(); new obj.method();` — member-expression callee

use oxc_ast::AstKind;
use oxc_ast::ast::{CallExpression, Expression, NewExpression};
use oxc_semantic::{AstNodes, Scoping, SymbolId};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "inconsistent-function-call";

/// Returns `true` if the symbol with `symbol_id` is declared as a function
/// declaration or as a function/arrow expression initializer in the current
/// file.
fn is_function_declaration<'a>(
    scoping: &Scoping,
    nodes: &AstNodes<'a>,
    symbol_id: SymbolId,
) -> bool {
    let decl = scoping.symbol_declaration(symbol_id);
    match nodes.get_node(decl).kind() {
        AstKind::Function(_) => true,
        AstKind::VariableDeclarator(declarator) => {
            if let Some(init) = &declarator.init {
                return matches!(
                    init,
                    Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                );
            }
            false
        }
        _ => false,
    }
}

impl<'a> Scanner<'a> {
    /// Records that an identifier was used as a plain call callee; called from
    /// `visit_call_expression`.
    pub(crate) fn record_call_inconsistent_function_call(&mut self, it: &CallExpression<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let Expression::Identifier(ident) = &it.callee else {
            return;
        };
        let Some(scoping) = self.scoping else { return };
        let Some(nodes) = self.nodes else { return };
        let Some(reference_id) = ident.reference_id.get() else {
            return;
        };
        let Some(symbol_id) = scoping.get_reference(reference_id).symbol_id() else {
            return;
        };
        if !is_function_declaration(scoping, nodes, symbol_id) {
            return;
        }
        let pos = self
            .fn_call_new_records
            .iter()
            .position(|e| e.0 == symbol_id);
        if let Some(i) = pos {
            self.fn_call_new_records[i].1 = true;
        } else {
            self.fn_call_new_records
                .push((symbol_id, true, false, it.span));
        }
    }

    /// Records that an identifier was used as a constructor (`new`) callee;
    /// called from `visit_new_expression`.
    pub(crate) fn record_new_inconsistent_function_call(&mut self, it: &NewExpression<'a>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let Expression::Identifier(ident) = &it.callee else {
            return;
        };
        let Some(scoping) = self.scoping else { return };
        let Some(nodes) = self.nodes else { return };
        let Some(reference_id) = ident.reference_id.get() else {
            return;
        };
        let Some(symbol_id) = scoping.get_reference(reference_id).symbol_id() else {
            return;
        };
        if !is_function_declaration(scoping, nodes, symbol_id) {
            return;
        }
        let pos = self
            .fn_call_new_records
            .iter()
            .position(|e| e.0 == symbol_id);
        if let Some(i) = pos {
            self.fn_call_new_records[i].2 = true;
        } else {
            self.fn_call_new_records
                .push((symbol_id, false, true, it.span));
        }
    }

    /// Called from `visit_program` after the AST walk; reports any symbol that
    /// was invoked both as a plain call and as a constructor.
    pub(crate) fn finalize_inconsistent_function_call(&mut self) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let mut to_report: SmallVec<[Span; 4]> = SmallVec::new();
        for i in 0..self.fn_call_new_records.len() {
            if self.fn_call_new_records[i].1 && self.fn_call_new_records[i].2 {
                to_report.push(self.fn_call_new_records[i].3);
            }
        }
        for span in to_report {
            self.report(RULE_NAME, "inconsistentFunctionCall", span);
        }
    }
}
