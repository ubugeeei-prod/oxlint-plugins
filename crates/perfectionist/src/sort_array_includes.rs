//! Oxc AST port of Perfectionist's `sort-array-includes` v5.10.0 contract.

#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "serde_json option maps require String keys, AST element inventories are user-sized, and numeric literal names must match JavaScript String(value) semantics."
)]

use oxc_ast::{
    Comment,
    ast::{Argument, ArrayExpressionElement, CallExpression, Expression, NewExpression, Program},
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::{Map, Value};

use crate::{
    sort_named_specifiers::{
        RuleContract, RuleOptions, SortableNode, check_specifiers, compute_group, is_rule_disabled,
        is_same_line, matches_regex, movable_leading_comment_start,
    },
    types::{LineIndex, RuleDiagnostic},
};

const CONTRACT: RuleContract = RuleContract {
    rule: "sort-array-includes",
    selector: "literal",
    order_message_id: "unexpectedArrayIncludesOrder",
    group_order_message_id: "unexpectedArrayIncludesGroupOrder",
    extra_spacing_message_id: "extraSpacingBetweenArrayIncludesMembers",
    missed_spacing_message_id: "missedSpacingBetweenArrayIncludesMembers",
    missed_comment_above_message_id: None,
};

pub(crate) fn check<'ast>(
    source_text: &'ast str,
    program: &Program<'ast>,
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    check_with_target(
        source_text,
        program,
        comments,
        raw_options,
        CONTRACT,
        ArrayRuleTarget::Includes,
    )
}

pub(crate) fn check_with_target<'ast>(
    source_text: &'ast str,
    program: &Program<'ast>,
    comments: &[Comment],
    raw_options: &Value,
    contract: RuleContract,
    target: ArrayRuleTarget,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let mut visitor = ArrayRuleVisitor {
        source_text,
        comments,
        raw_options,
        contract,
        target,
        lines: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
    };
    visitor.visit_program(program);
    visitor
        .diagnostics
        .sort_by_key(|diagnostic| (diagnostic.loc.start_line, diagnostic.loc.start_column));
    visitor.diagnostics
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ArrayRuleTarget {
    Includes,
    Sets,
}

struct ArrayRuleVisitor<'source, 'options> {
    source_text: &'source str,
    comments: &'source [Comment],
    raw_options: &'options Value,
    contract: RuleContract,
    target: ArrayRuleTarget,
    lines: LineIndex,
    diagnostics: SmallVec<[RuleDiagnostic; 8]>,
}

impl<'ast> Visit<'ast> for ArrayRuleVisitor<'ast, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        if self.target == ArrayRuleTarget::Includes {
            self.check_call(call);
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, expression: &NewExpression<'ast>) {
        if self.target == ArrayRuleTarget::Sets {
            self.check_set(expression);
        }
        walk::walk_new_expression(self, expression);
    }
}

impl<'ast> ArrayRuleVisitor<'ast, '_> {
    fn check_call(&mut self, call: &CallExpression<'ast>) {
        let Expression::StaticMemberExpression(member) = call.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "includes" {
            return;
        }

        match member.object.get_inner_expression() {
            Expression::ArrayExpression(array) => self.check_array(
                "ArrayExpression",
                array.span.end.saturating_sub(1),
                array
                    .elements
                    .iter()
                    .map(|element| match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            ArrayItem::Barrier(spread.span)
                        }
                        ArrayExpressionElement::Elision(elision) => {
                            ArrayItem::Barrier(elision.span)
                        }
                        element => element
                            .as_expression()
                            .map_or(ArrayItem::Barrier(element.span()), ArrayItem::Expression),
                    })
                    .collect(),
            ),
            Expression::NewExpression(expression)
                if matches!(
                    expression.callee.get_inner_expression(),
                    Expression::Identifier(identifier) if identifier.name == "Array"
                ) =>
            {
                self.check_array(
                    "NewExpression",
                    expression.span.end.saturating_sub(1),
                    expression
                        .arguments
                        .iter()
                        .map(|argument| match argument {
                            Argument::SpreadElement(spread) => ArrayItem::Barrier(spread.span),
                            argument => argument
                                .as_expression()
                                .map_or(ArrayItem::Barrier(argument.span()), ArrayItem::Expression),
                        })
                        .collect(),
                );
            }
            _ => {}
        }
    }

    fn check_set(&mut self, expression: &NewExpression<'ast>) {
        if !matches!(
            expression.callee.get_inner_expression(),
            Expression::Identifier(identifier) if identifier.name == "Set"
        ) {
            return;
        }
        let Some(argument) = expression
            .arguments
            .first()
            .and_then(Argument::as_expression)
        else {
            return;
        };

        match argument.get_inner_expression() {
            Expression::ArrayExpression(array) => self.check_array(
                "ArrayExpression",
                array.span.end.saturating_sub(1),
                array
                    .elements
                    .iter()
                    .map(|element| match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            ArrayItem::Barrier(spread.span)
                        }
                        ArrayExpressionElement::Elision(elision) => {
                            ArrayItem::Barrier(elision.span)
                        }
                        element => element
                            .as_expression()
                            .map_or(ArrayItem::Barrier(element.span()), ArrayItem::Expression),
                    })
                    .collect(),
            ),
            Expression::NewExpression(array)
                if matches!(
                    array.callee.get_inner_expression(),
                    Expression::Identifier(identifier) if identifier.name == "Array"
                ) =>
            {
                self.check_array(
                    "NewExpression",
                    array.span.end.saturating_sub(1),
                    array
                        .arguments
                        .iter()
                        .map(|argument| match argument {
                            Argument::SpreadElement(spread) => ArrayItem::Barrier(spread.span),
                            argument => argument
                                .as_expression()
                                .map_or(ArrayItem::Barrier(argument.span()), ArrayItem::Expression),
                        })
                        .collect(),
                );
            }
            _ => {}
        }
    }

    fn check_array(&mut self, ast_type: &str, container_end: u32, items: Vec<ArrayItem<'_, 'ast>>) {
        let names: SmallVec<[CompactString; 16]> = items
            .iter()
            .filter_map(|item| match item {
                ArrayItem::Expression(expression) => {
                    Some(expression_name(self.source_text, expression))
                }
                ArrayItem::Barrier(_) => None,
            })
            .collect();
        let selected = select_options(self.raw_options, ast_type, &names);
        let options = RuleOptions::from_object(with_defaults(selected));
        let mut segment: SmallVec<[(&Expression<'ast>, u32); 16]> = SmallVec::new();

        for (index, item) in items.iter().enumerate() {
            match item {
                ArrayItem::Expression(expression) => {
                    let boundary = items
                        .get(index + 1)
                        .map_or(container_end, |next| next.span().start);
                    segment.push((expression, boundary));
                }
                ArrayItem::Barrier(_) => {
                    self.check_segment(&options, &segment);
                    segment.clear();
                }
            }
        }
        self.check_segment(&options, &segment);
    }

    fn check_segment(&mut self, options: &RuleOptions, segment: &[(&Expression<'ast>, u32)]) {
        if segment.len() < 2 {
            return;
        }
        let mut nodes: SmallVec<[SortableNode<'ast>; 16]> = segment
            .iter()
            .filter_map(|(expression, boundary)| self.sortable_node(options, expression, *boundary))
            .collect();
        if nodes.len() < 2 {
            return;
        }
        check_specifiers(
            self.source_text,
            self.comments,
            options,
            &mut nodes,
            self.contract,
            &self.lines,
            &mut self.diagnostics,
        );
    }

    fn sortable_node(
        &self,
        options: &RuleOptions,
        expression: &Expression<'ast>,
        boundary: u32,
    ) -> Option<SortableNode<'ast>> {
        let span = expression.span();
        let source_start =
            movable_leading_comment_start(self.source_text, self.comments, span, options);
        let source_end = self
            .comments
            .iter()
            .filter(|comment| {
                comment.span.start >= span.end
                    && comment.span.end <= boundary
                    && is_same_line(self.source_text, span.end, comment.span.start)
            })
            .map(|comment| comment.span.end)
            .max()
            .unwrap_or(span.end);
        let source = source_for_span(self.source_text, Span::new(source_start, source_end))?;
        let node_source = source_for_span(self.source_text, span)?;
        let name = expression_name(self.source_text, expression);
        let group = compute_group(options, name.as_str(), &[], self.contract.selector);
        let group_index = options.group_index(group.as_str());
        Some(SortableNode {
            span,
            compare_name: name.clone(),
            name,
            source,
            source_start,
            source_end,
            size: node_source.encode_utf16().count(),
            group,
            group_index,
            partition_id: 0,
            is_disabled: is_rule_disabled(
                self.source_text,
                self.comments,
                span,
                self.contract.rule,
            ),
            is_ignored: false,
            preserve_order_in_group: false,
            is_type_import: false,
            dependencies: SmallVec::new(),
            dependency_names: SmallVec::new(),
            add_safety_semicolon_when_inline: false,
            use_original_groups_for_spacing: false,
            requires_comma_separator: true,
        })
    }
}

#[derive(Clone, Copy)]
enum ArrayItem<'node, 'ast> {
    Expression(&'node Expression<'ast>),
    Barrier(Span),
}

impl ArrayItem<'_, '_> {
    fn span(self) -> Span {
        match self {
            Self::Expression(expression) => expression.span(),
            Self::Barrier(span) => span,
        }
    }
}

fn expression_name(source_text: &str, expression: &Expression<'_>) -> CompactString {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => CompactString::from(literal.value.as_str()),
        Expression::NumericLiteral(literal) => CompactString::from(literal.value.to_string()),
        Expression::BooleanLiteral(literal) => {
            CompactString::from(if literal.value { "true" } else { "false" })
        }
        Expression::NullLiteral(_) => CompactString::from("null"),
        expression => source_for_span(source_text, expression.span())
            .map_or_else(|| CompactString::new(""), CompactString::from),
    }
}

fn select_options(
    raw_options: &Value,
    ast_type: &str,
    names: &[CompactString],
) -> Map<String, Value> {
    let candidates: SmallVec<[&Map<String, Value>; 8]> = match raw_options {
        Value::Array(values) => values.iter().filter_map(Value::as_object).collect(),
        Value::Object(object) => SmallVec::from_vec(vec![object]),
        _ => SmallVec::new(),
    };
    for candidate in candidates {
        let Some(condition) = candidate
            .get("useConfigurationIf")
            .and_then(Value::as_object)
        else {
            return candidate.clone();
        };
        if let Some(pattern) = condition.get("allNamesMatchPattern")
            && !names
                .iter()
                .all(|name| matches_regex(name.as_str(), pattern))
        {
            continue;
        }
        if let Some(selector) = condition.get("matchesAstSelector").and_then(Value::as_str)
            && !matches_ast_selector(selector, ast_type)
        {
            continue;
        }
        return candidate.clone();
    }
    Map::new()
}

fn matches_ast_selector(selector: &str, ast_type: &str) -> bool {
    let selector = selector.trim();
    selector == ast_type || selector == format!("* > {ast_type}")
}

fn with_defaults(mut selected: Map<String, Value>) -> Map<String, Value> {
    let defaults = serde_json::json!({
        "fallbackSort": { "type": "unsorted" },
        "newlinesInside": "newlinesBetween",
        "specialCharacters": "keep",
        "partitionByComment": false,
        "partitionByNewLine": false,
        "newlinesBetween": "ignore",
        "useConfigurationIf": {},
        "type": "alphabetical",
        "groups": ["literal"],
        "ignoreCase": true,
        "locales": "en-US",
        "customGroups": [],
        "alphabet": "",
        "order": "asc"
    });
    if let Some(defaults) = defaults.as_object() {
        for (key, value) in defaults {
            selected.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    selected
}

fn source_for_span(source_text: &str, span: Span) -> Option<&str> {
    source_text.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)
}
