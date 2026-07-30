//! Adapter for the `playwright` plugin (port of eslint-plugin-playwright).

use std::collections::BTreeMap;

use oxlint_plugins_playwright as core;

use super::EnabledFilter;
use crate::{PlaygroundDiagnostic, PluginInfo};

pub const PLUGIN: &str = "playwright";

pub fn info() -> PluginInfo {
    PluginInfo {
        plugin: PLUGIN,
        rules: core::implemented_playwright_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    }
}

pub fn scan(
    source_text: &str,
    filename: &str,
    filter: &EnabledFilter,
    out: &mut Vec<PlaygroundDiagnostic>,
) {
    for diagnostic in core::scan_playwright(source_text, filename) {
        if !filter.rule_enabled(PLUGIN, diagnostic.rule_name) {
            continue;
        }
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        if !diagnostic.data.message.is_empty() {
            data.insert("message".to_owned(), diagnostic.data.message.into_string());
        }
        push(&mut data, "amount", diagnostic.data.amount);
        push(&mut data, "count", diagnostic.data.count);
        push(&mut data, "depth", diagnostic.data.depth);
        push(&mut data, "max", diagnostic.data.max);
        push(&mut data, "method", diagnostic.data.method);
        push(&mut data, "restriction", diagnostic.data.restriction);
        push(&mut data, "role", diagnostic.data.role);
        push(&mut data, "functionName", diagnostic.data.function_name);
        push(&mut data, "pattern", diagnostic.data.pattern);
        push(&mut data, "tag", diagnostic.data.tag);
        push(&mut data, "word", diagnostic.data.word);
        push(&mut data, "s", diagnostic.data.s);
        out.push(PlaygroundDiagnostic {
            plugin: PLUGIN,
            rule: diagnostic.rule_name.to_owned(),
            message_id: diagnostic.message_id.to_owned(),
            data,
            start_line: diagnostic.loc.start_line,
            start_column: diagnostic.loc.start_column,
            end_line: diagnostic.loc.end_line,
            end_column: diagnostic.loc.end_column,
        });
    }
}

fn push(
    data: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<oxlint_plugins_carton::CompactString>,
) {
    if let Some(value) = value {
        data.insert(key.to_owned(), value.into_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_valid_title_data_and_exact_utf16_locations() {
        let filter = EnabledFilter::parse(r#"{"playwright":["valid-title"]}"#);
        let mut diagnostics = Vec::new();
        scan(
            "const emoji = \"🧪\";\ntest(\"\", () => {});\n",
            "fixture.ts",
            &filter,
            &mut diagnostics,
        );

        let valid_title = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.rule == "valid-title")
            .expect("valid-title diagnostic");
        assert_eq!(valid_title.message_id, "emptyTitle");
        assert_eq!(
            valid_title.data.get("functionName").map(String::as_str),
            Some("test")
        );
        assert_eq!(
            (
                valid_title.start_line,
                valid_title.start_column,
                valid_title.end_line,
                valid_title.end_column,
            ),
            (2, 0, 2, 18)
        );
    }

    #[test]
    fn rule_selection_excludes_the_other_pattern_rule() {
        let filter = EnabledFilter::parse(r#"{"playwright":["valid-test-tags"]}"#);
        let mut diagnostics = Vec::new();
        scan(
            "test(\"\", { tag: \"bad\" }, () => {});\n",
            "fixture.ts",
            &filter,
            &mut diagnostics,
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "valid-test-tags");
        assert_eq!(diagnostics[0].message_id, "invalidTagFormat");
    }

    #[test]
    fn preserves_threshold_message_data_locations_and_rule_selection() {
        let filter = EnabledFilter::parse(r#"{"playwright":["max-expects"]}"#);
        let mut diagnostics = Vec::new();
        scan(
            concat!(
                "const marker = \"🧪\";\n",
                "test(\"case\", () => {\n",
                "  expect(1).toBe(1); expect(2).toBe(2); expect(3).toBe(3);\n",
                "  expect(4).toBe(4); expect(5).toBe(5); expect(6).toBe(6);\n",
                "});\n",
            ),
            "fixture.ts",
            &filter,
            &mut diagnostics,
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "max-expects");
        assert_eq!(diagnostics[0].message_id, "exceededMaxAssertion");
        assert_eq!(
            diagnostics[0].data.get("count").map(String::as_str),
            Some("6")
        );
        assert_eq!(
            diagnostics[0].data.get("max").map(String::as_str),
            Some("5")
        );
        assert_eq!(
            (
                diagnostics[0].start_line,
                diagnostics[0].start_column,
                diagnostics[0].end_line,
                diagnostics[0].end_column,
            ),
            (4, 40, 4, 57)
        );
    }

    #[test]
    fn preserves_expect_expect_message_location_and_empty_data() {
        let filter = EnabledFilter::parse(r#"{"playwright":["expect-expect"]}"#);
        let mut diagnostics = Vec::new();
        scan(
            "\"🧪\"; test(\"empty\", () => {});\n",
            "fixture.ts",
            &filter,
            &mut diagnostics,
        );

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "expect-expect");
        assert_eq!(diagnostics[0].message_id, "noAssertions");
        assert!(diagnostics[0].data.is_empty());
        assert_eq!(
            (
                diagnostics[0].start_line,
                diagnostics[0].start_column,
                diagnostics[0].end_line,
                diagnostics[0].end_column,
            ),
            (1, 6, 1, 10)
        );
    }
}
