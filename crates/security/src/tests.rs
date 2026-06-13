use super::scan_security;
use crate::helpers::is_unsafe_regex;

#[test]
fn scans_core_security_rules() {
    let diagnostics = scan_security(
        "var fs = require('fs');\nfs.readFile(filename);\neval(name);\n",
        "fixture.js",
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_name == "detect-non-literal-fs-filename")
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_name == "detect-eval-with-expression")
    );
}

#[test]
fn treats_static_path_builders_as_static() {
    let diagnostics = scan_security(
        "import fs from 'fs';\nimport path from 'path';\nfs.readFileSync(path.resolve(__dirname, './index.html'));\n",
        "fixture.mjs",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn detects_nested_quantified_regexes() {
    assert!(is_unsafe_regex("(x+x+)+y"));
    assert!(is_unsafe_regex("x+x+)+y"));
    assert!(!is_unsafe_regex("^d+1337d+$"));
}
