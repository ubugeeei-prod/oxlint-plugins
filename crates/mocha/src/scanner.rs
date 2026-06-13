//! Scanner driver, scope/frame tracking, and entity-callback dispatch for the
//! mocha port. Per-rule checks live in `checks.rs`; statement and expression
//! traversal in `statements.rs` and `expressions.rs`.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::helpers::{
    argument_string_value, call_path, callback_from_argument, classify_mocha_path,
    direct_statement_mocha_span,
};
use crate::{
    Callback, CallbackBody, ContextKind, Diagnostic, Entity, EntityType, Layer, LineIndex,
    MochaOptions,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) options: &'a MochaOptions,
    pub(crate) valid_suite_regex: Option<Regex>,
    pub(crate) valid_test_regex: Option<Regex>,
    pub(crate) layers: SmallVec<[Layer; 8]>,
    pub(crate) suite_depth: u32,
    pub(crate) test_depth: u32,
    pub(crate) top_level_suites: u32,
    pub(crate) has_test_entity: bool,
    pub(crate) export_spans: SmallVec<[Span; 8]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn report(
        &mut self,
        rule_name: &'static str,
        message: impl Into<CompactString>,
        span: Span,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message: message.into(),
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    pub(crate) fn finish_program(&mut self) {
        if self.has_test_entity {
            let spans = self.export_spans.clone();
            for span in spans {
                self.report("no-exports", "Unexpected export from a test file", span);
            }
        }
    }

    pub(crate) fn scan_statement_list(
        &mut self,
        statements: &'a [Statement<'a>],
        context: ContextKind,
        direct_spacing_scope: bool,
    ) {
        let mut previous_end_line = None;
        for statement in statements {
            if direct_spacing_scope {
                if let Some(entity_span) = direct_statement_mocha_span(statement)
                    && let Some(end_line) = previous_end_line
                {
                    let (start_line, _) = self
                        .line_index
                        .position_for_offset(self.source_text, entity_span.start);
                    if start_line.saturating_sub(end_line) < 2 {
                        self.report(
                            "consistent-spacing-between-blocks",
                            "Expected line break before this statement.",
                            entity_span,
                        );
                    }
                }
                previous_end_line = Some(
                    self.line_index
                        .position_for_offset(self.source_text, statement.span().end)
                        .0,
                );
            }
            self.scan_statement(statement, context);
        }
    }

    pub(crate) fn scan_function(&mut self, function: &'a Function<'a>) {
        if let Some(body) = &function.body {
            self.scan_statement_list(&body.statements, ContextKind::Other, false);
        }
    }

    pub(crate) fn scan_arrow_function(&mut self, function: &'a ArrowFunctionExpression<'a>) {
        self.scan_statement_list(&function.body.statements, ContextKind::Other, false);
    }

    pub(crate) fn scan_class(&mut self, class: &'a Class<'a>) {
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.scan_statement_list(&block.body, ContextKind::Other, false);
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value);
                }
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ContextKind::Other);
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn scan_entity_callback(&mut self, entity: &Entity<'a>) {
        let Some(callback) = entity.callback else {
            return;
        };
        match entity.entity_type {
            EntityType::Suite => {
                self.suite_depth += 1;
                self.layers.push(Layer::default());
                self.scan_callback_body(callback.body, ContextKind::SuiteCallback);
                let layer = self.layers.pop().expect("suite layer exists");
                if layer.test_count == 1 {
                    for (hook_name, span) in layer.hooks {
                        if !self
                            .options
                            .no_hooks_for_single_case_allowed
                            .iter()
                            .any(|allowed| allowed.as_str() == hook_name.as_str())
                        {
                            self.report(
                                "no-hooks-for-single-case",
                                crate::helpers::compact_format(format_args!(
                                    "Unexpected use of Mocha `{}` hook for a single test case",
                                    crate::helpers::display_call_name(hook_name.as_str())
                                )),
                                span,
                            );
                        }
                    }
                }
                self.suite_depth = self.suite_depth.saturating_sub(1);
            }
            EntityType::TestCase => {
                self.test_depth += 1;
                self.scan_callback_body(callback.body, ContextKind::TestCallback);
                self.test_depth = self.test_depth.saturating_sub(1);
            }
            EntityType::Hook => {
                self.scan_callback_body(callback.body, ContextKind::HookCallback);
            }
        }
    }

    fn scan_callback_body(&mut self, body: CallbackBody<'a>, context: ContextKind) {
        match body {
            CallbackBody::Function(body) => self.scan_statement_list(
                &body.statements,
                context,
                context == ContextKind::SuiteCallback,
            ),
        }
    }

    pub(crate) fn entity_for_call(&self, call: &'a CallExpression<'a>) -> Option<Entity<'a>> {
        let path = call_path(call)?;
        let (name, entity_type, interface, modifier) = classify_mocha_path(&path)?;
        Some(Entity {
            name: CompactString::from(name),
            entity_type,
            interface,
            modifier,
            span: call.span,
            title: call
                .arguments
                .first()
                .and_then(argument_string_value)
                .map(CompactString::from),
            callback: call.arguments.last().and_then(callback_from_argument),
        })
    }
}
