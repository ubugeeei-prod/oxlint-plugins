//! Scanner driver: walks literals and JSX tags to emit unocss diagnostics.

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::literals::{
    collect_literals, is_jsx_class_literal, is_uno_call_literal, variable_name_in_statement,
};
use crate::ordering::{
    is_unocss_token, join_tokens, prefix_with_space, sort_class_tokens, sorted_class_string,
};
use crate::tags::{
    IGNORED_ATTRIBUTIFY_ATTRIBUTES, find_tag_end, is_attr_name_part, is_identifier_part,
    is_identifier_start, skip_attribute_value,
};
use crate::types::{
    Diagnostic, DiagnosticFix, LineIndex, LiteralSpan, ReportData, UnocssOptions,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: UnocssOptions,
    pub(crate) variable_regexes: SmallVec<[Regex; 4]>,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan_literals(&mut self) {
        for literal in collect_literals(self.source_text) {
            if literal.content.trim().is_empty() || literal.content.contains('\\') {
                continue;
            }

            let class_context = is_jsx_class_literal(self.source_text, literal);
            if class_context {
                self.check_blocklist(literal);
                self.check_class_compile(literal);
            }

            if class_context
                || is_uno_call_literal(
                    self.source_text,
                    literal.full_start,
                    &self.options.uno_functions,
                )
                || self.is_uno_variable_literal(literal.full_start)
            {
                self.check_order(literal);
            }
        }
    }

    pub(crate) fn scan_attributify(&mut self) {
        let bytes = self.source_text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] != b'<' || index + 1 >= bytes.len() || bytes[index + 1] == b'/' {
                index += 1;
                continue;
            }
            if !is_identifier_start(bytes[index + 1]) {
                index += 1;
                continue;
            }

            let Some(tag_end) = find_tag_end(self.source_text, index + 1) else {
                break;
            };
            self.check_tag_attributify(index, tag_end);
            index = tag_end + 1;
        }
    }

    fn check_tag_attributify(&mut self, tag_start: usize, tag_end: usize) {
        let mut cursor = tag_start + 1;
        let bytes = self.source_text.as_bytes();
        while cursor < tag_end && is_identifier_part(bytes[cursor]) {
            cursor += 1;
        }

        let mut attrs: SmallVec<[(CompactString, usize, usize); 8]> = SmallVec::new();
        while cursor < tag_end {
            while cursor < tag_end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor >= tag_end || bytes[cursor] == b'/' {
                break;
            }
            if !is_identifier_start(bytes[cursor]) && bytes[cursor] != b':' {
                cursor += 1;
                continue;
            }

            let name_start = cursor;
            cursor += 1;
            while cursor < tag_end && is_attr_name_part(bytes[cursor]) {
                cursor += 1;
            }
            let name_end = cursor;
            let name = &self.source_text[name_start..name_end];
            while cursor < tag_end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }

            if cursor < tag_end && bytes[cursor] == b'=' {
                cursor = skip_attribute_value(self.source_text, cursor + 1, tag_end);
                continue;
            }

            let lower = name.to_ascii_lowercase();
            if IGNORED_ATTRIBUTIFY_ATTRIBUTES.contains(&lower.as_str()) {
                continue;
            }

            self.check_blocked_token(name, Span::new(name_start as u32, name_end as u32));
            if is_unocss_token(name) {
                attrs.push((CompactString::from(name), name_start, name_end));
            }
        }

        if attrs.len() < 2 {
            return;
        }

        let names: SmallVec<[&str; 8]> = attrs.iter().map(|(name, _, _)| name.as_str()).collect();
        let sorted = sort_class_tokens(names.as_slice());
        let sorted_text = join_tokens(sorted.as_slice());
        let input_text = join_tokens(names.as_slice());
        if sorted_text == input_text {
            return;
        }

        let contiguous = attrs
            .windows(2)
            .all(|pair| self.source_text[pair[0].2..pair[1].1].trim().is_empty());
        let fix = if contiguous {
            let start = attrs[0].1;
            let end = attrs[attrs.len() - 1].2;
            Some(DiagnosticFix {
                start: start as u32,
                end: end as u32,
                replacement: sorted_text,
            })
        } else {
            None
        };
        self.report(
            "order-attributify",
            "invalid-order",
            Span::new(tag_start as u32, tag_end as u32),
            ReportData {
                fix,
                ..ReportData::default()
            },
        );
    }

    fn check_blocklist(&mut self, literal: LiteralSpan<'_>) {
        for token in literal.content.split_whitespace() {
            self.check_blocked_token(
                token,
                Span::new(literal.content_start as u32, literal.content_end as u32),
            );
        }
    }

    fn check_blocked_token(&mut self, token: &str, span: Span) {
        let matches: SmallVec<[(CompactString, CompactString); 2]> = self
            .options
            .blocklist
            .iter()
            .filter(|entry| entry.name.as_str() == token)
            .map(|entry| (entry.name.clone(), entry.reason.clone()))
            .collect();

        for (name, reason) in matches {
            self.report(
                "blocklist",
                "in-blocklist",
                span,
                ReportData {
                    name: Some(name),
                    reason: Some(reason),
                    ..ReportData::default()
                },
            );
        }
    }

    fn check_class_compile(&mut self, literal: LiteralSpan<'_>) {
        let trimmed = literal.content.trim_start();
        if trimmed.is_empty() {
            return;
        }
        let expected_prefix = prefix_with_space(self.options.class_compile_prefix.trim());
        if trimmed.starts_with(expected_prefix.as_str()) {
            return;
        }

        let mut replacement = expected_prefix.clone();
        replacement.push_str(literal.content);
        let fix = if self.options.class_compile_enable_fix {
            Some(DiagnosticFix {
                start: literal.content_start as u32,
                end: literal.content_end as u32,
                replacement,
            })
        } else {
            None
        };
        self.report(
            "enforce-class-compile",
            "missing",
            Span::new(literal.content_start as u32, literal.content_end as u32),
            ReportData {
                fix,
                prefix: Some(self.options.class_compile_prefix.clone()),
                ..ReportData::default()
            },
        );
    }

    fn check_order(&mut self, literal: LiteralSpan<'_>) {
        let Some(replacement) = sorted_class_string(literal.content) else {
            return;
        };
        self.report(
            "order",
            "invalid-order",
            Span::new(literal.content_start as u32, literal.content_end as u32),
            ReportData {
                fix: Some(DiagnosticFix {
                    start: literal.content_start as u32,
                    end: literal.content_end as u32,
                    replacement,
                }),
                ..ReportData::default()
            },
        );
    }

    fn is_uno_variable_literal(&self, start: usize) -> bool {
        let statement_start = self.source_text[..start]
            .rfind(';')
            .map_or(0, |index| index + 1);
        let statement = &self.source_text[statement_start..start];
        let Some(name) = variable_name_in_statement(statement) else {
            return false;
        };
        self.variable_regexes
            .iter()
            .any(|regex| regex.is_match(name))
    }

    fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        data: ReportData,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fix: data.fix,
            name: data.name,
            reason: data.reason,
            prefix: data.prefix,
        });
    }
}
