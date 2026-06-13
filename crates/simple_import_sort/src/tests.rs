use crate::{SimpleImportSortOptions, scan_simple_import_sort};

#[test]
fn sorts_import_chunks_and_specifiers() {
    let source = [
        "import z from 'z';",
        "import { beta, alpha as renamed } from 'pkg';",
        "import fs from 'node:fs';",
        "import './setup';",
        "import local from './local';",
    ]
    .join("\n");
    let diagnostics =
        scan_simple_import_sort(&source, "fixture.js", &SimpleImportSortOptions::default());

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "imports");
    let fix = diagnostics[0].fix.as_ref().expect("fix");
    assert_eq!(
        fix.replacement.as_str(),
        [
            "import './setup';",
            "",
            "import fs from 'node:fs';",
            "",
            "import { alpha as renamed, beta } from 'pkg';",
            "import z from 'z';",
            "",
            "import local from './local';",
        ]
        .join("\n")
    );
}

#[test]
fn sorts_export_chunks_and_local_specifiers() {
    let source = [
        "export { zed } from 'z';",
        "export * from 'a';",
        "export { d, a as c, b };",
    ]
    .join("\n");
    let diagnostics =
        scan_simple_import_sort(&source, "fixture.js", &SimpleImportSortOptions::default());

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].rule_name, "exports");
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("fix")
            .replacement
            .as_str(),
        ["export * from 'a';", "export { zed } from 'z';"].join("\n")
    );
    // Local export specifiers sorted: b, a as c, d → a as c, b, d
    // external names: zed→zed, a→a, b→b, c→c, d→d
    // export { d, a as c, b }: external names are d, c, b → sort → b, c, d
    // b sorts as 'b', a as c sorts by external 'c', d sorts as 'd'
    // so sorted order: b, a as c, d
    assert_eq!(
        diagnostics[1]
            .fix
            .as_ref()
            .expect("fix")
            .replacement
            .as_str(),
        "export { b, a as c, d };"
    );
}

#[test]
fn handles_comments_after_import() {
    let source = "import x2 from \"b\"\nimport x1 from \"a\" // a\n\n;/* comment */[].forEach()";
    let diagnostics =
        scan_simple_import_sort(source, "fixture.js", &SimpleImportSortOptions::default());

    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0].fix.as_ref().expect("fix");
    assert_eq!(
        fix.replacement.as_str(),
        "import x1 from \"a\" // a\nimport x2 from \"b\""
    );
}

#[test]
fn preserves_comments_before_imports() {
    let source =
        "import c from \"c\"\n// b1\n\n// b2\nimport b from \"b\"\n// a\n\nimport a from \"a\"";
    let diagnostics =
        scan_simple_import_sort(source, "fixture.js", &SimpleImportSortOptions::default());

    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0].fix.as_ref().expect("fix");
    assert_eq!(
        fix.replacement.as_str(),
        "// a\nimport a from \"a\"\n// b1\n// b2\nimport b from \"b\"\nimport c from \"c\""
    );
}

#[test]
fn sorts_specifiers_with_comments() {
    let source =
        "import {\n  // c\n  c,\n  b, // b\n  a\n  // last\n} from \"specifiers-comments\"";
    let diagnostics =
        scan_simple_import_sort(source, "fixture.js", &SimpleImportSortOptions::default());

    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0].fix.as_ref().expect("fix");
    assert_eq!(
        fix.replacement.as_str(),
        "import {\n  a,\n  b, // b\n  // c\n  c\n  // last\n} from \"specifiers-comments\""
    );
}

#[test]
fn collator_handles_accents() {
    // 'ä' should sort as 'a' (base sensitivity) → before 'b'
    let source = "import b from '.';\nimport a from 'ä';";
    let diagnostics =
        scan_simple_import_sort(source, "fixture.js", &SimpleImportSortOptions::default());

    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0].fix.as_ref().expect("fix");
    // 'ä' is in package group; '.' is relative → different groups
    assert_eq!(
        fix.replacement.as_str(),
        "import a from 'ä';\n\nimport b from '.';"
    );
}
