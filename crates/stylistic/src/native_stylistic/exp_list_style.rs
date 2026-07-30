//! Native implementation of experimental `@stylistic/exp-list-style`.
//!
//! Oxc identifies every supported JavaScript and TypeScript list node while
//! the shared lexer supplies ESLint-compatible token, comment, and byte ranges.
//! JSON/JSONC files use the same token model with a small bracket-tree walker.

use std::collections::{BTreeMap, HashMap};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::scope::ScopeFlags;
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::{
    context::{Scan, first_option},
    lexer::TokenKind,
};

const RULE: &str = "exp-list-style";
const SHOULD_SPACING: &str = "shouldSpacing";
const SHOULD_NOT_SPACING: &str = "shouldNotSpacing";
const SHOULD_WRAP: &str = "shouldWrap";
const SHOULD_NOT_WRAP: &str = "shouldNotWrap";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spacing {
    Always,
    Never,
}

#[derive(Clone, Copy, Debug)]
struct BaseConfig {
    spacing: Spacing,
    /// `None` represents JavaScript's `Number.POSITIVE_INFINITY`.
    max_items: Option<usize>,
    min_items: usize,
}

impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            spacing: Spacing::Never,
            max_items: None,
            min_items: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct PartialConfig {
    spacing: Option<Spacing>,
    max_items: Option<usize>,
    max_items_present: bool,
    min_items: Option<usize>,
}

impl PartialConfig {
    fn apply(self, config: &mut BaseConfig) {
        if let Some(spacing) = self.spacing {
            config.spacing = spacing;
        }
        if self.max_items_present {
            config.max_items = self.max_items;
        }
        if let Some(min_items) = self.min_items {
            config.min_items = min_items;
        }
    }

    fn merge(&mut self, other: Self) {
        if other.spacing.is_some() {
            self.spacing = other.spacing;
        }
        if other.max_items_present {
            self.max_items = other.max_items;
            self.max_items_present = true;
        }
        if other.min_items.is_some() {
            self.min_items = other.min_items;
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Override {
    Off,
    Config(PartialConfig),
}

#[derive(Clone, Debug)]
struct Options {
    base: BaseConfig,
    overrides: HashMap<String, Override>,
}

impl Options {
    fn resolve(&self, paren: ParenType, node_type: &str) -> Option<BaseConfig> {
        let paren_override = self.overrides.get(paren.key()).copied();
        let node_override = self.overrides.get(node_type).copied();
        if matches!(node_override, Some(Override::Off))
            || (node_override.is_none() && matches!(paren_override, Some(Override::Off)))
        {
            return None;
        }

        let mut resolved = self.base;
        if let Some(Override::Config(config)) = paren_override {
            config.apply(&mut resolved);
        }
        if let Some(Override::Config(config)) = node_override {
            config.apply(&mut resolved);
        }
        Some(resolved)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParenType {
    Square,
    Curly,
    Round,
    Angle,
}

impl ParenType {
    const fn key(self) -> &'static str {
        match self {
            Self::Square => "[]",
            Self::Curly => "{}",
            Self::Round => "()",
            Self::Angle => "<>",
        }
    }

    const fn delimiters(self) -> (&'static str, &'static str) {
        match self {
            Self::Square => ("[", "]"),
            Self::Curly => ("{", "}"),
            Self::Round => ("(", ")"),
            Self::Angle => ("<", ">"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TokenRef {
    start: usize,
    end: usize,
    kind: TokenKind,
    original_index: Option<usize>,
}

impl TokenRef {
    fn text(self, source: &str) -> &str {
        source.get(self.start..self.end).unwrap_or_default()
    }
}

/// Checks all supported list nodes and appends source-ordered diagnostics.
pub(crate) fn check_exp_list_style(
    source: &str,
    filename: Option<&str>,
    raw_options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let initial_len = diagnostics.len();
    let scan = Scan::new(source);
    let options = normalize_options(raw_options);

    if filename.is_some_and(is_json_filename) {
        let mut checker = ListChecker {
            source,
            scan: &scan,
            options: &options,
            diagnostics,
        };
        checker.check_json_document();
    } else {
        parse_and_check(source, filename, &scan, &options, diagnostics);
    }

    diagnostics[initial_len..].sort_by_key(|diagnostic| {
        (
            diagnostic.range.start,
            diagnostic.range.end,
            message_order(&diagnostic.message_id),
        )
    });
}

fn parse_and_check(
    source: &str,
    filename: Option<&str>,
    scan: &Scan<'_>,
    options: &Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_source(source, source_type, scan, options, diagnostics);
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_source(source, source_type, scan, options, diagnostics) {
            return;
        }
    }
}

fn parse_source(
    source: &str,
    source_type: SourceType,
    scan: &Scan<'_>,
    options: &Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let checker = ListChecker {
        source,
        scan,
        options,
        diagnostics,
    };
    let mut visitor = ExpListStyleVisitor { checker };
    visitor.visit_program(&parsed.program);
    true
}

struct ExpListStyleVisitor<'source, 'scan, 'options, 'diagnostics> {
    checker: ListChecker<'source, 'scan, 'options, 'diagnostics>,
}

impl<'ast> Visit<'ast> for ExpListStyleVisitor<'_, '_, '_, '_> {
    fn visit_array_expression(&mut self, node: &ArrayExpression<'ast>) {
        let items = node
            .elements
            .iter()
            .map(|element| {
                (!matches!(element, ArrayExpressionElement::Elision(_))).then(|| element.span())
            })
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "ArrayExpression",
            ParenType::Square,
            &items,
            PairStrategy::Exact,
        );
        walk::walk_array_expression(self, node);
    }

    fn visit_array_pattern(&mut self, node: &ArrayPattern<'ast>) {
        let mut items = node
            .elements
            .iter()
            .map(|element| element.as_ref().map(GetSpan::span))
            .collect::<Vec<_>>();
        items.extend(node.rest.iter().map(|rest| Some(rest.span())));
        self.checker.check(
            node.span,
            "ArrayPattern",
            ParenType::Square,
            &items,
            PairStrategy::Exact,
        );
        walk::walk_array_pattern(self, node);
    }

    fn visit_arrow_function_expression(&mut self, node: &ArrowFunctionExpression<'ast>) {
        let items = formal_parameter_spans(&node.params, None);
        self.checker.check(
            node.span,
            "ArrowFunctionExpression",
            ParenType::Round,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_arrow_function_expression(self, node);
    }

    fn visit_call_expression(&mut self, node: &CallExpression<'ast>) {
        let items = node
            .arguments
            .iter()
            .map(|argument| Some(argument.span()))
            .collect::<Vec<_>>();
        let anchor = node
            .type_arguments
            .as_ref()
            .map_or_else(|| node.callee.span(), |arguments| arguments.span);
        self.checker.check(
            node.span,
            "CallExpression",
            ParenType::Round,
            &items,
            PairStrategy::AfterAnchor(anchor.end),
        );
        walk::walk_call_expression(self, node);
    }

    fn visit_new_expression(&mut self, node: &NewExpression<'ast>) {
        let items = node
            .arguments
            .iter()
            .map(|argument| Some(argument.span()))
            .collect::<Vec<_>>();
        let anchor = node
            .type_arguments
            .as_ref()
            .map_or_else(|| node.callee.span(), |arguments| arguments.span);
        self.checker.check(
            node.span,
            "NewExpression",
            ParenType::Round,
            &items,
            PairStrategy::AfterAnchor(anchor.end),
        );
        walk::walk_new_expression(self, node);
    }

    fn visit_function(&mut self, node: &Function<'ast>, flags: ScopeFlags) {
        let node_type = match node.r#type {
            FunctionType::FunctionDeclaration => Some("FunctionDeclaration"),
            FunctionType::FunctionExpression => Some("FunctionExpression"),
            FunctionType::TSDeclareFunction => Some("TSDeclareFunction"),
            FunctionType::TSEmptyBodyFunctionExpression => None,
        };
        if let Some(node_type) = node_type {
            let items = formal_parameter_spans(
                &node.params,
                node.this_param.as_ref().map(|parameter| parameter.span()),
            );
            self.checker.check(
                node.span,
                node_type,
                ParenType::Round,
                &items,
                PairStrategy::AroundItems,
            );
        }
        walk::walk_function(self, node, flags);
    }

    fn visit_if_statement(&mut self, node: &IfStatement<'ast>) {
        self.checker.check(
            node.span,
            "IfStatement",
            ParenType::Round,
            &[Some(node.test.span())],
            PairStrategy::AroundItems,
        );
        walk::walk_if_statement(self, node);
    }

    fn visit_import_declaration(&mut self, node: &ImportDeclaration<'ast>) {
        let items = node
            .specifiers
            .iter()
            .flatten()
            .filter_map(|specifier| match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    Some(Some(specifier.span))
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => None,
            })
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "ImportDeclaration",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        if let Some(with_clause) = &node.with_clause {
            let attributes = with_clause
                .with_entries
                .iter()
                .map(|attribute| Some(attribute.span))
                .collect::<Vec<_>>();
            self.checker.check(
                node.span,
                "ImportAttributes",
                ParenType::Curly,
                &attributes,
                PairStrategy::AroundItems,
            );
        }
        walk::walk_import_declaration(self, node);
    }

    fn visit_export_named_declaration(&mut self, node: &ExportNamedDeclaration<'ast>) {
        let items = node
            .specifiers
            .iter()
            .map(|specifier| Some(specifier.span))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "ExportNamedDeclaration",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        if let Some(with_clause) = &node.with_clause {
            let attributes = with_clause
                .with_entries
                .iter()
                .map(|attribute| Some(attribute.span))
                .collect::<Vec<_>>();
            self.checker.check(
                node.span,
                "ImportAttributes",
                ParenType::Curly,
                &attributes,
                PairStrategy::AroundItems,
            );
        }
        walk::walk_export_named_declaration(self, node);
    }

    fn visit_export_all_declaration(&mut self, node: &ExportAllDeclaration<'ast>) {
        if let Some(with_clause) = &node.with_clause {
            let attributes = with_clause
                .with_entries
                .iter()
                .map(|attribute| Some(attribute.span))
                .collect::<Vec<_>>();
            self.checker.check(
                node.span,
                "ImportAttributes",
                ParenType::Curly,
                &attributes,
                PairStrategy::AroundItems,
            );
        }
        walk::walk_export_all_declaration(self, node);
    }

    fn visit_object_expression(&mut self, node: &ObjectExpression<'ast>) {
        let items = node
            .properties
            .iter()
            .map(|property| Some(property.span()))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "ObjectExpression",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_object_expression(self, node);
    }

    fn visit_object_pattern(&mut self, node: &ObjectPattern<'ast>) {
        let mut items = node
            .properties
            .iter()
            .map(|property| Some(property.span))
            .collect::<Vec<_>>();
        items.extend(node.rest.iter().map(|rest| Some(rest.span())));
        self.checker.check(
            node.span,
            "ObjectPattern",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_object_pattern(self, node);
    }

    fn visit_ts_enum_body(&mut self, node: &TSEnumBody<'ast>) {
        let items = node
            .members
            .iter()
            .map(|member| Some(member.span))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "TSEnumBody",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_ts_enum_body(self, node);
    }

    fn visit_ts_function_type(&mut self, node: &TSFunctionType<'ast>) {
        let items = formal_parameter_spans(
            &node.params,
            node.this_param.as_ref().map(|parameter| parameter.span()),
        );
        self.checker.check(
            node.span,
            "TSFunctionType",
            ParenType::Round,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_ts_function_type(self, node);
    }

    fn visit_ts_interface_body(&mut self, node: &TSInterfaceBody<'ast>) {
        let items = node
            .body
            .iter()
            .map(|signature| Some(signature.span()))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "TSInterfaceBody",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_ts_interface_body(self, node);
    }

    fn visit_ts_tuple_type(&mut self, node: &TSTupleType<'ast>) {
        let items = node
            .element_types
            .iter()
            .map(|element| Some(element.span()))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "TSTupleType",
            ParenType::Square,
            &items,
            PairStrategy::Exact,
        );
        walk::walk_ts_tuple_type(self, node);
    }

    fn visit_ts_type_literal(&mut self, node: &TSTypeLiteral<'ast>) {
        let items = node
            .members
            .iter()
            .map(|signature| Some(signature.span()))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "TSTypeLiteral",
            ParenType::Curly,
            &items,
            PairStrategy::AroundItems,
        );
        walk::walk_ts_type_literal(self, node);
    }

    fn visit_ts_type_parameter_declaration(&mut self, node: &TSTypeParameterDeclaration<'ast>) {
        let items = node
            .params
            .iter()
            .map(|parameter| Some(parameter.span))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "TSTypeParameterDeclaration",
            ParenType::Angle,
            &items,
            PairStrategy::Exact,
        );
        walk::walk_ts_type_parameter_declaration(self, node);
    }

    fn visit_ts_type_parameter_instantiation(&mut self, node: &TSTypeParameterInstantiation<'ast>) {
        let items = node
            .params
            .iter()
            .map(|parameter| Some(parameter.span()))
            .collect::<Vec<_>>();
        self.checker.check(
            node.span,
            "TSTypeParameterInstantiation",
            ParenType::Angle,
            &items,
            PairStrategy::Exact,
        );
        walk::walk_ts_type_parameter_instantiation(self, node);
    }
}

fn formal_parameter_spans(
    parameters: &FormalParameters<'_>,
    this_parameter: Option<Span>,
) -> Vec<Option<Span>> {
    let mut spans = Vec::with_capacity(
        parameters
            .items
            .len()
            .saturating_add(usize::from(parameters.rest.is_some()))
            .saturating_add(usize::from(this_parameter.is_some())),
    );
    spans.extend(this_parameter.map(Some));
    spans.extend(
        parameters
            .items
            .iter()
            .map(|parameter| Some(parameter.span)),
    );
    spans.extend(
        parameters
            .rest
            .iter()
            .map(|parameter| Some(parameter.span())),
    );
    spans.sort_unstable_by_key(|span| span.map_or(u32::MAX, |span| span.start));
    spans
}

#[derive(Clone, Copy)]
enum PairStrategy {
    Exact,
    AroundItems,
    AfterAnchor(u32),
}

struct ListChecker<'source, 'scan, 'options, 'diagnostics> {
    source: &'source str,
    scan: &'scan Scan<'source>,
    options: &'options Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl ListChecker<'_, '_, '_, '_> {
    fn check(
        &mut self,
        root_span: Span,
        node_type: &str,
        paren: ParenType,
        items: &[Option<Span>],
        strategy: PairStrategy,
    ) {
        if items.is_empty() {
            return;
        }
        let Some((left, right)) = self.find_pair(root_span, items, paren, strategy) else {
            return;
        };
        self.check_pair(root_span, node_type, paren, items, left, right);
    }

    fn check_pair(
        &mut self,
        root_span: Span,
        node_type: &str,
        paren: ParenType,
        items: &[Option<Span>],
        left: TokenRef,
        right: TokenRef,
    ) {
        let Some(config) = self.options.resolve(paren, node_type) else {
            return;
        };
        let fits_single_line = same_line(self.source, left.end, right.start)
            && config
                .max_items
                .is_none_or(|maximum| items.len() <= maximum);
        if fits_single_line {
            self.check_spacing(left, right, config);
        } else {
            self.check_wrap(root_span, node_type, items, left, right, config);
        }
    }

    fn check_spacing(&mut self, left: TokenRef, right: TokenRef, config: BaseConfig) {
        let Some(first) = self.immediate_after(left) else {
            return;
        };
        let Some(last) = self.immediate_before(right) else {
            return;
        };
        self.check_space_gap(left, first, config.spacing);
        self.check_space_gap(last, right, config.spacing);
    }

    fn check_space_gap(&mut self, previous: TokenRef, next: TokenRef, spacing: Spacing) {
        let spaced = previous.end < next.start;
        match (spaced, spacing) {
            (false, Spacing::Always) => {
                self.report(
                    previous,
                    next,
                    SHOULD_SPACING,
                    Some((previous.end, previous.end, " ")),
                );
            }
            (true, Spacing::Never) => {
                self.report(
                    previous,
                    next,
                    SHOULD_NOT_SPACING,
                    Some((previous.end, next.start, "")),
                );
            }
            _ => {}
        }
    }

    fn check_wrap(
        &mut self,
        root_span: Span,
        node_type: &str,
        items: &[Option<Span>],
        left: TokenRef,
        right: TokenRef,
        config: BaseConfig,
    ) {
        let Some(token_after_left) = self.significant_after(left) else {
            return;
        };
        let first_target = items
            .first()
            .and_then(|item| *item)
            .and_then(|span| self.first_token(span))
            .unwrap_or(token_after_left);
        let root_single_line = !has_line_terminator(
            self.source
                .get(root_span.start as usize..root_span.end as usize)
                .unwrap_or_default(),
        );
        let need_wrap = if root_single_line {
            config
                .max_items
                .is_some_and(|maximum| items.len() > maximum)
        } else {
            items.len() >= config.min_items && !same_line(self.source, left.end, first_target.start)
        };

        self.check_wrap_gap(node_type, items.len(), left, token_after_left, need_wrap);
        for (index, item) in items.iter().enumerate() {
            let Some(span) = item else {
                continue;
            };
            let Some(first) = self.first_token(*span) else {
                continue;
            };
            if index == 0 && same_token(first, token_after_left) {
                continue;
            }
            let Some(previous) = self.token_before_item(first, left) else {
                continue;
            };
            self.check_wrap_gap(node_type, items.len(), previous, first, need_wrap);
        }
        if let Some(previous) = self.significant_before(right) {
            self.check_wrap_gap(node_type, items.len(), previous, right, need_wrap);
        }
    }

    fn check_wrap_gap(
        &mut self,
        node_type: &str,
        item_count: usize,
        previous: TokenRef,
        next: TokenRef,
        need_wrap: bool,
    ) {
        let on_same_line = same_line(self.source, previous.end, next.start);
        if on_same_line == need_wrap {
            let fix = if self.comments_exist_between(previous, next) {
                None
            } else if need_wrap {
                Some((next.start, next.start, "\n"))
            } else {
                let replacement = if item_count == 1 {
                    ""
                } else if matches!(node_type, "TSInterfaceBody" | "TSTypeLiteral")
                    && !previous.text(self.source).ends_with([',', ';'])
                {
                    ","
                } else {
                    ""
                };
                Some((previous.end, next.start, replacement))
            };
            self.report(
                previous,
                next,
                if need_wrap {
                    SHOULD_WRAP
                } else {
                    SHOULD_NOT_WRAP
                },
                fix,
            );
        }
    }

    fn report(
        &mut self,
        previous: TokenRef,
        next: TokenRef,
        message_id: &'static str,
        fix: Option<(usize, usize, &'static str)>,
    ) {
        let (Ok(start), Ok(end)) = (u32::try_from(previous.end), u32::try_from(next.start)) else {
            return;
        };
        let previous_value = token_value(previous, self.source);
        let next_value = token_value(next, self.source);
        let message_prefix = match message_id {
            SHOULD_SPACING => "Should have space between",
            SHOULD_NOT_SPACING => "Should not have space(s) between",
            SHOULD_WRAP => "Should have line break between",
            SHOULD_NOT_WRAP => "Should not have line break(s) between",
            _ => return,
        };
        let mut message = String::with_capacity(
            message_prefix.len() + previous_value.len() + next_value.len() + 11,
        );
        message.push_str(message_prefix);
        message.push_str(" '");
        message.push_str(previous_value);
        message.push_str("' and '");
        message.push_str(next_value);
        message.push('\'');
        let data = BTreeMap::from([
            ("next".to_owned(), next_value.to_owned()),
            ("prev".to_owned(), previous_value.to_owned()),
        ]);
        let suggestions = fix
            .and_then(|(fix_start, fix_end, replacement)| {
                Some(LintSuggestion {
                    message_id: message_id.to_owned(),
                    message: message.clone(),
                    fixes: std::iter::once(LintFix::replace_range(
                        TextRange::new(
                            u32::try_from(fix_start).ok()?,
                            u32::try_from(fix_end).ok()?,
                        ),
                        replacement,
                    ))
                    .collect(),
                })
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message,
            data,
            range: TextRange::new(start, end),
            suggestions,
        });
    }

    fn find_pair(
        &self,
        root_span: Span,
        items: &[Option<Span>],
        paren: ParenType,
        strategy: PairStrategy,
    ) -> Option<(TokenRef, TokenRef)> {
        if paren == ParenType::Angle {
            let start = usize::try_from(root_span.start).ok()?;
            let end = usize::try_from(root_span.end).ok()?;
            let left = TokenRef {
                start,
                end: start.checked_add(1)?,
                kind: TokenKind::Punctuator,
                original_index: None,
            };
            let right = TokenRef {
                start: end.checked_sub(1)?,
                end,
                kind: TokenKind::Punctuator,
                original_index: None,
            };
            return (left.text(self.source) == "<" && right.text(self.source) == ">")
                .then_some((left, right));
        }

        match strategy {
            PairStrategy::Exact => self.exact_pair(root_span, paren),
            PairStrategy::AfterAnchor(anchor) => {
                let left = self
                    .scan
                    .tokens()
                    .iter()
                    .enumerate()
                    .find(|(_, token)| {
                        token.start >= anchor as usize
                            && token.end <= root_span.end as usize
                            && !token.kind.is_comment()
                            && token.text(self.source) == paren.delimiters().0
                    })
                    .map(|(index, _)| self.token(index))?;
                let right = self.right_after_items(items, paren)?;
                Some((left, right))
            }
            PairStrategy::AroundItems => {
                let first = items.iter().flatten().next().copied()?;
                let last = items.iter().flatten().next_back().copied()?;
                let first_token = self.first_token(first)?;
                let left = self.previous_significant(first_token)?;
                let right = self.right_after_span(last, paren)?;
                let (left_delimiter, right_delimiter) = paren.delimiters();
                (left.text(self.source) == left_delimiter
                    && right.text(self.source) == right_delimiter)
                    .then_some((left, right))
            }
        }
    }

    fn exact_pair(&self, span: Span, paren: ParenType) -> Option<(TokenRef, TokenRef)> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let (left_text, right_text) = paren.delimiters();
        let left = self
            .scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, token)| token.start == start && token.text(self.source) == left_text)
            .map(|(index, _)| self.token(index))?;
        let right = self
            .scan
            .tokens()
            .iter()
            .enumerate()
            .rfind(|(_, token)| token.end == end && token.text(self.source) == right_text)
            .map(|(index, _)| self.token(index))?;
        Some((left, right))
    }

    fn right_after_items(&self, items: &[Option<Span>], paren: ParenType) -> Option<TokenRef> {
        let last = items.iter().flatten().next_back().copied()?;
        self.right_after_span(last, paren)
    }

    fn right_after_span(&self, span: Span, paren: ParenType) -> Option<TokenRef> {
        let last = self.last_token(span)?;
        let mut index = last.original_index?;
        loop {
            index = self.scan.next_significant(index)?;
            let token = self.token(index);
            if token.text(self.source) != "," {
                return (token.text(self.source) == paren.delimiters().1).then_some(token);
            }
        }
    }

    fn token(&self, index: usize) -> TokenRef {
        let token = self.scan.tokens()[index];
        TokenRef {
            start: token.start,
            end: token.end,
            kind: token.kind,
            original_index: Some(index),
        }
    }

    fn first_token(&self, span: Span) -> Option<TokenRef> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, token)| {
                !token.kind.is_comment() && token.start >= start && token.start < end
            })
            .map(|(index, _)| self.token(index))
    }

    fn last_token(&self, span: Span) -> Option<TokenRef> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .rfind(|(_, token)| {
                !token.kind.is_comment() && token.start >= start && token.end <= end
            })
            .map(|(index, _)| self.token(index))
    }

    fn immediate_after(&self, token: TokenRef) -> Option<TokenRef> {
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.start >= token.end)
            .map(|(index, _)| self.token(index))
    }

    fn immediate_before(&self, token: TokenRef) -> Option<TokenRef> {
        if token.original_index.is_none()
            && token.start > 0
            && self.source.as_bytes().get(token.start - 1) == Some(&b'>')
            && self
                .scan
                .tokens()
                .iter()
                .any(|candidate| candidate.start < token.start && candidate.end > token.start)
        {
            return Some(TokenRef {
                start: token.start - 1,
                end: token.start,
                kind: TokenKind::Punctuator,
                original_index: None,
            });
        }
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .rfind(|(_, candidate)| candidate.end <= token.start)
            .map(|(index, _)| self.token(index))
    }

    fn significant_after(&self, token: TokenRef) -> Option<TokenRef> {
        self.scan
            .tokens()
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.start >= token.end && !candidate.kind.is_comment())
            .map(|(index, _)| self.token(index))
    }

    fn significant_before(&self, token: TokenRef) -> Option<TokenRef> {
        let candidate = self.immediate_before(token)?;
        if !candidate.kind.is_comment() {
            return Some(candidate);
        }
        let index = candidate.original_index?;
        self.scan
            .prev_significant(index)
            .map(|index| self.token(index))
    }

    fn previous_significant(&self, token: TokenRef) -> Option<TokenRef> {
        let index = token.original_index?;
        self.scan
            .prev_significant(index)
            .map(|index| self.token(index))
    }

    fn token_before_item(&self, first: TokenRef, left: TokenRef) -> Option<TokenRef> {
        let mut previous = self.previous_significant(first)?;
        while previous.text(self.source) == "(" && !same_token(previous, left) {
            previous = self.previous_significant(previous)?;
        }
        Some(previous)
    }

    fn comments_exist_between(&self, previous: TokenRef, next: TokenRef) -> bool {
        self.scan.tokens().iter().any(|token| {
            token.kind.is_comment() && token.start >= previous.end && token.end <= next.start
        })
    }

    fn check_json_document(&mut self) {
        let Some((open, _)) = self.scan.tokens().iter().enumerate().find(|(_, token)| {
            !token.kind.is_comment() && matches!(token.text(self.source), "{" | "[")
        }) else {
            return;
        };
        self.check_json_container(open);
    }

    fn check_json_container(&mut self, open: usize) {
        let Some(close) = self.scan.partner(open) else {
            return;
        };
        let paren = match self.scan.token_text(open) {
            "{" => ParenType::Curly,
            "[" => ParenType::Square,
            _ => return,
        };
        let node_type = if paren == ParenType::Curly {
            "JSONObjectExpression"
        } else {
            "JSONArrayExpression"
        };
        let (items, children) = self.json_items(open, close, paren);
        if !items.is_empty() {
            let left = self.token(open);
            let right = self.token(close);
            let root_span = Span::new(left.start as u32, right.end as u32);
            self.check_pair(root_span, node_type, paren, &items, left, right);
        }
        for child in children {
            self.check_json_container(child);
        }
    }

    fn json_items(
        &self,
        open: usize,
        close: usize,
        paren: ParenType,
    ) -> (Vec<Option<Span>>, Vec<usize>) {
        let mut items = Vec::new();
        let mut children = Vec::new();
        let mut first: Option<usize> = None;
        let mut last: Option<usize> = None;
        let mut index = open + 1;
        while index < close {
            let token = &self.scan.tokens()[index];
            if token.kind.is_comment() {
                index += 1;
                continue;
            }
            let text = token.text(self.source);
            if matches!(text, "{" | "[")
                && let Some(partner) = self.scan.partner(index)
                && partner < close
            {
                first.get_or_insert(index);
                last = Some(partner);
                children.push(index);
                index = partner + 1;
                continue;
            }
            if text == "," {
                match (first, last) {
                    (Some(first), Some(last)) => items.push(Some(Span::new(
                        self.scan.tokens()[first].start as u32,
                        self.scan.tokens()[last].end as u32,
                    ))),
                    (None, None) if paren == ParenType::Square => items.push(None),
                    _ => {}
                }
                first = None;
                last = None;
                index += 1;
                continue;
            }
            first.get_or_insert(index);
            last = Some(index);
            index += 1;
        }
        if let (Some(first), Some(last)) = (first, last) {
            items.push(Some(Span::new(
                self.scan.tokens()[first].start as u32,
                self.scan.tokens()[last].end as u32,
            )));
        }
        (items, children)
    }
}

fn normalize_options(options: &Value) -> Options {
    let mut base = BaseConfig::default();
    let mut overrides = HashMap::new();
    overrides.insert(
        "{}".to_owned(),
        Override::Config(PartialConfig {
            spacing: Some(Spacing::Always),
            ..PartialConfig::default()
        }),
    );

    let Some(option) = first_option(options).and_then(Value::as_object) else {
        return Options { base, overrides };
    };
    parse_top_level(option).apply(&mut base);
    if let Some(Value::Object(raw_overrides)) = option.get("overrides") {
        for (key, value) in raw_overrides {
            let parsed = if value.as_str() == Some("off") {
                Override::Off
            } else {
                Override::Config(parse_override(value))
            };
            if key == "{}"
                && let (Some(Override::Config(existing)), Override::Config(additional)) =
                    (overrides.get_mut(key), parsed)
            {
                existing.merge(additional);
            } else {
                overrides.insert(key.clone(), parsed);
            }
        }
    }
    Options { base, overrides }
}

fn parse_top_level(object: &serde_json::Map<String, Value>) -> PartialConfig {
    PartialConfig {
        spacing: object
            .get("singleLine")
            .and_then(Value::as_object)
            .and_then(|single| single.get("spacing"))
            .map(|value| {
                if value.as_str() == Some("always") {
                    Spacing::Always
                } else {
                    Spacing::Never
                }
            }),
        max_items: object
            .get("singleLine")
            .and_then(Value::as_object)
            .and_then(|single| single.get("maxItems"))
            .map(normalize_count),
        max_items_present: object
            .get("singleLine")
            .and_then(Value::as_object)
            .is_some_and(|single| single.contains_key("maxItems")),
        min_items: object
            .get("multiLine")
            .and_then(Value::as_object)
            .and_then(|multi| multi.get("minItems"))
            .map(normalize_count),
    }
}

fn parse_override(value: &Value) -> PartialConfig {
    let Some(object) = value.as_object() else {
        return PartialConfig::default();
    };
    let single = object.get("singleLine").and_then(Value::as_object);
    let multiline = object.get("multiline").and_then(Value::as_object);
    PartialConfig {
        spacing: single
            .and_then(|single| single.get("spacing"))
            .map(|value| {
                if value.as_str() == Some("always") {
                    Spacing::Always
                } else {
                    Spacing::Never
                }
            }),
        max_items: single
            .and_then(|single| single.get("maxItems"))
            .map(normalize_count),
        max_items_present: single.is_some_and(|single| single.contains_key("maxItems")),
        min_items: multiline
            .and_then(|multi| multi.get("minItems"))
            .map(normalize_count),
    }
}

fn normalize_count(value: &Value) -> usize {
    value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .or_else(|| {
            value
                .as_f64()
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| value.floor().min(usize::MAX as f64) as usize)
        })
        .unwrap_or(0)
}

fn is_json_filename(filename: &str) -> bool {
    filename
        .rsplit_once('.')
        .is_some_and(|(_, extension)| matches!(extension, "json" | "jsonc" | "json5"))
}

fn token_value(token: TokenRef, source: &str) -> &str {
    let raw = token.text(source);
    match token.kind {
        TokenKind::LineComment => raw.get(2..).unwrap_or_default(),
        TokenKind::BlockComment => raw.get(2..raw.len().saturating_sub(2)).unwrap_or_default(),
        _ => raw,
    }
}

fn same_token(left: TokenRef, right: TokenRef) -> bool {
    left.start == right.start && left.end == right.end
}

fn same_line(source: &str, left_end: usize, right_start: usize) -> bool {
    !has_line_terminator(
        source
            .get(left_end.min(source.len())..right_start.min(source.len()))
            .unwrap_or_default(),
    )
}

fn has_line_terminator(text: &str) -> bool {
    text.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
        || text.contains(['\u{2028}', '\u{2029}'])
}

fn message_order(message_id: &str) -> u8 {
    match message_id {
        SHOULD_SPACING => 0,
        SHOULD_NOT_SPACING => 1,
        SHOULD_WRAP => 2,
        SHOULD_NOT_WRAP => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<Case>,
        invalid: Vec<Case>,
    }

    #[derive(Deserialize)]
    struct Generated {
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        fixable_invalid: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        code: String,
        #[serde(default)]
        options: Value,
        language: String,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
        #[serde(default)]
        authored_output: Option<String>,
        #[serde(default)]
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        data: BTreeMap<String, String>,
        range: [usize; 2],
        fix: Option<ExpectedFix>,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [usize; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/exp-list-style-v5.10.0.json"
        ))
        .expect("fixture parses")
    }

    fn filename(case: &Case) -> &'static str {
        if case.language == "json" {
            "fixture.json"
        } else {
            "fixture.ts"
        }
    }

    fn lint(source: &str, filename: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_exp_list_style(source, Some(filename), options, &mut diagnostics);
        diagnostics
    }

    fn utf16_to_byte(source: &str, target: usize) -> usize {
        let mut utf16 = 0usize;
        for (byte, character) in source.char_indices() {
            if utf16 >= target {
                return byte;
            }
            utf16 += character.len_utf16();
        }
        source.len()
    }

    fn fixes(diagnostics: &[LintDiagnostic]) -> Vec<LintFix> {
        diagnostics
            .iter()
            .flat_map(|diagnostic| diagnostic.suggestions.iter())
            .flat_map(|suggestion| suggestion.fixes.iter().cloned())
            .collect()
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = fixes(diagnostics);
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| (fix.range.start, fix.range.end));
        let mut output = String::new();
        let mut last = 0usize;
        for fix in fixes {
            let start = fix.range.start as usize;
            let end = fix.range.end as usize;
            if last > start {
                continue;
            }
            output.push_str(&source[last..start]);
            output.push_str(&fix.replacement_text);
            last = end;
        }
        output.push_str(&source[last..]);
        Some(output)
    }

    fn recursive_output(case: &Case) -> Option<String> {
        let mut output = case.code.clone();
        let mut changed = false;
        for _ in 0..100 {
            let diagnostics = lint(&output, filename(case), &case.options);
            let Some(next) = apply_fixes(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            assert_ne!(next, output, "non-progressing fix for {}", case.code);
            output = next;
            changed = true;
        }
        panic!("fixes did not converge for {}", case.code);
    }

    #[test]
    fn keeps_the_complete_authored_inventory() {
        let fixture = fixture();
        assert_eq!(fixture.generated.inventory.valid, 56);
        assert_eq!(fixture.generated.inventory.invalid, 56);
        assert_eq!(fixture.generated.inventory.diagnostics, 107);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 56);
    }

    #[test]
    fn accepts_every_authored_valid_case() {
        for case in fixture().valid {
            assert!(
                lint(&case.code, filename(&case), &case.options).is_empty(),
                "{}",
                case.code
            );
        }
    }

    #[test]
    fn replays_every_authored_invalid_diagnostic_fix_and_recursive_output() {
        for case in fixture().invalid {
            let diagnostics = lint(&case.code, filename(&case), &case.options);
            assert_eq!(
                diagnostics.len(),
                case.expected_diagnostics.len(),
                "{}",
                case.code
            );
            for (actual, expected) in diagnostics.iter().zip(&case.expected_diagnostics) {
                assert_eq!(actual.message_id, expected.message_id, "{}", case.code);
                assert_eq!(actual.message, expected.message, "{}", case.code);
                assert_eq!(actual.data, expected.data, "{}", case.code);
                assert_eq!(
                    actual.range,
                    TextRange::new(
                        utf16_to_byte(&case.code, expected.range[0]) as u32,
                        utf16_to_byte(&case.code, expected.range[1]) as u32,
                    ),
                    "{}",
                    case.code
                );
                match (&actual.suggestions[..], &expected.fix) {
                    ([], None) => {}
                    ([suggestion], Some(expected_fix)) => {
                        assert_eq!(suggestion.message_id, expected.message_id, "{}", case.code);
                        assert_eq!(suggestion.message, expected.message, "{}", case.code);
                        assert_eq!(suggestion.fixes.len(), 1, "{}", case.code);
                        let actual_fix = &suggestion.fixes[0];
                        assert_eq!(
                            actual_fix.range,
                            TextRange::new(
                                utf16_to_byte(&case.code, expected_fix.range[0]) as u32,
                                utf16_to_byte(&case.code, expected_fix.range[1]) as u32,
                            ),
                            "{}",
                            case.code
                        );
                        assert_eq!(
                            actual_fix.replacement_text, expected_fix.text,
                            "{}",
                            case.code
                        );
                    }
                    _ => panic!("fix mismatch for {}", case.code),
                }
            }
            assert_eq!(
                apply_fixes(&case.code, &diagnostics),
                case.authored_output,
                "{}",
                case.code
            );
            assert_eq!(recursive_output(&case), case.output, "{}", case.code);
        }
    }

    #[test]
    #[allow(clippy::disallowed_macros)]
    fn covers_js_ts_jsx_tsx_unicode_and_every_line_terminator() {
        let cases = [
            ("const π = [ 1 ];", "fixture.js"),
            ("const π: [number] = [ 1 ];", "fixture.ts"),
            ("const π = <Comp value={[ 1 ]} />;", "fixture.jsx"),
            (
                "const π: JSX.Element = <Comp value={[ 1 ]} />;",
                "fixture.tsx",
            ),
        ];
        for (source, filename) in cases {
            let diagnostics = lint(source, filename, &Value::Array(Vec::new()));
            assert_eq!(
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.message_id == SHOULD_NOT_SPACING)
                    .count(),
                2,
                "{filename}: {source}"
            );
        }

        let options = serde_json::json!([{ "singleLine": { "maxItems": 1 } }]);
        for separator in ["\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
            let source = format!("const π = [1,{separator}2];");
            let diagnostics = lint(&source, "fixture.js", &options);
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.message_id == SHOULD_NOT_WRAP),
                "{source:?}"
            );
        }
    }

    #[test]
    #[allow(clippy::disallowed_macros)]
    fn handles_comments_nested_generics_overrides_and_malformed_input_safely() {
        let comment = "foo(a,\n// comment\nb)";
        let diagnostics = lint(comment, "fixture.js", &Value::Array(Vec::new()));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message_id, SHOULD_NOT_WRAP);
        assert!(diagnostics[0].suggestions.is_empty());

        let nested = "const value = foo< Map<string, Set<number>> >(1);";
        let diagnostics = lint(nested, "fixture.ts", &Value::Array(Vec::new()));
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message_id == SHOULD_NOT_SPACING)
                .count(),
            2
        );

        let overrides = serde_json::json!([{
            "singleLine": { "spacing": "always" },
            "overrides": {
                "[]": "off",
                "ArrayExpression": { "singleLine": { "spacing": "never" } },
                "{}": { "multiline": { "minItems": 3 } }
            }
        }]);
        let diagnostics = lint(
            "const a = [ 1 ]; const b = { x: 1 };",
            "fixture.ts",
            &overrides,
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [SHOULD_NOT_SPACING, SHOULD_NOT_SPACING]
        );

        for malformed in ["const x = [", "foo<Bar(", "\u{1f600}", "/* unterminated"] {
            assert!(
                lint(malformed, "fixture.tsx", &serde_json::json!([null])).is_empty(),
                "{malformed:?}"
            );
        }
    }
}
