//! Pre-pass analysis: determine which `RegExpLiteral` nodes in a program are
//! used as *whole* patterns (i.e. the regex is fully applied, not merely
//! accessed via `.source`).  The result is consumed by the `no-lazy-ends`
//! check, which only fires when `ignorePartial: true` (the upstream default).
//!
//! A regex is considered a "whole pattern usage" when:
//!
//! 1. **Direct call object**: `/re/.test(str)` — the literal is the object of
//!    a member-call to a recognised regexp or string method.
//! 2. **Variable forwarding**: `const x = /re/; x.exec(str)` — the literal is
//!    the initialiser of a `const`/`let`/`var` binding that is *not* exported,
//!    and at least one read reference to that binding is the object of such a
//!    call.
//!
//! Recognised methods (regex as **object**): `test`, `exec`.
//! Recognised methods (regex as first **argument**): `match`, `matchAll`,
//! `search`, `split`, `replace`, `replaceAll`.
//!
//! Bare literals, `.source` accesses, and exported bindings are excluded,
//! matching upstream `ignorePartial: true` semantics.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, BindingPattern, ChainElement, Expression, ForStatementInit, ForStatementLeft,
    ObjectPropertyKind, Program, Statement,
};
use oxc_semantic::{AstNodes, NodeId, Scoping};
use oxlint_plugins_carton::{CompactString, FastHashSet};

/// Methods where the **regex is the receiver** (object position).
fn is_regexp_object_method(name: &str) -> bool {
    matches!(name, "test" | "exec")
}

/// Methods where the **regex is the first argument** (string-method pattern).
fn is_string_regexp_method(name: &str) -> bool {
    matches!(
        name,
        "match" | "matchAll" | "search" | "split" | "replace" | "replaceAll"
    )
}

/// Walk the program AST, collecting the `span.start` of every `RegExpLiteral`
/// that is provably used as a whole pattern.  Returns the set; the main
/// scanner stores it and guards `no-lazy-ends` on membership.
pub(crate) fn collect_whole_pattern_regex_spans<'a>(
    program: &'a Program<'a>,
    scoping: &'a Scoping,
    nodes: &'a AstNodes<'a>,
    source_text: &'a str,
) -> FastHashSet<u32> {
    let mut ctx = UsageCtx {
        scoping,
        nodes,
        spans: FastHashSet::default(),
        exported_names: FastHashSet::default(),
    };
    // Collect `/* exported name */` style exports from the program comments.
    // These appear in script-mode files processed by tools such as jshint/
    // eslint globals, and mean the binding is consumed outside this file.
    for comment in &program.comments {
        let span = comment.content_span();
        let text = span.source_text(source_text).trim();
        // The canonical form is `/* exported a, b, c */`.
        if let Some(rest) = text.strip_prefix("exported") {
            let rest = rest.trim();
            for name in rest.split(',') {
                let name = name.trim();
                if !name.is_empty() {
                    ctx.exported_names.insert(CompactString::from(name));
                }
            }
        }
    }
    ctx.walk_statements(&program.body);
    ctx.spans
}

struct UsageCtx<'a> {
    scoping: &'a Scoping,
    nodes: &'a AstNodes<'a>,
    /// Span-starts accumulated so far.
    spans: FastHashSet<u32>,
    /// Names exported via `/* exported name */` comment (script-mode).
    exported_names: FastHashSet<CompactString>,
}

impl<'a> UsageCtx<'a> {
    fn walk_statements(&mut self, stmts: &'a [Statement<'a>]) {
        for stmt in stmts {
            self.walk_statement(stmt);
        }
    }

    fn walk_statement(&mut self, stmt: &'a Statement<'a>) {
        match stmt {
            Statement::BlockStatement(block) => self.walk_statements(&block.body),
            Statement::ExpressionStatement(s) => self.walk_expr(&s.expression),
            Statement::IfStatement(s) => {
                self.walk_expr(&s.test);
                self.walk_statement(&s.consequent);
                if let Some(alt) = &s.alternate {
                    self.walk_statement(alt);
                }
            }
            Statement::ReturnStatement(s) => {
                if let Some(arg) = &s.argument {
                    self.walk_expr(arg);
                }
            }
            Statement::ThrowStatement(s) => self.walk_expr(&s.argument),
            Statement::WhileStatement(s) => {
                self.walk_expr(&s.test);
                self.walk_statement(&s.body);
            }
            Statement::DoWhileStatement(s) => {
                self.walk_statement(&s.body);
                self.walk_expr(&s.test);
            }
            Statement::ForStatement(s) => {
                if let Some(init) = &s.init {
                    match init {
                        ForStatementInit::VariableDeclaration(decl) => {
                            for declarator in &decl.declarations {
                                if let Some(init) = &declarator.init {
                                    self.walk_expr(init);
                                }
                            }
                        }
                        _ => {
                            if let Some(expr) = init.as_expression() {
                                self.walk_expr(expr);
                            }
                        }
                    }
                }
                if let Some(test) = &s.test {
                    self.walk_expr(test);
                }
                if let Some(update) = &s.update {
                    self.walk_expr(update);
                }
                self.walk_statement(&s.body);
            }
            Statement::ForInStatement(s) => {
                if let ForStatementLeft::VariableDeclaration(decl) = &s.left {
                    for declarator in &decl.declarations {
                        if let Some(init) = &declarator.init {
                            self.walk_expr(init);
                        }
                    }
                }
                self.walk_expr(&s.right);
                self.walk_statement(&s.body);
            }
            Statement::ForOfStatement(s) => {
                if let ForStatementLeft::VariableDeclaration(decl) = &s.left {
                    for declarator in &decl.declarations {
                        if let Some(init) = &declarator.init {
                            self.walk_expr(init);
                        }
                    }
                }
                self.walk_expr(&s.right);
                self.walk_statement(&s.body);
            }
            Statement::SwitchStatement(s) => {
                self.walk_expr(&s.discriminant);
                for case in &s.cases {
                    if let Some(test) = &case.test {
                        self.walk_expr(test);
                    }
                    self.walk_statements(&case.consequent);
                }
            }
            Statement::TryStatement(s) => {
                self.walk_statements(&s.block.body);
                if let Some(handler) = &s.handler {
                    self.walk_statements(&handler.body.body);
                }
                if let Some(finalizer) = &s.finalizer {
                    self.walk_statements(&finalizer.body);
                }
            }
            Statement::LabeledStatement(s) => self.walk_statement(&s.body),
            Statement::VariableDeclaration(decl) => {
                for declarator in &decl.declarations {
                    if let Some(init) = &declarator.init {
                        self.walk_expr(init);
                    }
                }
            }
            Statement::FunctionDeclaration(f) => {
                if let Some(body) = &f.body {
                    self.walk_statements(&body.statements);
                }
            }
            Statement::ClassDeclaration(_) => {}
            // Exported declarations: scan the inner declaration the same way,
            // but mark identifiers as exported — their regex initialisers are
            // not whole-pattern usages even if the variable is called elsewhere.
            Statement::ExportNamedDeclaration(export) => {
                if let Some(decl) = &export.declaration {
                    match decl {
                        oxc_ast::ast::Declaration::VariableDeclaration(var) => {
                            // Mark all declared names as ES-module-exported.
                            for declarator in &var.declarations {
                                self.mark_binding_exported(&declarator.id);
                                if let Some(init) = &declarator.init {
                                    self.walk_expr(init);
                                }
                            }
                        }
                        oxc_ast::ast::Declaration::FunctionDeclaration(f) => {
                            if let Some(body) = &f.body {
                                self.walk_statements(&body.statements);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                if let Some(expr) = export.declaration.as_expression() {
                    self.walk_expr(expr);
                }
            }
            _ => {}
        }
    }

    fn mark_binding_exported(&mut self, binding: &BindingPattern) {
        if let BindingPattern::BindingIdentifier(id) = binding {
            self.exported_names
                .insert(CompactString::from(id.name.as_str()));
        }
    }

    fn walk_expr(&mut self, expr: &'a Expression<'a>) {
        match expr.get_inner_expression() {
            Expression::CallExpression(call) => {
                self.check_call(call);
                // Also recurse into callee and arguments.
                self.walk_expr(&call.callee);
                for arg in &call.arguments {
                    self.walk_argument(arg);
                }
            }
            Expression::StaticMemberExpression(member) => {
                self.walk_expr(&member.object);
            }
            Expression::ComputedMemberExpression(member) => {
                self.walk_expr(&member.object);
                self.walk_expr(&member.expression);
            }
            Expression::BinaryExpression(bin) => {
                self.walk_expr(&bin.left);
                self.walk_expr(&bin.right);
            }
            Expression::LogicalExpression(log) => {
                self.walk_expr(&log.left);
                self.walk_expr(&log.right);
            }
            Expression::ConditionalExpression(cond) => {
                self.walk_expr(&cond.test);
                self.walk_expr(&cond.consequent);
                self.walk_expr(&cond.alternate);
            }
            Expression::ArrayExpression(arr) => {
                for el in &arr.elements {
                    if let Some(e) = el.as_expression() {
                        self.walk_expr(e);
                    }
                }
            }
            Expression::ObjectExpression(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            self.walk_expr(&p.value);
                        }
                        ObjectPropertyKind::SpreadProperty(s) => {
                            self.walk_expr(&s.argument);
                        }
                    }
                }
            }
            Expression::AssignmentExpression(assign) => {
                self.walk_expr(&assign.right);
            }
            Expression::SequenceExpression(seq) => {
                for e in &seq.expressions {
                    self.walk_expr(e);
                }
            }
            Expression::AwaitExpression(aw) => self.walk_expr(&aw.argument),
            Expression::UnaryExpression(un) => self.walk_expr(&un.argument),
            Expression::YieldExpression(y) => {
                if let Some(arg) = &y.argument {
                    self.walk_expr(arg);
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.check_call(call);
                    self.walk_expr(&call.callee);
                    for arg in &call.arguments {
                        self.walk_argument(arg);
                    }
                }
                ChainElement::StaticMemberExpression(member) => {
                    self.walk_expr(&member.object);
                }
                ChainElement::ComputedMemberExpression(member) => {
                    self.walk_expr(&member.object);
                    self.walk_expr(&member.expression);
                }
                _ => {}
            },
            Expression::FunctionExpression(f) => {
                if let Some(body) = &f.body {
                    self.walk_statements(&body.statements);
                }
            }
            Expression::ArrowFunctionExpression(f) => {
                self.walk_statements(&f.body.statements);
            }
            _ => {}
        }
    }

    fn walk_argument(&mut self, arg: &'a Argument<'a>) {
        if let Some(expr) = arg.as_expression() {
            self.walk_expr(expr);
        } else if let Argument::SpreadElement(spread) = arg {
            self.walk_expr(&spread.argument);
        }
    }

    /// Inspects one `CallExpression` node for "whole-pattern usage" and records
    /// the relevant `RegExpLiteral` spans.
    fn check_call(&mut self, call: &'a oxc_ast::ast::CallExpression<'a>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        let method = member.property.name.as_str();

        // Case A: regex is the object of `.test()` / `.exec()`.
        if is_regexp_object_method(method) {
            let obj = member.object.get_inner_expression();
            self.record_if_regex(obj);
            return;
        }

        // Case B: regex is the first argument to a string method.
        if is_string_regexp_method(method) {
            let Some(first_arg) = call.arguments.first().and_then(Argument::as_expression) else {
                return;
            };
            let first_arg = first_arg.get_inner_expression();
            self.record_if_regex(first_arg);
        }
    }

    /// Records `expr` as a whole-pattern regex if it is:
    /// - a `RegExpLiteral` directly, or
    /// - an `IdentifierReference` that resolves to a non-exported `const`/
    ///   `let`/`var` binding whose initialiser is a `RegExpLiteral`.
    fn record_if_regex(&mut self, expr: &'a Expression<'a>) {
        match expr {
            Expression::RegExpLiteral(lit) => {
                self.spans.insert(lit.span.start);
            }
            Expression::Identifier(ident) => {
                // Resolve the reference to its declaration symbol.
                let Some(ref_id) = ident.reference_id.get() else {
                    return;
                };
                let Some(symbol_id) = self.scoping.get_reference(ref_id).symbol_id() else {
                    // Free/global reference — not a local binding.
                    return;
                };
                // Check that the binding is not script-exported via comment.
                let sym_name = self.scoping.symbol_name(symbol_id);
                if self.exported_names.contains(sym_name) {
                    return;
                }
                // Walk up from the declaration to see if it's inside an ES-module
                // `export` statement.
                let decl_node_id = self.scoping.symbol_declaration(symbol_id);
                if self.is_node_in_export(decl_node_id) {
                    return;
                }
                // Check the initialiser.
                let decl_kind = self.nodes.get_node(decl_node_id).kind();
                let AstKind::VariableDeclarator(declarator) = decl_kind else {
                    return;
                };
                let Some(init) = &declarator.init else {
                    return;
                };
                let Expression::RegExpLiteral(lit) = init.get_inner_expression() else {
                    return;
                };
                self.spans.insert(lit.span.start);
            }
            _ => {}
        }
    }

    /// Returns `true` when the AST node at `node_id` (or one of its ancestors
    /// up to the program root) is an `ExportNamedDeclaration` or
    /// `ExportDefaultDeclaration` — meaning the binding is an ES-module export.
    fn is_node_in_export(&self, node_id: NodeId) -> bool {
        let mut current = node_id;
        // Walk up to the root (NodeId 0 is the Program node).
        loop {
            match self.nodes.get_node(current).kind() {
                AstKind::ExportNamedDeclaration(_) | AstKind::ExportDefaultDeclaration(_) => {
                    return true;
                }
                AstKind::Program(_) => return false,
                _ => {}
            }
            let parent = self.nodes.parent_id(current);
            if parent == current {
                // Reached the root without cycling.
                return false;
            }
            current = parent;
        }
    }
}
