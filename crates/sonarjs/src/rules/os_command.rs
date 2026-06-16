//! Rule `os-command` (SonarJS key S4721).
//!
//! Clean-room port from the public RSPEC S4721 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! Spawning a shell to run an OS command is security-sensitive: when any part of
//! the command line is built from untrusted input, the shell interpreter enables
//! command injection. Node's `child_process` family lets a caller opt into a
//! shell by passing `{ shell: true }` in the options object. The RSPEC marks the
//! shell-interpreter form of `spawn`, `spawnSync`, `exec`, `execSync`,
//! `execFile`, and `execFileSync` as sensitive.
//!
//! ## Zero-FP subset
//!
//! This port flags a `CallExpression` whose callee (after unwrapping
//! parentheses/`as`/etc. via `get_inner_expression`) is a static member
//! expression whose property name is one of `spawn`, `spawnSync`, `execFile`,
//! or `execFileSync`, AND one of whose arguments is an object literal containing
//! a `shell` property (key as a static identifier or string literal) whose value
//! is the boolean literal `true`. The call span is reported.
//!
//! The RSPEC additionally lists `exec` and `execSync`, but those are
//! deliberately EXCLUDED here: `.exec` collides with the extremely common
//! `RegExp.prototype.exec` (`regex.exec(str)`) and with many library `.exec`
//! methods, which would produce false positives. Restricting to the four
//! shell-option methods AND requiring an explicit `shell: true` keeps this
//! effectively zero-false-positive while still catching the dangerous
//! shell-interpreter form. The match keys off the property name only, so any
//! receiver (`cp.spawn`, `require('child_process').spawn`, …) is covered.
//! A non-literal `shell` value or `shell: false` is never flagged.
//!
//! ## Flagged
//! ```js
//! cp.spawn(cmd, { shell: true });        // shell interpreter requested
//! cp.spawnSync(cmd, { shell: true });    // shell interpreter requested
//! cp.execFile(cmd, { shell: true });     // shell interpreter requested
//! cp.execFileSync(cmd, { shell: true }); // shell interpreter requested
//! ```
//!
//! ## Not Flagged
//! ```js
//! cp.spawnSync("/usr/bin/file.exe", { shell: false }); // shell disabled
//! cp.spawn(cmd, { shell: someVar });                   // non-literal value
//! cp.spawn(cmd);                                        // no options object
//! cp.exec(cmd, { shell: true });                       // exec excluded (RegExp.exec FP)
//! regex.exec(str);                                      // RegExp.exec excluded
//! ```

use oxc_ast::ast::{Argument, CallExpression, Expression, ObjectPropertyKind, PropertyKey};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "os-command";

impl Scanner<'_> {
    pub(crate) fn check_os_command(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        if !matches!(
            member.property.name.as_str(),
            "spawn" | "spawnSync" | "execFile" | "execFileSync"
        ) {
            return;
        }
        let shell_true = it.arguments.iter().any(|arg| {
            let Argument::ObjectExpression(obj) = arg else {
                return false;
            };
            obj.properties.iter().any(|prop| {
                let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                    return false;
                };
                let key = match &prop.key {
                    PropertyKey::StaticIdentifier(ident) => ident.name.as_str(),
                    PropertyKey::StringLiteral(lit) => lit.value.as_str(),
                    _ => return false,
                };
                key == "shell" && matches!(&prop.value, Expression::BooleanLiteral(b) if b.value)
            })
        });
        if shell_true {
            self.report(RULE_NAME, "osCommand", it.span);
        }
    }
}
