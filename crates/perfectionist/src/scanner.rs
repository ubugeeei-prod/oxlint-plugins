//! Heuristic scanner for the perfectionist port.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::helpers::{extract_names, find_matching, is_unsorted, normalize_name};
use crate::types::{Diagnostic, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 24]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan(&mut self) {
        self.check_regex("sort-named-imports", r"import\s*\{\s*b\s*,\s*a\s*\}");
        self.check_regex("sort-named-exports", r"export\s*\{\s*b\s*,\s*a\s*\}");
        self.check_regex(
            "sort-imports",
            r#"(?s)import\s+.+from\s+['"]z['"].*import\s+.+from\s+['"]a['"]"#,
        );
        self.check_regex(
            "sort-exports",
            r#"(?s)export\s+.+from\s+['"]z['"].*export\s+.+from\s+['"]a['"]"#,
        );
        self.check_regex(
            "sort-import-attributes",
            r#"import\s+.+\s+with\s*\{\s*type\s*:\s*['"]json['"]\s*,\s*foo\s*:"#,
        );
        self.check_regex(
            "sort-export-attributes",
            r#"export\s+.+\s+with\s*\{\s*type\s*:\s*['"]json['"]\s*,\s*foo\s*:"#,
        );
        self.check_regex("sort-heritage-clauses", r"implements\s+Z\s*,\s*A");
        self.check_regex("sort-decorators", r"(?s)@Z\s+@A");
        self.check_regex(
            "sort-classes",
            r"(?s)class\s+Class\s*\{.*\bb\s*\([^)]*\).*?\ba\s*\([^)]*\)",
        );
        self.check_regex("sort-jsx-props", r"<Component\s+b=\{[^}]*\}\s+a=");
        self.check_regex("sort-variable-declarations", r"const\s+b\s*=.*,\s*a\s*=");
        self.check_regex(
            "sort-array-includes",
            r#"\[\s*['"]b['"]\s*,\s*['"]a['"]\s*\]\s*\.\s*includes\s*\("#,
        );

        self.check_delimited_after("sort-arrays", "const array", '[', ']');
        self.check_delimited_after("sort-sets", "new Set", '[', ']');
        self.check_delimited_after("sort-maps", "new Map", '[', ']');
        self.check_delimited_after("sort-objects", "const object", '{', '}');
        self.check_delimited_after("sort-object-types", "type ObjectType", '{', '}');
        self.check_delimited_after("sort-interfaces", "interface Interface", '{', '}');
        self.check_delimited_after("sort-enums", "enum Enum", '{', '}');
        self.check_type_sequence("sort-union-types", "type Union", '|');
        self.check_type_sequence("sort-intersection-types", "type Intersection", '&');
        self.check_switch_cases();
        self.check_modules();
    }

    fn check_regex(&mut self, rule_name: &'static str, pattern: &str) {
        let Some(regex) = Regex::new(pattern).ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                rule_name,
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn check_delimited_after(
        &mut self,
        rule_name: &'static str,
        marker: &str,
        open: char,
        close: char,
    ) {
        let Some(marker_index) = self.source_text.find(marker) else {
            return;
        };
        let Some(open_offset) = self.source_text[marker_index..].find(open) else {
            return;
        };
        let open_index = marker_index + open_offset;
        let close_index = if open == '=' {
            marker_index + self.source_text[marker_index..].find(close).unwrap_or(0)
        } else {
            find_matching(self.source_text, open_index, open, close).unwrap_or(open_index)
        };
        if close_index <= open_index {
            return;
        }
        let start = if open == '=' {
            open_index + 1
        } else {
            open_index + open.len_utf8()
        };
        let segment = &self.source_text[start..close_index];
        let names = extract_names(segment);
        if is_unsorted(names.as_slice()) {
            self.report(
                rule_name,
                Span::new(open_index as u32, (close_index + 1) as u32),
            );
        }
    }

    fn check_type_sequence(&mut self, rule_name: &'static str, marker: &str, delimiter: char) {
        let Some(marker_index) = self.source_text.find(marker) else {
            return;
        };
        let Some(equal_offset) = self.source_text[marker_index..].find('=') else {
            return;
        };
        let start = marker_index + equal_offset + 1;
        let end = self.source_text[start..]
            .find(';')
            .map_or(self.source_text.len(), |offset| start + offset);
        let names: SmallVec<[CompactString; 8]> = self.source_text[start..end]
            .split(delimiter)
            .map(normalize_name)
            .filter(|name| !name.is_empty())
            .collect();
        if is_unsorted(names.as_slice()) {
            self.report(rule_name, Span::new(start as u32, end as u32));
        }
    }

    fn check_switch_cases(&mut self) {
        let Some(regex) = Regex::new(r#"case\s+['"]b['"]\s*:.*case\s+['"]a['"]\s*:"#).ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                "sort-switch-case",
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn check_modules(&mut self) {
        let Some(regex) = Regex::new(r"(?s)const\s+z\b.*function\s+a\b").ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                "sort-modules",
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn report(&mut self, rule_name: &'static str, span: Span) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id: "unexpected",
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}
