//! Rule `file-name-differ-from-class` (SonarJS key S3317).
//!
//! Clean-room port. A file that exports exactly one named class should be named
//! after that class so that readers can locate the source without relying on
//! tooling. When the sole exported class name does not match the file's base
//! name the rule flags the class identifier.
//!
//! ## Matching convention
//!
//! The comparison is lenient to accommodate common naming styles:
//!
//! * **Directory and extensions are ignored** — only the stem (basename minus
//!   the last `.ext` suffix) is compared.
//! * **Separators are transparent** — hyphens (`-`) and underscores (`_`) are
//!   stripped from both the class name and the stem before comparing, so
//!   `MyClass`, `my-class`, `my_class`, and `myclass` are all considered
//!   equivalent.
//! * **Case is ignored** — the stripped forms are compared
//!   case-insensitively, so `Foo.ts` and `foo.ts` both match class `Foo`.
//!
//! ## Scope restriction
//!
//! The rule fires only when there is **exactly one** exported class with a
//! name in the file (either `export class X {}` or `export default class X
//! {}`). Files with no exported named class, or with two or more, are left
//! alone.
//!
//! ## Examples
//!
//! ```js
//! // file: bar.ts
//! export class Foo {}      // Noncompliant: class name 'Foo' ≠ stem 'bar'
//! ```
//!
//! ```js
//! // file: my-class.ts
//! export class MyClass {}  // Compliant: 'MyClass' → 'myclass' = 'my-class' → 'myclass'
//!
//! // file: Foo.ts
//! export class Foo {}      // Compliant: exact match (case-insensitive)
//!
//! // file: bar.ts
//! class Foo {}             // Compliant: Foo is not exported at all
//! export {};
//!
//! // file: bar.ts
//! export class A {}        // Compliant: two exports — rule does not fire
//! export class B {}
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description (S3317) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{Declaration, ExportDefaultDeclarationKind, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "file-name-differ-from-class";

/// Extract the class name and its identifier span from a statement that is
/// exactly an `export class Name {}` or `export default class Name {}`.
/// Returns `None` for any other statement or for anonymous classes.
fn exported_class_info<'a>(stmt: &'a Statement<'a>) -> Option<(&'a str, Span)> {
    match stmt {
        Statement::ExportNamedDeclaration(decl) => {
            let inner = decl.declaration.as_ref()?;
            let Declaration::ClassDeclaration(class) = inner else {
                return None;
            };
            let id = class.id.as_ref()?;
            Some((id.name.as_str(), id.span))
        }
        Statement::ExportDefaultDeclaration(decl) => {
            let ExportDefaultDeclarationKind::ClassDeclaration(class) = &decl.declaration else {
                return None;
            };
            let id = class.id.as_ref()?;
            Some((id.name.as_str(), id.span))
        }
        _ => None,
    }
}

/// Strips the directory prefix and the last file extension from `filename`,
/// returning the bare stem as a `&str` slice into the original string.
fn filename_stem(filename: &str) -> &str {
    let basename = match filename.rfind(['/', '\\']) {
        Some(pos) => &filename[pos + 1..],
        None => filename,
    };
    match basename.rfind('.') {
        Some(pos) => &basename[..pos],
        None => basename,
    }
}

/// Returns `true` when `class_name` and `stem` are equivalent under the
/// lenient matching convention: strip hyphens and underscores from both sides,
/// then compare case-insensitively, byte by byte.
fn names_match(class_name: &str, stem: &str) -> bool {
    let mut cn = class_name.bytes().filter(|&b| b != b'-' && b != b'_');
    let mut sn = stem.bytes().filter(|&b| b != b'-' && b != b'_');
    loop {
        match (cn.next(), sn.next()) {
            (None, None) => return true,
            (Some(a), Some(b)) => {
                if !a.eq_ignore_ascii_case(&b) {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

impl Scanner<'_> {
    /// Checks rule `file-name-differ-from-class`.
    ///
    /// Iterates the top-level statements of the program looking for exported
    /// named classes. When exactly one is found, its name is compared to the
    /// filename stem. A mismatch is reported on the class identifier.
    pub(crate) fn check_file_name_differ_from_class(
        &mut self,
        program: &oxc_ast::ast::Program<'_>,
    ) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let stem = filename_stem(self.filename);
        if stem.is_empty() {
            return;
        }

        let mut found: Option<(&str, Span)> = None;
        let mut count = 0u32;

        for stmt in &program.body {
            let Some((name, span)) = exported_class_info(stmt) else {
                continue;
            };
            count += 1;
            if count == 1 {
                found = Some((name, span));
            }
            if count >= 2 {
                return;
            }
        }

        if count != 1 {
            return;
        }

        let Some((class_name, span)) = found else {
            return;
        };

        if !names_match(class_name, stem) {
            self.report(RULE_NAME, "fileNameDifferFromClass", span);
        }
    }
}
