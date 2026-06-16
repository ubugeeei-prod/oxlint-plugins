//! Rule `xpath` (SonarJS key S4817).
//!
//! Clean-room port. Behaviour is reproduced from the public RSPEC S4817
//! description ("Executing XPath expressions is security-sensitive") only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! S4817 is a *security hotspot*, not a deterministic bug: evaluating an XPath
//! expression that is built from user-controlled input enables XPath injection,
//! so each flagged call is a spot that must be reviewed to confirm the
//! expression is validated or sanitised. The rule does not attempt taint
//! tracking; it surfaces the distinctive XPath-execution call shapes for review.
//!
//! This implements the zero-false-positive subset. A `CallExpression` is
//! flagged when its callee (after unwrapping parentheses via
//! `get_inner_expression`) is a `StaticMemberExpression` matching ANY of:
//!
//! - `document.evaluate(...)` — object is the `Identifier` `document` and the
//!   property is `evaluate`. The DOM Level 3 XPath entry point.
//! - `*.selectNodes(...)` — property `selectNodes`, any receiver. The name is
//!   distinctive to MSXML / IE XML DOM XPath execution.
//! - `*.SelectSingleNode(...)` / `*.selectSingleNode(...)` — property
//!   `SelectSingleNode` or `selectSingleNode`, any receiver. Likewise
//!   distinctive to MSXML.
//! - `xpath.select(...)` / `xpath.select1(...)` — object is the `Identifier`
//!   `xpath` and the property is `select` or `select1`. The `xpath` npm package
//!   entry points.
//!
//! Why `evaluate`, `select` and `select1` are *receiver-gated*: those method
//! names are generic and heavily reused outside XPath (e.g. a SQL builder's
//! `.select(...)`, D3's `select`/`select1` selectors, or a math library's
//! `.evaluate(...)`). Gating on the well-known XPath receivers `document` and
//! `xpath` keeps the rule zero-false-positive. By contrast `selectNodes`,
//! `SelectSingleNode` and `selectSingleNode` are distinctive enough to flag on
//! any receiver.
//!
//! **Flagged**:
//! - `document.evaluate(userinput, xmlDoc, null, XPathResult.ANY_TYPE, null)`
//! - `xmlDoc.selectNodes(userinput)`
//! - `xmlDoc.SelectSingleNode(userinput)` / `xmlDoc.selectSingleNode(userinput)`
//! - `xpath.select(userinput, doc)` / `xpath.select1(userinput, doc)`
//!
//! **Not flagged**:
//! - `db.select(cols)` / `d3.select(node)` — `select` on a non-`xpath`
//!   receiver (generic name).
//! - `expr.evaluate(scope)` — `evaluate` on a non-`document` receiver.
//! - `xpath.compile(expr)` / `document.querySelector(sel)` — unrelated methods.
//! - `document.evaluate` / `xpath.select` without invocation — a property
//!   access is not a call.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "xpath";

impl Scanner<'_> {
    pub(crate) fn check_xpath(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let property = member.property.name.as_str();
        let object_name = match member.object.get_inner_expression() {
            Expression::Identifier(ident) => Some(ident.name.as_str()),
            _ => None,
        };
        let is_xpath_execution = match property {
            // (a) document.evaluate(...) — receiver-gated on `document`.
            "evaluate" => object_name == Some("document"),
            // (b) .selectNodes(...) — distinctive name, any receiver.
            "selectNodes" => true,
            // (c) .SelectSingleNode(...) / .selectSingleNode(...) — distinctive.
            "SelectSingleNode" | "selectSingleNode" => true,
            // (d) xpath.select(...) / xpath.select1(...) — receiver-gated on
            //     `xpath` because bare select/select1 are generic.
            "select" | "select1" => object_name == Some("xpath"),
            _ => false,
        };
        if is_xpath_execution {
            self.report(RULE_NAME, "xpath", it.span);
        }
    }
}
