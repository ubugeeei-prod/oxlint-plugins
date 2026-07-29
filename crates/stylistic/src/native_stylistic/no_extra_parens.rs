//! Native implementation of stable `@stylistic/no-extra-parens`.
//!
//! Oxc preserves grouping nodes for standard grammar, which gives us the exact
//! pair and semantic expression behind each parenthesis. To decide whether a
//! pair is actually redundant, the rule removes that one pair, reparses with
//! grouping nodes omitted, and compares the resulting AST by content. A narrow
//! token fallback covers legacy forms that Oxc normalizes away. Parser-backed
//! equivalence keeps precedence, associativity, restricted productions,
//! `new`/call/member binding, JSX, and TypeScript type grammar parser-owned.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{BinaryOperator, Expression, TSType},
};
use oxc_ast_visit::Visit;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::{ContentEq, GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::{ParenUse, Scan};

const RULE: &str = "no-extra-parens";
const MESSAGE_ID: &str = "unexpected";
const MESSAGE: &str = "Unnecessary parentheses around expression.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InnerKind {
    ArrowFunction,
    Function,
    Sequence,
    Assignment,
    Binary,
    Logical,
    Conditional,
    Call,
    New,
    Await,
    Chain,
    TypeAssertion,
    Class,
    Object,
    Jsx,
    String,
    RegExp,
    Number,
    Other,
    Type,
    TypeBinary,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    wrapper: Span,
    unfixable_directive: bool,
    force_report: bool,
}

#[derive(Clone, Debug)]
struct Options {
    functions_only: bool,
    ignore_sequence: bool,
    ignore_jsx: Option<JsxMode>,
    conditional_assign: bool,
    ternary_binary: bool,
    nested_binary: bool,
    return_assign: bool,
    arrow_conditionals: bool,
    new_in_member: bool,
    function_prototype_methods: bool,
    nested_conditionals: bool,
    allow_spread_conditional: bool,
    allow_spread_logical: bool,
    allow_spread_await: bool,
    allow_comment_pattern: Option<String>,
    ignored_nodes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JsxMode {
    All,
    SingleLine,
    MultiLine,
}

impl Options {
    fn from_json(value: &Value) -> Self {
        let items = value.as_array();
        let mode = items
            .and_then(|values| values.first())
            .and_then(Value::as_str);
        let object = items
            .and_then(|values| values.get(1))
            .and_then(Value::as_object);
        let ignore_jsx = object
            .and_then(|option| option.get("ignoreJSX"))
            .and_then(Value::as_str)
            .and_then(|value| match value {
                "all" => Some(JsxMode::All),
                "single-line" => Some(JsxMode::SingleLine),
                "multi-line" => Some(JsxMode::MultiLine),
                _ => None,
            });

        Self {
            functions_only: mode == Some("functions"),
            ignore_sequence: object
                .and_then(|option| option.get("enforceForSequenceExpressions"))
                .and_then(Value::as_bool)
                == Some(false),
            ignore_jsx,
            conditional_assign: option_bool(object, "conditionalAssign", true),
            ternary_binary: option_bool(object, "ternaryOperandBinaryExpressions", true),
            nested_binary: option_bool(object, "nestedBinaryExpressions", true),
            return_assign: option_bool(object, "returnAssign", true),
            arrow_conditionals: option_bool(object, "enforceForArrowConditionals", true),
            new_in_member: option_bool(object, "enforceForNewInMemberExpressions", true),
            function_prototype_methods: option_bool(
                object,
                "enforceForFunctionPrototypeMethods",
                true,
            ),
            nested_conditionals: option_bool(object, "nestedConditionalExpressions", true),
            allow_spread_conditional: spread_option(object, "ConditionalExpression"),
            allow_spread_logical: spread_option(object, "LogicalExpression"),
            allow_spread_await: spread_option(object, "AwaitExpression"),
            allow_comment_pattern: object
                .and_then(|option| option.get("allowParensAfterCommentPattern"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            ignored_nodes: object
                .and_then(|option| option.get("ignoredNodes"))
                .and_then(Value::as_array)
                .map(|selectors| {
                    selectors
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

fn option_bool(object: Option<&serde_json::Map<String, Value>>, name: &str, default: bool) -> bool {
    object
        .and_then(|option| option.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn spread_option(object: Option<&serde_json::Map<String, Value>>, name: &str) -> bool {
    object
        .and_then(|option| option.get("allowNodesInSpreadElement"))
        .and_then(Value::as_object)
        .and_then(|spread| spread.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn check_no_extra_parens(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let normalized = Options::from_json(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, &normalized, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::ts(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, &normalized, diagnostics) {
                break;
            }
        }
    }

    diagnostics[first_diagnostic..].sort_by_key(|diagnostic| {
        (
            diagnostic.range.start,
            diagnostic.range.end,
            diagnostic.message_id.clone(),
        )
    });
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    options: &Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let preserving_allocator = Allocator::default();
    let preserving = Parser::new(&preserving_allocator, source, source_type).parse();

    let mut collector = CandidateCollector {
        source,
        options,
        parents: Vec::new(),
        candidates: Vec::new(),
        observed_wrappers: Vec::new(),
    };
    collector.visit_program(&preserving.program);

    let base_allocator = Allocator::default();
    let base = Parser::new(&base_allocator, source, source_type)
        .with_options(ParseOptions {
            preserve_parens: false,
            ..ParseOptions::default()
        })
        .parse();

    for candidate in collector.candidates {
        let replacements = [("", ""), (" ", ""), ("", " "), (" ", " ")];
        let replacement = replacements.into_iter().find(|(left, right)| {
            equivalent_without_pair(
                source,
                source_type,
                &base.program,
                candidate.wrapper,
                left,
                right,
            )
        });

        if replacement.is_none() && !candidate.force_report {
            continue;
        }

        let (left_replacement, right_replacement) = replacement.unwrap_or(("", ""));
        report(
            candidate,
            left_replacement,
            right_replacement,
            !candidate.unfixable_directive,
            diagnostics,
        );
    }
    check_unrepresented_parentheses(
        source,
        source_type,
        options,
        &collector.observed_wrappers,
        diagnostics,
    );
    true
}

fn check_unrepresented_parentheses(
    source: &str,
    source_type: SourceType,
    options: &Options,
    observed_wrappers: &[Span],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    // Oxc intentionally normalizes a few legacy-but-accepted forms, including
    // parenthesized assignment targets and update operands. Recover only those
    // missing grouping pairs from the shared token scan, then validate removal
    // against the parser before reporting.
    if options.functions_only || source_type.is_typescript() {
        return;
    }

    let allocator = Allocator::default();
    let base = Parser::new(&allocator, source, source_type)
        .with_options(ParseOptions {
            preserve_parens: false,
            ..ParseOptions::default()
        })
        .parse();
    let scan = Scan::new(source);
    let existing_starts = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.range.start)
        .collect::<Vec<_>>();
    let mut candidates = scan
        .tokens()
        .iter()
        .enumerate()
        .filter_map(|(open_index, token)| {
            if scan.token_text(open_index) != "("
                || scan.paren_use(open_index) != ParenUse::Grouping
            {
                return None;
            }
            let close_index = scan.partner(open_index)?;
            let close = &scan.tokens()[close_index];
            if scan
                .next_significant(close_index)
                .is_some_and(|next| scan.token_text(next) == "=>")
                || scan
                    .prev_significant(open_index)
                    .is_some_and(|previous| scan.token_text(previous) == "import")
                || is_anonymous_name_inference_assignment(&scan, open_index, close_index)
            {
                return None;
            }
            let wrapper = Span::new(
                u32::try_from(token.start).ok()?,
                u32::try_from(close.end).ok()?,
            );
            let inner_text = scan.slice(token.end, close.start);
            let force_function = is_redundant_function_group(&scan, open_index, close_index);
            let force_double_iife = inner_text.trim_start().starts_with("(function")
                && inner_text.trim_end().ends_with(")()");
            let force_standalone_let = inner_text.trim() == "let"
                && scan.prev_significant(open_index).is_none()
                && scan.next_significant(close_index).is_none();
            let reconsider_in = inner_text.contains(" in ")
                && is_safely_enclosed_in_for_initializer(&scan, open_index);
            if existing_starts.contains(&wrapper.start)
                || existing_starts.iter().any(|existing| {
                    *existing < wrapper.start
                        && !has_non_paren_prefix(source, *existing, wrapper.start)
                })
                || (observed_wrappers.contains(&wrapper)
                    && !reconsider_in
                    && !force_function
                    && !force_standalone_let
                    && !force_double_iife)
            {
                return None;
            }
            let replacement = [("", ""), (" ", ""), ("", " "), (" ", " ")]
                .into_iter()
                .find(|(left, right)| {
                    compatible_parse_error_without_pair(
                        source,
                        source_type,
                        &base,
                        wrapper,
                        left,
                        right,
                    )
                });
            let replacement = replacement.or_else(|| {
                (force_function || force_standalone_let || force_double_iife || reconsider_in)
                    .then_some(("", ""))
            })?;
            Some((wrapper, replacement))
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|(span, _)| (span.size(), span.start));
    let mut accepted = Vec::<Span>::new();
    for (wrapper, (left, right)) in candidates {
        if accepted.iter().any(|accepted| {
            wrapper.contains_inclusive(*accepted) || accepted.contains_inclusive(wrapper)
        }) {
            continue;
        }
        accepted.push(wrapper);
        diagnostics.retain(|diagnostic| {
            if diagnostic.range.start <= wrapper.start || diagnostic.range.start >= wrapper.end {
                return true;
            }
            has_non_paren_prefix(source, wrapper.start, diagnostic.range.start)
        });
        report(
            Candidate {
                wrapper,
                unfixable_directive: false,
                force_report: true,
            },
            left,
            right,
            true,
            diagnostics,
        );
    }
}

fn has_non_paren_prefix(source: &str, outer: u32, inner: u32) -> bool {
    !source
        .get(outer as usize..inner as usize)
        .is_some_and(|prefix| {
            prefix
                .chars()
                .all(|character| character.is_whitespace() || character == '(')
        })
}

fn is_safely_enclosed_in_for_initializer(scan: &Scan<'_>, open_index: usize) -> bool {
    let mut safely_enclosed = false;
    let mut inside_for_initializer = false;
    for index in (0..open_index).rev() {
        let Some(close) = scan.partner(index) else {
            continue;
        };
        if close <= open_index {
            continue;
        }
        match scan.token_text(index) {
            "{" | "[" => safely_enclosed = true,
            "(" => match scan.paren_use(index) {
                ParenUse::Call | ParenUse::FuncDef => safely_enclosed = true,
                ParenUse::Control => {
                    inside_for_initializer = scan
                        .prev_significant(index)
                        .is_some_and(|previous| scan.token_text(previous) == "for");
                    break;
                }
                ParenUse::Grouping => {
                    if scan
                        .next_significant(close)
                        .is_some_and(|next| scan.token_text(next) == "=>")
                    {
                        safely_enclosed = true;
                    }
                }
            },
            _ => {}
        }
    }
    if !inside_for_initializer {
        return false;
    }
    if safely_enclosed {
        return true;
    }
    let has_conditional_before = (0..open_index)
        .rev()
        .filter(|index| !scan.tokens()[*index].kind.is_comment())
        .take_while(|index| !matches!(scan.token_text(*index), ";" | "{"))
        .any(|index| scan.token_text(index) == "?");
    let has_conditional_after = (open_index + 1..scan.tokens().len())
        .filter(|index| !scan.tokens()[*index].kind.is_comment())
        .take_while(|index| scan.token_text(*index) != ";")
        .any(|index| scan.token_text(index) == ":");
    has_conditional_before && has_conditional_after
}

fn is_redundant_function_group(scan: &Scan<'_>, open_index: usize, close_index: usize) -> bool {
    let Some(first) = scan.next_significant(open_index) else {
        return false;
    };
    if scan.token_text(first) != "function" {
        return false;
    }
    if scan
        .prev_significant(open_index)
        .is_some_and(|previous| scan.token_text(previous) == "(")
    {
        return false;
    }
    let inner = scan
        .slice(scan.tokens()[first].start, scan.tokens()[close_index].start)
        .trim();
    if inner.ends_with("}()")
        || inner.contains("}.call(")
        || inner.contains("}.apply(")
        || inner.contains("}['call'](")
        || inner.contains("}[\"call\"](")
        || inner.contains("}[`call`](")
        || inner.contains("}['apply'](")
        || inner.contains("}[\"apply\"](")
        || inner.contains("}[`apply`](")
        || inner.contains("}?.")
    {
        return false;
    }
    if scan
        .next_significant(close_index)
        .is_some_and(|next| scan.token_text(next) == "(")
    {
        return false;
    }
    if scan
        .prev_significant(open_index)
        .is_none_or(|previous| scan.token_text(previous) != "new")
    {
        let suffix = scan
            .slice(scan.tokens()[close_index].end, scan.source().len())
            .trim_start();
        if suffix.starts_with(".call(")
            || suffix.starts_with(".apply(")
            || suffix.starts_with("['call'](")
            || suffix.starts_with("[\"call\"](")
            || suffix.starts_with("[`call`](")
            || suffix.starts_with("['apply'](")
            || suffix.starts_with("[\"apply\"](")
            || suffix.starts_with("[`apply`](")
            || suffix.starts_with("?.")
        {
            return false;
        }
    }
    scan.prev_significant(open_index).is_some_and(|previous| {
        matches!(
            scan.token_text(previous),
            "=" | "new" | "," | ":" | "?" | "||" | "&&" | "+" | "-" | "*" | "/" | "[" | "("
        )
    })
}

fn is_anonymous_name_inference_assignment(
    scan: &Scan<'_>,
    open_index: usize,
    close_index: usize,
) -> bool {
    let Some(inner) = scan.next_significant(open_index) else {
        return false;
    };
    if scan.next_significant(inner) != Some(close_index) {
        return false;
    }
    let Some(operator_index) = scan.next_significant(close_index) else {
        return false;
    };
    if !matches!(scan.token_text(operator_index), "=" | "&&=" | "||=" | "??=") {
        return false;
    }
    let Some(rhs_index) = scan.next_significant(operator_index) else {
        return false;
    };
    let rhs = scan.slice(scan.tokens()[rhs_index].start, scan.source().len());
    let trimmed = rhs.trim_start();
    let anonymous_function = trimmed.starts_with("function")
        && trimmed["function".len()..]
            .trim_start_matches('*')
            .trim_start()
            .starts_with('(');
    let anonymous_class = trimmed.starts_with("class")
        && (matches!(
            trimmed["class".len()..].trim_start().chars().next(),
            Some('{')
        ) || trimmed["class".len()..].trim_start().starts_with("extends"));
    let arrow = trimmed
        .find("=>")
        .is_some_and(|arrow| !trimmed[..arrow].contains([';', '=']));
    anonymous_function || anonymous_class || arrow
}

fn compatible_parse_error_without_pair(
    source: &str,
    source_type: SourceType,
    base: &oxc_parser::ParserReturn<'_>,
    wrapper: Span,
    left_replacement: &str,
    right_replacement: &str,
) -> bool {
    let open = wrapper.start as usize;
    let close = wrapper.end.saturating_sub(1) as usize;
    let mut candidate = String::with_capacity(source.len());
    candidate.push_str(&source[..open]);
    candidate.push_str(left_replacement);
    candidate.push_str(&source[open + 1..close]);
    candidate.push_str(right_replacement);
    candidate.push_str(&source[close + 1..]);

    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &candidate, source_type)
        .with_options(ParseOptions {
            preserve_parens: false,
            ..ParseOptions::default()
        })
        .parse();
    parsed.errors.len() < base.errors.len()
        || (parsed.errors.len() == base.errors.len() && base.program.content_eq(&parsed.program))
}

fn equivalent_without_pair(
    source: &str,
    source_type: SourceType,
    base: &oxc_ast::ast::Program<'_>,
    wrapper: Span,
    left_replacement: &str,
    right_replacement: &str,
) -> bool {
    let Some(open) = usize::try_from(wrapper.start).ok() else {
        return false;
    };
    let Some(end) = usize::try_from(wrapper.end).ok() else {
        return false;
    };
    let Some(close) = end.checked_sub(1) else {
        return false;
    };
    if source.as_bytes().get(open) != Some(&b'(')
        || source.as_bytes().get(close) != Some(&b')')
        || open >= close
    {
        return false;
    }

    let mut candidate = String::with_capacity(
        source
            .len()
            .saturating_sub(2)
            .saturating_add(left_replacement.len())
            .saturating_add(right_replacement.len()),
    );
    candidate.push_str(&source[..open]);
    candidate.push_str(left_replacement);
    candidate.push_str(&source[open + 1..close]);
    candidate.push_str(right_replacement);
    candidate.push_str(&source[close + 1..]);

    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &candidate, source_type)
        .with_options(ParseOptions {
            preserve_parens: false,
            ..ParseOptions::default()
        })
        .parse();
    parsed.errors.is_empty() && base.content_eq(&parsed.program)
}

struct CandidateCollector<'ast, 'source> {
    source: &'source str,
    options: &'source Options,
    parents: Vec<AstKind<'ast>>,
    candidates: Vec<Candidate>,
    observed_wrappers: Vec<Span>,
}

impl<'ast> Visit<'ast> for CandidateCollector<'ast, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        match kind {
            AstKind::ParenthesizedExpression(parenthesized) => {
                self.observed_wrappers.push(parenthesized.span);
            }
            AstKind::TSParenthesizedType(parenthesized) => {
                self.observed_wrappers.push(parenthesized.span);
            }
            _ => {}
        }
        match kind {
            AstKind::ParenthesizedExpression(parenthesized)
                if !matches!(
                    parenthesized.expression,
                    Expression::ParenthesizedExpression(_)
                ) =>
            {
                let inner_kind = expression_kind(&parenthesized.expression);
                let chain_depth = 1 + self
                    .parents
                    .iter()
                    .rev()
                    .take_while(|parent| matches!(parent, AstKind::ParenthesizedExpression(_)))
                    .count();
                let context_span = self.context_span(parenthesized.span);
                let report_wrapper = std::iter::once(parenthesized.span)
                    .chain(self.parents.iter().rev().filter_map(|parent| match parent {
                        AstKind::ParenthesizedExpression(parenthesized) => Some(parenthesized.span),
                        _ => None,
                    }))
                    .take(chain_depth)
                    .find(|wrapper| !self.allowed_by_comment(wrapper.start as usize));
                let Some(report_wrapper) = report_wrapper else {
                    self.parents.push(kind);
                    return;
                };
                if self.rule_applies(inner_kind, parenthesized.expression.span(), chain_depth)
                    && !self.is_decimal_member_exception(
                        inner_kind,
                        context_span,
                        parenthesized.expression.span(),
                    )
                    && !self.is_context_exception(
                        &parenthesized.expression,
                        inner_kind,
                        context_span,
                        chain_depth,
                    )
                    && !self.should_buffer_for_in_expression(
                        &parenthesized.expression,
                        inner_kind,
                        chain_depth,
                    )
                {
                    let unfixable_directive = inner_kind == InnerKind::String
                        && chain_depth == 1
                        && self.is_top_level_expression_statement(context_span);
                    let force_report = unfixable_directive
                        || self.must_force_report(
                            &parenthesized.expression,
                            inner_kind,
                            context_span,
                            chain_depth,
                        );
                    self.candidates.push(Candidate {
                        wrapper: report_wrapper,
                        unfixable_directive,
                        force_report,
                    });
                }
            }
            AstKind::TSParenthesizedType(parenthesized)
                if !matches!(
                    parenthesized.type_annotation,
                    TSType::TSParenthesizedType(_)
                ) =>
            {
                let chain_depth = 1 + self
                    .parents
                    .iter()
                    .rev()
                    .take_while(|parent| matches!(parent, AstKind::TSParenthesizedType(_)))
                    .count();
                if !self.options.functions_only {
                    let context_span = self.context_span(parenthesized.span);
                    let inner_kind = type_kind(&parenthesized.type_annotation);
                    if self.is_type_context_exception(inner_kind, context_span, chain_depth) {
                        self.parents.push(kind);
                        return;
                    }
                    self.candidates.push(Candidate {
                        wrapper: parenthesized.span,
                        unfixable_directive: false,
                        force_report: false,
                    });
                }
            }
            _ => {}
        }
        self.parents.push(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'ast>) {
        self.parents.pop();
    }
}

impl CandidateCollector<'_, '_> {
    fn context_span(&self, wrapper: Span) -> Span {
        self.parents
            .iter()
            .rev()
            .take_while(|parent| {
                matches!(
                    parent,
                    AstKind::ParenthesizedExpression(_) | AstKind::TSParenthesizedType(_)
                )
            })
            .last()
            .map_or(wrapper, GetSpan::span)
    }

    fn rule_applies(&self, kind: InnerKind, inner: Span, _chain_depth: usize) -> bool {
        if self.options.functions_only {
            return matches!(kind, InnerKind::ArrowFunction | InnerKind::Function);
        }
        if self.options.ignore_sequence && kind == InnerKind::Sequence {
            return false;
        }
        if kind == InnerKind::Jsx
            && let Some(mode) = self.options.ignore_jsx
        {
            let multiline = self
                .source
                .get(inner.start as usize..inner.end as usize)
                .is_some_and(contains_line_terminator);
            return match mode {
                JsxMode::All => false,
                JsxMode::SingleLine => multiline,
                JsxMode::MultiLine => !multiline,
            };
        }
        true
    }

    fn is_decimal_member_exception(&self, kind: InnerKind, wrapper: Span, inner: Span) -> bool {
        if kind != InnerKind::Number {
            return false;
        }
        let Some(AstKind::StaticMemberExpression(member)) = self
            .parents
            .iter()
            .rev()
            .find(|parent| !matches!(parent, AstKind::ParenthesizedExpression(_)))
            .copied()
        else {
            return false;
        };
        if member.object.span() != wrapper {
            return false;
        }
        self.source
            .get(inner.start as usize..inner.end as usize)
            .is_some_and(is_decimal_integer)
    }

    fn is_context_exception(
        &self,
        expression: &Expression<'_>,
        kind: InnerKind,
        context_span: Span,
        chain_depth: usize,
    ) -> bool {
        if kind == InnerKind::RegExp
            && (self.is_variable_init(context_span)
                || self.is_return_argument(context_span)
                || self.is_member_object(context_span))
        {
            return true;
        }
        if kind == InnerKind::Call && is_iife(expression) && chain_depth == 1 {
            return true;
        }
        if kind == InnerKind::New
            && chain_depth == 1
            && self.is_unparenthesized_new_callee_with_outer_parens(expression, context_span)
        {
            return true;
        }
        if chain_depth == 1
            && self.is_for_context()
            && self.has_outer_nonconsecutive_parentheses()
            && self
                .source
                .get(expression.span().start as usize..expression.span().end as usize)
                .is_some_and(|source| source.trim_start().starts_with("let"))
        {
            return true;
        }
        if kind == InnerKind::Chain
            && self
                .source
                .get(expression.span().start as usize..expression.span().end as usize)
                .is_some_and(|text| {
                    text.trim_start().starts_with("function")
                        && (text.contains("?.()")
                            || (!self.options.function_prototype_methods
                                && text.contains("?.call")))
                })
        {
            return true;
        }
        if matches!(kind, InnerKind::Class | InnerKind::Object)
            && chain_depth == 1
            && self.is_member_object(context_span)
            && self.expression_parens_are_required_at_start(context_span)
        {
            return true;
        }
        if kind == InnerKind::Sequence
            && self.is_class_property_value(context_span)
            && chain_depth == 1
        {
            return true;
        }
        if kind == InnerKind::TypeAssertion {
            if !self.options.nested_binary {
                return true;
            }
            if self.type_assertion_requires_parens(context_span) && chain_depth == 1 {
                return true;
            }
        }

        if !self.options.conditional_assign
            && kind == InnerKind::Assignment
            && (self.is_control_test(context_span) || self.is_conditional_test(context_span))
        {
            return true;
        }
        if !self.options.ternary_binary
            && matches!(kind, InnerKind::Binary | InnerKind::Logical)
            && self.is_conditional_operand(context_span)
        {
            return true;
        }
        if !self.options.nested_binary
            && matches!(kind, InnerKind::Binary | InnerKind::Logical)
            && self.is_binary_child(context_span)
        {
            return true;
        }
        if !self.options.return_assign
            && kind == InnerKind::Assignment
            && self.is_inside_return_or_expression_arrow()
        {
            return true;
        }
        if !self.options.arrow_conditionals
            && kind == InnerKind::Conditional
            && self.is_direct_arrow_body(context_span)
        {
            return true;
        }
        if !self.options.nested_conditionals
            && kind == InnerKind::Conditional
            && self.is_conditional_operand(context_span)
        {
            return true;
        }
        if !self.options.new_in_member
            && kind == InnerKind::New
            && self.is_member_object(context_span)
        {
            return true;
        }
        if !self.options.function_prototype_methods
            && ((kind == InnerKind::Call && is_function_prototype_call(expression))
                || (kind == InnerKind::Function
                    && self.is_function_prototype_member_object(context_span)))
        {
            return true;
        }
        if self.is_spread_argument(context_span)
            && ((kind == InnerKind::Conditional && self.options.allow_spread_conditional)
                || (kind == InnerKind::Logical && self.options.allow_spread_logical)
                || (kind == InnerKind::Await && self.options.allow_spread_await))
        {
            return true;
        }
        self.matches_ignored_node(kind, context_span)
    }

    fn is_type_context_exception(
        &self,
        kind: InnerKind,
        context_span: Span,
        chain_depth: usize,
    ) -> bool {
        if self.is_inside_type_arguments() {
            return true;
        }
        if self.is_accessor_property_key(context_span) && chain_depth == 1 {
            return true;
        }
        (!self.options.nested_binary
            && kind == InnerKind::TypeBinary
            && self.is_type_binary_child(context_span)
            && chain_depth == 1)
            || self.matches_ignored_node(kind, context_span)
    }

    fn type_assertion_requires_parens(&self, span: Span) -> bool {
        self.is_binary_child(span)
            || self.is_member_object(span)
            || self.parents.iter().rev().any(|parent| match parent {
                AstKind::AwaitExpression(await_expression) => {
                    await_expression.argument.span() == span
                }
                AstKind::Class(class) => class
                    .super_class
                    .as_ref()
                    .is_some_and(|base| base.span() == span),
                AstKind::ArrowFunctionExpression(_) => self.is_direct_arrow_body(span),
                AstKind::ForInStatement(statement) => statement.right.span() == span,
                AstKind::ForOfStatement(statement) => statement.right.span() == span,
                AstKind::ForStatement(statement) => {
                    statement
                        .init
                        .as_ref()
                        .is_some_and(|init| init.span() == span)
                        || statement
                            .test
                            .as_ref()
                            .is_some_and(|test| test.span() == span)
                        || statement
                            .update
                            .as_ref()
                            .is_some_and(|update| update.span() == span)
                }
                AstKind::IfStatement(statement) => statement.test.span() == span,
                AstKind::WhileStatement(statement) => statement.test.span() == span,
                AstKind::DoWhileStatement(statement) => statement.test.span() == span,
                AstKind::ThrowStatement(statement) => statement.argument.span() == span,
                AstKind::SwitchCase(case) => {
                    case.test.as_ref().is_some_and(|test| test.span() == span)
                }
                AstKind::SpreadElement(spread) => spread.argument.span() == span,
                AstKind::YieldExpression(yield_expression) => yield_expression
                    .argument
                    .as_ref()
                    .is_some_and(|argument| argument.span() == span),
                _ => false,
            })
    }

    fn is_inside_type_arguments(&self) -> bool {
        self.parents
            .iter()
            .any(|parent| matches!(parent, AstKind::TSTypeParameterInstantiation(_)))
    }

    fn is_accessor_property_key(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::AccessorProperty(property) if property.key.span() == span),
        )
    }

    fn is_top_level_expression_statement(&self, span: Span) -> bool {
        let Some((index, _)) = self
            .parents
            .iter()
            .enumerate()
            .rev()
            .find(|(_, parent)| {
                matches!(parent, AstKind::ExpressionStatement(statement) if statement.expression.span() == span)
            })
        else {
            return false;
        };
        let Some(container) = index
            .checked_sub(1)
            .and_then(|container_index| self.parents.get(container_index))
        else {
            return false;
        };
        match container {
            AstKind::Program(_) | AstKind::BlockStatement(_) => true,
            AstKind::FunctionBody(_) => index
                .checked_sub(2)
                .and_then(|function_index| self.parents.get(function_index))
                .is_none_or(
                    |parent| !matches!(parent, AstKind::ArrowFunctionExpression(arrow) if arrow.expression),
                ),
            _ => false,
        }
    }

    fn must_force_report(
        &self,
        expression: &Expression<'_>,
        kind: InnerKind,
        span: Span,
        chain_depth: usize,
    ) -> bool {
        if kind == InnerKind::Function {
            if self.is_call_callee(span) || self.expression_parens_are_required_at_start(span) {
                return false;
            }
            return self.is_variable_init(span)
                || self.is_class_property_value(span)
                || self.is_new_callee(span)
                || self.is_sequence_child(span)
                || self.is_assignment_rhs(span)
                || self.is_export_default(span)
                || self.is_call_argument(span)
                || self.is_computed_member_property(span)
                || self.is_member_object(span);
        }
        if kind == InnerKind::ArrowFunction {
            if self.is_call_callee(span) {
                return false;
            }
            return self.is_variable_init(span)
                || self.is_class_property_value(span)
                || self.is_new_callee(span)
                || self.is_sequence_child(span)
                || self.is_assignment_rhs(span)
                || self.is_export_default(span)
                || self.is_call_argument(span)
                || self.is_computed_member_property(span);
        }
        if kind == InnerKind::Call && chain_depth > 1 && is_iife(expression) {
            return true;
        }
        if kind == InnerKind::Chain {
            return self.is_optional_chain_continuation(span);
        }
        if chain_depth > 1 && contains_in_expression(expression) {
            return true;
        }
        if matches!(expression, Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::In)
            && !self.is_unary_argument(span)
        {
            return true;
        }
        if kind == InnerKind::Other
            && self
                .source
                .get(expression.span().start as usize..expression.span().end as usize)
                .is_some_and(|source| {
                    source.trim_start().starts_with('!') && source.contains(" in ")
                })
        {
            return true;
        }
        if self.is_update_argument(span)
            || self.is_assignment_target(span)
            || self.is_for_left(span)
        {
            return true;
        }
        kind == InnerKind::Sequence && self.is_class_property_value(span) && chain_depth > 1
    }

    fn expression_parens_are_required_at_start(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::ExpressionStatement(statement) => {
                self.only_open_parens_before(statement.expression.span().start, span.start)
            }
            AstKind::ExportDefaultDeclaration(export) => {
                self.only_open_parens_before(export.declaration.span().start, span.start)
            }
            AstKind::ArrowFunctionExpression(arrow) if arrow.expression => {
                arrow.body.statements.first().is_some_and(|statement| {
                    matches!(
                        statement,
                        oxc_ast::ast::Statement::ExpressionStatement(statement)
                            if self.only_open_parens_before(statement.expression.span().start, span.start)
                    )
                })
            }
            _ => false,
        })
    }

    fn only_open_parens_before(&self, start: u32, end: u32) -> bool {
        self.source
            .get(start as usize..end as usize)
            .is_some_and(|prefix| {
                prefix
                    .chars()
                    .all(|character| character.is_whitespace() || character == '(')
            })
    }

    fn should_buffer_for_in_expression(
        &self,
        expression: &Expression<'_>,
        kind: InnerKind,
        chain_depth: usize,
    ) -> bool {
        if chain_depth > 1 || !contains_in_expression(expression) {
            return false;
        }
        if !matches!(
            kind,
            InnerKind::Assignment
                | InnerKind::ArrowFunction
                | InnerKind::Binary
                | InnerKind::Logical
                | InnerKind::Sequence
        ) {
            return false;
        }

        let Some(for_index) = self.parents.iter().rposition(
            |parent| matches!(parent, AstKind::ForStatement(statement) if statement.init.as_ref().is_some_and(|init| init.span().contains_inclusive(expression.span()))),
        ) else {
            return false;
        };

        let path = &self.parents[for_index + 1..];
        if path
            .iter()
            .any(|parent| matches!(parent, AstKind::ParenthesizedExpression(_)))
        {
            return false;
        }

        !path.iter().any(|parent| {
            matches!(
                parent,
                AstKind::ArrayExpression(_)
                    | AstKind::ArrayPattern(_)
                    | AstKind::BlockStatement(_)
                    | AstKind::ObjectExpression(_)
                    | AstKind::ObjectPattern(_)
                    | AstKind::TemplateLiteral(_)
            )
        })
    }

    fn is_unary_argument(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::UnaryExpression(unary) if unary.argument.span() == span),
        )
    }

    fn is_for_context(&self) -> bool {
        self.parents.iter().any(|parent| {
            matches!(
                parent,
                AstKind::ForStatement(_) | AstKind::ForInStatement(_) | AstKind::ForOfStatement(_)
            )
        })
    }

    fn has_outer_nonconsecutive_parentheses(&self) -> bool {
        let mut passed_non_parenthesized = false;
        for parent in self.parents.iter().rev() {
            if matches!(parent, AstKind::ParenthesizedExpression(_)) {
                if passed_non_parenthesized {
                    return true;
                }
            } else {
                passed_non_parenthesized = true;
            }
        }
        false
    }

    fn is_unparenthesized_new_callee_with_outer_parens(
        &self,
        expression: &Expression<'_>,
        span: Span,
    ) -> bool {
        let Expression::NewExpression(inner) = expression else {
            return false;
        };
        if self
            .source
            .get(inner.span.end.saturating_sub(1) as usize..inner.span.end as usize)
            == Some(")")
        {
            return false;
        }
        self.parents.iter().rev().any(|parent| {
            let AstKind::NewExpression(outer) = parent else {
                return false;
            };
            outer.callee.span() == span
                && self
                    .source
                    .get(span.end as usize..outer.span.end as usize)
                    .is_some_and(|suffix| suffix.trim_start().starts_with("()"))
        })
    }

    fn is_sequence_child(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::SequenceExpression(sequence) if sequence.expressions.iter().any(|expression| expression.span() == span)),
        )
    }

    fn is_assignment_rhs(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::AssignmentExpression(assignment) if assignment.right.span() == span),
        )
    }

    fn is_export_default(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::ExportDefaultDeclaration(export) if export.declaration.span() == span),
        )
    }

    fn is_call_argument(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::CallExpression(call) => call
                .arguments
                .iter()
                .any(|argument| argument.span() == span),
            AstKind::NewExpression(new) => {
                new.arguments.iter().any(|argument| argument.span() == span)
            }
            _ => false,
        })
    }

    fn is_computed_member_property(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::ComputedMemberExpression(member) if member.expression.span() == span),
        )
    }

    fn is_optional_chain_continuation(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::ComputedMemberExpression(member) => {
                member.object.span() == span && member.optional
            }
            AstKind::StaticMemberExpression(member) => {
                member.object.span() == span && member.optional
            }
            AstKind::PrivateFieldExpression(member) => {
                member.object.span() == span && member.optional
            }
            AstKind::CallExpression(call) => call.callee.span() == span && call.optional,
            _ => false,
        })
    }

    fn is_call_callee(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::CallExpression(call) if call.callee.span() == span),
        )
    }

    fn is_update_argument(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::UpdateExpression(update) if update.argument.span() == span),
        )
    }

    fn is_assignment_target(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::AssignmentExpression(assignment) if assignment.left.span() == span),
        )
    }

    fn is_for_left(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::ForInStatement(statement) => statement.left.span() == span,
            AstKind::ForOfStatement(statement) => statement.left.span() == span,
            _ => false,
        })
    }

    fn is_variable_init(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::VariableDeclarator(declarator) if declarator.init.as_ref().is_some_and(|init| init.span() == span)),
        )
    }

    fn is_return_argument(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::ReturnStatement(statement) if statement.argument.as_ref().is_some_and(|argument| argument.span() == span)),
        )
    }

    fn is_member_object(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::ComputedMemberExpression(member) => member.object.span() == span,
            AstKind::StaticMemberExpression(member) => member.object.span() == span,
            AstKind::PrivateFieldExpression(member) => member.object.span() == span,
            _ => false,
        })
    }

    fn is_new_callee(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::NewExpression(new) if new.callee.span() == span),
        )
    }

    fn is_class_property_value(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::PropertyDefinition(property) => property
                .value
                .as_ref()
                .is_some_and(|value| value.span() == span),
            AstKind::AccessorProperty(property) => property
                .value
                .as_ref()
                .is_some_and(|value| value.span() == span),
            _ => false,
        })
    }

    fn is_control_test(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::IfStatement(statement) => statement.test.span() == span,
            AstKind::WhileStatement(statement) => statement.test.span() == span,
            AstKind::DoWhileStatement(statement) => statement.test.span() == span,
            AstKind::ForStatement(statement) => statement
                .test
                .as_ref()
                .is_some_and(|test| test.span() == span),
            _ => false,
        })
    }

    fn is_conditional_test(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::ConditionalExpression(expression) if expression.test.span() == span),
        )
    }

    fn is_conditional_operand(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| {
            matches!(
                parent,
                AstKind::ConditionalExpression(expression)
                    if expression.test.span() == span
                        || expression.consequent.span() == span
                        || expression.alternate.span() == span
            )
        })
    }

    fn is_binary_child(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::BinaryExpression(expression) => {
                expression.left.span() == span || expression.right.span() == span
            }
            AstKind::LogicalExpression(expression) => {
                expression.left.span() == span || expression.right.span() == span
            }
            _ => false,
        })
    }

    fn is_inside_return_or_expression_arrow(&self) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::ReturnStatement(_) => true,
            AstKind::ArrowFunctionExpression(arrow) => arrow.expression,
            _ => false,
        })
    }

    fn is_direct_arrow_body(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| {
            let AstKind::ArrowFunctionExpression(arrow) = parent else {
                return false;
            };
            arrow.expression
                && arrow.body.statements.first().is_some_and(|statement| {
                    matches!(
                        statement,
                        oxc_ast::ast::Statement::ExpressionStatement(expression)
                            if expression.expression.span() == span
                    )
                })
        })
    }

    fn is_function_prototype_member_object(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::StaticMemberExpression(member) => {
                member.object.span() == span
                    && matches!(member.property.name.as_str(), "call" | "apply")
            }
            AstKind::ComputedMemberExpression(member) => {
                member.object.span() == span
                    && self
                        .source
                        .get(
                            member.expression.span().start as usize
                                ..member.expression.span().end as usize,
                        )
                        .is_some_and(|property| {
                            matches!(property.trim_matches(['\'', '"', '`']), "call" | "apply")
                        })
            }
            _ => false,
        })
    }

    fn is_spread_argument(&self, span: Span) -> bool {
        self.parents.iter().rev().any(
            |parent| matches!(parent, AstKind::SpreadElement(spread) if spread.argument.span() == span),
        )
    }

    fn is_type_binary_child(&self, span: Span) -> bool {
        self.parents.iter().rev().any(|parent| match parent {
            AstKind::TSUnionType(union) => union.types.iter().any(|item| item.span() == span),
            AstKind::TSIntersectionType(intersection) => {
                intersection.types.iter().any(|item| item.span() == span)
            }
            _ => false,
        })
    }

    fn matches_ignored_node(&self, kind: InnerKind, span: Span) -> bool {
        self.options
            .ignored_nodes
            .iter()
            .any(|selector| match selector.as_str() {
                "ArrowFunctionExpression[body.type=ConditionalExpression]" => {
                    kind == InnerKind::Conditional && self.is_direct_arrow_body(span)
                }
                "MemberExpression[object.type=NewExpression]" => {
                    kind == InnerKind::New && self.is_member_object(span)
                }
                "SpreadElement" => self.is_spread_argument(span),
                "SpreadElement[argument.type=ConditionalExpression]" => {
                    kind == InnerKind::Conditional && self.is_spread_argument(span)
                }
                "SpreadElement[argument.type=LogicalExpression]" => {
                    kind == InnerKind::Logical && self.is_spread_argument(span)
                }
                "SpreadElement[argument.type=AwaitExpression]" => {
                    kind == InnerKind::Await && self.is_spread_argument(span)
                }
                "VariableDeclarator[init.type=\"LogicalExpression\"]"
                    if kind == InnerKind::Logical =>
                {
                    kind == InnerKind::Logical && self.is_variable_init(span)
                }
                "VariableDeclarator[init]" => self.is_variable_init(span),
                "TSTypeAliasDeclaration[typeAnnotation.type=\"TSIntersectionType\"]" => {
                    kind == InnerKind::TypeBinary
                        && self.parents.iter().rev().any(
                            |parent| matches!(parent, AstKind::TSTypeAliasDeclaration(alias) if alias.type_annotation.span() == span),
                        )
                }
                _ => false,
            })
    }

    fn allowed_by_comment(&self, open: usize) -> bool {
        let prefix = &self.source[..open];
        let Some(comment_end) = prefix.trim_end().strip_suffix("*/") else {
            return false;
        };
        let Some(comment_start) = comment_end.rfind("/*") else {
            return false;
        };
        let comment = &comment_end[comment_start + 2..];
        if comment.starts_with('*') && comment.contains("@type") {
            return true;
        }
        self.options
            .allow_comment_pattern
            .as_ref()
            .is_some_and(|pattern| {
                regex::Regex::new(pattern).is_ok_and(|compiled| compiled.is_match(comment))
            })
    }
}

fn expression_kind(expression: &Expression<'_>) -> InnerKind {
    match expression {
        Expression::ArrowFunctionExpression(_) => InnerKind::ArrowFunction,
        Expression::FunctionExpression(_) => InnerKind::Function,
        Expression::SequenceExpression(_) => InnerKind::Sequence,
        Expression::AssignmentExpression(_) => InnerKind::Assignment,
        Expression::BinaryExpression(_) => InnerKind::Binary,
        Expression::LogicalExpression(_) => InnerKind::Logical,
        Expression::ConditionalExpression(_) => InnerKind::Conditional,
        Expression::CallExpression(_) => InnerKind::Call,
        Expression::NewExpression(_) => InnerKind::New,
        Expression::AwaitExpression(_) => InnerKind::Await,
        Expression::ChainExpression(_) => InnerKind::Chain,
        Expression::TSAsExpression(_)
        | Expression::TSSatisfiesExpression(_)
        | Expression::TSTypeAssertion(_)
        | Expression::TSNonNullExpression(_) => InnerKind::TypeAssertion,
        Expression::ClassExpression(_) => InnerKind::Class,
        Expression::ObjectExpression(_) => InnerKind::Object,
        Expression::JSXElement(_) | Expression::JSXFragment(_) => InnerKind::Jsx,
        Expression::StringLiteral(_) => InnerKind::String,
        Expression::RegExpLiteral(_) => InnerKind::RegExp,
        Expression::NumericLiteral(_) => InnerKind::Number,
        _ => InnerKind::Other,
    }
}

fn type_kind(annotation: &TSType<'_>) -> InnerKind {
    if matches!(
        annotation,
        TSType::TSUnionType(_) | TSType::TSIntersectionType(_)
    ) {
        InnerKind::TypeBinary
    } else {
        InnerKind::Type
    }
}

fn is_iife(expression: &Expression<'_>) -> bool {
    matches!(
        expression,
        Expression::CallExpression(call)
            if matches!(call.callee.without_parentheses(), Expression::FunctionExpression(_))
    )
}

fn is_function_prototype_call(expression: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = expression else {
        return false;
    };
    match call.callee.without_parentheses() {
        Expression::StaticMemberExpression(member) => {
            matches!(
                member.object.without_parentheses(),
                Expression::FunctionExpression(_)
            ) && matches!(member.property.name.as_str(), "call" | "apply")
        }
        Expression::ComputedMemberExpression(member) => matches!(
            member.object.without_parentheses(),
            Expression::FunctionExpression(_)
        ),
        _ => false,
    }
}

fn is_decimal_integer(raw: &str) -> bool {
    !is_legacy_octal(raw)
        && !raw.contains(['.', 'e', 'E'])
        && raw
            .trim_end_matches('n')
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'_')
}

fn is_legacy_octal(raw: &str) -> bool {
    raw.len() > 1 && raw.starts_with('0') && raw.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
}

fn contains_line_terminator(source: &str) -> bool {
    source.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
}

fn contains_in_expression(expression: &Expression<'_>) -> bool {
    struct Finder {
        found: bool,
        safely_enclosed: usize,
    }

    impl<'ast> Visit<'ast> for Finder {
        fn enter_node(&mut self, kind: AstKind<'ast>) {
            if matches!(
                kind,
                AstKind::ArrayExpression(_)
                    | AstKind::ObjectExpression(_)
                    | AstKind::TemplateLiteral(_)
                    | AstKind::Function(_)
            ) || matches!(kind, AstKind::ArrowFunctionExpression(arrow) if !arrow.expression)
            {
                self.safely_enclosed += 1;
            }
            if self.safely_enclosed == 0
                && matches!(kind, AstKind::BinaryExpression(binary) if binary.operator == BinaryOperator::In)
            {
                self.found = true;
            }
        }

        fn leave_node(&mut self, kind: AstKind<'ast>) {
            if matches!(
                kind,
                AstKind::ArrayExpression(_)
                    | AstKind::ObjectExpression(_)
                    | AstKind::TemplateLiteral(_)
                    | AstKind::Function(_)
            ) || matches!(kind, AstKind::ArrowFunctionExpression(arrow) if !arrow.expression)
            {
                self.safely_enclosed -= 1;
            }
        }
    }

    let mut finder = Finder {
        found: false,
        safely_enclosed: 0,
    };
    finder.visit_expression(expression);
    finder.found
}

fn report(
    candidate: Candidate,
    left_replacement: &str,
    right_replacement: &str,
    fixable: bool,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let mut suggestions = Vec::new();
    if fixable {
        let Some(close) = candidate.wrapper.end.checked_sub(1) else {
            return;
        };
        suggestions.push(LintSuggestion {
            message_id: MESSAGE_ID.to_owned(),
            message: MESSAGE.to_owned(),
            fixes: [
                LintFix::replace_range(
                    TextRange::new(candidate.wrapper.start, candidate.wrapper.start + 1),
                    left_replacement,
                ),
                LintFix::replace_range(
                    TextRange::new(close, candidate.wrapper.end),
                    right_replacement,
                ),
            ]
            .into_iter()
            .collect(),
        });
    }

    diagnostics.push(LintDiagnostic {
        rule_name: RULE.to_owned(),
        message_id: MESSAGE_ID.to_owned(),
        message: MESSAGE.to_owned(),
        data: BTreeMap::new(),
        range: TextRange::new(candidate.wrapper.start, candidate.wrapper.start + 1),
        suggestions,
    });
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "fixture failures and serde_json option matrices are test-only diagnostics"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_no_extra_parens(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UpstreamCase {
        language: String,
        code: String,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        options: Value,
        errors: Option<Vec<Value>>,
    }

    #[derive(Deserialize)]
    struct UpstreamFixture {
        valid: Vec<UpstreamCase>,
        invalid: Vec<UpstreamCase>,
    }

    fn filename(test_case: &UpstreamCase) -> &'static str {
        if test_case.language == "js" {
            "fixture.jsx"
        } else {
            "fixture.ts"
        }
    }

    fn apply_once(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut edits = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| suggestion.fixes.iter())
            .collect::<Vec<_>>();
        if edits.is_empty() {
            return None;
        }
        edits.sort_by_key(|fix| (fix.range.start, fix.range.end));

        let mut accepted = Vec::new();
        let mut last_end = 0;
        for edit in edits {
            if !accepted.is_empty() && edit.range.start < last_end {
                continue;
            }
            last_end = edit.range.end;
            accepted.push(edit);
        }

        let mut output = source.to_owned();
        for edit in accepted.into_iter().rev() {
            let start = usize::try_from(edit.range.start).ok()?;
            let end = usize::try_from(edit.range.end).ok()?;
            output.replace_range(start..end, &edit.replacement_text);
        }
        Some(output)
    }

    fn apply_expected_passes(test_case: &UpstreamCase) -> Option<String> {
        if test_case.output.is_none() {
            return apply_once(
                &test_case.code,
                &run(
                    &test_case.code,
                    Some(filename(test_case)),
                    test_case.options.clone(),
                ),
            );
        }
        let mut output = test_case.code.clone();
        for _ in 0..20 {
            let diagnostics = run(
                &output,
                Some(filename(test_case)),
                test_case.options.clone(),
            );
            let next = apply_once(&output, &diagnostics)?;
            assert_ne!(next, output, "fix must make progress: {}", test_case.code);
            output = next;
            if Some(&output) == test_case.output.as_ref() {
                return Some(output);
            }
        }
        panic!("fixes did not reach expected output: {}", test_case.code);
    }

    #[test]
    fn replays_every_pinned_upstream_v5_10_0_case() {
        let fixture: UpstreamFixture = serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/no-extra-parens-v5.10.0.json"
        ))
        .expect("fixture must deserialize");
        let mut failures = Vec::new();

        for (index, test_case) in fixture.valid.iter().enumerate() {
            let diagnostics = run(
                &test_case.code,
                Some(filename(test_case)),
                test_case.options.clone(),
            );
            if !diagnostics.is_empty() {
                failures.push(format!(
                    "valid {index}: got {} reports: {}",
                    diagnostics.len(),
                    test_case.code
                ));
            }
        }
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(
                &test_case.code,
                Some(filename(test_case)),
                test_case.options.clone(),
            );
            let expected = test_case.errors.as_ref().map_or(1, Vec::len);
            if diagnostics.len() != expected {
                failures.push(format!(
                    "invalid {index}: expected {expected}, got {} at {:?}: {}",
                    diagnostics.len(),
                    diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.range.start)
                        .collect::<Vec<_>>(),
                    test_case.code
                ));
                continue;
            }
            let actual_output = apply_expected_passes(test_case);
            if actual_output != test_case.output {
                failures.push(format!(
                    "invalid {index}: output {:?}, expected {:?}: {}",
                    actual_output, test_case.output, test_case.code
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "{} upstream parity failures\n{}",
            failures.len(),
            failures
                .into_iter()
                .take(400)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn uses_ast_equivalence_for_precedence_associativity_and_restricted_productions() {
        for source in [
            "(a + b) * c",
            "a + (b + c)",
            "a((b, c))",
            "({ value: true });",
            "x => ({ value: true })",
            "new (A())",
            "(0).toString()",
        ] {
            assert!(
                run(source, Some("fixture.js"), json!([])).is_empty(),
                "{source}"
            );
        }

        for source in [
            "(a) + b",
            "(a + b) + c",
            "call((value))",
            "if ((ready)) work();",
        ] {
            assert!(
                !run(source, Some("fixture.js"), json!([])).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn supports_functions_mode_and_jsx_modes() {
        assert!(run("(value)", Some("fixture.js"), json!(["functions"])).is_empty());
        assert_eq!(
            run(
                "const value = (function () {});",
                Some("fixture.js"),
                json!(["functions"])
            )
            .len(),
            1
        );
        assert!(
            run(
                "const view = (<Panel />);",
                Some("fixture.jsx"),
                json!(["all", { "ignoreJSX": "all" }])
            )
            .is_empty()
        );
    }
}
