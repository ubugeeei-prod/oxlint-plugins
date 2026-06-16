//! Rule `publicly-writable-directories` (Sonar key S5443).
//!
//! Clean-room port. "Using publicly writable directories is
//! security-sensitive." A publicly (world-)writable directory such as the
//! shared temporary directory is accessible to every user and process on the
//! host. Hardcoding a path under one of these directories exposes the
//! application to race-condition and file-squatting attacks, where a hostile
//! local process pre-creates or swaps the file behind the application's back.
//! This rule is a security hotspot: it surfaces the access so a human can
//! review whether the usage is safe.
//!
//! This is a zero-false-positive subset that matches two distinctive,
//! unambiguous shapes:
//!
//! Shape (a) — a **string literal** whose value names a known publicly
//! writable directory. The value is exactly one of `/tmp`, `/var/tmp`,
//! `/usr/tmp`, `/dev/shm`, *or* it starts with one of those followed by a `/`
//! separator. The trailing-`/` boundary check avoids matching unrelated paths
//! such as `/tmpfoo` that merely share a prefix. Reported on the string
//! literal's span.
//!
//! Shape (b) — a **static member expression** `process.env.TMPDIR`,
//! `process.env.TMP`, or `process.env.TEMP`: the object is the static member
//! `process.env` (object identifier `process`, property `env`) and the
//! outer property is one of the temp-directory environment variables. Reported
//! on the member expression's span.
//!
//! **Flagged**:
//! - `let f = "/tmp/temporary_file";` — string under `/tmp`.
//! - `const d = "/var/tmp";` — exact known directory.
//! - `const d = "/dev/shm/cache";` — string under `/dev/shm`.
//! - `let t = process.env.TMPDIR;` — temp-directory environment variable.
//!
//! **Not flagged**:
//! - `const x = "/tmpfoo";` — no path boundary after `/tmp`.
//! - `const x = "/home/user/file";` — unrelated path.
//! - `const x = "tmp/file";` — not an absolute publicly-writable path.
//! - `process.env.PATH;` — different environment variable.
//! - `foo.env.TMPDIR;` — object is not the bare `process` identifier.
//!
//! Only the bare `process.env.X` member-access form is covered for shape (b);
//! indirect forms (e.g. a destructured `const { env } = process;`) are out of
//! scope for this syntactic check. Likewise shape (a) only inspects literal
//! string values, not dynamically constructed paths.
//!
//! Behaviour is reproduced from the public RSPEC S5443 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Expression, StaticMemberExpression, StringLiteral};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "publicly-writable-directories";

/// The known publicly (world-)writable directories matched by shape (a).
const PUBLIC_TEMP_DIRS: [&str; 4] = ["/tmp", "/var/tmp", "/usr/tmp", "/dev/shm"];

/// The temp-directory environment variable names matched by shape (b).
const TEMP_ENV_VARS: [&str; 3] = ["TMPDIR", "TMP", "TEMP"];

/// Returns `true` if `value` names a known publicly-writable directory: it is
/// exactly one of the known directories, or it starts with one of them followed
/// by a `/` path separator. A bare prefix such as `/tmpfoo` is rejected because
/// it lacks the boundary separator.
fn is_public_temp_path(value: &str) -> bool {
    PUBLIC_TEMP_DIRS.iter().any(|dir| {
        value == *dir
            || (value.len() > dir.len()
                && value.starts_with(dir)
                && value.as_bytes()[dir.len()] == b'/')
    })
}

impl Scanner<'_> {
    pub(crate) fn check_publicly_writable_directories_string(&mut self, it: &StringLiteral<'_>) {
        if is_public_temp_path(it.value.as_str()) {
            self.report(RULE_NAME, "publiclyWritableDirectories", it.span);
        }
    }

    pub(crate) fn check_publicly_writable_directories_member(
        &mut self,
        member: &StaticMemberExpression<'_>,
    ) {
        if !TEMP_ENV_VARS.contains(&member.property.name.as_str()) {
            return;
        }
        let Expression::StaticMemberExpression(inner) = member.object.get_inner_expression() else {
            return;
        };
        if inner.property.name != "env" {
            return;
        }
        let Expression::Identifier(obj) = inner.object.get_inner_expression() else {
            return;
        };
        if obj.name == "process" {
            self.report(RULE_NAME, "publiclyWritableDirectories", member.span);
        }
    }
}
