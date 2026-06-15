use super::{
    OnlyExportComponentsOptions, is_constant_export_expression_kind,
    is_excluded_test_like_filename, is_react_component_name, scan_only_export_components,
    should_scan_filename,
};

fn message_ids(
    source_text: &str,
    filename: &str,
    options: OnlyExportComponentsOptions,
) -> oxlint_plugins_carton::SmallVec<[&'static str; 8]> {
    scan_only_export_components(source_text, filename, &options)
        .into_iter()
        .map(|diagnostic| diagnostic.message_id)
        .collect()
}

#[test]
fn classifies_component_names() {
    let cases = [
        ("Foo", true),
        ("Foo2", true),
        ("Foo_", true),
        ("CMS", true),
        ("foo", false),
        ("_Foo", false),
        ("Foo-Bar", false),
        ("", false),
    ];

    #[allow(clippy::disallowed_macros)]
    {
        insta::assert_debug_snapshot!(cases.map(|(name, _)| (name, is_react_component_name(name))));
    }
    for (name, expected) in cases {
        assert_eq!(is_react_component_name(name), expected);
    }
}

#[test]
fn classifies_scannable_filenames() {
    let cases = [
        ("App.tsx", false),
        ("App.jsx", false),
        ("App.js", false),
        ("App.js", true),
        ("App.test.tsx", false),
        ("App.stories.jsx", false),
        ("App.ts", false),
    ];

    let actual = cases.map(|(filename, check_js)| {
        (
            filename,
            check_js,
            is_excluded_test_like_filename(filename),
            should_scan_filename(filename, check_js),
        )
    });
    #[allow(clippy::disallowed_macros)]
    {
        insta::assert_debug_snapshot!(actual);
    }
}

#[test]
fn classifies_constant_export_kinds() {
    assert!(is_constant_export_expression_kind("Literal"));
    assert!(is_constant_export_expression_kind("UnaryExpression"));
    assert!(is_constant_export_expression_kind("TemplateLiteral"));
    assert!(is_constant_export_expression_kind("BinaryExpression"));
    assert!(!is_constant_export_expression_kind("ObjectExpression"));
    assert!(!is_constant_export_expression_kind("CallExpression"));
}

#[test]
fn scans_only_export_components_in_rust() {
    let cases: [(&str, &str, OnlyExportComponentsOptions, &[&str]); 4] = [
        (
            "export const Foo = () => null;\nexport const foo = 1;\n",
            "Component.tsx",
            OnlyExportComponentsOptions::default(),
            &["namedExport"],
        ),
        (
            "const Foo = () => null;\n",
            "Component.tsx",
            OnlyExportComponentsOptions::default(),
            &["noExport"],
        ),
        (
            "export default memo(() => null);\n",
            "Component.tsx",
            OnlyExportComponentsOptions::default(),
            &["anonymousExport"],
        ),
        (
            "export const Foo = () => null;\nexport const answer = 42;\n",
            "Component.tsx",
            OnlyExportComponentsOptions {
                allow_constant_export: true,
                ..OnlyExportComponentsOptions::default()
            },
            &[],
        ),
    ];

    for (source_text, filename, options, expected) in cases {
        assert_eq!(
            message_ids(source_text, filename, options).as_slice(),
            expected
        );
    }
}
