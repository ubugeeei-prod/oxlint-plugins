//! Rule `no-require-or-define` (SonarJS key S3533).
//!
//! Clean-room port. ES modules (`import`/`export`) are the standard way to
//! include external code in modern JavaScript and TypeScript. The older
//! CommonJS `require()` loader and AMD `define()` mechanism should not be
//! used alongside ES module syntax.
//!
//! **Flagged** — a call expression whose callee is a bare identifier named
//! exactly `require` or `define`:
//! - `require('fs')` — CommonJS loader call.
//! - `const x = require('./utils')` — CommonJS assignment.
//! - `define(['dep'], function(dep) {})` — AMD module definition.
//!
//! **Not flagged**:
//! - `import x from 'fs'` — standard ES import statement.
//! - `import('./dynamic')` — dynamic `import()` expression (not a plain call).
//! - `foo.require('x')` — the callee is a member expression, not a bare
//!   identifier.
//! - `requireSomething()` — the callee name is not exactly `require`.
//! - `defineProperty(obj, 'x', {})` — the callee name is not exactly `define`.
//!
//! **TypeScript `import x = require('y')`**: this form parses as a
//! `TSImportEqualsDeclaration` (not a `CallExpression`) so the callee check
//! does not fire and it is NOT flagged. This matches the RSPEC intent, which
//! targets the CommonJS/AMD call patterns.
//!
//! **Follow-up (not yet implemented)**: `module.exports = ...` and
//! `exports.x = ...` assignment forms are the CommonJS export counterparts.
//! Including them would require assignment-target analysis; they are deferred
//! to a follow-up to keep this initial port zero-FP on the call forms.
//!
//! Behaviour is reproduced from the public RSPEC description (S3533) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-require-or-define";

impl Scanner<'_> {
    pub(crate) fn check_no_require_or_define(&mut self, expr: &CallExpression<'_>) {
        let Expression::Identifier(identifier) = expr.callee.get_inner_expression() else {
            return;
        };
        if matches!(identifier.name.as_str(), "require" | "define") {
            self.report(RULE_NAME, "noRequireOrDefine", expr.span);
        }
    }
}
