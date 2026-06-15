//! Rule `no-unsafe-unzip` (SonarJS key S5042).
//!
//! Clean-room port. Expanding archives without bounding the extracted size,
//! entry count, or compression ratio exposes the application to "zip-bomb"
//! denial-of-service attacks: a tiny archive can inflate into gigabytes of
//! data on disk or in memory.
//!
//! The upstream rule covers several archive libraries (`tar`, `adm-zip`,
//! `jszip`, `yauzl`, `extract-zip`). Most of those rely on generic method
//! names — `x`, `extract`, `open`, `loadAsync` — that collide heavily with
//! unrelated user code, so matching them by name alone would be noisy. This
//! port deliberately implements ONLY the unambiguous, zero-false-positive
//! subset: `adm-zip`'s `extractAllTo` method, whose name is distinctive enough
//! to be effectively unique to archive expansion. The secure `adm-zip` pattern
//! extracts entries individually via `extractEntryTo`, so that method is
//! intentionally NOT flagged. This narrow scope under-reports on purpose to
//! avoid false positives.
//!
//! **Flagged** — a `CallExpression` whose callee is a static member expression
//! whose property is named `extractAllTo`:
//! - `zip.extractAllTo(".")` — extracts the whole archive to a directory.
//! - `new AdmZip("f.zip").extractAllTo("./out")` — receiver expression is
//!   irrelevant; the distinctive method name is what matters.
//!
//! **Not flagged**:
//! - `zip.extractEntryTo(entry, ".")` — per-entry extraction, the secure path.
//! - `tar.x({ file: "f" })` — generic method name `x` (too common to flag).
//! - `extract("test.zip", { dir })` — generic free function `extract`.
//! - `foo()` — not a member call.
//!
//! Behaviour is reproduced from the public RSPEC S5042 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-unsafe-unzip";

impl Scanner<'_> {
    pub(crate) fn check_no_unsafe_unzip(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(m) = expr.callee.get_inner_expression() else {
            return;
        };
        if m.property.name != "extractAllTo" {
            return;
        }
        self.report(RULE_NAME, "unsafeUnzip", expr.span);
    }
}
