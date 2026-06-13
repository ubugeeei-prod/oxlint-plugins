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
    assert_eq!(
        diagnostics[0]
            .fix
            .as_ref()
            .expect("fix")
            .replacement
            .as_str(),
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
