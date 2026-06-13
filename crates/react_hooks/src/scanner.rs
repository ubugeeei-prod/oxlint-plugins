//! Scanner driver, frame tracking, and per-hook reporting for the react-hooks
//! port. Statement and expression traversal live in `statements.rs` and
//! `expressions.rs`.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::helpers::{method_name, object_is_pascal_case_identifier};
use crate::{
    Diagnostic, DiagnosticData, FunctionFrame, HookCall, LineIndex, is_hook_name,
    is_react_component_name,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) frames: SmallVec<[FunctionFrame; 8]>,
    pub(crate) class_depth: u32,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan_statement_list(&mut self, statements: &'a [Statement<'a>]) {
        for statement in statements {
            self.scan_statement(statement);
        }
    }

    pub(crate) fn scan_function(
        &mut self,
        function: &'a Function<'a>,
        name: Option<&'a str>,
        special_component_callback: bool,
    ) {
        let Some(body) = function.body.as_deref() else {
            return;
        };
        self.enter_function(
            name,
            function.r#async,
            self.class_depth > 0,
            special_component_callback,
            body,
        );
    }

    pub(crate) fn scan_arrow_function(
        &mut self,
        function: &'a ArrowFunctionExpression<'a>,
        name: Option<&'a str>,
        special_component_callback: bool,
    ) {
        self.enter_function(
            name,
            function.r#async,
            self.class_depth > 0,
            special_component_callback,
            &function.body,
        );
    }

    fn enter_function(
        &mut self,
        name: Option<&'a str>,
        async_function: bool,
        in_class: bool,
        special_component_callback: bool,
        body: &'a FunctionBody<'a>,
    ) {
        let valid_scope = special_component_callback
            || name.is_some_and(|name| is_react_component_name(name) || is_hook_name(name));
        let inside_valid_scope = valid_scope
            || self
                .current_frame()
                .is_some_and(FunctionFrame::is_inside_valid_scope);
        self.frames.push(FunctionFrame {
            name: name.map(CompactString::from),
            valid_scope,
            inside_valid_scope,
            async_function,
            in_class,
            conditional_depth: 0,
            loop_depth: 0,
            try_depth: 0,
            possible_early_return: false,
        });
        self.scan_statement_list(&body.statements);
        let _ = self.frames.pop();
    }

    pub(crate) fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class);
        }

        self.class_depth += 1;
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.scan_statement_list(&block.body);
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value, method_name(&method.key), false);
                }
                ClassElement::PropertyDefinition(property) => {
                    if property.computed {
                        self.scan_property_key(&property.key);
                    }
                    if let Some(value) = &property.value {
                        self.scan_expression(value);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if property.computed {
                        self.scan_property_key(&property.key);
                    }
                    if let Some(value) = &property.value {
                        self.scan_expression(value);
                    }
                }
                ClassElement::TSIndexSignature(_) => {}
            }
        }
        self.class_depth = self.class_depth.saturating_sub(1);
    }

    pub(crate) fn hook_call(&self, call: &'a CallExpression<'a>) -> Option<HookCall> {
        match call.callee.get_inner_expression() {
            Expression::Identifier(identifier) if is_hook_name(identifier.name.as_str()) => {
                Some(HookCall {
                    name: CompactString::from(identifier.name.as_str()),
                    span: identifier.span,
                    is_use: identifier.name == "use",
                })
            }
            Expression::StaticMemberExpression(member)
                if is_hook_name(member.property.name.as_str())
                    && object_is_pascal_case_identifier(&member.object) =>
            {
                Some(HookCall {
                    name: self.compact_text(member.span),
                    span: member.span,
                    is_use: member.property.name == "use",
                })
            }
            _ => None,
        }
    }

    pub(crate) fn report_hook_call(&mut self, hook_call: &HookCall) {
        let Some(frame) = self.current_frame() else {
            if self.class_depth > 0 {
                self.report("class", hook_call, None);
            } else {
                self.report("topLevel", hook_call, None);
            }
            return;
        };

        let message_id = if hook_call.is_use && frame.try_depth > 0 {
            Some("tryCatch")
        } else if !hook_call.is_use && frame.loop_depth > 0 {
            Some("loop")
        } else if frame.in_class {
            Some("class")
        } else if !frame.valid_scope {
            if frame.is_inside_valid_scope() {
                Some("callback")
            } else if frame.name.is_some() {
                Some("invalidFunction")
            } else {
                Some("callback")
            }
        } else if !hook_call.is_use && frame.async_function {
            Some("async")
        } else if !hook_call.is_use && (frame.conditional_depth > 0 || frame.possible_early_return)
        {
            Some("conditional")
        } else {
            None
        };

        if let Some(message_id) = message_id {
            let function_name = frame.name.as_ref().cloned();
            self.report(message_id, hook_call, function_name);
        }
    }

    fn report(
        &mut self,
        message_id: &'static str,
        hook_call: &HookCall,
        function_name: Option<CompactString>,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name: "rules-of-hooks",
            message_id,
            data: DiagnosticData {
                hook: Some(hook_call.name.clone()),
                function_name,
            },
            loc: self
                .line_index
                .loc_for_span(self.source_text, hook_call.span),
        });
    }

    fn compact_text(&self, span: Span) -> CompactString {
        let start = span.start as usize;
        let end = span.end as usize;
        self.source_text
            .get(start..end)
            .map_or_else(|| CompactString::from("React Hook"), CompactString::from)
    }

    pub(crate) fn with_conditional(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.current_frame_mut() {
            frame.conditional_depth += 1;
        }
        f(self);
        if let Some(frame) = self.current_frame_mut() {
            frame.conditional_depth = frame.conditional_depth.saturating_sub(1);
        }
    }

    pub(crate) fn with_loop(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.current_frame_mut() {
            frame.loop_depth += 1;
        }
        f(self);
        if let Some(frame) = self.current_frame_mut() {
            frame.loop_depth = frame.loop_depth.saturating_sub(1);
        }
    }

    pub(crate) fn with_try(&mut self, f: impl FnOnce(&mut Self)) {
        if let Some(frame) = self.current_frame_mut() {
            frame.try_depth += 1;
        }
        f(self);
        if let Some(frame) = self.current_frame_mut() {
            frame.try_depth = frame.try_depth.saturating_sub(1);
        }
    }

    pub(crate) fn current_frame(&self) -> Option<&FunctionFrame> {
        self.frames.last()
    }

    pub(crate) fn current_frame_mut(&mut self) -> Option<&mut FunctionFrame> {
        self.frames.last_mut()
    }
}

impl FunctionFrame {
    pub(crate) fn is_inside_valid_scope(&self) -> bool {
        self.valid_scope || self.inside_valid_scope
    }
}
