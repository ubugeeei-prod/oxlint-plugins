//! AST visitor that collects UnoCSS-relevant literals and JSX opening elements.
//!
//! This module replaces the former byte-scanning `literals.rs` / `tags.rs` with a
//! proper `oxc_ast_visit::Visit` traversal.  The result is three collections handed
//! back to `Scanner`:
//!
//! * `class_literals` — string-literal spans for JSX `class`/`className`/`classname`
//!   attributes (checked by blocklist, enforce-class-compile, order).
//! * `call_literals` — string-literal spans inside UnoCSS function arguments or UnoCSS
//!   variable initialisers (checked only for order).
//! * `opening_elements` — one entry per JSX opening element with valueless attribute
//!   spans for the `order-attributify` rule.

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Expression, JSXAttributeItem, JSXAttributeName,
    JSXAttributeValue, JSXOpeningElement, ObjectExpression, ObjectPropertyKind, TemplateLiteral,
    VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use regex::Regex;

use crate::types::LiteralSpan;

/// A collected JSX opening-element: the tag's span and its valueless attributes.
pub(crate) struct OpeningElement {
    /// Span of the opening tag (from `<` to `>`).
    pub(crate) span: Span,
    /// Valueless attributes in source order (name + attribute span).
    pub(crate) valueless_attrs: SmallVec<[(CompactString, Span); 8]>,
}

/// Visitor state accumulating all UnoCSS-relevant spans from one parse run.
pub(crate) struct UnocssVisitor<'src, 'a> {
    pub(crate) source_text: &'src str,
    pub(crate) uno_functions: &'a [CompactString],
    pub(crate) variable_regexes: &'a [Regex],

    /// Literals that are JSX class attribute values.
    pub(crate) class_literals: SmallVec<[LiteralSpan<'src>; 16]>,
    /// Literals in uno-call args / uno-variable inits.
    pub(crate) call_literals: SmallVec<[LiteralSpan<'src>; 16]>,
    /// JSX opening elements for `order-attributify`.
    pub(crate) opening_elements: SmallVec<[OpeningElement; 8]>,
}

impl<'src, 'a> UnocssVisitor<'src, 'a> {
    pub(crate) fn new(
        source_text: &'src str,
        uno_functions: &'a [CompactString],
        variable_regexes: &'a [Regex],
    ) -> Self {
        Self {
            source_text,
            uno_functions,
            variable_regexes,
            class_literals: SmallVec::new(),
            call_literals: SmallVec::new(),
            opening_elements: SmallVec::new(),
        }
    }

    /// Build a `LiteralSpan` from the `Span` of a quoted string literal.
    /// The content span strips the surrounding quote characters (one byte each).
    fn literal_span_from_string(&self, span: Span) -> Option<LiteralSpan<'src>> {
        let start = span.start as usize;
        let end = span.end as usize;
        if end <= start + 1 || end > self.source_text.len() {
            return None;
        }
        let content_start = start + 1;
        let content_end = end - 1;
        let content = &self.source_text[content_start..content_end];
        // Skip literals that contain backslash escapes (mirrors old scanner).
        if content.contains('\\') {
            return None;
        }
        Some(LiteralSpan {
            content_start,
            content_end,
            content,
        })
    }

    /// Collect usable string literals from a template literal's quasis.
    ///
    /// * Simple templates (no expressions): emitted as one span over the body.
    /// * Interpolated templates: each non-empty quasi is emitted individually.
    fn collect_template_literals(
        &self,
        tl: &TemplateLiteral<'_>,
        out: &mut SmallVec<[LiteralSpan<'src>; 16]>,
    ) {
        if tl.quasis.is_empty() {
            return;
        }
        if tl.expressions.is_empty() {
            // Simple template literal: treat body as a single string.
            let tl_start = tl.span.start as usize;
            let content_start = tl_start + 1;
            let content_end = tl.span.end as usize - 1;
            if content_end <= content_start || content_end > self.source_text.len() {
                return;
            }
            let content = &self.source_text[content_start..content_end];
            if content.contains('\\') {
                return;
            }
            out.push(LiteralSpan {
                content_start,
                content_end,
                content,
            });
        } else {
            // Interpolated: emit each quasi separately.
            for quasi in &tl.quasis {
                let raw = quasi.value.raw.as_str();
                if raw.contains('\\') || raw.trim().is_empty() {
                    continue;
                }
                let span_start = quasi.span.start as usize;
                let span_end = quasi.span.end as usize;
                if span_end > self.source_text.len() {
                    continue;
                }
                let token = &self.source_text[span_start..span_end];
                // Strip the `}` prefix when this is not the first quasi.
                let prefix = if token.starts_with('}') { 1 } else { 0 };
                // Strip the trailing `${` or closing `` ` ``.
                let suffix = if token.ends_with("${") {
                    2
                } else if token.ends_with('`') {
                    1
                } else {
                    0
                };
                let content_start = span_start + prefix;
                let content_end = span_end.saturating_sub(suffix);
                if content_end <= content_start {
                    continue;
                }
                let content = &self.source_text[content_start..content_end];
                if content.trim().is_empty() {
                    continue;
                }
                out.push(LiteralSpan {
                    content_start,
                    content_end,
                    content,
                });
            }
        }
    }

    /// Recursively collect string/template literals from an `Expression`.
    ///
    /// Mirrors upstream `checkPossibleLiteral`: handles string literals, template
    /// literals (simple and interpolated), tagged `String.raw\`…\``, conditional,
    /// logical, parenthesised, object-value, and array-element expressions.
    fn collect_expression_literals(
        &self,
        expr: &Expression<'_>,
        out: &mut SmallVec<[LiteralSpan<'src>; 16]>,
    ) {
        match expr {
            Expression::StringLiteral(lit) => {
                if let Some(ls) = self.literal_span_from_string(lit.span) {
                    out.push(ls);
                }
            }
            Expression::TemplateLiteral(tl) => {
                self.collect_template_literals(tl, out);
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.collect_template_literals(&tagged.quasi, out);
            }
            Expression::ConditionalExpression(cond) => {
                self.collect_expression_literals(&cond.consequent, out);
                self.collect_expression_literals(&cond.alternate, out);
            }
            Expression::LogicalExpression(log) => {
                self.collect_expression_literals(&log.left, out);
                self.collect_expression_literals(&log.right, out);
            }
            Expression::ParenthesizedExpression(p) => {
                self.collect_expression_literals(&p.expression, out);
            }
            Expression::ObjectExpression(obj) => {
                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(op) = prop {
                        self.collect_expression_literals(&op.value, out);
                    }
                }
            }
            Expression::ArrayExpression(arr) => {
                for el in &arr.elements {
                    if let Some(expr) = el.as_expression() {
                        self.collect_expression_literals(expr, out);
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect literals from an object passed as a UnoCSS-function argument,
    /// mirroring upstream `handleObjectExpression` in the CallExpression visitor:
    /// property VALUES (recursing into nested objects) AND property KEYS that are
    /// string / template / `String.raw` literals (e.g. `{ 'a b': x }`,
    /// `` { [`a b`]: x } ``). Upstream collects keys ONLY for call-argument
    /// objects, not variable-initialiser objects, so this is reached only from
    /// the call path; `collect_expression_literals` keeps the value-only behavior.
    fn collect_object_with_keys(
        &self,
        obj: &ObjectExpression<'_>,
        out: &mut SmallVec<[LiteralSpan<'src>; 16]>,
    ) {
        for prop in &obj.properties {
            let ObjectPropertyKind::ObjectProperty(op) = prop else {
                continue;
            };
            if let Expression::ObjectExpression(inner) = &op.value {
                self.collect_object_with_keys(inner, out);
            } else {
                self.collect_expression_literals(&op.value, out);
            }
            // Non-identifier keys (string/template/`String.raw`) expose an inner
            // expression; identifier keys (`small:`) return `None` and are skipped.
            if let Some(key_expr) = op.key.as_expression() {
                self.collect_expression_literals(key_expr, out);
            }
        }
    }

    /// Return the simple identifier callee name of a call expression, if any.
    fn callee_name<'x>(call: &'x CallExpression<'_>) -> Option<&'x str> {
        match &call.callee {
            Expression::Identifier(id) => Some(id.name.as_str()),
            _ => None,
        }
    }

    /// True if this call's callee matches a configured UnoCSS function (case-insensitive).
    fn is_uno_call(&self, call: &CallExpression<'_>) -> bool {
        let Some(name) = Self::callee_name(call) else {
            return false;
        };
        self.uno_functions
            .iter()
            .any(|f| f.as_str().eq_ignore_ascii_case(name))
    }

    /// True if `name` matches any of the compiled UnoCSS variable regexes.
    fn is_uno_variable_name(&self, name: &str) -> bool {
        self.variable_regexes.iter().any(|rx| rx.is_match(name))
    }

    /// Collect string literals from JSX class-attribute values into `out`.
    fn collect_jsx_class_value(
        &self,
        attr_value: &JSXAttributeValue<'_>,
        out: &mut SmallVec<[LiteralSpan<'src>; 16]>,
    ) {
        match attr_value {
            JSXAttributeValue::StringLiteral(lit) => {
                if let Some(ls) = self.literal_span_from_string(lit.span) {
                    out.push(ls);
                }
            }
            JSXAttributeValue::ExpressionContainer(ec) => {
                // JSXExpression inherits all Expression variants; use `as_expression()`.
                if let Some(inner) = ec.expression.as_expression() {
                    self.collect_expression_literals(inner, out);
                }
            }
            _ => {}
        }
    }

    /// Collect order-checkable literals from a UnoCSS-variable declarator's init.
    /// A no-op unless the declarator binds a simple identifier matching a
    /// configured UnoCSS variable regex.
    fn collect_uno_variable_init(&mut self, decl: &VariableDeclarator<'src>) {
        let BindingPattern::BindingIdentifier(id) = &decl.id else {
            return;
        };
        if !self.is_uno_variable_name(id.name.as_str()) {
            return;
        }
        let Some(init) = &decl.init else { return };
        let mut tmp: SmallVec<[LiteralSpan<'src>; 16]> = SmallVec::new();
        self.collect_expression_literals(init, &mut tmp);
        for ls in tmp {
            if !ls.content.trim().is_empty() {
                self.call_literals.push(ls);
            }
        }
    }
}

// ── JSX attribute name helpers ───────────────────────────────────────────────

fn is_jsx_class_attr(name: &str) -> bool {
    name.eq_ignore_ascii_case("class") || name.eq_ignore_ascii_case("classname")
}

const IGNORED_ATTRIBUTIFY_ATTRIBUTES: [&str; 4] = ["style", "class", "classname", "value"];

fn is_ignored_attributify_attr(name: &str) -> bool {
    IGNORED_ATTRIBUTIFY_ATTRIBUTES
        .iter()
        .any(|attr| name.eq_ignore_ascii_case(attr))
}

// ── oxc_ast_visit::Visit implementation ─────────────────────────────────────

impl<'src, 'a> Visit<'src> for UnocssVisitor<'src, 'a> {
    /// Handle JSX opening elements: collect class literals + valueless attrs.
    fn visit_jsx_opening_element(&mut self, elem: &JSXOpeningElement<'src>) {
        let mut valueless_attrs: SmallVec<[(CompactString, Span); 8]> = SmallVec::new();

        for attr_item in &elem.attributes {
            let JSXAttributeItem::Attribute(attr) = attr_item else {
                continue;
            };
            let attr_name = match &attr.name {
                JSXAttributeName::Identifier(id) => id.name.as_str(),
                JSXAttributeName::NamespacedName(_) => continue,
            };

            if is_jsx_class_attr(attr_name) {
                if let Some(value) = &attr.value {
                    let mut tmp: SmallVec<[LiteralSpan<'src>; 16]> = SmallVec::new();
                    self.collect_jsx_class_value(value, &mut tmp);
                    for ls in tmp {
                        if !ls.content.trim().is_empty() {
                            self.class_literals.push(ls);
                        }
                    }
                }
                continue;
            }

            // Valueless attribute (no `=value`) → candidate for order-attributify.
            if attr.value.is_none() && !is_ignored_attributify_attr(attr_name) {
                valueless_attrs.push((CompactString::from(attr_name), attr.span));
            }
        }

        // `order-attributify` needs at least two valueless attributes, so an
        // element with none can never produce a diagnostic — skip storing it.
        if !valueless_attrs.is_empty() {
            self.opening_elements.push(OpeningElement {
                span: elem.span,
                valueless_attrs,
            });
        }

        walk::walk_jsx_opening_element(self, elem);
    }

    /// Detect UnoCSS function calls and collect literals from their arguments.
    fn visit_call_expression(&mut self, call: &CallExpression<'src>) {
        if self.is_uno_call(call) {
            for arg in &call.arguments {
                if let Argument::SpreadElement(_) = arg {
                    continue;
                }
                if let Some(expr) = arg.as_expression() {
                    let mut tmp: SmallVec<[LiteralSpan<'src>; 16]> = SmallVec::new();
                    // A direct object argument also has its keys checked (upstream
                    // `handleObjectExpression`); other args collect values only.
                    if let Expression::ObjectExpression(obj) = expr {
                        self.collect_object_with_keys(obj, &mut tmp);
                    } else {
                        self.collect_expression_literals(expr, &mut tmp);
                    }
                    for ls in tmp {
                        if !ls.content.trim().is_empty() {
                            self.call_literals.push(ls);
                        }
                    }
                }
            }
        }
        walk::walk_call_expression(self, call);
    }

    /// Detect UnoCSS variable declarators and collect literals from their inits.
    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'src>) {
        self.collect_uno_variable_init(decl);
        walk::walk_variable_declarator(self, decl);
    }
}
