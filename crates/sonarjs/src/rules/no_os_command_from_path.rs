//! Rule `no-os-command-from-path` (SonarJS key S4036).
//!
//! Clean-room port from the public RSPEC S4036 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! "Searching OS commands in PATH is security-sensitive." Running a command by
//! its bare name relies on the `PATH` environment variable to locate the
//! executable. An attacker who controls `PATH` can place a malicious binary
//! earlier in the search order and have it executed in place of the intended
//! program. The remediation is to invoke the command through a fully-qualified
//! absolute path. This is a security hotspot, not a guaranteed vulnerability.
//!
//! ## Zero-FP subset
//!
//! This port flags a `CallExpression` whose callee (after unwrapping
//! parentheses/`as`/etc. via `get_inner_expression`) is a static member
//! expression whose property name is one of `spawn`, `spawnSync`, `execFile`,
//! or `execFileSync` (the distinctive `child_process` command-runner methods),
//! AND whose FIRST argument is a non-empty string literal that is a BARE
//! COMMAND NAME — its value contains neither `/` nor `\`. The call span is
//! reported. The match keys off the property name only, so any receiver
//! (`cp.spawn`, `require('child_process').spawn`, …) is covered.
//!
//! The RSPEC example itself uses `cp.exec`, but `exec` and `execSync` are
//! deliberately EXCLUDED here: the bare member name `.exec` collides with the
//! extremely common `RegExp.prototype.exec` (`re.exec('some string')`) and many
//! library `.exec` methods, which would produce false positives. Restricting to
//! the four distinctive runner methods AND requiring a bare-command string
//! literal (no path separator) keeps this effectively zero-false-positive.
//! Under-reporting the `exec` form is acceptable; over-reporting is not. (This
//! mirrors the exec/execSync exclusion already used by the merged `os-command`
//! rule, S4721.)
//!
//! A string literal that starts with `/`, or contains any `/` or `\` (e.g.
//! `/usr/bin/file.exe`, `./file.exe`, `bin/file.exe`, `C:\tools\x.exe`) is an
//! explicit path and is NOT flagged. A non-literal first argument is skipped.
//!
//! ## Flagged
//! ```js
//! cp.spawn('file.exe');         // bare command name → resolved via PATH
//! cp.spawnSync('git');          // bare command name → resolved via PATH
//! cp.execFile('node');          // bare command name → resolved via PATH
//! cp.execFileSync('ls');        // bare command name → resolved via PATH
//! ```
//!
//! ## Not Flagged
//! ```js
//! cp.spawn('/usr/bin/file.exe');   // absolute path
//! cp.spawn('./file.exe');          // relative path with separator
//! cp.spawn('bin/file.exe');        // path with separator
//! cp.spawn('C:\\tools\\x.exe');    // Windows path with backslash
//! cp.spawn(cmd);                   // non-literal argument
//! cp.exec('file.exe');             // exec excluded (RegExp.exec FP)
//! re.exec('some string');          // RegExp.exec excluded
//! ```

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-os-command-from-path";

impl Scanner<'_> {
    pub(crate) fn check_no_os_command_from_path(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        if !matches!(
            member.property.name.as_str(),
            "spawn" | "spawnSync" | "execFile" | "execFileSync"
        ) {
            return;
        }
        let Some(Argument::StringLiteral(first)) = it.arguments.first() else {
            return;
        };
        let command = first.value.as_str();
        if command.is_empty() || command.contains('/') || command.contains('\\') {
            return;
        }
        self.report(RULE_NAME, "noOsCommandFromPath", it.span);
    }
}
