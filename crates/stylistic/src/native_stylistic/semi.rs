//! Native implementation of stable `@stylistic/semi`.
//!
//! Oxc identifies exactly the statement and class-field nodes owned by the
//! upstream rule. The shared lexer supplies ESLint-compatible token/comment
//! boundaries for ASI hazard checks and `FixTracker`-style removal fixes.

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind, AstType,
    ast::{
        ExportDefaultDeclarationKind, ForStatementInit, ForStatementLeft, FunctionType,
        PropertyDefinition, PropertyDefinitionType, PropertyKey,
    },
};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::{
    context::first_option,
    lexer::{Token, TokenKind, tokenize},
};

const RULE: &str = "semi";
const MISSING_SEMI: &str = "Missing semicolon.";
const EXTRA_SEMI: &str = "Extra semicolon.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContinuationChars {
    Any,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug)]
struct Options {
    never: bool,
    omit_last_in_one_line_block: bool,
    omit_last_in_one_line_class_body: bool,
    continuation_chars: ContinuationChars,
}

impl Options {
    fn from_value(value: &Value) -> Self {
        let never = first_option(value).and_then(Value::as_str) == Some("never");
        let object = value.as_array().and_then(|items| items.get(1));
        let continuation_chars = match object
            .and_then(|item| item.get("beforeStatementContinuationChars"))
            .and_then(Value::as_str)
        {
            Some("always") => ContinuationChars::Always,
            Some("never") => ContinuationChars::Never,
            _ => ContinuationChars::Any,
        };
        Self {
            never,
            omit_last_in_one_line_block: object
                .and_then(|item| item.get("omitLastInOneLineBlock"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            omit_last_in_one_line_class_body: object
                .and_then(|item| item.get("omitLastInOneLineClassBody"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            continuation_chars,
        }
    }
}

pub(crate) fn check_semi(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let options = Options::from_value(options);
    let first_diagnostic = diagnostics.len();

    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok()) {
        let _ = parse_and_check(source, source_type, &tokens, options, diagnostics);
    } else {
        for source_type in [
            SourceType::tsx(),
            SourceType::ts(),
            SourceType::jsx().with_unambiguous(true),
            SourceType::jsx().with_script(true),
        ] {
            if parse_and_check(source, source_type, &tokens, options, diagnostics) {
                break;
            }
        }
    }

    diagnostics[first_diagnostic..].sort_by_key(|diagnostic| {
        (
            diagnostic.range.start,
            diagnostic.range.end,
            message_order(&diagnostic.message_id),
        )
    });
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    tokens: &[Token],
    options: Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut collector = SemiCollector {
        parents: Vec::new(),
        excluded_variable_declarations: Vec::new(),
        excluded_expression_statements: Vec::new(),
        arrow_block_closes: Vec::new(),
        targets: Vec::new(),
    };
    collector.visit_program(&parsed.program);
    let mut complete_tokens = tokens.to_vec();
    ensure_statement_semicolon_tokens(source, &collector.targets, &mut complete_tokens);

    let mut checker = SemiChecker {
        source,
        tokens: &complete_tokens,
        options,
        arrow_block_closes: &collector.arrow_block_closes,
        diagnostics,
    };
    for target in &collector.targets {
        checker.check_target(target);
    }
    true
}

#[derive(Clone, Copy, Debug)]
struct Parent {
    kind: AstType,
    span: Span,
}

#[derive(Clone, Copy, Debug)]
enum AsiHazardAfter {
    Always,
    Never,
    Return(bool),
}

#[derive(Clone, Copy, Debug)]
struct PropertyHazard {
    unsafe_uninitialized_name: bool,
}

#[derive(Clone, Copy, Debug)]
struct Target {
    span: Span,
    parent: Option<Parent>,
    asi_hazard_after: AsiHazardAfter,
    regular_property: Option<PropertyHazard>,
}

struct SemiCollector {
    parents: Vec<Parent>,
    excluded_variable_declarations: Vec<Span>,
    excluded_expression_statements: Vec<Span>,
    arrow_block_closes: Vec<u32>,
    targets: Vec<Target>,
}

impl<'ast> Visit<'ast> for SemiCollector {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        match kind {
            AstKind::ForStatement(statement) => {
                if let Some(ForStatementInit::VariableDeclaration(declaration)) = &statement.init {
                    self.excluded_variable_declarations.push(declaration.span);
                }
            }
            AstKind::ForInStatement(statement) => {
                if let ForStatementLeft::VariableDeclaration(declaration) = &statement.left {
                    self.excluded_variable_declarations.push(declaration.span);
                }
            }
            AstKind::ForOfStatement(statement) => {
                if let ForStatementLeft::VariableDeclaration(declaration) = &statement.left {
                    self.excluded_variable_declarations.push(declaration.span);
                }
            }
            AstKind::ArrowFunctionExpression(arrow) => {
                if arrow.expression {
                    self.excluded_expression_statements
                        .extend(arrow.body.statements.iter().map(GetSpan::span));
                } else {
                    self.arrow_block_closes
                        .push(arrow.body.span.end.saturating_sub(1));
                }
            }
            _ => {}
        }

        let parent = self.parents.last().copied();
        match kind {
            AstKind::VariableDeclaration(declaration)
                if !self
                    .excluded_variable_declarations
                    .contains(&declaration.span) =>
            {
                self.push(declaration.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::ExpressionStatement(statement)
                if !self
                    .excluded_expression_statements
                    .contains(&statement.span) =>
            {
                self.push(statement.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::ReturnStatement(statement) => {
                self.push(
                    statement.span,
                    parent,
                    AsiHazardAfter::Return(statement.argument.is_some()),
                    None,
                );
            }
            AstKind::ThrowStatement(statement) => {
                self.push(statement.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::DoWhileStatement(statement) => {
                self.push(statement.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::DebuggerStatement(statement) => {
                self.push(statement.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::BreakStatement(statement) => {
                self.push(statement.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::ContinueStatement(statement) => {
                self.push(statement.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::ImportDeclaration(declaration) => {
                self.push(declaration.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::ExportAllDeclaration(declaration) => {
                self.push(declaration.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::ExportNamedDeclaration(declaration) if declaration.declaration.is_none() => {
                self.push(declaration.span, parent, AsiHazardAfter::Never, None);
            }
            AstKind::ExportDefaultDeclaration(declaration)
                if !matches!(
                    declaration.declaration,
                    ExportDefaultDeclarationKind::FunctionDeclaration(_)
                        | ExportDefaultDeclarationKind::ClassDeclaration(_)
                        | ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)
                ) =>
            {
                self.push(declaration.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::PropertyDefinition(property) => {
                let regular = property.r#type == PropertyDefinitionType::PropertyDefinition;
                self.push(
                    property.span,
                    parent,
                    AsiHazardAfter::Always,
                    regular.then(|| property_hazard(property)),
                );
            }
            AstKind::AccessorProperty(property) => {
                self.push(property.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::Function(function)
                if matches!(
                    function.r#type,
                    FunctionType::TSDeclareFunction | FunctionType::TSEmptyBodyFunctionExpression
                ) =>
            {
                self.push(function.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::TSExportAssignment(assignment) => {
                self.push(assignment.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::TSImportEqualsDeclaration(declaration) => {
                self.push(declaration.span, parent, AsiHazardAfter::Always, None);
            }
            AstKind::TSTypeAliasDeclaration(declaration) => {
                self.push(declaration.span, parent, AsiHazardAfter::Always, None);
            }
            _ => {}
        }

        self.parents.push(Parent {
            kind: kind.ty(),
            span: kind.span(),
        });
    }

    fn leave_node(&mut self, _kind: AstKind<'ast>) {
        self.parents.pop();
    }
}

impl SemiCollector {
    fn push(
        &mut self,
        span: Span,
        parent: Option<Parent>,
        asi_hazard_after: AsiHazardAfter,
        regular_property: Option<PropertyHazard>,
    ) {
        self.targets.push(Target {
            span,
            parent,
            asi_hazard_after,
            regular_property,
        });
    }
}

fn property_hazard(property: &PropertyDefinition<'_>) -> PropertyHazard {
    let identifier_name = if property.computed {
        None
    } else if let PropertyKey::StaticIdentifier(identifier) = &property.key {
        Some(identifier.name.as_str())
    } else {
        None
    };
    let unsafe_name = identifier_name.is_some_and(|name| matches!(name, "get" | "set" | "static"));
    let static_static = property.r#static && identifier_name == Some("static");
    PropertyHazard {
        unsafe_uninitialized_name: unsafe_name && !static_static && property.value.is_none(),
    }
}

fn ensure_statement_semicolon_tokens(source: &str, targets: &[Target], tokens: &mut Vec<Token>) {
    for target in targets {
        let end = target.span.end as usize;
        for start in [end.saturating_sub(1), end] {
            if source.as_bytes().get(start) != Some(&b';')
                || tokens
                    .iter()
                    .any(|token| token.start == start && token.end == start + 1)
            {
                continue;
            }
            tokens.push(Token {
                kind: TokenKind::Punctuator,
                start,
                end: start + 1,
            });
        }
    }
    tokens.sort_by_key(|token| (token.start, token.end));
}

struct SemiChecker<'source, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: Options,
    arrow_block_closes: &'source [u32],
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl SemiChecker<'_, '_> {
    fn check_target(&mut self, target: &Target) {
        let Some(last_index) = self.last_significant_token(target.span) else {
            return;
        };
        let is_semi = self.token_text(last_index) == ";";

        if self.options.never {
            if is_semi && self.can_remove_semicolon(target, last_index) {
                self.report_extra(last_index);
            } else if !is_semi
                && self.options.continuation_chars == ContinuationChars::Always
                && target.regular_property.is_none()
                && self
                    .next_significant_after(target.span.end)
                    .is_some_and(|next| self.maybe_asi_hazard_before(next))
            {
                self.report_missing(last_index);
            }
            return;
        }

        let one_line_exception = self.is_last_in_one_line_container(target, last_index);
        if is_semi && one_line_exception {
            self.report_extra(last_index);
        } else if !is_semi && !one_line_exception {
            self.report_missing(last_index);
        }
    }

    fn can_remove_semicolon(&self, target: &Target, semi_index: usize) -> bool {
        let next = self.next_significant_index(semi_index);
        if next.is_none() || next.is_some_and(|index| matches!(self.token_text(index), "}" | ";")) {
            return true;
        }

        if target.regular_property.is_some_and(|property| {
            property.unsafe_uninitialized_name
                || next.is_some_and(|index| {
                    matches!(self.token_text(index), "*" | "in" | "instanceof")
                })
        }) {
            return false;
        }

        let previous = self.previous_significant_index(semi_index);
        if previous
            .zip(next)
            .is_some_and(|(previous, next)| self.same_line(previous, next))
        {
            return false;
        }

        if target.regular_property.is_none()
            && self.options.continuation_chars == ContinuationChars::Never
            && !self.maybe_asi_hazard_after(target, previous)
        {
            return true;
        }

        next.is_some_and(|index| !self.maybe_asi_hazard_before(index))
    }

    fn maybe_asi_hazard_after(&self, target: &Target, previous: Option<usize>) -> bool {
        if previous.is_some_and(|index| {
            self.token_text(index) == "}"
                && u32::try_from(self.tokens[index].start)
                    .ok()
                    .is_some_and(|start| self.arrow_block_closes.contains(&start))
        }) {
            return false;
        }
        match target.asi_hazard_after {
            AsiHazardAfter::Always => true,
            AsiHazardAfter::Never => false,
            AsiHazardAfter::Return(has_argument) => has_argument,
        }
    }

    fn maybe_asi_hazard_before(&self, index: usize) -> bool {
        let text = self.token_text(index);
        text != "++"
            && text != "--"
            && text
                .as_bytes()
                .first()
                .is_some_and(|first| matches!(first, b'-' | b'[' | b'(' | b'/' | b'+' | b'`'))
    }

    fn is_last_in_one_line_container(&self, target: &Target, last_index: usize) -> bool {
        let Some(next) = self.next_significant_index(last_index) else {
            return false;
        };
        if self.token_text(next) != "}" {
            return false;
        }
        let Some(parent) = target.parent else {
            return false;
        };
        let option_enabled = match parent.kind {
            AstType::BlockStatement | AstType::FunctionBody | AstType::StaticBlock => {
                self.options.omit_last_in_one_line_block
            }
            AstType::ClassBody => self.options.omit_last_in_one_line_class_body,
            _ => false,
        };
        option_enabled && self.braces_are_single_line(parent.span)
    }

    fn braces_are_single_line(&self, span: Span) -> bool {
        let Some(open) = self.tokens.iter().find(|token| {
            token.start >= span.start as usize
                && token.end <= span.end as usize
                && token.text(self.source) == "{"
        }) else {
            return false;
        };
        let Some(close) = self.tokens.iter().rev().find(|token| {
            token.start >= span.start as usize
                && token.end <= span.end as usize
                && token.text(self.source) == "}"
        }) else {
            return false;
        };
        !has_line_terminator(&self.source[open.end..close.start])
    }

    fn last_significant_token(&self, span: Span) -> Option<usize> {
        let start = span.start as usize;
        let end = span.end as usize;
        let last = self
            .tokens
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, token)| {
                (!token.kind.is_comment() && token.start >= start && token.end <= end)
                    .then_some(index)
            })?;
        self.next_significant_index(last)
            .filter(|index| {
                self.tokens[*index].start == end && self.tokens[*index].text(self.source) == ";"
            })
            .or(Some(last))
    }

    fn next_significant_after(&self, offset: u32) -> Option<usize> {
        let offset = offset as usize;
        self.tokens.iter().enumerate().find_map(|(index, token)| {
            (!token.kind.is_comment() && token.start >= offset).then_some(index)
        })
    }

    fn next_significant_index(&self, index: usize) -> Option<usize> {
        self.tokens[index + 1..]
            .iter()
            .position(|token| !token.kind.is_comment())
            .map(|offset| index + offset + 1)
    }

    fn previous_significant_index(&self, index: usize) -> Option<usize> {
        self.tokens[..index]
            .iter()
            .rposition(|token| !token.kind.is_comment())
    }

    fn same_line(&self, left: usize, right: usize) -> bool {
        !has_line_terminator(
            &self.source
                [self.tokens[left].end.min(self.tokens[right].start)..self.tokens[right].start],
        )
    }

    fn token_text(&self, index: usize) -> &str {
        self.tokens[index].text(self.source)
    }

    fn report_missing(&mut self, last_index: usize) {
        let at = self.tokens[last_index].end;
        let end = next_location_byte(self.source, at);
        self.report(
            "missingSemi",
            MISSING_SEMI,
            at,
            end,
            LintFix::replace_range(byte_range(at, at), ";"),
        );
    }

    fn report_extra(&mut self, semi_index: usize) {
        let semi = self.tokens[semi_index];
        let previous = self.previous_significant_index(semi_index);
        let next = self.next_significant_index(semi_index);
        let fix_start = previous.map_or(semi.start, |index| {
            let token = self.tokens[index];
            if token.end > semi.start {
                return self.source[..semi.start]
                    .char_indices()
                    .next_back()
                    .map_or(semi.start, |(start, _)| start);
            }
            self.previous_significant_index(index)
                .filter(|previous| {
                    self.token_text(*previous) == "#" && self.tokens[*previous].end == token.start
                })
                .map_or(token.start, |previous| self.tokens[previous].start)
        });
        let fix_end = next.map_or(semi.end, |index| self.tokens[index].end);
        let mut replacement = String::with_capacity(fix_end.saturating_sub(fix_start + 1));
        replacement.push_str(&self.source[fix_start..semi.start]);
        replacement.push_str(&self.source[semi.end..fix_end]);
        self.report(
            "extraSemi",
            EXTRA_SEMI,
            semi.start,
            semi.end,
            LintFix::replace_range(byte_range(fix_start, fix_end), replacement),
        );
    }

    fn report(
        &mut self,
        message_id: &'static str,
        message: &'static str,
        start: usize,
        end: usize,
        fix: LintFix,
    ) {
        let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
            return;
        };
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            data: Default::default(),
            range: TextRange::new(start, end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: message_id.to_owned(),
                message: message.to_owned(),
                fixes: std::iter::once(fix).collect(),
            })
            .collect(),
        });
    }
}

fn next_location_byte(source: &str, offset: usize) -> usize {
    let Some(suffix) = source.get(offset..) else {
        return offset;
    };
    if suffix.is_empty() {
        return offset;
    }
    if suffix.starts_with("\r\n") {
        return offset + 2;
    }
    offset + suffix.chars().next().map_or(0, char::len_utf8)
}

fn has_line_terminator(text: &str) -> bool {
    text.chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn byte_range(start: usize, end: usize) -> TextRange {
    TextRange::new(
        u32::try_from(start).unwrap_or(u32::MAX),
        u32::try_from(end).unwrap_or(u32::MAX),
    )
}

fn message_order(message_id: &str) -> u8 {
    match message_id {
        "missingSemi" => 0,
        "extraSemi" => 1,
        _ => 2,
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the option and ASI regression matrix readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;
    use crate::{StylisticRuleConfig, StylisticRunConfig, run_stylistic_lint};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        #[serde(rename = "__generated")]
        generated: Generated,
        valid: Vec<FixtureCase>,
        invalid: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Generated {
        version: String,
        source_commit: String,
        inventory: Inventory,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Inventory {
        valid: usize,
        invalid: usize,
        diagnostics: usize,
        unfixable_invalid: usize,
        javascript: LanguageInventory,
        typescript: LanguageInventory,
    }

    #[derive(Deserialize)]
    struct LanguageInventory {
        valid: usize,
        invalid: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCase {
        code: String,
        language: String,
        #[serde(default)]
        options: Value,
        #[serde(default)]
        expected_diagnostics: Vec<ExpectedDiagnostic>,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        recursive_output: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedDiagnostic {
        message_id: String,
        message: String,
        range: [u32; 2],
        fix: ExpectedFix,
    }

    #[derive(Deserialize)]
    struct ExpectedFix {
        range: [u32; 2],
        text: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/semi-v5.10.0.json"
        ))
        .expect("generated upstream fixture is valid JSON")
    }

    fn run(source: &str, options: &Value, filename: &str) -> Vec<LintDiagnostic> {
        run_stylistic_lint(
            source,
            &StylisticRunConfig {
                filename: Some(filename.to_owned()),
                rules: vec![StylisticRuleConfig {
                    name: RULE.to_owned(),
                    options: options.clone(),
                }],
            },
        )
        .expect("semi runs")
    }

    fn filename(language: &str) -> &str {
        if language == "ts" {
            "fixture.ts"
        } else {
            "fixture.js"
        }
    }

    fn fixes(diagnostics: &[LintDiagnostic]) -> Vec<&LintFix> {
        diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect()
    }

    fn fixed_output(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = fixes(diagnostics);
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    fn recursive_output(source: &str, options: &Value, filename: &str) -> Option<String> {
        let mut output = source.to_owned();
        let mut changed = false;
        for _ in 0..10 {
            let diagnostics = run(&output, options, filename);
            let Some(next) = fixed_output(&output, &diagnostics) else {
                return changed.then_some(output);
            };
            assert_ne!(next, output, "fix pass must make progress");
            output = next;
            changed = true;
        }
        panic!("semi fixes did not converge");
    }

    #[test]
    fn fixture_is_the_complete_pinned_stable_inventory() {
        let fixture = fixture();
        assert_eq!(fixture.generated.version, "5.10.0");
        assert_eq!(
            fixture.generated.source_commit,
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(fixture.generated.inventory.valid, 199);
        assert_eq!(fixture.generated.inventory.invalid, 152);
        assert_eq!(fixture.generated.inventory.diagnostics, 158);
        assert_eq!(fixture.generated.inventory.unfixable_invalid, 0);
        assert_eq!(fixture.generated.inventory.javascript.valid, 179);
        assert_eq!(fixture.generated.inventory.javascript.invalid, 130);
        assert_eq!(fixture.generated.inventory.typescript.valid, 20);
        assert_eq!(fixture.generated.inventory.typescript.invalid, 22);
    }

    #[test]
    fn replays_every_upstream_valid_case() {
        let fixture = fixture();
        for case in fixture.valid {
            let actual = run(&case.code, &case.options, filename(&case.language));
            assert!(
                actual.is_empty(),
                "upstream valid case reported {:?}:\n{}",
                actual,
                case.code
            );
        }
    }

    #[test]
    fn replays_every_upstream_invalid_diagnostic_fix_and_recursive_output() {
        let fixture = fixture();
        for case in fixture.invalid {
            let filename = filename(&case.language);
            let diagnostics = run(&case.code, &case.options, filename);
            assert_eq!(
                diagnostics.len(),
                case.expected_diagnostics.len(),
                "diagnostic count mismatch:\n{}\nactual: {diagnostics:#?}",
                case.code
            );
            for (actual, expected) in diagnostics.iter().zip(&case.expected_diagnostics) {
                assert_eq!(actual.message_id, expected.message_id, "{}", case.code);
                assert_eq!(actual.message, expected.message, "{}", case.code);
                assert_eq!(
                    [actual.range.start, actual.range.end],
                    expected.range,
                    "{}",
                    case.code
                );
                let actual_fix = actual
                    .suggestions
                    .first()
                    .and_then(|suggestion| suggestion.fixes.first())
                    .expect("semi diagnostics are fixable");
                assert_eq!(
                    [actual_fix.range.start, actual_fix.range.end],
                    expected.fix.range,
                    "{}",
                    case.code
                );
                assert_eq!(
                    actual_fix.replacement_text, expected.fix.text,
                    "{}",
                    case.code
                );
            }
            assert_eq!(
                fixed_output(&case.code, &diagnostics),
                case.output,
                "single-pass output mismatch:\n{}",
                case.code
            );
            assert_eq!(
                recursive_output(&case.code, &case.options, filename),
                case.recursive_output,
                "recursive output mismatch:\n{}",
                case.code
            );
        }
    }

    #[test]
    fn covers_modes_options_class_fields_and_asi_hazards() {
        let source = concat!(
            "const first = 1;\r\n",
            "const arrow = () => {};\r\n",
            "[first].forEach(use)\r\n",
            "class Example { get;\r\n",
            "  safe = 1;\r\n",
            "  *method() {}\r\n",
            "}\r\n",
        );
        let diagnostics = run(
            source,
            &json!(["never", { "beforeStatementContinuationChars": "never" }]),
            "fixture.js",
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["extraSemi", "extraSemi"]
        );
        assert_eq!(fixes(&diagnostics).len(), 2);

        let always = run(
            "if (ok) { run(); }\nclass C { field; }\n",
            &json!(["always", {
                "omitLastInOneLineBlock": true,
                "omitLastInOneLineClassBody": true
            }]),
            "fixture.js",
        );
        assert_eq!(
            always
                .iter()
                .map(|diagnostic| diagnostic.message_id.as_str())
                .collect::<Vec<_>>(),
            ["extraSemi", "extraSemi"]
        );
    }

    #[test]
    fn supports_typescript_tsx_unicode_crlf_comments_and_fixtracker_ranges() {
        let source = concat!(
            "type 日本語 = { value: string };\r\n",
            "declare function café(): void;\r\n",
            "class 絵文字 { accessor donnée; }\r\n",
            "const view = <div>😀</div>;\r\n",
        );
        let diagnostics = run(source, &json!(["never"]), "fixture.tsx");
        assert_eq!(diagnostics.len(), 4);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message_id == "extraSemi")
        );
        for diagnostic in &diagnostics {
            let fix = &diagnostic.suggestions[0].fixes[0];
            assert!(fix.range.start < diagnostic.range.start);
            assert!(fix.range.end >= diagnostic.range.end);
        }
        let fixed = fixed_output(source, &diagnostics).expect("all semicolons are removable");
        assert!(!fixed.contains("};\r\n"));
        assert!(fixed.contains("<div>😀</div>\r\n"));
    }

    #[test]
    fn invalid_options_fall_back_without_panicking_or_widening_the_rule() {
        assert_eq!(
            run(
                "const value = 1\n",
                &json!(["sometimes", {
                    "omitLastInOneLineBlock": "yes",
                    "beforeStatementContinuationChars": 42
                }]),
                "fixture.js"
            )
            .len(),
            1
        );
        assert!(
            run(
                "for (let index = 0; index < 1; index++) {}\ninterface T { value: string; }\n",
                &json!([null, []]),
                "fixture.ts"
            )
            .is_empty()
        );
        assert!(run("const broken =", &Value::Null, "fixture.js").is_empty());
    }
}
