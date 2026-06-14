//! AST scanner for the storybook port. Contains the `Scanner` struct and
//! every traversal / rule check method as an `impl Scanner` block.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, AssignmentTarget, BindingPattern,
    CallExpression, ChainElement, Class, ClassElement, Declaration, ExportDefaultDeclarationKind,
    ExportNamedDeclaration, Expression, FormalParameters, Function, FunctionBody,
    ImportDeclaration, ImportDeclarationSpecifier, ModuleExportName, ObjectExpression,
    ObjectProperty, ObjectPropertyKind, PropertyKey, Statement, StaticMemberExpression,
    VariableDeclaration, VariableDeclarator,
};
use oxc_semantic::{AstNodes, Scoping, SymbolId};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

use crate::helpers::*;
use crate::{
    Descriptor, Diagnostic, DiagnosticData, DiagnosticFix, FUNCTIONS_TO_AWAIT, FunctionFrame,
    LineIndex, MetaResolution, NamedExport, ParentKind, StoryFilters, StorybookOptions,
    VariableInfo,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) options: &'a StorybookOptions,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    /// Scoping from semantic analysis; resolves an export's references for
    /// `prefer-pascal-case` renaming.
    pub(crate) scoping: &'a Scoping,
    /// AST nodes from semantic analysis; maps a reference back to its identifier.
    pub(crate) nodes: &'a AstNodes<'a>,
    pub(crate) variables: FastHashMap<CompactString, VariableInfo<'a>>,
    pub(crate) function_stack: SmallVec<[FunctionFrame; 8]>,
    pub(crate) first_non_import_span: Option<Span>,
    pub(crate) component_name: Option<CompactString>,
    pub(crate) imported_component_name: Option<CompactString>,
    pub(crate) has_default_export: bool,
    pub(crate) has_csf4_meta: bool,
    pub(crate) has_stories_of_import: bool,
    pub(crate) has_storybook_expect_import: bool,
    pub(crate) user_event_is_non_storybook: bool,
    pub(crate) named_exports: SmallVec<[NamedExport; 8]>,
    pub(crate) story_filters: StoryFilters,
    pub(crate) has_meta: bool,
    pub(crate) expect_invocations: SmallVec<[Span; 8]>,
}

impl<'a> Scanner<'a> {
    fn rule_enabled(&self, name: &'static str) -> bool {
        self.options
            .rule_names
            .iter()
            .any(|rule_name| rule_name == name)
    }

    fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, DiagnosticData::default(), span);
    }

    fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
    ) {
        if !self.rule_enabled(rule_name) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fixes: SmallVec::new(),
        });
    }

    fn report_with_fixes(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        data: DiagnosticData,
        span: Span,
        fixes: SmallVec<[DiagnosticFix; 2]>,
    ) {
        if !self.rule_enabled(rule_name) {
            return;
        }
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
            fixes,
        });
    }

    pub(crate) fn prepass_program(&mut self, statements: &'a [Statement<'a>]) {
        for statement in statements {
            if self.first_non_import_span.is_none()
                && !matches!(statement, Statement::ImportDeclaration(_))
            {
                self.first_non_import_span = Some(statement.span());
            }

            match statement {
                Statement::ImportDeclaration(declaration) => self.prepass_import(declaration),
                Statement::VariableDeclaration(declaration) => {
                    self.prepass_variable_declaration(declaration, true);
                }
                Statement::ExportNamedDeclaration(declaration) => {
                    if let Some(declaration) = &declaration.declaration {
                        self.prepass_declaration(declaration, true);
                    }
                    self.collect_named_exports(declaration);
                }
                Statement::ExportDefaultDeclaration(declaration) => {
                    self.has_default_export = true;
                    if let Some(meta) = self.resolve_meta_from_default(&declaration.declaration) {
                        self.has_meta = true;
                        self.story_filters = story_filters_from_meta(meta.object);
                    }
                }
                _ => {}
            }
        }
    }

    fn prepass_declaration(&mut self, declaration: &'a Declaration<'a>, top_level: bool) {
        if let Declaration::VariableDeclaration(declaration) = declaration {
            self.prepass_variable_declaration(declaration, top_level);
        }
    }

    fn prepass_variable_declaration(
        &mut self,
        declaration: &'a VariableDeclaration<'a>,
        top_level: bool,
    ) {
        for declarator in &declaration.declarations {
            let Some(name) = binding_identifier_name(&declarator.id) else {
                continue;
            };
            if name == "userEvent" {
                self.user_event_is_non_storybook = true;
            }
            if top_level && let Some(init) = &declarator.init {
                self.variables.insert(
                    CompactString::from(name),
                    VariableInfo {
                        init,
                        type_annotation_span: declarator.type_annotation.as_ref().map(|t| t.span),
                    },
                );
                if call_property_name(init) == Some("meta") {
                    self.has_csf4_meta = true;
                }
            }
        }
    }

    fn prepass_import(&mut self, declaration: &'a ImportDeclaration<'a>) {
        let package_name = declaration.source.value.as_str();
        if let Some(component_name) = &self.component_name
            && package_name
                .strip_prefix("./")
                .is_some_and(|rest| rest.starts_with(component_name.as_str()))
            && import_has_local_name(declaration, component_name)
        {
            self.imported_component_name = Some(component_name.clone());
        }

        if let Some(specifiers) = &declaration.specifiers {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        if module_export_name(&specifier.imported) == Some("storiesOf") {
                            self.has_stories_of_import = true;
                        }
                        if module_export_name(&specifier.imported) == Some("expect")
                            && matches!(
                                package_name,
                                "@storybook/jest" | "@storybook/test" | "storybook/test"
                            )
                        {
                            self.has_storybook_expect_import = true;
                        }
                        if module_export_name(&specifier.imported) == Some("userEvent")
                            && !matches!(
                                package_name,
                                "@storybook/testing-library" | "@storybook/test" | "storybook/test"
                            )
                        {
                            self.user_event_is_non_storybook = true;
                        }
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        if specifier.local.name == "userEvent" {
                            self.user_event_is_non_storybook = true;
                        }
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {}
                }
            }
        }
    }

    fn collect_named_exports(&mut self, declaration: &'a ExportNamedDeclaration<'a>) {
        if declaration.declaration.is_none() {
            for specifier in &declaration.specifiers {
                if let Some(name) = module_export_name(&specifier.exported) {
                    self.named_exports.push(NamedExport {
                        name: CompactString::from(name),
                        span: specifier.exported.span(),
                        symbol_id: None,
                    });
                }
            }
            return;
        }

        match declaration.declaration.as_ref() {
            Some(Declaration::VariableDeclaration(declaration)) => {
                for declarator in &declaration.declarations {
                    if let Some(name) = binding_identifier_name(&declarator.id) {
                        self.named_exports.push(NamedExport {
                            name: CompactString::from(name),
                            span: declarator.id.span(),
                            symbol_id: binding_symbol_id(&declarator.id),
                        });
                    }
                }
            }
            Some(Declaration::FunctionDeclaration(function)) => {
                if let Some(id) = &function.id {
                    self.named_exports.push(NamedExport {
                        name: CompactString::from(id.name.as_str()),
                        span: id.span,
                        symbol_id: id.symbol_id.get(),
                    });
                }
            }
            _ => {}
        }
    }

    pub(crate) fn scan_statement_list(&mut self, statements: &'a [Statement<'a>]) {
        for statement in statements {
            self.scan_statement(statement);
        }
    }

    fn scan_statement(&mut self, statement: &'a Statement<'a>) {
        match statement {
            Statement::ImportDeclaration(declaration) => self.scan_import_declaration(declaration),
            Statement::ExportDefaultDeclaration(declaration) => {
                self.scan_export_default_declaration(&declaration.declaration, declaration.span);
                if let Some(expression) = declaration.declaration.as_expression() {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Statement::ExportNamedDeclaration(declaration) => {
                self.scan_export_named_declaration(declaration);
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration);
                }
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration)
            }
            Statement::ExpressionStatement(statement) => {
                self.scan_expression_statement(&statement.expression, statement.span);
            }
            Statement::BlockStatement(block) => self.scan_statement_list(&block.body),
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, ParentKind::ReturnArgument);
                }
            }
            Statement::IfStatement(statement) => {
                self.scan_expression(&statement.test, ParentKind::Other);
                self.scan_statement(&statement.consequent);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate);
                }
            }
            Statement::ForStatement(statement) => {
                if let Some(init) = &statement.init {
                    self.scan_for_init(init);
                }
                if let Some(test) = &statement.test {
                    self.scan_expression(test, ParentKind::Other);
                }
                if let Some(update) = &statement.update {
                    self.scan_expression(update, ParentKind::Other);
                }
                self.scan_statement(&statement.body);
            }
            Statement::ForInStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::ForOfStatement(statement) => {
                self.scan_for_left(&statement.left);
                self.scan_expression(&statement.right, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body);
                self.scan_expression(&statement.test, ParentKind::Other);
            }
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant, ParentKind::Other);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, ParentKind::Other);
                    }
                    self.scan_statement_list(&case.consequent);
                }
            }
            Statement::TryStatement(statement) => {
                self.scan_statement_list(&statement.block.body);
                if let Some(handler) = &statement.handler {
                    self.scan_statement_list(&handler.body.body);
                }
                if let Some(finalizer) = &statement.finalizer {
                    self.scan_statement_list(&finalizer.body);
                }
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument, ParentKind::Other);
            }
            Statement::LabeledStatement(statement) => self.scan_statement(&statement.body),
            Statement::FunctionDeclaration(function) => self.scan_function(function),
            Statement::ClassDeclaration(class) => self.scan_class(class),
            _ => {}
        }
    }

    fn scan_declaration(&mut self, declaration: &'a Declaration<'a>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration)
            }
            Declaration::FunctionDeclaration(function) => self.scan_function(function),
            Declaration::ClassDeclaration(class) => self.scan_class(class),
            _ => {}
        }
    }

    fn scan_for_init(&mut self, init: &'a oxc_ast::ast::ForStatementInit<'a>) {
        match init {
            oxc_ast::ast::ForStatementInit::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            _ => {
                if let Some(expression) = init.as_expression() {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
        }
    }

    fn scan_for_left(&mut self, left: &'a oxc_ast::ast::ForStatementLeft<'a>) {
        if let oxc_ast::ast::ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    fn scan_import_declaration(&mut self, declaration: &'a ImportDeclaration<'a>) {
        let package_name = declaration.source.value.as_str();

        if let Some((renderer_package, suggestions)) = renderer_framework_suggestions(package_name)
        {
            self.report_with_data(
                "no-renderer-packages",
                "noRendererPackages",
                DiagnosticData {
                    renderer_package: Some(CompactString::from(renderer_package)),
                    suggestions: Some(CompactString::from(suggestions)),
                    ..DiagnosticData::default()
                },
                declaration.span,
            );
        }

        if package_name.contains("@testing-library") {
            let mut fixes = SmallVec::new();
            fixes.push(DiagnosticFix {
                start: declaration.source.span.start.saturating_add(1),
                end: declaration.source.span.end.saturating_sub(1),
                replacement: CompactString::from("storybook/test"),
            });
            if import_has_default_specifier(declaration)
                && let Some(fix) = self.fix_default_import_specifiers(declaration)
            {
                fixes.push(fix);
            }
            self.report_with_fixes(
                "use-storybook-testing-library",
                "dontUseTestingLibraryDirectly",
                DiagnosticData {
                    library: Some(CompactString::from(package_name)),
                    ..DiagnosticData::default()
                },
                declaration.span,
                fixes,
            );
        }

        if let Some(specifiers) = &declaration.specifiers {
            for specifier in specifiers {
                if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier
                    && module_export_name(&specifier.imported) == Some("storiesOf")
                {
                    self.report("no-stories-of", "doNotUseStoriesOf", specifier.span);
                }
            }
        }
    }

    fn fix_default_import_specifiers(
        &self,
        declaration: &'a ImportDeclaration<'a>,
    ) -> Option<DiagnosticFix> {
        let specifiers = declaration.specifiers.as_ref()?;
        let first = specifiers.first()?;
        let last = specifiers.last()?;
        let start = first.span().start;
        let mut end = last.span().end;
        let search_start = end.saturating_sub(1) as usize;
        let import_end = declaration.source.span.start as usize;
        if search_start < self.source_text.len()
            && import_end <= self.source_text.len()
            && search_start <= import_end
            && let Some(relative) = self.source_text[search_start..import_end].find('}')
        {
            end = (search_start + relative + 1) as u32;
        }
        let text = source_slice(self.source_text, Span::new(start, end));
        let flattened = text
            .replace(['{', '}'], "")
            .split_whitespace()
            .collect::<SmallVec<[&str; 8]>>()
            .join(" ");
        let mut replacement = CompactString::new("{ ");
        replacement.push_str(&flattened);
        replacement.push_str(" }");
        Some(DiagnosticFix {
            start,
            end,
            replacement,
        })
    }

    fn scan_export_default_declaration(
        &mut self,
        declaration: &'a ExportDefaultDeclarationKind<'a>,
        export_span: Span,
    ) {
        let Some(meta) = self.resolve_meta_from_default(declaration) else {
            return;
        };
        self.check_meta_rules(&meta, export_span);
    }

    fn check_meta_rules(&mut self, meta: &MetaResolution<'a>, export_span: Span) {
        if find_object_property(meta.object, "component").is_none() {
            self.report("csf-component", "missingComponentProperty", export_span);
        }

        if let Some(title_property) = find_object_property(meta.object, "title") {
            if let Some(raw_title) = raw_string_literal(self.source_text, &title_property.value)
                && raw_title.contains('|')
            {
                let mut fixes = SmallVec::new();
                fixes.push(DiagnosticFix {
                    start: title_property.value.span().start,
                    end: title_property.value.span().end,
                    replacement: CompactString::from(raw_title.replace('|', "/")),
                });
                self.report_with_fixes(
                    "hierarchy-separator",
                    "deprecatedHierarchySeparator",
                    DiagnosticData {
                        meta_title: Some(CompactString::from(raw_title)),
                        ..DiagnosticData::default()
                    },
                    title_property.span,
                    fixes,
                );
            }

            let mut fixes = SmallVec::new();
            fixes.push(self.remove_range_fix(title_property.span));
            self.report_with_fixes(
                "no-title-property-in-meta",
                "noTitleInMeta",
                DiagnosticData::default(),
                title_property.span,
                fixes,
            );
        }

        for property_name in ["title", "args"] {
            if let Some(property) = find_object_property(meta.object, property_name)
                && !is_inline_property_value(&property.value)
            {
                self.report_with_data(
                    "meta-inline-properties",
                    "metaShouldHaveInlineProperties",
                    DiagnosticData {
                        property: Some(CompactString::from(property_name)),
                        ..DiagnosticData::default()
                    },
                    property.span,
                );
            }
        }

        if !meta.satisfies {
            let mut fixes = SmallVec::new();
            if let Some(type_annotation_span) = meta.type_annotation_span {
                let type_text = source_slice(self.source_text, type_annotation_span);
                let type_text = type_text.trim_start().trim_start_matches(':').trim_start();
                fixes.push(DiagnosticFix {
                    start: type_annotation_span.start,
                    end: type_annotation_span.end,
                    replacement: CompactString::new(""),
                });
                let mut replacement = CompactString::new(" satisfies ");
                replacement.push_str(type_text);
                fixes.push(DiagnosticFix {
                    start: meta.object.span.end,
                    end: meta.object.span.end,
                    replacement,
                });
            } else if let Some(as_span) = meta.as_expression_span
                && let Some(fix) = self.as_to_satisfies_fix(meta.object.span, as_span)
            {
                fixes.push(fix);
            }
            self.report_with_fixes(
                "meta-satisfies-type",
                "metaShouldSatisfyType",
                DiagnosticData::default(),
                meta.object.span,
                fixes,
            );
        }

        if let Some(addons_property) = find_object_property(meta.object, "addons")
            && let Expression::ArrayExpression(array) = addons_property.value.get_inner_expression()
        {
            self.report_uninstalled_addons(array);
        }
    }

    fn as_to_satisfies_fix(&self, object_span: Span, as_span: Span) -> Option<DiagnosticFix> {
        let start = object_span.end as usize;
        let end = as_span.end as usize;
        if start > end || end > self.source_text.len() {
            return None;
        }
        let suffix = &self.source_text[start..end];
        let relative = suffix.find(" as ")?;
        let replacement_start = (start + relative) as u32;
        Some(DiagnosticFix {
            start: replacement_start,
            end: replacement_start + 4,
            replacement: CompactString::from(" satisfies "),
        })
    }

    fn remove_range_fix(&self, span: Span) -> DiagnosticFix {
        let mut end = span.end;
        if self
            .source_text
            .as_bytes()
            .get(end as usize)
            .is_some_and(|byte| *byte == b',')
        {
            end += 1;
        }
        DiagnosticFix {
            start: span.start,
            end,
            replacement: CompactString::new(""),
        }
    }

    fn scan_export_named_declaration(&mut self, declaration: &'a ExportNamedDeclaration<'a>) {
        if let Some(Declaration::VariableDeclaration(variable)) = &declaration.declaration {
            for declarator in &variable.declarations {
                self.check_exported_story_object(declarator);
                if binding_identifier_name(&declarator.id) == Some("addons")
                    && let Some(Expression::ArrayExpression(array)) = declarator
                        .init
                        .as_ref()
                        .map(Expression::get_inner_expression)
                {
                    self.report_uninstalled_addons(array);
                }
            }
        }
    }

    fn check_exported_story_object(&mut self, declarator: &'a VariableDeclarator<'a>) {
        let Some(name) = binding_identifier_name(&declarator.id) else {
            return;
        };
        let Some(Expression::ObjectExpression(object)) = declarator
            .init
            .as_ref()
            .map(Expression::get_inner_expression)
        else {
            return;
        };
        for property_name in ["name", "storyName"] {
            if let Some(property) = find_object_property(object, property_name)
                && let Some(value) = string_literal_value(&property.value)
                && value == story_name_from_export(name)
            {
                let mut fixes = SmallVec::new();
                fixes.push(self.remove_range_fix(property.span));
                self.report_with_fixes(
                    "no-redundant-story-name",
                    "storyNameIsRedundant",
                    DiagnosticData::default(),
                    property.span,
                    fixes,
                );
            }
        }
    }

    fn check_prefer_pascal_case(&mut self, name: &str, span: Span, symbol_id: Option<SymbolId>) {
        if name.starts_with('_') || is_pascal_case(name) {
            return;
        }
        let pascal = to_pascal_case(name);
        let mut fixes = SmallVec::new();
        fixes.push(DiagnosticFix {
            start: span.start,
            end: span.end,
            replacement: pascal.clone(),
        });
        // Upstream renames every (non-declaration) reference to the export too, so the
        // autofix never leaves a dangling lowercase use such as `primary.foo`.
        if let Some(symbol_id) = symbol_id {
            for reference in self.scoping.get_resolved_references(symbol_id) {
                if let AstKind::IdentifierReference(identifier) =
                    self.nodes.get_node(reference.node_id()).kind()
                {
                    fixes.push(DiagnosticFix {
                        start: identifier.span.start,
                        end: identifier.span.end,
                        replacement: pascal.clone(),
                    });
                }
            }
        }
        self.report_with_fixes(
            "prefer-pascal-case",
            "usePascalCase",
            DiagnosticData {
                name: Some(CompactString::from(name)),
                ..DiagnosticData::default()
            },
            span,
            fixes,
        );
    }

    fn scan_expression_statement(&mut self, expression: &'a Expression<'a>, statement_span: Span) {
        if let Expression::AssignmentExpression(assignment) = expression.get_inner_expression() {
            if let Some((object_name, "storyName")) = assignment_static_member(&assignment.left)
                && let Some(value) = string_literal_value(&assignment.right)
                && value == story_name_from_export(object_name)
            {
                let mut fixes = SmallVec::new();
                fixes.push(DiagnosticFix {
                    start: statement_span.start,
                    end: statement_span.end,
                    replacement: CompactString::new(""),
                });
                self.report_with_fixes(
                    "no-redundant-story-name",
                    "storyNameIsRedundant",
                    DiagnosticData::default(),
                    assignment.span,
                    fixes,
                );
            }

            if let Expression::ObjectExpression(object) = assignment.right.get_inner_expression()
                && let Some(addons_property) = find_object_property(object, "addons")
                && let Expression::ArrayExpression(array) =
                    addons_property.value.get_inner_expression()
            {
                self.report_uninstalled_addons(array);
            }
        }
        self.scan_expression(expression, ParentKind::Other);
    }

    fn scan_variable_declaration(&mut self, declaration: &'a VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            // Upstream await-interactions resets its "userEvent came from storybook"
            // flag on EVERY `userEvent` variable declarator (nested ones included),
            // disabling the rule when `userEvent` is locally redefined. The prepass
            // only sees top-level declarations, so catch nested bindings here. A
            // `const`/`let` is always declared before it is used (TDZ), so the flag
            // is set before any `userEvent` call in scope is checked.
            if binding_identifier_name(&declarator.id) == Some("userEvent") {
                self.user_event_is_non_storybook = true;
            }
            if let Some(init) = &declarator.init {
                self.scan_expression(init, ParentKind::Other);
            }
        }
    }

    fn scan_function(&mut self, function: &'a Function<'a>) {
        self.function_stack.push(FunctionFrame {
            span: function.span,
            is_async: function.r#async,
            context_param: None,
        });
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
        let _ = self.function_stack.pop();
    }

    fn scan_arrow_function(&mut self, function: &'a oxc_ast::ast::ArrowFunctionExpression<'a>) {
        self.function_stack.push(FunctionFrame {
            span: function.span,
            is_async: function.r#async,
            context_param: context_param_name(&function.params),
        });
        if function.expression
            && function.body.statements.len() == 1
            && let Statement::ExpressionStatement(statement) = &function.body.statements[0]
        {
            self.scan_expression(&statement.expression, ParentKind::ArrowBody);
        } else {
            self.scan_function_body(&function.body);
        }
        let _ = self.function_stack.pop();
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        self.scan_statement_list(&body.statements);
    }

    fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, ParentKind::Other);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => self.scan_statement_list(&block.body),
                ClassElement::MethodDefinition(method) => self.scan_function(&method.value),
                ClassElement::PropertyDefinition(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ParentKind::Other);
                    }
                }
                ClassElement::AccessorProperty(property) => {
                    if let Some(value) = &property.value {
                        self.scan_expression(value, ParentKind::Other);
                    }
                }
                ClassElement::TSIndexSignature(_) => {}
            }
        }
    }

    fn scan_expression(&mut self, expression: &'a Expression<'a>, parent: ParentKind) {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => {
                self.check_call_expression(call, parent);
                self.scan_expression(&call.callee, ParentKind::CallCallee);
                for argument in &call.arguments {
                    self.scan_argument(argument);
                }
            }
            Expression::ChainExpression(chain) => {
                self.scan_chain_element(&chain.expression, parent)
            }
            Expression::StaticMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            Expression::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
            }
            Expression::AssignmentExpression(assignment) => {
                self.scan_assignment_target(&assignment.left);
                self.scan_expression(&assignment.right, ParentKind::Other);
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(expression) = element.as_expression() {
                        self.scan_expression(expression, ParentKind::Other);
                    } else if let ArrayExpressionElement::SpreadElement(spread) = element {
                        self.scan_expression(&spread.argument, ParentKind::Other);
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed
                                && let Some(expression) = property.key.as_expression()
                            {
                                self.scan_expression(expression, ParentKind::Other);
                            }
                            self.scan_expression(&property.value, ParentKind::Other);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument, ParentKind::Other);
                        }
                    }
                }
            }
            Expression::ArrowFunctionExpression(function) => self.scan_arrow_function(function),
            Expression::FunctionExpression(function) => self.scan_function(function),
            Expression::ClassExpression(class) => self.scan_class(class),
            Expression::AwaitExpression(await_expression) => {
                self.scan_expression(&await_expression.argument, ParentKind::AwaitArgument);
            }
            Expression::BinaryExpression(binary) => {
                self.scan_expression(&binary.left, ParentKind::Other);
                self.scan_expression(&binary.right, ParentKind::Other);
            }
            Expression::LogicalExpression(logical) => {
                self.scan_expression(&logical.left, ParentKind::Other);
                self.scan_expression(&logical.right, ParentKind::Other);
            }
            Expression::ConditionalExpression(conditional) => {
                self.scan_expression(&conditional.test, ParentKind::Other);
                self.scan_expression(&conditional.consequent, ParentKind::Other);
                self.scan_expression(&conditional.alternate, ParentKind::Other);
            }
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::UnaryExpression(unary) => {
                self.scan_expression(&unary.argument, ParentKind::Other)
            }
            Expression::UpdateExpression(_) => {}
            Expression::YieldExpression(yield_expression) => {
                if let Some(argument) = &yield_expression.argument {
                    self.scan_expression(argument, ParentKind::Other);
                }
            }
            Expression::TemplateLiteral(template) => {
                for expression in &template.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.scan_expression(&tagged.tag, ParentKind::Other);
                for expression in &tagged.quasi.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::ImportExpression(import) => {
                self.scan_expression(&import.source, ParentKind::Other);
                if let Some(options) = &import.options {
                    self.scan_expression(options, ParentKind::Other);
                }
            }
            _ => {}
        }
    }

    fn scan_chain_element(&mut self, element: &'a ChainElement<'a>, parent: ParentKind) {
        match element {
            ChainElement::CallExpression(call) => {
                self.check_call_expression(call, parent);
                self.scan_expression(&call.callee, ParentKind::CallCallee);
                for argument in &call.arguments {
                    self.scan_argument(argument);
                }
            }
            ChainElement::StaticMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            ChainElement::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
            }
            ChainElement::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            ChainElement::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression, ParentKind::Other);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument, ParentKind::Other);
        }
    }

    fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>) {
        match target {
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
            }
            AssignmentTarget::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            AssignmentTarget::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, ParentKind::Other);
            }
            AssignmentTarget::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, ParentKind::Other);
            }
            AssignmentTarget::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, ParentKind::Other);
            }
            AssignmentTarget::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, ParentKind::Other);
            }
            _ => {}
        }
    }

    fn check_call_expression(&mut self, call: &'a CallExpression<'a>, parent: ParentKind) {
        if call.callee.is_specific_id("expect") {
            self.expect_invocations.push(call.callee.span());
        }

        if let Some(method) = method_that_should_be_awaited(call, self.user_event_is_non_storybook)
            && !matches!(
                parent,
                ParentKind::AwaitArgument | ParentKind::ReturnArgument | ParentKind::ArrowBody
            )
        {
            let mut fixes = SmallVec::new();
            fixes.push(DiagnosticFix {
                start: call.span.start,
                end: call.span.start,
                replacement: CompactString::from("await "),
            });
            if let Some(frame) = self.function_stack.last()
                && !frame.is_async
            {
                fixes.push(DiagnosticFix {
                    start: frame.span.start,
                    end: frame.span.start,
                    replacement: CompactString::from("async "),
                });
            }
            self.report_with_fixes(
                "await-interactions",
                "interactionShouldBeAwaited",
                DiagnosticData {
                    method: Some(CompactString::from(method)),
                    ..DiagnosticData::default()
                },
                call.span,
                fixes,
            );
        }

        if is_play_call(call) && !self.call_passes_context(call) {
            self.report(
                "context-in-play-function",
                "passContextToPlayFunction",
                call.span,
            );
        }
    }

    fn call_passes_context(&self, call: &'a CallExpression<'a>) -> bool {
        let Some(frame) = self.function_stack.last() else {
            return false;
        };
        let Some(context_name) = &frame.context_param else {
            return false;
        };
        let Some(argument) = call.arguments.first() else {
            return false;
        };

        if call.arguments.len() == 1
            && let Some(Expression::Identifier(identifier)) = argument
                .as_expression()
                .map(Expression::get_inner_expression)
        {
            return identifier.name == context_name.as_str();
        }

        let Some(Expression::ObjectExpression(object)) = argument
            .as_expression()
            .map(Expression::get_inner_expression)
        else {
            return false;
        };

        object.properties.iter().any(|property| {
            if let ObjectPropertyKind::SpreadProperty(spread) = property
                && let Expression::Identifier(identifier) = spread.argument.get_inner_expression()
            {
                return identifier.name == context_name.as_str();
            }
            false
        })
    }

    fn report_uninstalled_addons(&mut self, addons: &'a ArrayExpression<'a>) {
        for element in &addons.elements {
            let Some((addon_name, span)) = addon_name_from_array_element(element) else {
                continue;
            };
            if is_local_addon(addon_name)
                || self
                    .options
                    .ignored_addons
                    .iter()
                    .any(|ignored| ignored == addon_name)
                || self
                    .options
                    .installed_addons
                    .iter()
                    .any(|installed| installed == cleaned_addon_name(addon_name).as_str())
            {
                continue;
            }

            self.report_with_data(
                "no-uninstalled-addons",
                "addonIsNotInstalled",
                DiagnosticData {
                    addon_name: Some(CompactString::from(addon_name)),
                    package_json_path: Some(self.options.package_json_path.clone()),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
    }

    fn resolve_meta_from_default(
        &self,
        declaration: &'a ExportDefaultDeclarationKind<'a>,
    ) -> Option<MetaResolution<'a>> {
        let expression = declaration.as_expression()?;
        self.resolve_meta_expression(expression, None)
    }

    fn resolve_meta_expression(
        &self,
        expression: &'a Expression<'a>,
        type_annotation_span: Option<Span>,
    ) -> Option<MetaResolution<'a>> {
        match expression {
            Expression::ObjectExpression(object) => Some(MetaResolution {
                object,
                satisfies: false,
                type_annotation_span,
                as_expression_span: None,
            }),
            Expression::TSSatisfiesExpression(expression) => {
                let mut resolved =
                    self.resolve_meta_expression(&expression.expression, type_annotation_span)?;
                resolved.satisfies = true;
                Some(resolved)
            }
            Expression::TSAsExpression(expression) => {
                let mut resolved =
                    self.resolve_meta_expression(&expression.expression, type_annotation_span)?;
                resolved.as_expression_span = Some(expression.span);
                Some(resolved)
            }
            Expression::TSNonNullExpression(expression) => {
                self.resolve_meta_expression(&expression.expression, type_annotation_span)
            }
            Expression::TSInstantiationExpression(expression) => {
                self.resolve_meta_expression(&expression.expression, type_annotation_span)
            }
            Expression::ParenthesizedExpression(expression) => {
                self.resolve_meta_expression(&expression.expression, type_annotation_span)
            }
            Expression::Identifier(identifier) => self
                .variables
                .get(identifier.name.as_str())
                .and_then(|variable| {
                    self.resolve_meta_expression(variable.init, variable.type_annotation_span)
                }),
            _ => None,
        }
    }

    pub(crate) fn finish_program(&mut self) {
        if !self.has_csf4_meta && !self.has_default_export && !self.has_stories_of_import {
            let span = self.first_non_import_span.unwrap_or_else(|| {
                Span::new(0, u32::try_from(self.source_text.len()).unwrap_or(u32::MAX))
            });
            let component_name = self.imported_component_name.clone();
            let replacement = component_name.map_or_else(
                || CompactString::from("export default {}\n"),
                |component_name| {
                    let mut replacement = CompactString::new("export default { component: ");
                    replacement.push_str(component_name.as_str());
                    replacement.push_str(" }\n");
                    replacement
                },
            );
            let mut fixes = SmallVec::new();
            fixes.push(DiagnosticFix {
                start: span.start,
                end: span.start,
                replacement,
            });
            self.report_with_fixes(
                "default-exports",
                "shouldHaveDefaultExport",
                DiagnosticData::default(),
                span,
                fixes,
            );
        }

        if self.has_meta && !self.has_stories_of_import {
            let has_story_export = self
                .named_exports
                .iter()
                .any(|export| is_story_export(export.name.as_str(), &self.story_filters));
            if !has_story_export {
                let span = self.first_non_import_span.unwrap_or_else(|| {
                    Span::new(0, u32::try_from(self.source_text.len()).unwrap_or(u32::MAX))
                });
                let message_id = if self.story_filters.has_filter {
                    "shouldHaveStoryExportWithFilters"
                } else {
                    "shouldHaveStoryExport"
                };
                self.report("story-exports", message_id, span);
            }
        }

        if !self.has_stories_of_import {
            let pending: SmallVec<[(CompactString, Span, Option<SymbolId>); 8]> = self
                .named_exports
                .iter()
                .map(|export| (export.name.clone(), export.span, export.symbol_id))
                .collect();
            for (name, span, symbol_id) in pending {
                if is_story_export(name.as_str(), &self.story_filters) {
                    self.check_prefer_pascal_case(name.as_str(), span, symbol_id);
                }
            }
        }

        if !self.has_storybook_expect_import {
            let pending = self.expect_invocations.clone();
            for span in pending {
                self.report("use-storybook-expect", "useExpectFromStorybook", span);
            }
        }
    }
}
