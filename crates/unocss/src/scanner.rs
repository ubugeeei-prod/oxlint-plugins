//! Scanner driver: walks the oxc AST to emit unocss diagnostics.
//!
//! The actual AST traversal is performed by [`crate::visitor::UnocssVisitor`].
//! This module consumes the collected spans and runs the per-rule check logic.

use oxc_ast::ast::Program;
use oxc_ast_visit::Visit;
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::ordering::{
    is_unocss_token, join_tokens, prefix_with_space, sort_class_tokens, sorted_class_string,
};
use crate::types::{Diagnostic, DiagnosticFix, LineIndex, LiteralSpan, ReportData, UnocssOptions};
use crate::visitor::{OpeningElement, UnocssVisitor};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) options: UnocssOptions,
    pub(crate) variable_regexes: SmallVec<[Regex; 4]>,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
}

impl<'a> Scanner<'a> {
    /// Run all rule checks by first collecting AST-based spans via the visitor,
    /// then applying per-rule logic.
    ///
    /// Diagnostic ordering (preserved from old scanner):
    ///  1. For each class literal: blocklist (per-token), enforce-class-compile, order.
    ///  2. For each call/variable literal: order only.
    ///  3. For each JSX opening element: order-attributify.
    pub(crate) fn run(&mut self, program: &Program<'a>) {
        // Run the AST visitor in a scoped block so its immutable borrows of
        // `self.options.uno_functions` / `self.variable_regexes` end before the
        // `&mut self` check calls below. The collected spans borrow only
        // `source_text` (lifetime `'a`), not `self`, so they move out cleanly.
        let (class_literals, call_literals, opening_elements) = {
            let mut visitor = UnocssVisitor::new(
                self.source_text,
                &self.options.uno_functions,
                &self.variable_regexes,
            );
            visitor.visit_program(program);
            (
                visitor.class_literals,
                visitor.call_literals,
                visitor.opening_elements,
            )
        };

        // Phase 1 – class literals.
        for lit in class_literals {
            self.check_blocklist(lit);
            self.check_class_compile(lit);
            self.check_order(lit);
        }

        // Phase 2 – call / variable literals (order only).
        for lit in call_literals {
            self.check_order(lit);
        }

        // Phase 3 – attributify (after all literal diagnostics).
        for elem in opening_elements {
            self.check_element_attributify(&elem);
        }
    }

    // ── Rule implementations ────────────────────────────────────────────────

    fn check_blocklist(&mut self, literal: LiteralSpan<'_>) {
        for token in literal.content.split_whitespace() {
            // `split_whitespace` yields subslices of `content`, so the token's
            // byte offset within the literal is its pointer distance from the
            // content start. Report the precise token span, not the whole string.
            let offset = token.as_ptr() as usize - literal.content.as_ptr() as usize;
            let start = literal.content_start + offset;
            self.check_blocked_token(token, Span::new(start as u32, (start + token.len()) as u32));
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

    fn check_element_attributify(&mut self, elem: &OpeningElement) {
        let attrs = &elem.valueless_attrs;

        // Blocklist check for every valueless attribute (mirrors old scanner).
        for (name, span) in attrs {
            self.check_blocked_token(name.as_str(), *span);
        }

        // Filter to UnoCSS tokens only for order-attributify.
        let uno_attrs: SmallVec<[(CompactString, Span); 8]> = attrs
            .iter()
            .filter(|(name, _)| is_unocss_token(name.as_str()))
            .cloned()
            .collect();

        if uno_attrs.len() < 2 {
            return;
        }

        let names: SmallVec<[&str; 8]> = uno_attrs.iter().map(|(n, _)| n.as_str()).collect();
        let sorted = sort_class_tokens(names.as_slice());
        let sorted_text = join_tokens(sorted.as_slice());
        let input_text = join_tokens(names.as_slice());
        if sorted_text == input_text {
            return;
        }

        // Build a fix only when the attrs are contiguous in source. `get`
        // guards against any degenerate (reversed/out-of-range) span pair so a
        // parser edge case can never panic at the NAPI boundary.
        let contiguous = uno_attrs.windows(2).all(|pair| {
            let range = pair[0].1.end as usize..pair[1].1.start as usize;
            self.source_text
                .get(range)
                .is_some_and(|between| between.trim().is_empty())
        });
        let fix = if contiguous {
            let start = uno_attrs[0].1.start;
            let end = uno_attrs[uno_attrs.len() - 1].1.end;
            Some(DiagnosticFix {
                start,
                end,
                replacement: sorted_text,
            })
        } else {
            None
        };
        self.report(
            "order-attributify",
            "invalid-order",
            elem.span,
            ReportData {
                fix,
                ..ReportData::default()
            },
        );
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
