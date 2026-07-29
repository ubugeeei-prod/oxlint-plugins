//! Native implementation of stable `@stylistic/nonblock-statement-body-position`.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{IfStatement, Statement},
};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::lexer::{Token, tokenize};

const RULE: &str = "nonblock-statement-body-position";
const EXPECT_NO_LINEBREAK_ID: &str = "expectNoLinebreak";
const EXPECT_NO_LINEBREAK_MESSAGE: &str = "Expected no linebreak before this statement.";
const EXPECT_LINEBREAK_ID: &str = "expectLinebreak";
const EXPECT_LINEBREAK_MESSAGE: &str = "Expected a linebreak before this statement.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Beside,
    Below,
    Any,
}

#[derive(Clone, Copy)]
enum Keyword {
    If,
    Else,
    While,
    Do,
    For,
}

impl Keyword {
    const fn option_key(self) -> &'static str {
        match self {
            Self::If => "if",
            Self::Else => "else",
            Self::While => "while",
            Self::Do => "do",
            Self::For => "for",
        }
    }
}

#[derive(Clone, Copy)]
struct Options {
    default: Position,
    overrides: [Option<Position>; 5],
}

impl Options {
    fn from_json(options: &Value) -> Self {
        let values = options.as_array();
        let default = values
            .and_then(|values| values.first())
            .and_then(Position::from_value)
            .unwrap_or(Position::Beside);
        let override_object = values
            .and_then(|values| values.get(1))
            .and_then(|value| value.get("overrides"))
            .and_then(Value::as_object);
        let keywords = [
            Keyword::If,
            Keyword::Else,
            Keyword::While,
            Keyword::Do,
            Keyword::For,
        ];
        let mut overrides = [None; 5];
        for (index, keyword) in keywords.into_iter().enumerate() {
            overrides[index] = override_object
                .and_then(|object| object.get(keyword.option_key()))
                .and_then(Position::from_value);
        }
        Self { default, overrides }
    }

    fn for_keyword(self, keyword: Keyword) -> Position {
        let index = match keyword {
            Keyword::If => 0,
            Keyword::Else => 1,
            Keyword::While => 2,
            Keyword::Do => 3,
            Keyword::For => 4,
        };
        self.overrides[index].unwrap_or(self.default)
    }
}

impl Position {
    fn from_value(value: &Value) -> Option<Self> {
        match value.as_str()? {
            "beside" => Some(Self::Beside),
            "below" => Some(Self::Below),
            "any" => Some(Self::Any),
            _ => None,
        }
    }
}

pub(crate) fn check_nonblock_statement_body_position(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let tokens = tokenize(source);
    let options = Options::from_json(options);
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

    diagnostics[first_diagnostic..]
        .sort_by_key(|diagnostic| (diagnostic.range.start, diagnostic.range.end));
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
    let mut visitor = NonblockStatementBodyPosition {
        source,
        tokens,
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct NonblockStatementBodyPosition<'source, 'diagnostics> {
    source: &'source str,
    tokens: &'source [Token],
    options: Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for NonblockStatementBodyPosition<'_, '_> {
    fn enter_node(&mut self, kind: AstKind<'ast>) {
        match kind {
            AstKind::IfStatement(statement) => self.validate_if(statement),
            AstKind::WhileStatement(statement) => {
                self.validate_statement(&statement.body, Keyword::While);
            }
            AstKind::DoWhileStatement(statement) => {
                self.validate_statement(&statement.body, Keyword::Do);
            }
            AstKind::ForStatement(statement) => {
                self.validate_statement(&statement.body, Keyword::For);
            }
            AstKind::ForInStatement(statement) => {
                self.validate_statement(&statement.body, Keyword::For);
            }
            AstKind::ForOfStatement(statement) => {
                self.validate_statement(&statement.body, Keyword::For);
            }
            _ => {}
        }
    }
}

impl NonblockStatementBodyPosition<'_, '_> {
    fn validate_if(&mut self, statement: &IfStatement<'_>) {
        self.validate_statement(&statement.consequent, Keyword::If);
        if let Some(alternate) = &statement.alternate
            && !matches!(alternate, Statement::IfStatement(_))
        {
            self.validate_statement(alternate, Keyword::Else);
        }
    }

    fn validate_statement(&mut self, statement: &Statement<'_>, keyword: Keyword) {
        let position = self.options.for_keyword(keyword);
        if position == Position::Any || matches!(statement, Statement::BlockStatement(_)) {
            return;
        }

        let span = statement.span();
        let Some(previous) = previous_significant_before(self.tokens, span.start) else {
            return;
        };
        let token = self.tokens[previous];
        let start = usize::try_from(span.start).unwrap_or(usize::MAX);
        let on_same_line = same_line(self.source, token.end, start);

        if on_same_line && position == Position::Below {
            self.report(
                span,
                EXPECT_LINEBREAK_ID,
                EXPECT_LINEBREAK_MESSAGE,
                Some(LintFix::replace_range(byte_range(start, start), "\n")),
            );
        } else if !on_same_line && position == Position::Beside {
            let fix = if !comments_between(self.tokens, previous, start) {
                Some(LintFix::replace_range(byte_range(token.end, start), " "))
            } else {
                None
            };
            self.report(
                span,
                EXPECT_NO_LINEBREAK_ID,
                EXPECT_NO_LINEBREAK_MESSAGE,
                fix,
            );
        }
    }

    fn report(
        &mut self,
        span: Span,
        message_id: &'static str,
        message: &'static str,
        fix: Option<LintFix>,
    ) {
        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE.to_owned(),
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            range: byte_range(
                usize::try_from(span.start).unwrap_or(usize::MAX),
                usize::try_from(span.end).unwrap_or(usize::MAX),
            ),
            suggestions: fix
                .map(|fix| LintSuggestion {
                    message_id: message_id.to_owned(),
                    message: message.to_owned(),
                    fixes: std::iter::once(fix).collect(),
                })
                .into_iter()
                .collect(),
            data: BTreeMap::new(),
        });
    }
}

fn previous_significant_before(tokens: &[Token], start: u32) -> Option<usize> {
    let start = usize::try_from(start).ok()?;
    (0..tokens.len())
        .rev()
        .find(|&index| tokens[index].end <= start && !tokens[index].kind.is_comment())
}

fn comments_between(tokens: &[Token], left: usize, right_start: usize) -> bool {
    tokens
        .iter()
        .skip(left + 1)
        .take_while(|token| token.start < right_start)
        .any(|token| token.kind.is_comment())
}

fn same_line(source: &str, start: usize, end: usize) -> bool {
    !source[start.min(source.len())..end.min(source.len())]
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}'))
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
    reason = "serde_json::json keeps the option matrix readable"
)]
mod tests {
    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../npm/stylistic/test/fixtures/nonblock-statement-body-position-v5.10.0.json"
    ));

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
    }

    #[derive(Deserialize)]
    struct TestCase {
        code: String,
        #[serde(default)]
        options: Value,
        output: Option<String>,
        #[serde(default)]
        errors: Vec<ExpectedError>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedError {
        message_id: String,
    }

    fn run(source: &str, filename: Option<&str>, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_nonblock_statement_body_position(source, filename, options, &mut diagnostics);
        diagnostics
    }

    fn fixed(source: &str, diagnostics: &[LintDiagnostic]) -> Option<String> {
        let mut fixes = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.suggestions.first())
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        if fixes.is_empty() {
            return None;
        }
        fixes.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));
        let mut output = source.to_owned();
        for fix in fixes {
            output.replace_range(
                usize::try_from(fix.range.start).expect("start fits usize")
                    ..usize::try_from(fix.range.end).expect("end fits usize"),
                &fix.replacement_text,
            );
        }
        Some(output)
    }

    #[test]
    fn replays_every_pinned_upstream_case_and_exact_output() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture is valid");
        assert_eq!(fixture.generated.inventory.valid, 31);
        assert_eq!(fixture.generated.inventory.invalid, 17);
        assert_eq!(fixture.generated.inventory.diagnostics, 19);
        assert_eq!(fixture.generated.inventory.fixable_invalid, 17);
        for (index, test_case) in fixture.valid.iter().enumerate() {
            assert!(
                run(&test_case.code, Some("fixture.tsx"), &test_case.options).is_empty(),
                "valid case {index} reported diagnostics:\n{}",
                test_case.code
            );
        }
        for (index, test_case) in fixture.invalid.iter().enumerate() {
            let diagnostics = run(&test_case.code, Some("fixture.tsx"), &test_case.options);
            assert_eq!(
                diagnostics
                    .iter()
                    .map(|diagnostic| &diagnostic.message_id)
                    .collect::<Vec<_>>(),
                test_case
                    .errors
                    .iter()
                    .map(|error| &error.message_id)
                    .collect::<Vec<_>>(),
                "invalid case {index} messages:\n{}",
                test_case.code
            );
            assert_eq!(
                fixed(&test_case.code, &diagnostics),
                test_case.output,
                "invalid case {index} output:\n{}",
                test_case.code
            );
        }
    }

    #[test]
    fn covers_every_statement_kind_default_below_any_and_overrides() {
        let source = "if (a) one(); else two(); while (b) three(); do four(); while (c); for (;;) five(); for (x in y) six(); for (x of y) seven();";
        assert!(run(source, Some("fixture.js"), &json!([])).is_empty());
        assert_eq!(run(source, Some("fixture.js"), &json!(["below"])).len(), 7);
        assert!(
            run(
                source,
                Some("fixture.js"),
                &json!(["any", { "overrides": { "if": "any", "else": "any", "while": "any", "do": "any", "for": "any" } }])
            )
            .is_empty()
        );
        let overridden = run(
            "if (a) one(); while (b) two();",
            Some("fixture.js"),
            &json!(["beside", { "overrides": { "while": "below" } }]),
        );
        assert_eq!(overridden.len(), 1);
        assert_eq!(overridden[0].message_id, EXPECT_LINEBREAK_ID);
    }

    #[test]
    fn ignores_blocks_and_else_if_but_checks_nested_nonblock_statements() {
        let source = "if (a) { one(); } else if (b) while (c)\nthree();";
        let diagnostics = run(source, Some("fixture.js"), &json!(["beside"]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message_id, EXPECT_NO_LINEBREAK_ID);
    }

    #[test]
    fn preserves_comment_safety_for_beside_and_below_fixes() {
        let beside = "if (ready)\n/* keep */\nrun();";
        let diagnostics = run(beside, Some("fixture.js"), &json!(["beside"]));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggestions.is_empty());

        let below = "if (ready) /* keep */ run();";
        let diagnostics = run(below, Some("fixture.js"), &json!(["below"]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            fixed(below, &diagnostics).as_deref(),
            Some("if (ready) /* keep */ \nrun();")
        );
    }

    #[test]
    fn supports_all_javascript_line_terminators_and_crlf() {
        for separator in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
            let source = format!("if (ready){separator}run();");
            let diagnostics = run(&source, Some("fixture.js"), &json!(["beside"]));
            assert_eq!(diagnostics.len(), 1, "{separator:?}");
            assert_eq!(
                fixed(&source, &diagnostics).as_deref(),
                Some("if (ready) run();"),
                "{separator:?}"
            );
        }
    }

    #[test]
    fn preserves_utf8_byte_ranges_and_typescript_tsx_parsing() {
        let source = "if (準備) 実行(); else <View />;";
        let diagnostics = run(source, Some("fixture.tsx"), &json!(["below"]));
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0].range.start,
            u32::try_from(source.find("実行").expect("body")).expect("range")
        );
        assert_eq!(
            diagnostics[1].range.start,
            u32::try_from(source.find("<View").expect("alternate")).expect("range")
        );
        assert_eq!(
            fixed(source, &diagnostics).as_deref(),
            Some("if (準備) \n実行(); else \n<View />;")
        );
    }

    #[test]
    fn invalid_syntax_and_unknown_options_do_not_create_false_positives() {
        assert!(run("if (ready", Some("fixture.js"), &json!(["below"])).is_empty());
        assert!(
            run(
                "if (ready) run();",
                Some("fixture.js"),
                &json!(["unknown", { "overrides": { "if": "unknown" } }])
            )
            .is_empty()
        );
    }
}
