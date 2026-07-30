//! Native AST implementation of stable `@stylistic/padding-line-between-statements`.

use std::{collections::BTreeMap, sync::LazyLock};

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{ClassType, FunctionType},
};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use super::lexer::{Token, tokenize};
use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "padding-line-between-statements";
const EXPECTED_ID: &str = "expectedBlankLine";
const EXPECTED_MESSAGE: &str = "Expected blank line before this statement.";
const UNEXPECTED_ID: &str = "unexpectedBlankLine";
const UNEXPECTED_MESSAGE: &str = "Unexpected blank line before this statement.";

static CJS_EXPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:module\s*\.\s*)?exports(?:\s*\.|\s*\[|\s*=)")
        .expect("CJS export regex is valid")
});
static LEGACY_EXPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:module\s*\.\s*exports(?:\s*\.|\s*\[|\s*=)|exports\s*(?:\.|\[))")
        .expect("legacy export regex is valid")
});
static REMOVE_PADDING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^(\s*?[\r\n\u{2028}\u{2029}])\s*[\r\n\u{2028}\u{2029}](\s*;?)$")
        .expect("padding removal regex is valid")
});

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Padding {
    Any,
    Never,
    Always,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaddingConfig {
    blank_line: Padding,
    prev: StatementOption,
    next: StatementOption,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StatementOption {
    One(StatementMatcher),
    Many(Vec<StatementMatcher>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StatementMatcher {
    Name(String),
    Selector(SelectorMatcher),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectorMatcher {
    selector: String,
    #[serde(default)]
    line_mode: Option<LineMode>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LineMode {
    Any,
    Singleline,
    Multiline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeKind {
    Directive,
    Block,
    Empty,
    Expression,
    Function,
    FunctionOverload,
    Class,
    Break,
    Continue,
    Debugger,
    Do,
    For,
    If,
    Return,
    Switch,
    Throw,
    Try,
    While,
    With,
    Import,
    Export,
    ExportNamedFunction,
    ExportNamedOverload,
    VariableVar,
    VariableLet,
    VariableConst,
    VariableUsing,
    TypeAlias,
    Interface,
    Enum,
    SwitchCase,
    SwitchDefault,
    TSMethod,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    report_span: Span,
    match_span: Span,
    kind: NodeKind,
}

#[derive(Clone, Copy)]
struct Pair {
    prev: Candidate,
    next: Candidate,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum StackKind {
    Program,
    Block,
    FunctionBody,
    Switch,
    SwitchCase,
    StaticBlock,
    TSInterfaceBody,
    TSModuleBlock,
    TSTypeLiteral,
    TSDeclareFunction,
    TSMethod,
    Other,
}

pub(crate) fn check_padding_line_between_statements(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let Ok(configs) = serde_json::from_value::<Vec<PaddingConfig>>(options.clone()) else {
        return;
    };
    if configs.is_empty() {
        return;
    }

    let source_types = source_types(filename);
    for source_type in source_types {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, source_type).parse();
        if parsed.errors.is_empty() {
            run_on_program(source, &parsed.program, &configs, diagnostics);
            return;
        }
    }

    // RuleTester permits top-level return through parser options. Oxc still
    // produces the complete statement tree for that recoverable parse error.
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        source,
        if filename.is_some_and(is_typescript_path) {
            SourceType::tsx()
        } else {
            SourceType::jsx().with_script(true)
        },
    )
    .parse();
    run_on_program(source, &parsed.program, &configs, diagnostics);
}

fn source_types(filename: Option<&str>) -> [SourceType; 4] {
    if let Some(path) = filename
        && let Ok(source_type) = SourceType::from_path(path)
    {
        return [source_type; 4];
    }
    [
        SourceType::tsx(),
        SourceType::ts(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ]
}

fn is_typescript_path(path: &str) -> bool {
    path.ends_with(".ts")
        || path.ends_with(".tsx")
        || path.ends_with(".mts")
        || path.ends_with(".cts")
}

fn run_on_program<'ast>(
    source: &str,
    program: &'ast oxc_ast::ast::Program<'ast>,
    configs: &[PaddingConfig],
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let lines = Lines::new(source);
    let mut visitor = PaddingVisitor {
        source,
        configs,
        tokens: &tokens,
        lines: &lines,
        diagnostics,
        stack: Vec::new(),
        scopes: Vec::new(),
        pairs: Vec::new(),
    };
    visitor.visit_program(program);
    for pair in std::mem::take(&mut visitor.pairs) {
        visitor.verify_pair(pair);
    }
}

struct PaddingVisitor<'source, 'config, 'diagnostics> {
    source: &'source str,
    configs: &'config [PaddingConfig],
    tokens: &'config [Token],
    lines: &'config Lines,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
    stack: Vec<StackKind>,
    scopes: Vec<Option<Candidate>>,
    pairs: Vec<Pair>,
}

impl<'ast> Visit<'ast> for PaddingVisitor<'_, '_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        let parent = self.stack.last().copied();
        let tag = stack_kind(kind);

        if is_candidate_parent(parent)
            && let Some(candidate) = candidate_from_ast(kind, self.source)
            && candidate_allowed_by_parent(parent, candidate.kind)
        {
            self.collect(candidate);
        }

        self.stack.push(tag);
        if opens_scope(tag) {
            self.scopes.push(None);
        }
    }

    fn leave_node(&mut self, kind: AstKind<'ast>) {
        let tag = stack_kind(kind);
        if opens_scope(tag) {
            self.scopes.pop();
        }
        self.stack.pop();
    }
}

impl PaddingVisitor<'_, '_, '_> {
    fn collect(&mut self, candidate: Candidate) {
        let Some(previous) = self.scopes.last_mut() else {
            return;
        };
        if let Some(prev) = *previous {
            self.pairs.push(Pair {
                prev,
                next: candidate,
            });
        }
        *previous = Some(candidate);
    }

    fn verify_pair(&mut self, pair: Pair) {
        let padding = self
            .configs
            .iter()
            .rev()
            .find(|config| {
                self.matches_option(pair.prev, &config.prev)
                    && self.matches_option(pair.next, &config.next)
            })
            .map_or(Padding::Any, |config| config.blank_line);
        let padding_pairs = self.padding_pairs(pair);

        match padding {
            Padding::Any => {}
            Padding::Never if !padding_pairs.is_empty() => {
                let suggestion = if padding_pairs.len() == 1 {
                    let (prev, next) = padding_pairs[0];
                    let gap = &self.source[prev.end..next.start];
                    let replacement = REMOVE_PADDING
                        .captures(gap)
                        .map(|captures| {
                            let mut replacement =
                                String::with_capacity(captures[1].len() + captures[2].len());
                            replacement.push_str(&captures[1]);
                            replacement.push_str(&captures[2]);
                            replacement
                        })
                        .unwrap_or_else(|| gap.to_owned());
                    Some(LintFix::replace_range(
                        TextRange::new(prev.end as u32, next.start as u32),
                        replacement,
                    ))
                } else {
                    None
                };
                self.report(pair.next, UNEXPECTED_ID, UNEXPECTED_MESSAGE, suggestion);
            }
            Padding::Always if padding_pairs.is_empty() => {
                let (offset, text) = self.always_fix(pair);
                self.report(
                    pair.next,
                    EXPECTED_ID,
                    EXPECTED_MESSAGE,
                    Some(LintFix::replace_range(
                        TextRange::new(offset as u32, offset as u32),
                        text,
                    )),
                );
            }
            Padding::Never | Padding::Always => {}
        }
    }

    fn matches_option(&self, candidate: Candidate, option: &StatementOption) -> bool {
        match option {
            StatementOption::One(matcher) => self.matches(candidate, matcher),
            StatementOption::Many(matchers) => matchers
                .iter()
                .any(|matcher| self.matches(candidate, matcher)),
        }
    }

    fn matches(&self, candidate: Candidate, matcher: &StatementMatcher) -> bool {
        match matcher {
            StatementMatcher::Name(name) => self.matches_name(candidate, name),
            StatementMatcher::Selector(selector) => {
                self.matches_selector(candidate, &selector.selector)
                    && match selector.line_mode.unwrap_or(LineMode::Any) {
                        LineMode::Any => true,
                        LineMode::Singleline => self.is_single_line(candidate.match_span),
                        LineMode::Multiline => !self.is_single_line(candidate.match_span),
                    }
            }
        }
    }

    fn matches_name(&self, candidate: Candidate, name: &str) -> bool {
        let base = name
            .strip_prefix("singleline-")
            .or_else(|| name.strip_prefix("multiline-"))
            .unwrap_or(name);
        let matched = match base {
            "*" => true,
            "exports" => {
                candidate.kind == NodeKind::Expression
                    && LEGACY_EXPORT.is_match(self.candidate_text(candidate))
            }
            "require" => {
                matches!(
                    candidate.kind,
                    NodeKind::VariableVar
                        | NodeKind::VariableLet
                        | NodeKind::VariableConst
                        | NodeKind::VariableUsing
                ) && first_initializer(self.candidate_text(candidate))
                    .is_some_and(|init| require_call(init, false))
            }
            "directive" => candidate.kind == NodeKind::Directive,
            "iife" => {
                candidate.kind == NodeKind::Expression && is_iife(self.candidate_text(candidate))
            }
            "block-like" => self.is_block_like(candidate),
            "block" => candidate.kind == NodeKind::Block,
            "empty" => candidate.kind == NodeKind::Empty,
            "function" => candidate.kind == NodeKind::Function,
            "ts-method" => candidate.kind == NodeKind::TSMethod,
            "break" => candidate.kind == NodeKind::Break,
            "case" => candidate.kind == NodeKind::SwitchCase,
            "class" => candidate.kind == NodeKind::Class,
            "continue" => candidate.kind == NodeKind::Continue,
            "debugger" => candidate.kind == NodeKind::Debugger,
            "default" => candidate.kind == NodeKind::SwitchDefault,
            "do" => candidate.kind == NodeKind::Do,
            "for" => candidate.kind == NodeKind::For,
            "if" => candidate.kind == NodeKind::If,
            "import" => candidate.kind == NodeKind::Import,
            "switch" => candidate.kind == NodeKind::Switch,
            "throw" => candidate.kind == NodeKind::Throw,
            "try" => candidate.kind == NodeKind::Try,
            "while" => candidate.kind == NodeKind::While,
            "with" => candidate.kind == NodeKind::With,
            "cjs-export" => {
                candidate.kind == NodeKind::Expression
                    && CJS_EXPORT.is_match(self.candidate_text(candidate))
            }
            "cjs-import" => {
                matches!(
                    candidate.kind,
                    NodeKind::VariableVar
                        | NodeKind::VariableLet
                        | NodeKind::VariableConst
                        | NodeKind::VariableUsing
                ) && first_initializer(self.candidate_text(candidate))
                    .is_some_and(|init| require_call(init, true))
            }
            "enum" => candidate.kind == NodeKind::Enum,
            "interface" => candidate.kind == NodeKind::Interface,
            "function-overload" => candidate.kind == NodeKind::FunctionOverload,
            "expression" => candidate.kind == NodeKind::Expression,
            "return" => candidate.kind == NodeKind::Return,
            "export" => matches!(
                candidate.kind,
                NodeKind::Export | NodeKind::ExportNamedFunction | NodeKind::ExportNamedOverload
            ),
            "var" => candidate.kind == NodeKind::VariableVar,
            "let" => candidate.kind == NodeKind::VariableLet,
            "const" => candidate.kind == NodeKind::VariableConst,
            "using" => candidate.kind == NodeKind::VariableUsing,
            "type" => candidate.kind == NodeKind::TypeAlias,
            _ => false,
        };

        if !matched {
            return false;
        }
        if name.starts_with("singleline-") {
            self.is_single_line(candidate.match_span)
        } else if name.starts_with("multiline-") {
            !self.is_single_line(candidate.match_span)
        } else {
            true
        }
    }

    fn matches_selector(&self, candidate: Candidate, selector: &str) -> bool {
        match selector {
            r#"FunctionDeclaration[id.name="bar"]"# => {
                candidate.kind == NodeKind::Function
                    && function_name(self.candidate_text(candidate)) == Some("bar")
            }
            r#"ExpressionStatement[expression.callee.name="foo"]"# => {
                candidate.kind == NodeKind::Expression
                    && call_name(self.candidate_text(candidate)) == Some("foo")
            }
            r#"ExpressionStatement[expression.callee.name="bar"]"# => {
                candidate.kind == NodeKind::Expression
                    && call_name(self.candidate_text(candidate)) == Some("bar")
            }
            "IfStatement" => candidate.kind == NodeKind::If,
            r#"ExportNamedDeclaration[declaration.type="FunctionDeclaration"]"# => {
                candidate.kind == NodeKind::ExportNamedFunction
            }
            r#"ExportNamedDeclaration[declaration.type="TSDeclareFunction"]"# => {
                candidate.kind == NodeKind::ExportNamedOverload
            }
            _ => false,
        }
    }

    fn is_block_like(&self, candidate: Candidate) -> bool {
        if candidate.kind == NodeKind::Do
            && self
                .candidate_text(candidate)
                .trim_start()
                .starts_with("do{")
        {
            return true;
        }
        if candidate.kind == NodeKind::Expression && is_iife(self.candidate_text(candidate)) {
            return true;
        }

        let text = self.candidate_text(candidate);
        let without_semi = text.trim_end().trim_end_matches(';').trim_end();
        if !without_semi.ends_with('}') {
            return false;
        }
        match candidate.kind {
            NodeKind::Block
            | NodeKind::Function
            | NodeKind::Class
            | NodeKind::If
            | NodeKind::For
            | NodeKind::Switch
            | NodeKind::Try
            | NodeKind::While
            | NodeKind::With
            | NodeKind::ExportNamedFunction => true,
            NodeKind::Expression
            | NodeKind::VariableVar
            | NodeKind::VariableLet
            | NodeKind::VariableConst
            | NodeKind::VariableUsing => text.contains("function") || text.contains("=>"),
            _ => false,
        }
    }

    fn padding_pairs(&self, pair: Pair) -> Vec<(Token, Token)> {
        let Some(mut index) = self.actual_last_token_index(pair.prev) else {
            return Vec::new();
        };
        let mut prev = self.tokens[index];
        let mut pairs = Vec::new();
        if self
            .lines
            .line_of(pair.next.report_span.start as usize)
            .saturating_sub(self.lines.line_of(prev.end))
            < 2
        {
            return pairs;
        }
        loop {
            index += 1;
            let Some(next) = self.tokens.get(index).copied() else {
                break;
            };
            if self
                .lines
                .line_of(next.start)
                .saturating_sub(self.lines.line_of(prev.end))
                >= 2
            {
                pairs.push((prev, next));
            }
            prev = next;
            if prev.start >= pair.next.report_span.start as usize {
                break;
            }
        }
        pairs
    }

    fn always_fix(&self, pair: Pair) -> (usize, &'static str) {
        let Some(mut index) = self.actual_last_token_index(pair.prev) else {
            return (pair.prev.report_span.end as usize, "\n");
        };
        let mut prev = self.tokens[index];
        let mut next_start = pair.next.report_span.start as usize;

        while let Some(next) = self.tokens.get(index + 1).copied() {
            if next.start >= pair.next.report_span.start as usize {
                next_start = next.start;
                break;
            }
            if self.lines.same_line(prev.end, next.start) {
                prev = next;
                index += 1;
            } else {
                next_start = next.start;
                break;
            }
        }
        let text = if self.lines.same_line(prev.end, next_start) {
            "\n\n"
        } else {
            "\n"
        };
        (prev.end, text)
    }

    fn actual_last_token_index(&self, candidate: Candidate) -> Option<usize> {
        let start = candidate.report_span.start as usize;
        let end = candidate.report_span.end as usize;
        let mut index = self
            .tokens
            .iter()
            .rposition(|token| token.start >= start && token.end <= end)?;
        let token = self.tokens[index];
        if token.text(self.source) == ";"
            && index > 0
            && let Some(next) = self.tokens.get(index + 1)
            && self.tokens[index - 1].start >= start
            && !self
                .lines
                .same_line(self.tokens[index - 1].end, token.start)
            && self.lines.same_line(token.end, next.start)
        {
            index -= 1;
        }
        Some(index)
    }

    fn report(
        &mut self,
        candidate: Candidate,
        message_id: &str,
        message: &str,
        fix: Option<LintFix>,
    ) {
        let span = if self.is_single_line(candidate.report_span) {
            candidate.report_span
        } else {
            Span::new(
                candidate.report_span.start,
                self.lines.line_end(candidate.report_span.start as usize) as u32,
            )
        };
        let suggestions = fix.map_or_else(Vec::new, |fix| {
            std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect()
        });
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: TextRange::new(span.start, span.end),
            suggestions,
        });
    }

    fn candidate_text(&self, candidate: Candidate) -> &str {
        &self.source[candidate.match_span.start as usize..candidate.match_span.end as usize]
    }

    fn is_single_line(&self, span: Span) -> bool {
        self.lines.same_line(span.start as usize, span.end as usize)
    }
}

fn candidate_from_ast(kind: AstKind<'_>, source: &str) -> Option<Candidate> {
    let report_span = kind.span();
    let (match_span, node_kind) = match kind {
        AstKind::Directive(_) => (report_span, NodeKind::Directive),
        AstKind::BlockStatement(_) => (report_span, NodeKind::Block),
        AstKind::EmptyStatement(_) => (report_span, NodeKind::Empty),
        AstKind::ExpressionStatement(_) => (report_span, NodeKind::Expression),
        AstKind::Function(function) => (
            report_span,
            if function.r#type == FunctionType::TSDeclareFunction {
                NodeKind::FunctionOverload
            } else {
                NodeKind::Function
            },
        ),
        AstKind::Class(class) if class.r#type == ClassType::ClassDeclaration => {
            (report_span, NodeKind::Class)
        }
        AstKind::BreakStatement(_) => (report_span, NodeKind::Break),
        AstKind::ContinueStatement(_) => (report_span, NodeKind::Continue),
        AstKind::DebuggerStatement(_) => (report_span, NodeKind::Debugger),
        AstKind::DoWhileStatement(_) => (report_span, NodeKind::Do),
        AstKind::ForStatement(_) | AstKind::ForInStatement(_) | AstKind::ForOfStatement(_) => {
            (report_span, NodeKind::For)
        }
        AstKind::IfStatement(_) => (report_span, NodeKind::If),
        AstKind::ReturnStatement(_) => (report_span, NodeKind::Return),
        AstKind::SwitchStatement(_) => (report_span, NodeKind::Switch),
        AstKind::ThrowStatement(_) => (report_span, NodeKind::Throw),
        AstKind::TryStatement(_) => (report_span, NodeKind::Try),
        AstKind::WhileStatement(_) => (report_span, NodeKind::While),
        AstKind::WithStatement(_) => (report_span, NodeKind::With),
        AstKind::ImportDeclaration(_) => (report_span, NodeKind::Import),
        AstKind::ExportNamedDeclaration(_) => (
            report_span,
            export_kind(&source[report_span.start as usize..report_span.end as usize]),
        ),
        AstKind::ExportDefaultDeclaration(_) | AstKind::ExportAllDeclaration(_) => {
            (report_span, NodeKind::Export)
        }
        AstKind::VariableDeclaration(declaration) => (
            report_span,
            match declaration.kind {
                oxc_ast::ast::VariableDeclarationKind::Var => NodeKind::VariableVar,
                oxc_ast::ast::VariableDeclarationKind::Let => NodeKind::VariableLet,
                oxc_ast::ast::VariableDeclarationKind::Const => NodeKind::VariableConst,
                oxc_ast::ast::VariableDeclarationKind::Using
                | oxc_ast::ast::VariableDeclarationKind::AwaitUsing => NodeKind::VariableUsing,
            },
        ),
        AstKind::TSTypeAliasDeclaration(_) => (report_span, NodeKind::TypeAlias),
        AstKind::TSInterfaceDeclaration(_) => (report_span, NodeKind::Interface),
        AstKind::TSEnumDeclaration(_) => (report_span, NodeKind::Enum),
        AstKind::SwitchCase(case) => (
            report_span,
            if case.test.is_some() {
                NodeKind::SwitchCase
            } else {
                NodeKind::SwitchDefault
            },
        ),
        AstKind::TSMethodSignature(_) => (report_span, NodeKind::TSMethod),
        AstKind::LabeledStatement(label) => {
            let mut statement = &label.body;
            while let oxc_ast::ast::Statement::LabeledStatement(inner) = statement {
                statement = &inner.body;
            }
            let match_span = statement.span();
            (
                match_span,
                kind_from_text(&source[match_span.start as usize..match_span.end as usize]),
            )
        }
        _ => return None,
    };
    Some(Candidate {
        report_span,
        match_span,
        kind: node_kind,
    })
}

fn kind_from_text(text: &str) -> NodeKind {
    let text = text.trim_start();
    if text.starts_with('{') {
        NodeKind::Block
    } else if text.starts_with(';') {
        NodeKind::Empty
    } else if text.starts_with("break") {
        NodeKind::Break
    } else if text.starts_with("continue") {
        NodeKind::Continue
    } else if text.starts_with("debugger") {
        NodeKind::Debugger
    } else if text.starts_with("do") {
        NodeKind::Do
    } else if text.starts_with("for") {
        NodeKind::For
    } else if text.starts_with("if") {
        NodeKind::If
    } else if text.starts_with("return") {
        NodeKind::Return
    } else if text.starts_with("switch") {
        NodeKind::Switch
    } else if text.starts_with("throw") {
        NodeKind::Throw
    } else if text.starts_with("try") {
        NodeKind::Try
    } else if text.starts_with("while") {
        NodeKind::While
    } else if text.starts_with("with") {
        NodeKind::With
    } else if text.starts_with("function") {
        if text.contains('{') {
            NodeKind::Function
        } else {
            NodeKind::FunctionOverload
        }
    } else if text.starts_with("class") {
        NodeKind::Class
    } else if text.starts_with("var") {
        NodeKind::VariableVar
    } else if text.starts_with("let") {
        NodeKind::VariableLet
    } else if text.starts_with("const") {
        NodeKind::VariableConst
    } else if text.starts_with("using") || text.starts_with("await using") {
        NodeKind::VariableUsing
    } else if text.starts_with("import") {
        NodeKind::Import
    } else if text.starts_with("export") {
        export_kind(text)
    } else if text.starts_with("type") {
        NodeKind::TypeAlias
    } else if text.starts_with("interface") {
        NodeKind::Interface
    } else if text.starts_with("enum") {
        NodeKind::Enum
    } else {
        NodeKind::Expression
    }
}

fn export_kind(text: &str) -> NodeKind {
    let rest = text
        .trim_start()
        .strip_prefix("export")
        .unwrap_or(text)
        .trim_start();
    if rest.starts_with("function") {
        if rest.contains('{') {
            NodeKind::ExportNamedFunction
        } else {
            NodeKind::ExportNamedOverload
        }
    } else {
        NodeKind::Export
    }
}

fn stack_kind(kind: AstKind<'_>) -> StackKind {
    match kind {
        AstKind::Program(_) => StackKind::Program,
        AstKind::BlockStatement(_) => StackKind::Block,
        AstKind::FunctionBody(_) => StackKind::FunctionBody,
        AstKind::SwitchStatement(_) => StackKind::Switch,
        AstKind::SwitchCase(_) => StackKind::SwitchCase,
        AstKind::StaticBlock(_) => StackKind::StaticBlock,
        AstKind::TSInterfaceBody(_) => StackKind::TSInterfaceBody,
        AstKind::TSModuleBlock(_) => StackKind::TSModuleBlock,
        AstKind::TSTypeLiteral(_) => StackKind::TSTypeLiteral,
        AstKind::Function(function) if function.r#type == FunctionType::TSDeclareFunction => {
            StackKind::TSDeclareFunction
        }
        AstKind::TSMethodSignature(_) => StackKind::TSMethod,
        _ => StackKind::Other,
    }
}

fn opens_scope(kind: StackKind) -> bool {
    !matches!(kind, StackKind::Other)
}

fn is_candidate_parent(parent: Option<StackKind>) -> bool {
    parent.is_some_and(opens_scope)
}

fn candidate_allowed_by_parent(parent: Option<StackKind>, kind: NodeKind) -> bool {
    match parent {
        Some(StackKind::Switch) => {
            matches!(kind, NodeKind::SwitchCase | NodeKind::SwitchDefault)
        }
        Some(StackKind::TSInterfaceBody | StackKind::TSTypeLiteral) => kind == NodeKind::TSMethod,
        Some(
            StackKind::Program
            | StackKind::Block
            | StackKind::FunctionBody
            | StackKind::SwitchCase
            | StackKind::StaticBlock
            | StackKind::TSModuleBlock,
        ) => !matches!(
            kind,
            NodeKind::SwitchCase | NodeKind::SwitchDefault | NodeKind::TSMethod
        ),
        _ => false,
    }
}

fn first_initializer(text: &str) -> Option<&str> {
    text.split_once('=').map(|(_, init)| init.trim_start())
}

fn require_call(initializer: &str, strict: bool) -> bool {
    if strict {
        initializer.starts_with("require(")
    } else {
        initializer.starts_with("require(") || initializer.starts_with("require (")
    }
}

fn is_iife(text: &str) -> bool {
    let text = text.trim_end().trim_end_matches(';').trim_end();
    if !(text.contains("function") || text.contains("=>")) {
        return false;
    }
    text.rsplit_once('}')
        .is_some_and(|(_, suffix)| suffix.contains('('))
}

fn function_name(text: &str) -> Option<&str> {
    text.trim_start()
        .strip_prefix("function")?
        .trim_start()
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '$')
        })
        .next()
        .filter(|name| !name.is_empty())
}

fn call_name(text: &str) -> Option<&str> {
    let text = text.trim_start();
    let end = text.find('(')?;
    let name = text[..end].trim();
    (!name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'))
    .then_some(name)
}

struct Lines {
    starts: Vec<usize>,
    ends: Vec<usize>,
}

impl Lines {
    fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut starts = Vec::new();
        starts.push(0);
        let mut ends = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    ends.push(index);
                    index += 2;
                    starts.push(index);
                }
                b'\r' | b'\n' => {
                    ends.push(index);
                    index += 1;
                    starts.push(index);
                }
                0xe2 if bytes.get(index + 1) == Some(&0x80)
                    && matches!(bytes.get(index + 2), Some(0xa8) | Some(0xa9)) =>
                {
                    ends.push(index);
                    index += 3;
                    starts.push(index);
                }
                _ => index += 1,
            }
        }
        if ends.len() < starts.len() {
            ends.push(source.len());
        }
        Self { starts, ends }
    }

    fn line_of(&self, offset: usize) -> usize {
        self.starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1)
    }

    fn same_line(&self, left: usize, right: usize) -> bool {
        self.line_of(left) == self.line_of(right)
    }

    fn line_end(&self, offset: usize) -> usize {
        let line = self.line_of(offset);
        self.ends.get(line).copied().unwrap_or(offset)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::Value;

    use super::check_padding_line_between_statements;
    use crate::{LintDiagnostic, TextRange};

    #[derive(Deserialize)]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<TestCase>,
        invalid: Vec<TestCase>,
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
        unfixable_invalid: usize,
        total: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TestCase {
        code: String,
        language: String,
        options: Value,
        #[serde(default)]
        parser_options: Value,
        #[serde(default)]
        errors: Vec<ExpectedError>,
        #[serde(default)]
        output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedError {
        message_id: String,
        message: String,
        line: usize,
        column: usize,
        end_line: usize,
        end_column: usize,
        fix: Option<ExpectedFix>,
    }

    #[derive(Debug, Deserialize)]
    struct ExpectedFix {
        range: [usize; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/padding-line-between-statements-v5.10.0.json"
        ))
        .expect("generated padding-line-between-statements fixture is valid JSON")
    }

    fn filename(test_case: &TestCase) -> &'static str {
        if test_case.language == "typescript" {
            "fixture.ts"
        } else if test_case
            .parser_options
            .pointer("/ecmaFeatures/jsx")
            .and_then(Value::as_bool)
            == Some(true)
        {
            "fixture.jsx"
        } else {
            "fixture.js"
        }
    }

    fn run(test_case: &TestCase, code: &str) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_padding_line_between_statements(
            code,
            Some(filename(test_case)),
            &test_case.options,
            &mut diagnostics,
        );
        diagnostics
    }

    fn line_column_range(code: &str, error: &ExpectedError) -> TextRange {
        TextRange::new(
            line_column_to_byte(code, error.line, error.column) as u32,
            line_column_to_byte(code, error.end_line, error.end_column) as u32,
        )
    }

    fn line_column_to_byte(code: &str, line: usize, column: usize) -> usize {
        let mut current_line = 1;
        let mut line_start = 0;
        let bytes = code.as_bytes();
        let mut index = 0;
        while current_line < line && index < bytes.len() {
            match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    index += 2;
                    current_line += 1;
                    line_start = index;
                }
                b'\r' | b'\n' => {
                    index += 1;
                    current_line += 1;
                    line_start = index;
                }
                0xe2 if bytes.get(index + 1) == Some(&0x80)
                    && matches!(bytes.get(index + 2), Some(0xa8) | Some(0xa9)) =>
                {
                    index += 3;
                    current_line += 1;
                    line_start = index;
                }
                _ => index += 1,
            }
        }
        let utf16_column = column.saturating_sub(1);
        line_start
            + code[line_start..]
                .char_indices()
                .take_while(|(_, character)| {
                    !matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}')
                })
                .scan(0, |units, (offset, character)| {
                    let before = *units;
                    *units += character.len_utf16();
                    Some((before, offset))
                })
                .find_map(|(units, offset)| (units == utf16_column).then_some(offset))
                .unwrap_or_else(|| {
                    code[line_start..]
                        .find(['\r', '\n', '\u{2028}', '\u{2029}'])
                        .unwrap_or(code.len() - line_start)
                })
    }

    fn utf16_offset_to_byte(code: &str, offset: usize) -> usize {
        let mut units = 0;
        for (byte, character) in code.char_indices() {
            if units == offset {
                return byte;
            }
            units += character.len_utf16();
        }
        code.len()
    }

    fn apply_fixes(code: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| (fix.range.start, fix.range.end));
        let mut output = String::with_capacity(code.len());
        let mut cursor = 0;
        for fix in fixes {
            let start = fix.range.start as usize;
            let end = fix.range.end as usize;
            if start < cursor {
                continue;
            }
            output.push_str(&code[cursor..start]);
            output.push_str(&fix.replacement_text);
            cursor = end;
        }
        output.push_str(&code[cursor..]);
        Some(output)
    }

    fn converge(test_case: &TestCase) -> Option<String> {
        let mut code = test_case.code.clone();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(test_case, &code);
            let Some(output) = apply_fixes(&code, &diagnostics) else {
                break;
            };
            if output == code {
                break;
            }
            changed = true;
            code = output;
        }
        changed.then_some(code)
    }

    #[test]
    fn pinned_inventory_is_complete() {
        let fixture = fixture();
        assert_eq!(fixture.generated.inventory.valid, 419);
        assert_eq!(fixture.generated.inventory.invalid, 323);
        assert_eq!(fixture.generated.inventory.diagnostics, 339);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 323);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 0);
        assert_eq!(fixture.generated.inventory.total, 742);
    }

    #[test]
    fn accepts_every_upstream_valid_case() {
        let fixture = fixture();
        for (index, test_case) in fixture.valid.iter().enumerate() {
            let diagnostics = run(test_case, &test_case.code);
            assert!(
                diagnostics.is_empty(),
                "valid case {index} produced diagnostics for {:?}: {diagnostics:#?}",
                test_case.code
            );
        }
    }

    #[test]
    fn replays_every_upstream_invalid_case_exactly() {
        let fixture = fixture();
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(test_case, &test_case.code);
            assert_eq!(
                diagnostics.len(),
                test_case.errors.len(),
                "invalid case {index}: {:?}\n{diagnostics:#?}",
                test_case.code
            );
            for (diagnostic, expected) in diagnostics.iter().zip(&test_case.errors) {
                assert_eq!(
                    diagnostic.message_id, expected.message_id,
                    "invalid case {index}"
                );
                assert_eq!(diagnostic.message, expected.message, "invalid case {index}");
                assert_eq!(
                    diagnostic.range,
                    line_column_range(&test_case.code, expected),
                    "invalid case {index}"
                );
                match (&diagnostic.suggestions.first(), &expected.fix) {
                    (Some(suggestion), Some(fix)) => {
                        assert_eq!(suggestion.message_id, expected.message_id);
                        assert_eq!(suggestion.message, expected.message);
                        assert_eq!(suggestion.fixes.len(), 1);
                        assert_eq!(
                            suggestion.fixes[0].range,
                            TextRange::new(
                                utf16_offset_to_byte(&test_case.code, fix.range[0]) as u32,
                                utf16_offset_to_byte(&test_case.code, fix.range[1]) as u32,
                            ),
                            "invalid case {index}"
                        );
                        assert_eq!(
                            suggestion.fixes[0].replacement_text, fix.text,
                            "invalid case {index}"
                        );
                    }
                    (None, None) => {}
                    (actual, expected) => {
                        panic!("invalid case {index} fix mismatch: {actual:?} / {expected:?}")
                    }
                }
            }
            assert_eq!(
                converge(test_case),
                test_case.output,
                "invalid case {index}: {:?}",
                test_case.code
            );
        }
    }

    #[test]
    fn multiple_comment_separated_padding_sequences_are_unfixable() {
        let test_case = TestCase {
            code: "foo();\n\n// one\n\nbar();".to_owned(),
            language: "javascript".to_owned(),
            options: serde_json::from_str(r#"[{"blankLine":"never","prev":"*","next":"*"}]"#)
                .expect("inline options are valid JSON"),
            parser_options: Value::Null,
            errors: Vec::new(),
            output: None,
        };
        let diagnostics = run(&test_case, &test_case.code);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message_id, super::UNEXPECTED_ID);
        assert!(diagnostics[0].suggestions.is_empty());
    }
}
