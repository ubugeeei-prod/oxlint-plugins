//! Native implementation of `@stylistic/curly-newline`.
//!
//! The stable rule applies only to statement, function, class, switch, static,
//! and TypeScript module bodies. Object literals and patterns deliberately do
//! not participate. Oxc supplies the structural classification while the
//! shared lexer supplies exact comment-aware brace boundaries.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::*};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::first_option;
use super::lexer::{Token, tokenize};

const RULE: &str = "curly-newline";

const UNEXPECTED_BEFORE: &str = "unexpectedLinebreakBeforeClosingBrace";
const UNEXPECTED_AFTER: &str = "unexpectedLinebreakAfterOpeningBrace";
const EXPECTED_BEFORE: &str = "expectedLinebreakBeforeClosingBrace";
const EXPECTED_AFTER: &str = "expectedLinebreakAfterOpeningBrace";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Specialization {
    IfStatementConsequent,
    IfStatementAlternative,
    DoWhileStatement,
    ForInStatement,
    ForOfStatement,
    ForStatement,
    WhileStatement,
    SwitchStatement,
    SwitchCase,
    TryStatementBlock,
    TryStatementHandler,
    TryStatementFinalizer,
    BlockStatement,
    ArrowFunctionExpression,
    FunctionDeclaration,
    FunctionExpression,
    Property,
    ClassBody,
    StaticBlock,
    WithStatement,
    TSModuleBlock,
}

impl Specialization {
    const fn key(self) -> &'static str {
        match self {
            Self::IfStatementConsequent => "IfStatementConsequent",
            Self::IfStatementAlternative => "IfStatementAlternative",
            Self::DoWhileStatement => "DoWhileStatement",
            Self::ForInStatement => "ForInStatement",
            Self::ForOfStatement => "ForOfStatement",
            Self::ForStatement => "ForStatement",
            Self::WhileStatement => "WhileStatement",
            Self::SwitchStatement => "SwitchStatement",
            Self::SwitchCase => "SwitchCase",
            Self::TryStatementBlock => "TryStatementBlock",
            Self::TryStatementHandler => "TryStatementHandler",
            Self::TryStatementFinalizer => "TryStatementFinalizer",
            Self::BlockStatement => "BlockStatement",
            Self::ArrowFunctionExpression => "ArrowFunctionExpression",
            Self::FunctionDeclaration => "FunctionDeclaration",
            Self::FunctionExpression => "FunctionExpression",
            Self::Property => "Property",
            Self::ClassBody => "ClassBody",
            Self::StaticBlock => "StaticBlock",
            Self::WithStatement => "WithStatement",
            Self::TSModuleBlock => "TSModuleBlock",
        }
    }
}

#[derive(Clone, Copy)]
struct Options {
    consistent: bool,
    multiline: bool,
    min_elements: usize,
}

impl Options {
    const DEFAULT: Self = Self {
        consistent: true,
        multiline: false,
        min_elements: usize::MAX,
    };

    fn from_value(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::DEFAULT;
        };

        if let Some(keyword) = value.as_str() {
            return match keyword {
                "always" => Self {
                    consistent: false,
                    multiline: false,
                    min_elements: 0,
                },
                "never" => Self {
                    consistent: false,
                    multiline: false,
                    min_elements: usize::MAX,
                },
                _ => Self::DEFAULT,
            };
        }

        let Some(object) = value.as_object() else {
            return Self::DEFAULT;
        };
        Self {
            consistent: object
                .get("consistent")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            multiline: object
                .get("multiline")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            min_elements: object
                .get("minElements")
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(usize::MAX),
        }
    }
}

struct RuleOptions<'options> {
    root: Option<&'options Value>,
    base: Options,
}

impl<'options> RuleOptions<'options> {
    fn from_json(options: &'options Value) -> Self {
        let root = first_option(options);
        Self {
            root,
            base: Options::from_value(root),
        }
    }

    fn for_specialization(&self, specialization: Specialization) -> Options {
        let specialized = self
            .root
            .and_then(Value::as_object)
            .and_then(|object| object.get(specialization.key()));
        specialized.map_or(self.base, |value| Options::from_value(Some(value)))
    }
}

pub(crate) fn check_curly_newline(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let options = RuleOptions::from_json(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|value| SourceType::from_path(value).ok()) {
        let _ = parse_and_check(source, source_type, &tokens, &options, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::ts(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, &tokens, &options, diagnostics) {
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
    tokens: &[Token],
    options: &RuleOptions<'_>,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = CurlyNewlineVisitor {
        source,
        tokens,
        options,
        ancestors: Vec::new(),
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct CurlyNewlineVisitor<'ast, 'source, 'options, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: &'options RuleOptions<'options>,
    ancestors: Vec<AstKind<'ast>>,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for CurlyNewlineVisitor<'ast, '_, '_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        self.ancestors.push(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'ast>) {
        self.ancestors.pop();
    }

    fn visit_block_statement(&mut self, block: &BlockStatement<'ast>) {
        self.check(
            block.span,
            block.body.len(),
            self.block_specialization(block),
        );
        walk::walk_block_statement(self, block);
    }

    fn visit_function_body(&mut self, body: &FunctionBody<'ast>) {
        if let Some(specialization) = self.function_specialization() {
            self.check(
                body.span,
                body.directives.len() + body.statements.len(),
                specialization,
            );
        }
        walk::walk_function_body(self, body);
    }

    fn visit_switch_statement(&mut self, statement: &SwitchStatement<'ast>) {
        self.check(
            statement.span,
            statement.cases.len(),
            Specialization::SwitchStatement,
        );
        walk::walk_switch_statement(self, statement);
    }

    fn visit_class_body(&mut self, body: &ClassBody<'ast>) {
        self.check(body.span, body.body.len(), Specialization::ClassBody);
        walk::walk_class_body(self, body);
    }

    fn visit_static_block(&mut self, block: &StaticBlock<'ast>) {
        self.check(block.span, block.body.len(), Specialization::StaticBlock);
        walk::walk_static_block(self, block);
    }

    fn visit_ts_module_block(&mut self, block: &TSModuleBlock<'ast>) {
        self.check(
            block.span,
            block.directives.len() + block.body.len(),
            Specialization::TSModuleBlock,
        );
        walk::walk_ts_module_block(self, block);
    }
}

impl CurlyNewlineVisitor<'_, '_, '_, '_> {
    fn block_specialization(&self, block: &BlockStatement<'_>) -> Specialization {
        let Some(parent) = self.ancestors.last() else {
            return Specialization::BlockStatement;
        };
        match parent {
            AstKind::IfStatement(statement) if statement.consequent.span() == block.span => {
                Specialization::IfStatementConsequent
            }
            AstKind::IfStatement(statement)
                if statement
                    .alternate
                    .as_ref()
                    .is_some_and(|alternate| alternate.span() == block.span) =>
            {
                Specialization::IfStatementAlternative
            }
            AstKind::DoWhileStatement(_) => Specialization::DoWhileStatement,
            AstKind::ForInStatement(_) => Specialization::ForInStatement,
            AstKind::ForOfStatement(_) => Specialization::ForOfStatement,
            AstKind::ForStatement(_) => Specialization::ForStatement,
            AstKind::WhileStatement(_) => Specialization::WhileStatement,
            AstKind::WithStatement(_) => Specialization::WithStatement,
            AstKind::TryStatement(statement) if statement.block.span == block.span => {
                Specialization::TryStatementBlock
            }
            AstKind::TryStatement(statement)
                if statement
                    .finalizer
                    .as_ref()
                    .is_some_and(|finalizer| finalizer.span == block.span) =>
            {
                Specialization::TryStatementFinalizer
            }
            AstKind::CatchClause(_) => Specialization::TryStatementHandler,
            AstKind::SwitchCase(case)
                if case.consequent.len() == 1 && case.consequent[0].span() == block.span =>
            {
                Specialization::SwitchCase
            }
            _ => Specialization::BlockStatement,
        }
    }

    fn function_specialization(&self) -> Option<Specialization> {
        match self.ancestors.last()? {
            AstKind::ArrowFunctionExpression(arrow) if !arrow.expression => {
                Some(Specialization::ArrowFunctionExpression)
            }
            AstKind::Function(function) => match function.r#type {
                FunctionType::FunctionDeclaration => Some(Specialization::FunctionDeclaration),
                FunctionType::FunctionExpression => {
                    let property_method = self
                        .ancestors
                        .iter()
                        .rev()
                        .nth(1)
                        .is_some_and(|ancestor| {
                            matches!(ancestor, AstKind::ObjectProperty(property) if property.method)
                        });
                    Some(if property_method {
                        Specialization::Property
                    } else {
                        Specialization::FunctionExpression
                    })
                }
                FunctionType::TSDeclareFunction | FunctionType::TSEmptyBodyFunctionExpression => {
                    None
                }
            },
            _ => None,
        }
    }

    fn check(&mut self, span: Span, element_count: usize, specialization: Specialization) {
        let Some(braces) = BraceTokens::find(self.tokens, span, self.source) else {
            return;
        };
        let options = self.options.for_specialization(specialization);
        let first_with_comments = braces.open + 1;
        let last_with_comments = braces.close - 1;
        let has_comments_first = self.tokens[first_with_comments].kind.is_comment();
        let has_comments_last = self.tokens[last_with_comments].kind.is_comment();
        let first = (braces.open + 1..=braces.close)
            .find(|&index| !self.tokens[index].kind.is_comment())
            .unwrap_or(braces.close);
        let last = (braces.open..braces.close)
            .rev()
            .find(|&index| !self.tokens[index].kind.is_comment())
            .unwrap_or(braces.open);

        let needs_linebreaks = element_count >= options.min_elements
            || (options.multiline
                && element_count > 0
                && !same_line(
                    self.source,
                    &self.tokens[last_with_comments],
                    &self.tokens[first_with_comments],
                ));
        let opening_break = !same_line(self.source, &self.tokens[braces.open], &self.tokens[first]);
        let closing_break = !same_line(self.source, &self.tokens[last], &self.tokens[braces.close]);

        if needs_linebreaks {
            if !opening_break {
                let fix = (!has_comments_first).then(|| {
                    LintFix::replace_range(
                        byte_range(self.tokens[braces.open].end, self.tokens[braces.open].end),
                        "\n",
                    )
                });
                self.report(
                    braces.open,
                    EXPECTED_AFTER,
                    "Expected a line break after this opening brace.",
                    fix,
                );
            }
            if !closing_break {
                let fix = (!has_comments_last).then(|| {
                    LintFix::replace_range(
                        byte_range(
                            self.tokens[braces.close].start,
                            self.tokens[braces.close].start,
                        ),
                        "\n",
                    )
                });
                self.report(
                    braces.close,
                    EXPECTED_BEFORE,
                    "Expected a line break before this closing brace.",
                    fix,
                );
            }
            return;
        }

        if opening_break && (!options.consistent || !closing_break) {
            let fix = (!has_comments_first).then(|| {
                LintFix::remove_range(byte_range(
                    self.tokens[braces.open].end,
                    self.tokens[first].start,
                ))
            });
            self.report(
                braces.open,
                UNEXPECTED_AFTER,
                "Unexpected line break after this opening brace.",
                fix,
            );
        }
        if closing_break && (!options.consistent || !opening_break) {
            let fix = (!has_comments_last).then(|| {
                LintFix::remove_range(byte_range(
                    self.tokens[last].end,
                    self.tokens[braces.close].start,
                ))
            });
            self.report(
                braces.close,
                UNEXPECTED_BEFORE,
                "Unexpected line break before this closing brace.",
                fix,
            );
        }
    }

    fn report(
        &mut self,
        token_index: usize,
        message_id: &str,
        message: &str,
        fix: Option<LintFix>,
    ) {
        let token = &self.tokens[token_index];
        let suggestions = fix
            .map(|fix| LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .into_iter()
            .collect();
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: BTreeMap::new(),
            range: byte_range(token.start, token.end),
            suggestions,
        });
    }
}

#[derive(Clone, Copy)]
struct BraceTokens {
    open: usize,
    close: usize,
}

impl BraceTokens {
    fn find(tokens: &[Token], span: Span, source: &str) -> Option<Self> {
        let start = usize::try_from(span.start).ok()?;
        let end = usize::try_from(span.end).ok()?;
        let close = tokens
            .iter()
            .rposition(|token| token.end == end && token.text(source) == "}")?;
        let mut depth = 0usize;
        for index in (0..=close).rev() {
            let token = &tokens[index];
            if token.start < start {
                break;
            }
            match token.text(source) {
                "}" => depth += 1,
                "{" => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(Self { open: index, close });
                    }
                }
                _ => {}
            }
        }
        None
    }
}

fn same_line(source: &str, left: &Token, right: &Token) -> bool {
    line_number(source, left.end) == line_number(source, right.start)
}

fn line_number(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .chars()
        .filter(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        .count()
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the upstream option matrix readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_curly_newline(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(source: &str, options: Value) -> Vec<String> {
        run(source, None, options)
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    #[test]
    fn covers_default_always_never_multiline_min_elements_and_consistency() {
        assert!(run("{}", None, Value::Null).is_empty());
        assert!(run("{\n}", None, Value::Null).is_empty());
        assert_eq!(
            ids("{}", json!(["always"])),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert_eq!(
            ids("{\nvoid 0\n}", json!(["never"])),
            [UNEXPECTED_AFTER, UNEXPECTED_BEFORE]
        );
        assert_eq!(
            ids("{void 0\n}", json!([{ "consistent": true }])),
            [UNEXPECTED_BEFORE]
        );
        assert_eq!(
            ids("{void 0;\nvoid 0}", json!([{ "multiline": true }])),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert_eq!(
            ids("{void 0;void 0}", json!([{ "minElements": 2 }])),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
    }

    #[test]
    fn classifies_every_stable_specialization() {
        let cases = [
            ("if(true){}", "IfStatementConsequent"),
            ("if(true){}else{}", "IfStatementAlternative"),
            ("do{}while(true)", "DoWhileStatement"),
            ("for(const x in y){}", "ForInStatement"),
            ("for(const x of y){}", "ForOfStatement"),
            ("for(;;){}", "ForStatement"),
            ("while(true){}", "WhileStatement"),
            ("switch(x){}", "SwitchStatement"),
            ("switch(x){case 0:{}}", "SwitchCase"),
            ("try{}catch(e){}", "TryStatementBlock"),
            ("try{}catch(e){}", "TryStatementHandler"),
            ("try{}finally{}", "TryStatementFinalizer"),
            ("{{}}", "BlockStatement"),
            ("(()=>{})", "ArrowFunctionExpression"),
            ("function f(){}", "FunctionDeclaration"),
            ("(function(){})", "FunctionExpression"),
            ("({m(){}})", "Property"),
            ("class C{}", "ClassBody"),
            ("class C{static{}}", "StaticBlock"),
            ("with(x){}", "WithStatement"),
            ("namespace N{}", "TSModuleBlock"),
        ];

        for (source, specialization) in cases {
            let diagnostics = run(
                source,
                (specialization == "TSModuleBlock").then_some("sample.ts"),
                json!([{ specialization: "always" }]),
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message_id == EXPECTED_AFTER),
                "{specialization}: {source}"
            );
        }
    }

    #[test]
    fn property_override_applies_only_to_object_methods() {
        assert_eq!(
            ids(
                "({ method() {}, value: function() {} });",
                json!([{ "Property": "always" }])
            ),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
    }

    #[test]
    fn preserves_comments_as_unfixable_boundaries() {
        let diagnostics = run("{/* first */void 0/* last */}", None, json!(["always"]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            [EXPECTED_AFTER, EXPECTED_BEFORE]
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.suggestions.is_empty())
        );
    }

    #[test]
    fn tracks_nested_blocks_without_treating_object_literals_as_blocks() {
        let diagnostics = run(
            "if ({ value: { nested: true } }) { while (ready) {} }",
            None,
            json!(["always"]),
        );
        assert_eq!(diagnostics.len(), 4);
    }

    #[test]
    fn supports_typescript_tsx_unicode_and_explicit_filenames() {
        assert_eq!(
            run("namespace 日本語 {}", Some("sample.ts"), json!(["always"])).len(),
            2
        );
        assert_eq!(
            run(
                "const view = <Box>{() => {}}</Box>;",
                Some("sample.tsx"),
                json!([{ "ArrowFunctionExpression": "always" }])
            )
            .len(),
            2
        );
    }

    #[test]
    fn recognizes_all_javascript_line_terminators() {
        for linebreak in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("{{{linebreak}void 0{linebreak}}}");
            assert!(run(&source, None, json!(["always"])).is_empty());
            assert_eq!(run(&source, None, json!(["never"])).len(), 2);
        }
    }

    #[test]
    fn reports_byte_ranges_after_utf8_prefixes_and_exact_fixes() {
        let source = "const 日本語 = true; if (日本語) {}";
        let diagnostics = run(source, None, json!(["always"]));
        let opening = source.rfind('{').unwrap();
        let closing = source.rfind('}').unwrap();
        assert_eq!(diagnostics[0].range, byte_range(opening, opening + 1));
        assert_eq!(diagnostics[1].range, byte_range(closing, closing + 1));
        assert_eq!(
            diagnostics[0].suggestions[0].fixes[0].range,
            byte_range(opening + 1, opening + 1)
        );
        assert_eq!(
            diagnostics[1].suggestions[0].fixes[0].range,
            byte_range(closing, closing)
        );
    }

    #[test]
    fn ignores_object_type_interface_enum_and_parse_failures() {
        for (source, filename) in [
            ("const object = {\nvalue: true\n};", None),
            ("type Shape = {\nvalue: boolean\n};", Some("sample.ts")),
            ("interface Shape {\nvalue: boolean\n}", Some("sample.ts")),
            ("enum Shape {\nValue\n}", Some("sample.ts")),
            ("if (true) {", None),
        ] {
            assert!(
                run(source, filename, json!(["never"])).is_empty(),
                "{source}"
            );
        }
    }
}
