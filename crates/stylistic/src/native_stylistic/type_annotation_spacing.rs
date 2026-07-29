//! Spacing around TypeScript type annotations.
//!
//! Unlike the punctuation-only stylistic rules, this rule needs the AST to
//! distinguish variables, parameters, properties, return types, and function
//! type arrows. Oxc is used only when this rule is enabled; the remaining
//! stylistic rules keep sharing the allocation-light token scan.

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{BindingPattern, TSMappedType, TSTypeAnnotation},
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

const RULE: &str = "type-annotation-spacing";

#[derive(Clone, Copy, Debug)]
struct Spacing {
    before: bool,
    after: bool,
}

impl Spacing {
    const COLON_DEFAULT: Self = Self {
        before: false,
        after: true,
    };
    const ARROW_DEFAULT: Self = Self {
        before: true,
        after: true,
    };

    fn merge(self, value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return self;
        };
        Self {
            before: value
                .get("before")
                .and_then(Value::as_bool)
                .unwrap_or(self.before),
            after: value
                .get("after")
                .and_then(Value::as_bool)
                .unwrap_or(self.after),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ArrowSpacing {
    Check(Spacing),
    Ignore,
}

#[derive(Clone, Copy, Debug)]
struct RuleSet {
    colon: Spacing,
    arrow: ArrowSpacing,
    variable: Spacing,
    property: Spacing,
    parameter: Spacing,
    return_type: Spacing,
}

impl RuleSet {
    fn from_options(options: &Value) -> Self {
        let root = match options {
            Value::Array(items) => items.first(),
            Value::Null => None,
            value => Some(value),
        };
        let globals = Spacing {
            before: root
                .and_then(|value| value.get("before"))
                .and_then(Value::as_bool)
                .unwrap_or(Spacing::COLON_DEFAULT.before),
            after: root
                .and_then(|value| value.get("after"))
                .and_then(Value::as_bool)
                .unwrap_or(Spacing::COLON_DEFAULT.after),
        };
        let overrides = root.and_then(|value| value.get("overrides"));
        let colon = globals.merge(overrides.and_then(|value| value.get("colon")));

        // Arrow defaults differ from colons, but explicitly supplied globals
        // override both defaults before the arrow-specific override is applied.
        let arrow_globals = Spacing {
            before: root
                .and_then(|value| value.get("before"))
                .and_then(Value::as_bool)
                .unwrap_or(Spacing::ARROW_DEFAULT.before),
            after: root
                .and_then(|value| value.get("after"))
                .and_then(Value::as_bool)
                .unwrap_or(Spacing::ARROW_DEFAULT.after),
        };
        let arrow_override = overrides.and_then(|value| value.get("arrow"));
        let arrow = if arrow_override.and_then(Value::as_str) == Some("ignore") {
            ArrowSpacing::Ignore
        } else {
            ArrowSpacing::Check(arrow_globals.merge(arrow_override))
        };

        Self {
            colon,
            arrow,
            variable: colon.merge(overrides.and_then(|value| value.get("variable"))),
            property: colon.merge(overrides.and_then(|value| value.get("property"))),
            parameter: colon.merge(overrides.and_then(|value| value.get("parameter"))),
            return_type: colon.merge(overrides.and_then(|value| value.get("returnType"))),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum AnnotationContext {
    Colon,
    Variable,
    Property,
    Parameter,
    ReturnType,
    Arrow,
}

pub(crate) fn check_type_annotation_spacing(
    source: &str,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        // A TSX source cannot be identified from the source-wide native API.
        // Retry as TSX so annotations in JSX-bearing files remain covered.
        let parsed_tsx = Parser::new(&allocator, source, SourceType::tsx()).parse();
        if parsed_tsx.errors.is_empty() {
            let mut visitor = AnnotationVisitor::new(
                source,
                RuleSet::from_options(options),
                parsed_tsx
                    .program
                    .comments
                    .iter()
                    .map(|comment| (comment.span.start as usize, comment.span.end as usize))
                    .collect(),
                diagnostics,
            );
            visitor.visit_program(&parsed_tsx.program);
        }
        return;
    }

    let mut visitor = AnnotationVisitor::new(
        source,
        RuleSet::from_options(options),
        parsed
            .program
            .comments
            .iter()
            .map(|comment| (comment.span.start as usize, comment.span.end as usize))
            .collect(),
        diagnostics,
    );
    visitor.visit_program(&parsed.program);
}

struct AnnotationVisitor<'src, 'out> {
    source: &'src str,
    rules: RuleSet,
    comments: Vec<(usize, usize)>,
    parents: Vec<AnnotationContext>,
    diagnostics: &'out mut Vec<LintDiagnostic>,
}

impl<'src, 'out> AnnotationVisitor<'src, 'out> {
    fn new(
        source: &'src str,
        rules: RuleSet,
        comments: Vec<(usize, usize)>,
        diagnostics: &'out mut Vec<LintDiagnostic>,
    ) -> Self {
        Self {
            source,
            rules,
            comments,
            parents: Vec::new(),
            diagnostics,
        }
    }

    fn check(&mut self, type_start: usize, context: AnnotationContext) {
        let Some(punctuation) = Punctuation::before_type(self.source, type_start, &self.comments)
        else {
            return;
        };
        let spacing = match (context, punctuation.kind) {
            (AnnotationContext::Arrow, PunctuationKind::Arrow) => match self.rules.arrow {
                ArrowSpacing::Check(spacing) => spacing,
                ArrowSpacing::Ignore => return,
            },
            (_, PunctuationKind::Arrow) => return,
            (AnnotationContext::Variable, _) => self.rules.variable,
            (AnnotationContext::Property, _) => self.rules.property,
            (AnnotationContext::Parameter, _) => self.rules.parameter,
            (AnnotationContext::ReturnType, _) => self.rules.return_type,
            _ => self.rules.colon,
        };

        punctuation.report(self.source, spacing, self.diagnostics);
    }
}

impl<'src> Visit<'src> for AnnotationVisitor<'src, '_> {
    fn enter_node(&mut self, kind: AstKind<'src>) {
        let context = match kind {
            AstKind::VariableDeclarator(node)
                if matches!(node.id, BindingPattern::BindingIdentifier(_)) =>
            {
                AnnotationContext::Variable
            }
            AstKind::FormalParameter(node)
                if matches!(node.pattern, BindingPattern::BindingIdentifier(_)) =>
            {
                AnnotationContext::Parameter
            }
            AstKind::TSThisParameter(_) => AnnotationContext::Parameter,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => {
                AnnotationContext::ReturnType
            }
            AstKind::PropertyDefinition(_)
            | AstKind::AccessorProperty(_)
            | AstKind::TSPropertySignature(_)
            | AstKind::TSIndexSignature(_)
            | AstKind::TSCallSignatureDeclaration(_)
            | AstKind::TSMethodSignature(_)
            | AstKind::TSConstructSignatureDeclaration(_) => AnnotationContext::Property,
            AstKind::TSFunctionType(_) | AstKind::TSConstructorType(_) => AnnotationContext::Arrow,
            _ => AnnotationContext::Colon,
        };
        self.parents.push(context);
    }

    fn leave_node(&mut self, _kind: AstKind<'src>) {
        self.parents.pop();
    }

    fn visit_ts_type_annotation(&mut self, annotation: &TSTypeAnnotation<'src>) {
        let context = self
            .parents
            .last()
            .copied()
            .unwrap_or(AnnotationContext::Colon);
        self.check(annotation.type_annotation.span().start as usize, context);
        walk::walk_ts_type_annotation(self, annotation);
    }

    fn visit_ts_mapped_type(&mut self, mapped: &TSMappedType<'src>) {
        if let Some(annotation) = &mapped.type_annotation {
            self.check(annotation.span().start as usize, AnnotationContext::Colon);
        }
        walk::walk_ts_mapped_type(self, mapped);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PunctuationKind {
    Colon,
    Arrow,
}

#[derive(Clone, Copy, Debug)]
struct Punctuation {
    kind: PunctuationKind,
    token_start: usize,
    token_end: usize,
    group_start: usize,
    previous_end: usize,
    type_start: usize,
    display: &'static str,
    optional_gap: Option<(usize, usize)>,
}

impl Punctuation {
    fn before_type(source: &str, type_start: usize, comments: &[(usize, usize)]) -> Option<Self> {
        let bytes = source.as_bytes();
        let mut cursor = type_start.min(bytes.len());
        cursor = skip_spaces_and_comments_back(bytes, cursor, comments);

        // Parenthesized type nodes may expose the inner type span. Match
        // SourceCode#getTokenBefore(..., token => token.value !== '(').
        while cursor > 0 && bytes[cursor - 1] == b'(' {
            cursor -= 1;
            cursor = skip_spaces_and_comments_back(bytes, cursor, comments);
        }

        let (kind, token_start, token_end) = if cursor > 0 && bytes[cursor - 1] == b':' {
            (PunctuationKind::Colon, cursor - 1, cursor)
        } else if cursor >= 2 && &bytes[cursor - 2..cursor] == b"=>" {
            (PunctuationKind::Arrow, cursor - 2, cursor)
        } else {
            return None;
        };

        let mut group_start = token_start;
        let mut display = match kind {
            PunctuationKind::Colon => ":",
            PunctuationKind::Arrow => "=>",
        };
        let mut optional_gap = None;

        if kind == PunctuationKind::Colon {
            let optional_end = skip_spaces_and_comments_back(bytes, token_start, comments);
            if optional_end > 0 && bytes[optional_end - 1] == b'?' {
                let question_start = optional_end - 1;
                optional_gap = Some((optional_end, token_start));
                group_start = question_start;
                display = "?:";

                let modifier_end = skip_spaces_and_comments_back(bytes, question_start, comments);
                if modifier_end > 0 {
                    match bytes[modifier_end - 1] {
                        b'+' => {
                            group_start = modifier_end - 1;
                            display = "+?:";
                        }
                        b'-' => {
                            group_start = modifier_end - 1;
                            display = "-?:";
                        }
                        _ => {}
                    }
                }
            }
        }

        let previous_end = skip_spaces_and_comments_back(bytes, group_start, comments);
        Some(Self {
            kind,
            token_start,
            token_end,
            group_start,
            previous_end,
            type_start,
            display,
            optional_gap,
        })
    }

    fn report(self, source: &str, spacing: Spacing, diagnostics: &mut Vec<LintDiagnostic>) {
        if self.group_start < self.token_start {
            self.report_before(source, spacing, diagnostics);
            self.report_optional_gap(source, diagnostics);
            self.report_after(source, spacing, diagnostics);
        } else {
            self.report_optional_gap(source, diagnostics);
            self.report_after(source, spacing, diagnostics);
            self.report_before(source, spacing, diagnostics);
        }
    }

    fn report_optional_gap(self, source: &str, diagnostics: &mut Vec<LintDiagnostic>) {
        if let Some((gap_start, gap_end)) = self.optional_gap {
            if has_spacing(source, gap_start, gap_end) {
                push(
                    diagnostics,
                    "unexpectedSpaceBetween",
                    "Unexpected space between the '?' and the ':'.",
                    self.token_start,
                    self.token_end,
                    gap_start,
                    gap_end,
                    "",
                    &[("previousToken", "?"), ("type", ":")],
                );
            }
        }
    }

    fn report_after(self, source: &str, spacing: Spacing, diagnostics: &mut Vec<LintDiagnostic>) {
        let after_has_space = has_spacing(source, self.token_end, self.type_start);
        if spacing.after && !after_has_space {
            push(
                diagnostics,
                "expectedSpaceAfter",
                message_after(self.display, true),
                self.token_start,
                self.token_end,
                self.token_end,
                self.token_end,
                " ",
                &[("type", self.display)],
            );
        } else if !spacing.after && after_has_space {
            push(
                diagnostics,
                "unexpectedSpaceAfter",
                message_after(self.display, false),
                self.token_start,
                self.token_end,
                self.token_end,
                self.type_start,
                "",
                &[("type", self.display)],
            );
        }
    }

    fn report_before(self, source: &str, spacing: Spacing, diagnostics: &mut Vec<LintDiagnostic>) {
        let report_end = if self.kind == PunctuationKind::Arrow {
            self.group_start + 2
        } else {
            self.group_start + 1
        };
        let before_has_space = has_spacing(source, self.previous_end, self.group_start);
        if spacing.before && !before_has_space {
            push(
                diagnostics,
                "expectedSpaceBefore",
                message_before(self.display, true),
                self.group_start,
                report_end,
                self.previous_end,
                self.previous_end,
                " ",
                &[("type", self.display)],
            );
        } else if !spacing.before && before_has_space {
            push(
                diagnostics,
                "unexpectedSpaceBefore",
                message_before(self.display, false),
                self.group_start,
                report_end,
                self.previous_end,
                self.group_start,
                "",
                &[("type", self.display)],
            );
        }
    }
}

fn skip_spaces_and_comments_back(
    bytes: &[u8],
    mut cursor: usize,
    comments: &[(usize, usize)],
) -> usize {
    loop {
        while cursor > 0 && is_whitespace(bytes[cursor - 1]) {
            cursor -= 1;
        }
        let Some((comment_start, _)) = comments
            .iter()
            .rev()
            .find(|(_, comment_end)| *comment_end == cursor)
        else {
            return cursor;
        };
        cursor = *comment_start;
    }
}

fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn has_spacing(source: &str, start: usize, end: usize) -> bool {
    start < end
        && source
            .as_bytes()
            .get(start..end)
            .is_some_and(|gap| gap.iter().any(|byte| is_whitespace(*byte)))
}

fn message_after(operator: &str, expected: bool) -> &'static str {
    match (operator, expected) {
        (":", true) => "Expected a space after the ':'.",
        ("?:", true) => "Expected a space after the '?:'.",
        ("+?:", true) => "Expected a space after the '+?:'.",
        ("-?:", true) => "Expected a space after the '-?:'.",
        ("=>", true) => "Expected a space after the '=>'.",
        (":", false) => "Unexpected space after the ':'.",
        ("?:", false) => "Unexpected space after the '?:'.",
        ("+?:", false) => "Unexpected space after the '+?:'.",
        ("-?:", false) => "Unexpected space after the '-?:'.",
        ("=>", false) => "Unexpected space after the '=>'.",
        _ => "Unexpected type annotation spacing.",
    }
}

fn message_before(operator: &str, expected: bool) -> &'static str {
    match (operator, expected) {
        (":", true) => "Expected a space before the ':'.",
        ("?:", true) => "Expected a space before the '?:'.",
        ("+?:", true) => "Expected a space before the '+?:'.",
        ("-?:", true) => "Expected a space before the '-?:'.",
        ("=>", true) => "Expected a space before the '=>'.",
        (":", false) => "Unexpected space before the ':'.",
        ("?:", false) => "Unexpected space before the '?:'.",
        ("+?:", false) => "Unexpected space before the '+?:'.",
        ("-?:", false) => "Unexpected space before the '-?:'.",
        ("=>", false) => "Unexpected space before the '=>'.",
        _ => "Unexpected type annotation spacing.",
    }
}

#[allow(clippy::too_many_arguments)]
fn push(
    diagnostics: &mut Vec<LintDiagnostic>,
    message_id: &'static str,
    message: &'static str,
    report_start: usize,
    report_end: usize,
    fix_start: usize,
    fix_end: usize,
    replacement: &'static str,
    data: &[(&str, &str)],
) {
    let (Ok(report_start), Ok(report_end), Ok(fix_start), Ok(fix_end)) = (
        u32::try_from(report_start),
        u32::try_from(report_end),
        u32::try_from(fix_start),
        u32::try_from(fix_end),
    ) else {
        return;
    };
    diagnostics.push(LintDiagnostic {
        rule_name: RULE.to_owned(),
        message_id: message_id.to_owned(),
        message: message.to_owned(),
        data: data
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
        range: TextRange::new(report_start, report_end),
        suggestions: std::iter::once(LintSuggestion {
            message_id: message_id.to_owned(),
            message: message.to_owned(),
            fixes: std::iter::once(LintFix::replace_range(
                TextRange::new(fix_start, fix_end),
                replacement,
            ))
            .collect(),
        })
        .collect(),
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        valid: Vec<Case>,
        invalid: Vec<Case>,
    }

    #[derive(Deserialize)]
    struct Case {
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
        line: usize,
        column: usize,
    }

    fn run(source: &str, options: &Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_type_annotation_spacing(source, options, &mut diagnostics);
        diagnostics
    }

    fn options(json: &str) -> Value {
        serde_json::from_str(json).expect("test options are valid JSON")
    }

    fn message_ids(diagnostics: &[LintDiagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_id.as_str())
            .collect()
    }

    fn line_column(source: &str, offset: u32) -> (usize, usize) {
        let offset = (offset as usize).min(source.len());
        let prefix = &source[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        let column = source[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>()
            + 1;
        (line, column)
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut edits = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        edits.sort_by_key(|fix| std::cmp::Reverse((fix.range.start, fix.range.end)));

        let mut output = source.to_owned();
        for fix in edits {
            output.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        output
    }

    fn upstream_fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/type-annotation-spacing.json"
        ))
        .expect("generated upstream fixture is valid JSON")
    }

    #[test]
    fn replays_all_upstream_valid_cases() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.valid.len(), 255);

        for case in fixture.valid {
            let diagnostics = run(&case.code, &case.options);
            assert!(
                diagnostics.is_empty(),
                "expected valid case to pass:\n{}\nreported: {:?}",
                case.code,
                message_ids(&diagnostics),
            );
        }
    }

    #[test]
    fn replays_all_upstream_invalid_cases_with_exact_fixes_and_locations() {
        let fixture = upstream_fixture();
        assert_eq!(fixture.invalid.len(), 223);

        for case in fixture.invalid {
            let diagnostics = run(&case.code, &case.options);
            let actual_ids = message_ids(&diagnostics);
            let expected_ids = case
                .errors
                .iter()
                .map(|error| error.message_id.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_ids, expected_ids,
                "message mismatch for:\n{}",
                case.code
            );

            let actual_locations = diagnostics
                .iter()
                .map(|diagnostic| line_column(&case.code, diagnostic.range.start))
                .collect::<Vec<_>>();
            let expected_locations = case
                .errors
                .iter()
                .map(|error| (error.line, error.column))
                .collect::<Vec<_>>();
            assert_eq!(
                actual_locations, expected_locations,
                "location mismatch for:\n{}",
                case.code
            );

            assert_eq!(
                apply_fixes(&case.code, &diagnostics),
                case.output.expect("every upstream invalid case has output"),
                "fix mismatch for:\n{}",
                case.code
            );
        }
    }

    #[test]
    fn uses_context_specific_overrides_without_expression_false_positives() {
        let source = concat!(
            "const value : string = object ? left : right;\n",
            "function f(param : number) : boolean { return param > 0; }\n",
            "interface Box { property : string; method() : void; }\n",
            "type Callback = (input : string) =>number;\n",
        );
        let options = options(
            r#"[{
                "overrides": {
                    "variable": { "before": false, "after": true },
                    "parameter": { "before": false, "after": true },
                    "property": { "before": false, "after": true },
                    "returnType": { "before": false, "after": true },
                    "arrow": { "before": true, "after": true }
                }
            }]"#,
        );
        let diagnostics = run(source, &options);

        assert_eq!(
            message_ids(&diagnostics),
            [
                "unexpectedSpaceBefore",
                "unexpectedSpaceBefore",
                "unexpectedSpaceBefore",
                "unexpectedSpaceBefore",
                "unexpectedSpaceBefore",
                "unexpectedSpaceBefore",
                "expectedSpaceAfter",
            ]
        );
        // The conditional-expression colon is not a type annotation.
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.range.start != 31)
        );
    }

    #[test]
    fn preserves_exact_optional_operator_ranges_and_utf8_offsets() {
        let source = "type 日本語<T> = { [P in keyof T] +? :T[P] }";
        let diagnostics = run(source, &Value::Null);

        assert_eq!(
            message_ids(&diagnostics),
            [
                "unexpectedSpaceBefore",
                "unexpectedSpaceBetween",
                "expectedSpaceAfter",
            ]
        );
        let colon = source.find(':').expect("fixture has a colon");
        let plus = source.find('+').expect("fixture has a plus");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range)
                .collect::<Vec<_>>(),
            [
                TextRange::new(plus as u32, plus as u32 + 1),
                TextRange::new(colon as u32, colon as u32 + 1),
                TextRange::new(colon as u32, colon as u32 + 1),
            ]
        );
        assert_eq!(
            apply_fixes(source, &diagnostics),
            "type 日本語<T> = { [P in keyof T]+?: T[P] }"
        );
    }

    #[test]
    fn arrow_ignore_does_not_disable_colons() {
        let options = options(r#"[{ "overrides": { "arrow": "ignore" } }]"#);
        let diagnostics = run("type F = (value : string)=>number;", &options);
        assert_eq!(message_ids(&diagnostics), ["unexpectedSpaceBefore"]);
    }

    #[test]
    fn every_diagnostic_has_one_exact_whitespace_fix() {
        let diagnostics = run(
            "type F = { field :string; optional? :number };",
            &Value::Null,
        );
        assert!(!diagnostics.is_empty());

        for diagnostic in diagnostics {
            assert_eq!(diagnostic.suggestions.len(), 1);
            assert_eq!(diagnostic.suggestions[0].fixes.len(), 1);
            assert!(
                diagnostic.suggestions[0].fixes[0]
                    .replacement_text
                    .is_empty()
                    || diagnostic.suggestions[0].fixes[0].replacement_text == " "
            );
        }
    }

    #[test]
    fn treats_comments_as_non_tokens_when_measuring_and_fixing_spaces() {
        assert_eq!(
            run("let value:/*comment*/ string;", &Value::Null),
            Vec::<LintDiagnostic>::new()
        );

        let missing_after = run("let value:/*comment*/string;", &Value::Null);
        assert_eq!(message_ids(&missing_after), ["expectedSpaceAfter"]);
        assert_eq!(
            apply_fixes("let value:/*comment*/string;", &missing_after),
            "let value: /*comment*/string;"
        );

        let no_after = options(r#"[{ "after": false }]"#);
        let unexpected_after = run("let value: /*comment*/ string;", &no_after);
        assert_eq!(message_ids(&unexpected_after), ["unexpectedSpaceAfter"]);
        assert_eq!(
            apply_fixes("let value: /*comment*/ string;", &unexpected_after),
            "let value:string;"
        );

        let unexpected_before = run("let value /*comment*/ : string;", &Value::Null);
        assert_eq!(message_ids(&unexpected_before), ["unexpectedSpaceBefore"]);
        assert_eq!(
            apply_fixes("let value /*comment*/ : string;", &unexpected_before),
            "let value: string;"
        );

        let require_before = options(r#"[{ "before": true }]"#);
        let missing_before = run("let value/*comment*/: string;", &require_before);
        assert_eq!(message_ids(&missing_before), ["expectedSpaceBefore"]);
        assert_eq!(
            apply_fixes("let value/*comment*/: string;", &missing_before),
            "let value /*comment*/: string;"
        );
    }

    #[test]
    fn option_matrix_matches_all_four_colon_and_arrow_spacing_modes() {
        let cases = [
            (
                options(r#"[{ "before": false, "after": false }]"#),
                "let value:string; type F = ()=>number;",
            ),
            (
                options(r#"[{ "before": false, "after": true }]"#),
                "let value: string; type F = ()=> number;",
            ),
            (
                options(r#"[{ "before": true, "after": false }]"#),
                "let value :string; type F = () =>number;",
            ),
            (
                options(r#"[{ "before": true, "after": true }]"#),
                "let value : string; type F = () => number;",
            ),
        ];

        for (options, source) in cases {
            assert_eq!(run(source, &options), Vec::<LintDiagnostic>::new());
        }
    }

    #[test]
    fn ignores_non_annotations_and_parse_failures() {
        let false_positives = [
            "const object = { key: value };",
            "const result = condition ? left : right;",
            "label: for (;;) break label;",
            "switch (value) { case 1: break; }",
            "const url = 'https://example.test';",
            "const broken = (:;",
        ];

        let results = false_positives
            .into_iter()
            .map(|source| (source, run(source, &Value::Null)))
            .collect::<BTreeMap<_, _>>();
        assert!(results.values().all(|diagnostics| diagnostics.is_empty()));
    }
}
