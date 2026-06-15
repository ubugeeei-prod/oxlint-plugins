//! Rule `file-permissions` (SonarJS key S2612).
//!
//! Clean-room port. On Unix-like systems the "others" permission category
//! covers every user that is neither the file's owner nor a member of its
//! group. Granting read/write/execute access to "others" can expose sensitive
//! data, allow tampering, or enable privilege escalation, so setting such loose
//! permissions is a security hotspot. The two distinctive Node.js APIs that
//! configure permissions are the `fs` `chmod` family and `process.umask`.
//!
//! This implements ONLY the unambiguous, zero-false-positive subset: calls
//! whose mode argument is a NUMERIC literal (typically an octal literal such as
//! `0o777`). Dynamic or string modes are deliberately ignored (under-report,
//! never over-report).
//!
//! ## Octal-bit logic
//!
//! A Unix mode's lowest three bits (`mode & 0o007`) are the "others"
//! permissions.
//! - **chmod family**: the mode *grants* permission, so "others" has access
//!   whenever any of those bits is set: `(value & 0o007) != 0`. So `0o777`,
//!   `0o666`, and `0o007` are flagged; `0o750` and `0o640` (no others bits) are
//!   not.
//! - **umask**: the value *masks off* permission, so a secure umask restricts
//!   all three "others" bits (`0o007`). It is flagged when it fails to do so:
//!   `(value & 0o007) != 0o007`. So `0o000` and `0o022` are flagged; `0o077`
//!   (and `0o007`) are not.
//!
//! Values are read from `NumericLiteral.value` (an `f64`) and only considered
//! when they are a non-negative integer within the `0..=0o7777` mode range;
//! anything else is skipped.
//!
//! ## Flagged
//! ```js
//! fs.chmodSync("/x", 0o777);   // chmod grants rwx to others
//! fs.chmod("/x", 0o666, cb);   // mode is the last (non-callback) argument
//! filehandle.lchmod(0o007);    // any chmod-family member, any receiver
//! process.umask(0o000);        // permissive umask leaves others open
//! umask(0o022);                // bare umask call (identifier callee)
//! ```
//!
//! ## Not flagged
//! ```js
//! fs.chmodSync("/x", 0o750);   // no "others" bits set
//! process.umask(0o077);        // restricts all "others" bits
//! fs.chmodSync("/x", mode);    // dynamic (non-literal) mode
//! fs.writeFileSync("/x", d);   // unrelated callee
//! ```
//!
//! Note: the chmod-family check keys off the property name only, so
//! `foo.chmodSync(0o777)` on an unrelated receiver is also flagged. The
//! `chmod*`/`umask` names are distinctive to permission APIs, so keeping the
//! match name-based stays effectively zero-false-positive while catching
//! `fs.promises.chmod(...)` and `FileHandle.chmod(...)` without type analysis.
//!
//! Behaviour is reproduced from the public RSPEC S2612 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "file-permissions";

/// Largest valid Unix mode value (`0o7777`, including the setuid/setgid/sticky
/// bits) as an `f64`; values outside `0..=this` are not real permission modes.
const MAX_MODE: f64 = 0o7777 as f64;

/// Returns the integer value of `arg` when it is a numeric literal holding a
/// non-negative integer within the valid mode range, else `None`.
fn mode_value(arg: &Argument<'_>) -> Option<u32> {
    let expr = arg.as_expression()?;
    let Expression::NumericLiteral(lit) = expr.get_inner_expression() else {
        return None;
    };
    let value = lit.value;
    if value.fract() != 0.0 || !(0.0..=MAX_MODE).contains(&value) {
        return None;
    }
    Some(value as u32)
}

impl Scanner<'_> {
    pub(crate) fn check_file_permissions(&mut self, expr: &CallExpression<'_>) {
        // The chmod family is only meaningful as a member call (`fs.chmod`,
        // `filehandle.chmod`); `umask` is matched both as a member call
        // (`process.umask`) and as a bare identifier call (`umask(...)`).
        let (name, is_member) = match expr.callee.get_inner_expression() {
            Expression::StaticMemberExpression(member) => (member.property.name.as_str(), true),
            Expression::Identifier(ident) => (ident.name.as_str(), false),
            _ => return,
        };
        if is_member && is_chmod_name(name) {
            self.check_chmod_mode(expr);
        } else if name == "umask" {
            self.check_umask_mode(expr);
        }
    }

    fn check_chmod_mode(&mut self, expr: &CallExpression<'_>) {
        // The mode is the trailing argument in the synchronous form
        // `chmodSync(path, mode)`, or the one just before the callback in the
        // asynchronous form `chmod(path, mode, cb)`. Try the last argument
        // first, then fall back to the second-to-last when the last is not a
        // numeric literal (i.e. it is the callback).
        let n = expr.arguments.len();
        if n == 0 {
            return;
        }
        let value = match mode_value(&expr.arguments[n - 1]) {
            Some(v) => Some(v),
            None if n >= 2 => mode_value(&expr.arguments[n - 2]),
            None => None,
        };
        let Some(value) = value else {
            return;
        };
        if (value & 0o007) != 0 {
            self.report(RULE_NAME, "weakFilePermissions", expr.span);
        }
    }

    fn check_umask_mode(&mut self, expr: &CallExpression<'_>) {
        if expr.arguments.len() != 1 {
            return;
        }
        let Some(value) = mode_value(&expr.arguments[0]) else {
            return;
        };
        if (value & 0o007) != 0o007 {
            self.report(RULE_NAME, "weakFilePermissions", expr.span);
        }
    }
}

/// Returns `true` for the `fs` chmod-family method names whose final argument
/// is a permission mode.
fn is_chmod_name(name: &str) -> bool {
    matches!(
        name,
        "chmod" | "chmodSync" | "fchmod" | "fchmodSync" | "lchmod" | "lchmodSync"
    )
}
