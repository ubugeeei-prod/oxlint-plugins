//! AST scanner for the security port. Contains the `Scanner` struct and
//! every traversal / rule check method as an `impl Scanner` block.

#![allow(
    unused_imports,
    reason = "The scanner uses a wide cross-section of AST node types; not every method touches every type."
)]

use oxc_ast::ast::*;
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, FastHashMap, SmallVec};

use crate::helpers::*;
use crate::{
    AccessPath, Binding, CHILD_PROCESS_PACKAGES, Diagnostic, DiagnosticData, FS_PACKAGES,
    LineIndex, PATH_CONSTRUCTION_METHODS, PATH_PACKAGES, PATH_STATIC_MEMBERS, ParentKind, Scope,
    TIMING_KEYWORDS, URL_PACKAGES,
};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 16]>,
    pub(crate) scopes: SmallVec<[Scope; 8]>,
    pub(crate) csrf_seen: bool,
    pub(crate) comment_spans: SmallVec<[Span; 16]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        let _ = self.scopes.pop();
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes
            .last_mut()
            .expect("scanner always has an active scope")
    }

    fn bind(&mut self, name: &str, binding: Binding) {
        self.current_scope_mut()
            .bindings
            .insert(CompactString::from(name), binding);
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.bindings.get(name))
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
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    pub(crate) fn scan_program(&mut self, body: &'a [Statement<'a>]) {
        for statement in body {
            self.scan_statement(statement);
        }
    }

    fn scan_statement(&mut self, statement: &'a Statement<'a>) {
        match statement {
            Statement::BlockStatement(block) => {
                for statement in &block.body {
                    self.scan_statement(statement);
                }
            }
            Statement::ExpressionStatement(statement) => {
                self.scan_expression(&statement.expression, ParentKind::None);
            }
            Statement::IfStatement(statement) => {
                self.check_possible_timing_attack(statement.span, &statement.test);
                self.scan_expression(&statement.test, ParentKind::Other);
                self.scan_statement(&statement.consequent);
                if let Some(alternate) = &statement.alternate {
                    self.scan_statement(alternate);
                }
            }
            Statement::ReturnStatement(statement) => {
                if let Some(argument) = &statement.argument {
                    self.scan_expression(argument, ParentKind::Other);
                }
            }
            Statement::ThrowStatement(statement) => {
                self.scan_expression(&statement.argument, ParentKind::Other);
            }
            Statement::WhileStatement(statement) => {
                self.scan_expression(&statement.test, ParentKind::Other);
                self.scan_statement(&statement.body);
            }
            Statement::DoWhileStatement(statement) => {
                self.scan_statement(&statement.body);
                self.scan_expression(&statement.test, ParentKind::Other);
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
            Statement::SwitchStatement(statement) => {
                self.scan_expression(&statement.discriminant, ParentKind::Other);
                for case in &statement.cases {
                    if let Some(test) = &case.test {
                        self.scan_expression(test, ParentKind::Other);
                    }
                    for statement in &case.consequent {
                        self.scan_statement(statement);
                    }
                }
            }
            Statement::TryStatement(statement) => {
                for statement in &statement.block.body {
                    self.scan_statement(statement);
                }
                if let Some(handler) = &statement.handler {
                    self.push_scope();
                    if let Some(param) = &handler.param {
                        self.bind_pattern_unknown(&param.pattern);
                    }
                    for statement in &handler.body.body {
                        self.scan_statement(statement);
                    }
                    self.pop_scope();
                }
                if let Some(finalizer) = &statement.finalizer {
                    for statement in &finalizer.body {
                        self.scan_statement(statement);
                    }
                }
            }
            Statement::LabeledStatement(statement) => {
                self.scan_statement(&statement.body);
            }
            Statement::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Statement::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_function(function);
            }
            Statement::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_class(class);
            }
            Statement::ImportDeclaration(declaration) => {
                self.scan_import_declaration(declaration);
            }
            Statement::ExportNamedDeclaration(declaration) => {
                if let Some(declaration) = &declaration.declaration {
                    self.scan_declaration(declaration);
                }
            }
            Statement::ExportDefaultDeclaration(declaration) => match &declaration.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    self.scan_function(function);
                }
                ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                    self.scan_class(class);
                }
                _ => {
                    if let Some(expression) = declaration.declaration.as_expression() {
                        self.scan_expression(expression, ParentKind::Other);
                    }
                }
            },
            _ => {}
        }
    }

    fn scan_declaration(&mut self, declaration: &'a Declaration<'a>) {
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            Declaration::FunctionDeclaration(function) => {
                if let Some(id) = &function.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_function(function);
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    self.bind(id.name.as_str(), Binding::Unknown);
                }
                self.scan_class(class);
            }
            _ => {}
        }
    }

    fn scan_for_init(&mut self, init: &'a ForStatementInit<'a>) {
        match init {
            ForStatementInit::VariableDeclaration(declaration) => {
                self.scan_variable_declaration(declaration);
            }
            _ => {
                if let Some(expression) = init.as_expression() {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
        }
    }

    fn scan_for_left(&mut self, left: &'a ForStatementLeft<'a>) {
        if let ForStatementLeft::VariableDeclaration(declaration) = left {
            self.scan_variable_declaration(declaration);
        }
    }

    fn scan_import_declaration(&mut self, declaration: &'a ImportDeclaration<'a>) {
        let package_name = declaration.source.value.as_str();
        let interesting = is_interesting_package(package_name);

        if let Some(specifiers) = &declaration.specifiers {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        let imported = module_export_name(&specifier.imported);
                        let binding = if interesting {
                            imported.map(|name| {
                                Binding::Import(AccessPath {
                                    package_name: CompactString::from(package_name),
                                    path: small_path([name]),
                                })
                            })
                        } else {
                            None
                        }
                        .unwrap_or(Binding::Unknown);
                        self.bind(specifier.local.name.as_str(), binding);
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        let binding = if interesting {
                            Binding::Import(AccessPath {
                                package_name: CompactString::from(package_name),
                                path: SmallVec::new(),
                            })
                        } else {
                            Binding::Unknown
                        };
                        self.bind(specifier.local.name.as_str(), binding);
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        let binding = if interesting {
                            Binding::Import(AccessPath {
                                package_name: CompactString::from(package_name),
                                path: SmallVec::new(),
                            })
                        } else {
                            Binding::Unknown
                        };
                        self.bind(specifier.local.name.as_str(), binding);
                    }
                }
            }
        }
    }

    fn scan_variable_declaration(&mut self, declaration: &'a VariableDeclaration<'a>) {
        for declarator in &declaration.declarations {
            if let Some(init) = &declarator.init {
                if let Some(path) = self.import_access_path(init, &INTERESTING_PACKAGES) {
                    self.bind_pattern_from_import(&declarator.id, &path);
                } else if self.is_static_expression(init, 0) {
                    self.bind_pattern_static_or_unknown(&declarator.id, true);
                } else {
                    self.bind_pattern_unknown(&declarator.id);
                }
                self.scan_expression(init, ParentKind::VariableInit);
            } else {
                self.bind_pattern_unknown(&declarator.id);
            }
        }
    }

    fn scan_function(&mut self, function: &'a Function<'a>) {
        self.push_scope();
        for param in &function.params.items {
            self.bind_pattern_unknown(&param.pattern);
            if let Some(initializer) = &param.initializer {
                self.scan_expression(initializer, ParentKind::Other);
            }
        }
        if let Some(rest) = &function.params.rest {
            self.bind_pattern_unknown(&rest.rest.argument);
        }
        if let Some(body) = &function.body {
            self.scan_function_body(body);
        }
        self.pop_scope();
    }

    fn scan_function_body(&mut self, body: &'a FunctionBody<'a>) {
        for statement in &body.statements {
            self.scan_statement(statement);
        }
    }

    fn scan_class(&mut self, class: &'a Class<'a>) {
        if let Some(super_class) = &class.super_class {
            self.scan_expression(super_class, ParentKind::Other);
        }
        for element in &class.body.body {
            match element {
                ClassElement::StaticBlock(block) => {
                    self.push_scope();
                    for statement in &block.body {
                        self.scan_statement(statement);
                    }
                    self.pop_scope();
                }
                ClassElement::MethodDefinition(method) => {
                    self.scan_function(&method.value);
                }
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
                    self.scan_argument(argument, ParentKind::CallArgument);
                }
            }
            Expression::NewExpression(new_expression) => {
                self.check_new_expression(new_expression);
                self.scan_expression(&new_expression.callee, ParentKind::NewCallee);
                for argument in &new_expression.arguments {
                    self.scan_argument(argument, ParentKind::NewArgument);
                }
            }
            Expression::AssignmentExpression(assignment) => {
                self.check_disable_mustache_escape(
                    assignment.span,
                    &assignment.left,
                    &assignment.right,
                );
                self.scan_assignment_target(&assignment.left, ParentKind::AssignmentLeft);
                self.scan_expression(&assignment.right, ParentKind::AssignmentRight);
            }
            Expression::StaticMemberExpression(member) => {
                if member.property.name == "pseudoRandomBytes" {
                    self.report("detect-pseudoRandomBytes", "found", member.span);
                }
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            Expression::ComputedMemberExpression(member) => {
                self.check_object_injection(member.span, &member.expression, parent);
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
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
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(expression) = array_element_expression(element) {
                        self.scan_expression(expression, ParentKind::Other);
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed {
                                self.scan_property_key(&property.key);
                            }
                            self.scan_expression(&property.value, ParentKind::Other);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.scan_expression(&spread.argument, ParentKind::Other);
                        }
                    }
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
            Expression::FunctionExpression(function) => self.scan_function(function),
            Expression::ArrowFunctionExpression(function) => {
                self.push_scope();
                for param in &function.params.items {
                    self.bind_pattern_unknown(&param.pattern);
                }
                for statement in &function.body.statements {
                    self.scan_statement(statement);
                }
                self.pop_scope();
            }
            Expression::ClassExpression(class) => self.scan_class(class),
            Expression::SequenceExpression(sequence) => {
                for expression in &sequence.expressions {
                    self.scan_expression(expression, ParentKind::Other);
                }
            }
            Expression::AwaitExpression(await_expression) => {
                self.scan_expression(&await_expression.argument, ParentKind::Other);
            }
            Expression::UnaryExpression(unary) => {
                self.scan_expression(&unary.argument, ParentKind::Other);
            }
            Expression::UpdateExpression(_) => {}
            Expression::YieldExpression(yield_expression) => {
                if let Some(argument) = &yield_expression.argument {
                    self.scan_expression(argument, ParentKind::Other);
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.check_call_expression(call, parent);
                    self.scan_expression(&call.callee, ParentKind::CallCallee);
                    for argument in &call.arguments {
                        self.scan_argument(argument, ParentKind::CallArgument);
                    }
                }
                ChainElement::StaticMemberExpression(member) => {
                    self.scan_expression(&member.object, ParentKind::MemberObject);
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.check_object_injection(member.span, &member.expression, parent);
                    self.scan_expression(&member.object, ParentKind::MemberObject);
                    self.scan_expression(&member.expression, ParentKind::Other);
                }
                ChainElement::PrivateFieldExpression(member) => {
                    self.scan_expression(&member.object, ParentKind::MemberObject);
                }
                ChainElement::TSNonNullExpression(expression) => {
                    self.scan_expression(&expression.expression, parent);
                }
            },
            Expression::RegExpLiteral(literal)
                if is_unsafe_regex(literal.regex.pattern.text.as_str()) =>
            {
                self.report("detect-unsafe-regex", "literal", literal.span);
            }
            _ => {}
        }
    }

    fn scan_property_key(&mut self, key: &'a PropertyKey<'a>) {
        if let Some(expression) = key.as_expression() {
            self.scan_expression(expression, ParentKind::Other);
        }
    }

    fn scan_argument(&mut self, argument: &'a Argument<'a>, parent: ParentKind) {
        if let Some(expression) = argument.as_expression() {
            self.scan_expression(expression, parent);
        } else if let Argument::SpreadElement(spread) = argument {
            self.scan_expression(&spread.argument, parent);
        }
    }

    fn scan_assignment_target(&mut self, target: &'a AssignmentTarget<'a>, parent: ParentKind) {
        match target {
            AssignmentTarget::ComputedMemberExpression(member) => {
                self.check_object_injection(member.span, &member.expression, parent);
                self.scan_expression(&member.object, ParentKind::MemberObject);
                self.scan_expression(&member.expression, ParentKind::Other);
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            AssignmentTarget::PrivateFieldExpression(member) => {
                self.scan_expression(&member.object, ParentKind::MemberObject);
            }
            AssignmentTarget::TSAsExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            AssignmentTarget::TSSatisfiesExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            AssignmentTarget::TSNonNullExpression(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            AssignmentTarget::TSTypeAssertion(expression) => {
                self.scan_expression(&expression.expression, parent);
            }
            _ => {}
        }
    }

    fn check_call_expression(&mut self, call: &'a CallExpression<'a>, parent: ParentKind) {
        if let Some(package_name) = self.require_package_name(call)
            && CHILD_PROCESS_PACKAGES.contains(&package_name)
            && !matches!(
                parent,
                ParentKind::VariableInit | ParentKind::AssignmentRight | ParentKind::MemberObject
            )
        {
            self.report_with_data(
                "detect-child-process",
                "require",
                DiagnosticData {
                    value: Some(CompactString::from(package_name)),
                    ..DiagnosticData::default()
                },
                call.span,
            );
        }

        if call.callee.is_specific_id("eval")
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
            && !argument.is_literal()
        {
            self.report_with_data(
                "detect-eval-with-expression",
                "nonLiteral",
                DiagnosticData {
                    argument_type: Some(CompactString::from(expression_type(argument))),
                    ..DiagnosticData::default()
                },
                call.span,
            );
        }

        if call.callee.is_specific_id("require")
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
            && !self.is_static_expression(argument, 0)
        {
            self.report("detect-non-literal-require", "nonLiteral", call.span);
        }

        if let Some(path) = self.import_access_path(&call.callee, &CHILD_PROCESS_PACKAGES)
            && path.path.len() == 1
            && path.path[0].as_str() == "exec"
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
            && !self.is_static_expression(argument, 0)
        {
            self.report("detect-child-process", "execNonLiteral", call.span);
        }

        self.check_buffer_noassert(call);
        self.check_no_csrf_before_method_override(call);
        self.check_non_literal_fs_filename(call);
    }

    fn check_new_expression(&mut self, new_expression: &'a NewExpression<'a>) {
        if new_expression.callee.is_specific_id("Buffer")
            && let Some(argument) = new_expression
                .arguments
                .first()
                .and_then(Argument::as_expression)
            && !argument.is_literal()
        {
            self.report("detect-new-buffer", "found", new_expression.span);
        }

        if new_expression.callee.is_specific_id("RegExp")
            && let Some(argument) = new_expression
                .arguments
                .first()
                .and_then(Argument::as_expression)
        {
            if !self.is_static_expression(argument, 0) {
                self.report(
                    "detect-non-literal-regexp",
                    "nonLiteral",
                    new_expression.span,
                );
            } else if let Some(pattern) = string_literal_value(argument)
                && is_unsafe_regex(pattern)
            {
                self.report("detect-unsafe-regex", "newRegExp", new_expression.span);
            }
        }
    }

    fn check_buffer_noassert(&mut self, call: &'a CallExpression<'a>) {
        let Some(method) = static_member_property(&call.callee) else {
            return;
        };
        let index = if BUFFER_READ_METHODS.contains(&method) {
            Some(1)
        } else if BUFFER_WRITE_METHODS.contains(&method) {
            Some(2)
        } else {
            None
        };

        if let Some(index) = index
            && let Some(argument) = call.arguments.get(index).and_then(Argument::as_expression)
            && matches!(argument.get_inner_expression(), Expression::BooleanLiteral(value) if value.value)
        {
            self.report_with_data(
                "detect-buffer-noassert",
                "found",
                DiagnosticData {
                    method: Some(CompactString::from(method)),
                    ..DiagnosticData::default()
                },
                call.callee.span(),
            );
        }
    }

    fn check_no_csrf_before_method_override(&mut self, call: &'a CallExpression<'a>) {
        if !call.callee.is_specific_member_access("express", "csrf")
            && !call
                .callee
                .is_specific_member_access("express", "methodOverride")
        {
            return;
        }

        if call
            .callee
            .is_specific_member_access("express", "methodOverride")
            && self.csrf_seen
        {
            self.report("detect-no-csrf-before-method-override", "found", call.span);
        }
        if call.callee.is_specific_member_access("express", "csrf") {
            self.csrf_seen = true;
        }
    }

    fn check_disable_mustache_escape(
        &mut self,
        span: Span,
        left: &'a AssignmentTarget<'a>,
        right: &'a Expression<'a>,
    ) {
        if !matches!(left, AssignmentTarget::StaticMemberExpression(member) if member.property.name == "escapeMarkup")
        {
            return;
        }
        if matches!(right.get_inner_expression(), Expression::BooleanLiteral(value) if !value.value)
        {
            self.report("detect-disable-mustache-escape", "found", span);
        }
    }

    fn check_non_literal_fs_filename(&mut self, call: &'a CallExpression<'a>) {
        if call.callee.is_specific_id("require") || call.arguments.iter().all(argument_is_literal) {
            return;
        }

        let Some(path) = self.import_access_path(&call.callee, &FS_PACKAGES) else {
            return;
        };
        let fn_name = match path.path.as_slice() {
            [name] => name.as_str(),
            [_, name] => name.as_str(),
            _ => return,
        };
        let Some(indices_to_check) = fs_argument_indices(fn_name) else {
            return;
        };

        let mut indices: SmallVec<[usize; 2]> = SmallVec::new();
        for index in indices_to_check {
            if let Some(argument) = call.arguments.get(*index).and_then(Argument::as_expression)
                && !self.is_static_expression(argument, 0)
            {
                indices.push(*index);
            }
        }

        if !indices.is_empty() {
            let joined = join_usize(&indices);
            self.report_with_data(
                "detect-non-literal-fs-filename",
                "nonLiteral",
                DiagnosticData {
                    fn_name: Some(CompactString::from(fn_name)),
                    package_name: Some(path.package_name),
                    indices: Some(joined),
                    ..DiagnosticData::default()
                },
                call.span,
            );
        }
    }

    fn check_object_injection(
        &mut self,
        span: Span,
        property: &'a Expression<'a>,
        parent: ParentKind,
    ) {
        if !matches!(property.get_inner_expression(), Expression::Identifier(_)) {
            return;
        }
        let message_id = match parent {
            ParentKind::VariableInit => "variable",
            ParentKind::CallCallee => "functionCall",
            _ => "generic",
        };
        self.report("detect-object-injection", message_id, span);
    }

    fn check_possible_timing_attack(&mut self, span: Span, test: &'a Expression<'a>) {
        let Expression::BinaryExpression(binary) = test.get_inner_expression() else {
            return;
        };
        if !matches!(
            binary.operator,
            BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
        ) {
            return;
        }
        if contains_timing_keyword(&binary.left) {
            self.report_with_data(
                "detect-possible-timing-attacks",
                "found",
                DiagnosticData {
                    side: Some(CompactString::from("left")),
                    ..DiagnosticData::default()
                },
                span,
            );
        } else if contains_timing_keyword(&binary.right) {
            self.report_with_data(
                "detect-possible-timing-attacks",
                "found",
                DiagnosticData {
                    side: Some(CompactString::from("right")),
                    ..DiagnosticData::default()
                },
                span,
            );
        }
    }

    pub(crate) fn scan_bidi_characters(&mut self) {
        for (start, ch) in self.source_text.char_indices() {
            if !is_dangerous_bidi(ch) {
                continue;
            }
            let end = start + ch.len_utf8();
            let in_comment = self
                .comment_spans
                .iter()
                .any(|span| span.start as usize <= start && end <= span.end as usize);
            let line_text = source_line_at(self.source_text, start);
            self.report_with_data(
                "detect-bidi-characters",
                if in_comment { "comment" } else { "code" },
                DiagnosticData {
                    text: Some(CompactString::from(line_text)),
                    ..DiagnosticData::default()
                },
                Span::new(start as u32, end as u32),
            );
        }
    }

    fn require_package_name(&self, call: &'a CallExpression<'a>) -> Option<&'a str> {
        if !call.callee.is_specific_id("require") {
            return None;
        }
        let Some(Expression::StringLiteral(literal)) =
            call.arguments.first().and_then(Argument::as_expression)
        else {
            return None;
        };
        Some(literal.value.as_str())
    }

    fn import_access_path(
        &self,
        expression: &'a Expression<'a>,
        package_names: &[&str],
    ) -> Option<AccessPath> {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => match self.lookup(identifier.name.as_str()) {
                Some(Binding::Import(path))
                    if package_names.contains(&path.package_name.as_str()) =>
                {
                    Some(path.clone())
                }
                _ => None,
            },
            Expression::StaticMemberExpression(member) => {
                let mut path = self.import_access_path(&member.object, package_names)?;
                path.path
                    .push(CompactString::from(member.property.name.as_str()));
                Some(path)
            }
            Expression::CallExpression(call) => {
                let package_name = self.require_package_name(call)?;
                if !package_names.contains(&package_name) {
                    return None;
                }
                Some(AccessPath {
                    package_name: CompactString::from(package_name),
                    path: SmallVec::new(),
                })
            }
            _ => None,
        }
    }

    fn is_static_expression(&self, expression: &'a Expression<'a>, depth: usize) -> bool {
        if depth > 32 {
            return false;
        }

        match expression.get_inner_expression() {
            Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(template) => template
                .expressions
                .iter()
                .all(|expression| self.is_static_expression(expression, depth + 1)),
            Expression::BinaryExpression(binary) => {
                self.is_static_expression(&binary.left, depth + 1)
                    && self.is_static_expression(&binary.right, depth + 1)
            }
            Expression::Identifier(identifier) => match identifier.name.as_str() {
                "__dirname" | "__filename" => true,
                name => matches!(self.lookup(name), Some(Binding::Static)),
            },
            Expression::StaticMemberExpression(member) => {
                is_import_meta_url(expression)
                    || self.is_static_path_member(member.property.name.as_str(), &member.object)
            }
            Expression::CallExpression(call) => {
                self.is_static_path_call(call, depth)
                    || self.is_static_file_url_to_path(call, depth)
                    || self.is_static_require_resolve(call, depth)
                    || self.is_static_process_cwd(call)
            }
            _ => false,
        }
    }

    fn is_static_path_member(&self, name: &str, object: &'a Expression<'a>) -> bool {
        if !PATH_STATIC_MEMBERS.contains(&name) {
            return false;
        }
        self.import_access_path(object, &PATH_PACKAGES).is_some()
    }

    fn is_static_path_call(&self, call: &'a CallExpression<'a>, depth: usize) -> bool {
        let Some(path) = self.import_access_path(&call.callee, &PATH_PACKAGES) else {
            return false;
        };
        let method = match path.path.as_slice() {
            [name] => name.as_str(),
            [namespace, name] if namespace.as_str() == "posix" => name.as_str(),
            _ => return false,
        };
        PATH_CONSTRUCTION_METHODS.contains(&method)
            && !call.arguments.is_empty()
            && call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expression| self.is_static_expression(expression, depth + 1))
            })
    }

    fn is_static_file_url_to_path(&self, call: &'a CallExpression<'a>, depth: usize) -> bool {
        let Some(path) = self.import_access_path(&call.callee, &URL_PACKAGES) else {
            return false;
        };
        matches!(path.path.as_slice(), [name] if name.as_str() == "fileURLToPath")
            && !call.arguments.is_empty()
            && call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expression| self.is_static_expression(expression, depth + 1))
            })
    }

    fn is_static_require_resolve(&self, call: &'a CallExpression<'a>, depth: usize) -> bool {
        if !call.callee.is_specific_member_access("require", "resolve") {
            return false;
        }
        if matches!(
            self.lookup("require"),
            Some(Binding::Unknown | Binding::Import(_))
        ) {
            return false;
        }
        !call.arguments.is_empty()
            && call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|expression| self.is_static_expression(expression, depth + 1))
            })
    }

    fn is_static_process_cwd(&self, call: &'a CallExpression<'a>) -> bool {
        call.callee.is_specific_member_access("process", "cwd")
            && !matches!(
                self.lookup("process"),
                Some(Binding::Unknown | Binding::Import(_))
            )
    }

    fn bind_pattern_from_import(&mut self, pattern: &'a BindingPattern<'a>, path: &AccessPath) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                self.bind(identifier.name.as_str(), Binding::Import(path.clone()));
            }
            BindingPattern::ObjectPattern(pattern) => {
                for property in &pattern.properties {
                    self.bind_object_property_from_import(property, path);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::ArrayPattern(pattern) => {
                for element in pattern.elements.iter().flatten() {
                    self.bind_pattern_unknown(element);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::AssignmentPattern(pattern) => {
                self.bind_pattern_from_import(&pattern.left, path);
            }
        }
    }

    fn bind_object_property_from_import(
        &mut self,
        property: &'a BindingProperty<'a>,
        base: &AccessPath,
    ) {
        let Some(key_name) = property_key_name(&property.key) else {
            self.bind_pattern_unknown(&property.value);
            return;
        };
        let mut path = base.clone();
        path.path.push(CompactString::from(key_name));
        self.bind_pattern_from_import(&property.value, &path);
    }

    fn bind_pattern_static_or_unknown(&mut self, pattern: &'a BindingPattern<'a>, is_static: bool) {
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                self.bind(
                    identifier.name.as_str(),
                    if is_static {
                        Binding::Static
                    } else {
                        Binding::Unknown
                    },
                );
            }
            BindingPattern::ObjectPattern(pattern) => {
                for property in &pattern.properties {
                    self.bind_pattern_static_or_unknown(&property.value, false);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::ArrayPattern(pattern) => {
                for element in pattern.elements.iter().flatten() {
                    self.bind_pattern_static_or_unknown(element, false);
                }
                if let Some(rest) = &pattern.rest {
                    self.bind_pattern_unknown(&rest.argument);
                }
            }
            BindingPattern::AssignmentPattern(pattern) => {
                self.bind_pattern_static_or_unknown(&pattern.left, false);
            }
        }
    }

    fn bind_pattern_unknown(&mut self, pattern: &'a BindingPattern<'a>) {
        self.bind_pattern_static_or_unknown(pattern, false);
    }
}
