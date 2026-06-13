//! TypeScript type traversal and type-shape checks for the functional scanner.

use oxc_ast::ast::*;

use crate::helpers::{
    call_arguments_match_params, has_mixed_signatures, interface_has_mutable_members,
    is_mutable_collection_name, is_mutable_type, single_returned_call, type_reference_name,
};
use crate::scanner::Scanner;

impl<'a> Scanner<'a> {
    pub(crate) fn check_prefer_tacit(&mut self, function: &'a ArrowFunctionExpression<'a>) {
        if function.params.rest.is_some() || function.params.items.is_empty() {
            return;
        }
        let Some(call) = single_returned_call(&function.body) else {
            return;
        };
        if !call_arguments_match_params(call, &function.params) {
            return;
        }
        self.report(
            "prefer-tacit",
            "generic",
            "Potentially unnecessary function wrapper.",
            function.span,
        );
    }

    pub(crate) fn scan_type_alias_declaration(
        &mut self,
        declaration: &'a TSTypeAliasDeclaration<'a>,
    ) {
        if let TSType::TSTypeLiteral(type_literal) = &declaration.type_annotation
            && has_mixed_signatures(&type_literal.members)
        {
            self.report(
                "no-mixed-types",
                "generic",
                "Only the same kind of members allowed in types.",
                declaration.span,
            );
        }
        if is_mutable_type(&declaration.type_annotation) {
            self.report(
                "type-declaration-immutability",
                "AtLeast",
                "This type declaration contains mutable members.",
                declaration.span,
            );
        }
        self.scan_type(&declaration.type_annotation);
    }

    pub(crate) fn scan_interface_declaration(
        &mut self,
        declaration: &'a TSInterfaceDeclaration<'a>,
    ) {
        if has_mixed_signatures(&declaration.body.body) {
            self.report(
                "no-mixed-types",
                "generic",
                "Only the same kind of members allowed in types.",
                declaration.span,
            );
        }
        if interface_has_mutable_members(&declaration.body.body) {
            self.report(
                "type-declaration-immutability",
                "AtLeast",
                "This type declaration contains mutable members.",
                declaration.span,
            );
        }
        for signature in &declaration.body.body {
            self.scan_signature(signature);
        }
    }

    pub(crate) fn scan_signature(&mut self, signature: &'a TSSignature<'a>) {
        match signature {
            TSSignature::TSMethodSignature(method) => {
                self.report(
                    "prefer-property-signatures",
                    "generic",
                    "Use a property signature instead of a method signature",
                    method.span,
                );
                if let Some(return_type) = &method.return_type {
                    self.scan_return_type(return_type);
                }
            }
            TSSignature::TSPropertySignature(property) => {
                if property.readonly && self.options.readonly_type_mode == "generic" {
                    self.report(
                        "readonly-type",
                        "generic",
                        "Readonly type using 'readonly' keyword is forbidden. Use 'Readonly<T>' instead.",
                        property.span,
                    );
                }
                if !property.readonly {
                    self.report(
                        "prefer-readonly-type",
                        "property",
                        "A readonly modifier is required.",
                        property.span,
                    );
                }
                if let Some(type_annotation) = &property.type_annotation {
                    self.scan_type(&type_annotation.type_annotation);
                }
            }
            TSSignature::TSIndexSignature(signature) => {
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
            TSSignature::TSCallSignatureDeclaration(signature) => {
                if let Some(return_type) = &signature.return_type {
                    self.scan_return_type(return_type);
                }
            }
            TSSignature::TSConstructSignatureDeclaration(signature) => {
                if let Some(return_type) = &signature.return_type {
                    self.scan_return_type(return_type);
                }
            }
        }
    }

    pub(crate) fn scan_type(&mut self, ty: &'a TSType<'a>) {
        match ty {
            TSType::TSArrayType(array) => {
                self.report(
                    "prefer-readonly-type",
                    "array",
                    "Only readonly arrays allowed.",
                    array.span,
                );
                self.report(
                    "prefer-immutable-types",
                    "parameter",
                    "Only readonly types allowed.",
                    array.span,
                );
                self.scan_type(&array.element_type);
            }
            TSType::TSTupleType(tuple) => {
                self.report(
                    "prefer-readonly-type",
                    "tuple",
                    "Only readonly tuples allowed.",
                    tuple.span,
                );
            }
            TSType::TSTypeReference(reference) => {
                if type_reference_name(reference).is_some_and(is_mutable_collection_name) {
                    self.report(
                        "prefer-readonly-type",
                        "type",
                        "Only readonly types allowed.",
                        reference.span,
                    );
                    self.report(
                        "prefer-immutable-types",
                        "parameter",
                        "Only readonly types allowed.",
                        reference.span,
                    );
                }
                if let Some(arguments) = &reference.type_arguments {
                    for ty in &arguments.params {
                        self.scan_type(ty);
                    }
                }
            }
            TSType::TSTypeOperatorType(operator) => {
                self.scan_type_operator(operator);
            }
            TSType::TSTypeLiteral(literal) => {
                if has_mixed_signatures(&literal.members) {
                    self.report(
                        "no-mixed-types",
                        "generic",
                        "Only the same kind of members allowed in types.",
                        literal.span,
                    );
                }
                for signature in &literal.members {
                    self.scan_signature(signature);
                }
            }
            TSType::TSUnionType(union) => {
                for ty in &union.types {
                    self.scan_type(ty);
                }
            }
            TSType::TSIntersectionType(intersection) => {
                for ty in &intersection.types {
                    self.scan_type(ty);
                }
            }
            TSType::TSParenthesizedType(parenthesized) => {
                self.scan_type(&parenthesized.type_annotation);
            }
            TSType::TSFunctionType(function) => {
                self.scan_return_type(&function.return_type);
            }
            _ => {}
        }
    }

    fn scan_type_operator(&mut self, operator: &'a TSTypeOperator<'a>) {
        if operator.operator == TSTypeOperatorOperator::Readonly {
            if self.options.readonly_type_mode == "keyword" {
                self.report(
                    "readonly-type",
                    "keyword",
                    "Readonly type using 'Readonly<T>' is forbidden. Use 'readonly' keyword instead.",
                    operator.span,
                );
            }
        } else {
            self.scan_type(&operator.type_annotation);
        }
    }
}
