//! Rule `prefer-regexp-exec` (SonarJS key S6594).
//!
//! Clean-room port. `String.prototype.match()` returns an array-like match
//! result and has special behaviour for global regular expressions. When the
//! argument is a non-global regular-expression literal and only the first match
//! result is needed, `RegExp.prototype.exec()` is the more direct API.
//!
//! This port is deliberately conservative: it only reports
//! `<expr>.match(/pattern/flags)` when the argument is a regular-expression
//! literal without the `g` flag. Dynamic patterns and global regular
//! expressions are skipped.
//!
//! Behaviour is reproduced from public rule documentation and independently
//! authored examples only; no upstream source, tests, fixtures, or message
//! strings were consulted or copied.

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "prefer-regexp-exec";

fn regex_literal_has_global_flag(regex: &oxc_ast::ast::RegExpLiteral<'_>) -> bool {
    regex
        .raw
        .as_ref()
        .and_then(|raw| raw.as_str().rsplit_once('/').map(|(_, flags)| flags))
        .is_some_and(|flags| flags.contains('g'))
}

impl Scanner<'_> {
    pub(crate) fn check_prefer_regexp_exec(&mut self, call: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "match" || call.arguments.len() != 1 {
            return;
        }
        let Some(argument) = call.arguments.first().and_then(Argument::as_expression) else {
            return;
        };
        let Expression::RegExpLiteral(regex) = argument.get_inner_expression() else {
            return;
        };
        if regex_literal_has_global_flag(regex) {
            return;
        }
        self.report(RULE_NAME, "preferRegExpExec", call.span);
    }
}
