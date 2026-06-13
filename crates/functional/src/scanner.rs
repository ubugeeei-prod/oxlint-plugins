//! Scanner state, reporting, and function/class walkers for the functional
//! port. Statement traversal lives in `statements.rs`, expression traversal in
//! `expressions.rs`, and TypeScript type checks in `types.rs`.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::helpers::{is_mutable_type, property_key_name};
use crate::{
    Diagnostic, EnforceParameterCount, FunctionContext, FunctionParamMeta, FunctionalOptions,
    LineIndex,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 32]>,
    pub(crate) options: &'a FunctionalOptions,
    /// True while traversing the type arguments of a `Readonly<...>` reference,
    /// so `prefer-property-signatures` can honor `ignoreIfReadonlyWrapped`.
    pub(crate) within_readonly: bool,
    pub(crate) ignore_identifier_regexes: SmallVec<[regex::Regex; 4]>,
    pub(crate) ignore_code_regexes: SmallVec<[regex::Regex; 4]>,
}

/// A function passed to `.then(...)`/`.catch(...)` is a promise handler: a throw
/// inside it rejects the surrounding promise (no-throw `allowToRejectPromises`).
fn meta_is_promise_handler(meta: &FunctionParamMeta<'_>) -> bool {
    meta.is_lambda_arg && matches!(meta.enclosing_call_property, Some("then") | Some("catch"))
}

impl<'a> Scanner<'a> {
    pub(crate) fn report(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        message: &'static str,
        span: Span,
    ) {
        if self.options.has_rule(rule_name) {
            self.diagnostics.push(Diagnostic {
                rule_name,
                message_id,
                message: message.into(),
                loc: self.line_index.loc_for_span(self.source_text, span),
            });
        }
    }

    pub(crate) fn scan_statement_list(
        &mut self,
        statements: &'a [Statement<'a>],
        context: FunctionContext,
    ) {
        for statement in statements {
            self.scan_statement(statement, context);
        }
    }

    pub(crate) fn scan_function(
        &mut self,
        function: &'a Function<'a>,
        meta: FunctionParamMeta<'a>,
    ) {
        self.scan_function_parameters(&function.params, function.span, meta);
        if let Some(return_type) = &function.return_type {
            self.scan_return_type(return_type);
        }
        let context = FunctionContext {
            in_async_function: function.r#async,
            in_try_with_catch: false,
            in_function: true,
            in_promise_handler: meta_is_promise_handler(&meta),
        };
        if let Some(body) = &function.body {
            self.scan_function_body(body, context);
        }
    }

    pub(crate) fn scan_arrow_function(
        &mut self,
        function: &'a ArrowFunctionExpression<'a>,
        meta: FunctionParamMeta<'a>,
    ) {
        self.scan_function_parameters(&function.params, function.span, meta);
        self.check_prefer_tacit(function);
        if let Some(return_type) = &function.return_type {
            self.scan_return_type(return_type);
        }
        let context = FunctionContext {
            in_async_function: function.r#async,
            in_try_with_catch: false,
            in_function: true,
            in_promise_handler: meta_is_promise_handler(&meta),
        };
        self.scan_function_body(&function.body, context);
    }

    fn function_is_ignored_by_prefix_selector(&self, meta: &FunctionParamMeta<'a>) -> bool {
        if self.options.ignore_prefix_selector_names.is_empty() {
            return false;
        }
        let prop = match meta.enclosing_call_property {
            Some(p) => p,
            None => return false,
        };
        for name in &self.options.ignore_prefix_selector_names {
            if name == prop {
                return true;
            }
        }
        false
    }

    fn scan_function_parameters(
        &mut self,
        params: &'a FormalParameters<'a>,
        span: Span,
        meta: FunctionParamMeta<'a>,
    ) {
        // Check ignoreIdentifierPattern — if the function name matches, skip all
        // functional-parameters checks for this function.
        let name_is_ignored = match meta.name {
            Some(name) => self.matches_ignore_identifier(name),
            None => false,
        };
        let prefix_selector_ignored = self.function_is_ignored_by_prefix_selector(&meta);
        let is_ignored = name_is_ignored || prefix_selector_ignored;

        if !is_ignored {
            if let Some(rest) = &params.rest {
                let allow_rest = self.options.allow_rest_parameter;
                if !allow_rest {
                    self.report(
                        "functional-parameters",
                        "restParam",
                        "Unexpected rest parameter. Use a regular parameter of type array instead.",
                        rest.span,
                    );
                }
            }

            // Parameter count enforcement.
            let enforce = self.options.enforce_parameter_count;
            if enforce != EnforceParameterCount::Off {
                // Determine whether to skip the count check.
                let skip_iife = meta.is_iife && self.options.enforce_count_ignore_iife;
                let skip_getter_setter =
                    meta.is_getter_setter && self.options.enforce_count_ignore_getters_setters;
                let skip_lambda = meta.is_lambda_arg && self.options.enforce_count_ignore_lambda;
                let skip_count = skip_iife || skip_getter_setter || skip_lambda;

                if !skip_count {
                    // Upstream counts node.params.length which includes the rest element.
                    let param_count =
                        params.items.len() + if params.rest.is_some() { 1 } else { 0 };
                    if enforce == EnforceParameterCount::AtLeastOne && param_count == 0 {
                        self.report(
                            "functional-parameters",
                            "paramCountAtLeastOne",
                            "Functions must have at least one parameter.",
                            span,
                        );
                    } else if enforce == EnforceParameterCount::ExactlyOne && param_count != 1 {
                        self.report(
                            "functional-parameters",
                            "paramCountExactlyOne",
                            "Functions must have exactly one parameter.",
                            span,
                        );
                    }
                }
            }
        }

        for param in &params.items {
            if let Some(type_annotation) = &param.type_annotation {
                self.scan_type(&type_annotation.type_annotation);
                if is_mutable_type(&type_annotation.type_annotation) {
                    self.report(
                        "prefer-immutable-types",
                        "parameter",
                        "Only readonly types allowed.",
                        type_annotation.span,
                    );
                }
            }
            if param.readonly && self.options.readonly_type_mode == "generic" {
                self.report(
                    "readonly-type",
                    "generic",
                    "Readonly type using 'readonly' keyword is forbidden. Use 'Readonly<T>' instead.",
                    param.span,
                );
            }
            if let Some(init) = &param.initializer {
                self.scan_expression(
                    init,
                    FunctionContext {
                        in_async_function: false,
                        in_try_with_catch: false,
                        in_function: false,
                        in_promise_handler: false,
                    },
                );
            }
        }
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>, context: FunctionContext) {
        self.scan_statement_list(&body.statements, context);
    }

    pub(crate) fn scan_return_type(&mut self, return_type: &'a oxc_ast::ast::TSTypeAnnotation<'a>) {
        match &return_type.type_annotation {
            TSType::TSVoidKeyword(_) => {
                self.report(
                    "no-return-void",
                    "generic",
                    "Function must return a value.",
                    return_type.span,
                );
            }
            TSType::TSNullKeyword(_) => {
                self.report(
                    "no-return-void",
                    "generic",
                    "Function must return a value.",
                    return_type.span,
                );
            }
            TSType::TSUndefinedKeyword(_) => {
                self.report(
                    "no-return-void",
                    "generic",
                    "Function must return a value.",
                    return_type.span,
                );
            }
            _ => {}
        }
        self.scan_type(&return_type.type_annotation);
    }

    pub(crate) fn matches_ignore_identifier(&self, name: &str) -> bool {
        for regex in &self.ignore_identifier_regexes {
            if regex.is_match(name) {
                return true;
            }
        }
        false
    }

    pub(crate) fn class_is_ignored(&self, class: &Class<'a>) -> bool {
        if let Some(id) = &class.id {
            let name = id.name.as_str();
            if self
                .ignore_identifier_regexes
                .iter()
                .any(|re| re.is_match(name))
            {
                return true;
            }
        }
        if !self.ignore_code_regexes.is_empty() {
            let span = class.span;
            let code = self
                .source_text
                .get(span.start as usize..span.end as usize)
                .unwrap_or("");
            if self.ignore_code_regexes.iter().any(|re| re.is_match(code)) {
                return true;
            }
        }
        false
    }

    pub(crate) fn scan_class(&mut self, class: &'a Class<'a>, context: FunctionContext) {
        if !self.class_is_ignored(class) {
            self.report(
                "no-classes",
                "generic",
                "Unexpected class, use functions not classes.",
                class.span,
            );
            if class.r#abstract {
                self.report(
                    "no-class-inheritance",
                    "abstract",
                    "Unexpected abstract class.",
                    class.span,
                );
            }
            if class.super_class.is_some() {
                self.report(
                    "no-class-inheritance",
                    "extends",
                    "Unexpected class inheritance.",
                    class.span,
                );
            }
        }
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, context);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => self.scan_statement_list(&block.body, context),
                ClassElement::MethodDefinition(method) => {
                    let is_getter_setter = matches!(
                        method.kind,
                        MethodDefinitionKind::Get | MethodDefinitionKind::Set
                    );
                    let meta = FunctionParamMeta {
                        name: property_key_name(&method.key),
                        is_getter_setter,
                        ..FunctionParamMeta::default()
                    };
                    self.scan_function(&method.value, meta);
                }
                ClassElement::PropertyDefinition(property) => {
                    if let Some(type_annotation) = &property.type_annotation {
                        self.scan_type(&type_annotation.type_annotation);
                        if is_mutable_type(&type_annotation.type_annotation) && !property.readonly {
                            self.report(
                                "prefer-readonly-type",
                                "property",
                                "A readonly modifier is required.",
                                property.span,
                            );
                        }
                    }
                    if let Some(value) = &property.value {
                        self.scan_expression(value, context);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, context);
                    }
                }
                ClassElement::TSIndexSignature(signature) => {
                    if !signature.readonly {
                        self.report(
                            "prefer-readonly-type",
                            "property",
                            "A readonly modifier is required.",
                            signature.span,
                        );
                    }
                    self.scan_type(&signature.type_annotation.type_annotation);
                }
            }
        }
    }
}
