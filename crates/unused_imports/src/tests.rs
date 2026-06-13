use oxlint_plugins_carton::CompactString;

use crate::{UnusedImportsOptions, scan_unused_imports};

fn apply_first_fix(source: &str, filename: &str) -> CompactString {
    let diagnostics = scan_unused_imports(source, filename, &UnusedImportsOptions::default());
    let fix = diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic.fix.as_ref())
        .expect("expected fix");
    let mut output = CompactString::new("");
    output.push_str(&source[..fix.start as usize]);
    output.push_str(&fix.replacement);
    output.push_str(&source[fix.end as usize..]);
    output
}

#[test]
fn removes_unused_named_import() {
    let source =
        "import x from \"package\";\nimport { a, b } from \"./utils\";\nconst c = b(x);\n";
    let diagnostics = scan_unused_imports(source, "file.js", &UnusedImportsOptions::default());
    assert_eq!(diagnostics[0].rule_name, "no-unused-imports");
    assert_eq!(diagnostics[0].message, "'a' is defined but never used.");
    assert_eq!(
        apply_first_fix(source, "file.js"),
        "import x from \"package\";\nimport { b } from \"./utils\";\nconst c = b(x);\n"
    );
}

#[test]
fn removes_whole_import_and_preserves_comment_spacing() {
    let source = "import y from \"package\";\nimport { a } from \"./utils\";\n\n/** c is the number 4 */\nconst c = y;\n";
    assert_eq!(
        apply_first_fix(source, "file.js"),
        "import y from \"package\";\n\n/** c is the number 4 */\nconst c = y;\n"
    );
}

#[test]
fn removes_last_named_import_with_default_left() {
    let source = "import fallback, { a } from \"./utils\";\nconsole.log(fallback);\n";
    assert_eq!(
        apply_first_fix(source, "file.js"),
        "import fallback from \"./utils\";\nconsole.log(fallback);\n"
    );
}

#[test]
fn honors_jsdoc_identifier_references() {
    let source = "import { UsedInJSDoc } from \"./used\";\n/** Reference to {@link UsedInJSDoc} */\nconst example = \"test\";\n";
    let diagnostics = scan_unused_imports(source, "file.js", &UnusedImportsOptions::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.rule_name != "no-unused-imports")
    );
}

#[test]
fn type_usage_counts_as_used() {
    let source =
        "import type { SomeType } from \"./types\";\nconst value: SomeType = {} as SomeType;\n";
    let diagnostics = scan_unused_imports(source, "file.ts", &UnusedImportsOptions::default());
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message != "'SomeType' is defined but never used.")
    );
}

#[test]
fn shadowed_inner_name_does_not_mark_import_used() {
    let source = "import { a } from \"./utils\";\nfunction fn(a) { return a; }\nfn(1);\n";
    let diagnostics = scan_unused_imports(source, "file.js", &UnusedImportsOptions::default());
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_name == "no-unused-imports"
                && diagnostic.message == "'a' is defined but never used.")
    );
}
