//! Native implementation of stable `@stylistic/jsx-pascal-case`.
//!
//! Oxc supplies semantic JSX names for identifiers, namespaces, and arbitrarily
//! nested member expressions. Validation deliberately follows upstream's
//! UTF-16-oriented JavaScript predicates and reports the whole opening element.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    JSXElementName, JSXMemberExpression, JSXMemberExpressionObject, JSXOpeningElement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use regex::Regex;
use serde_json::Value;

use crate::{LintDiagnostic, TextRange};

use super::context::{first_option, option_object_bool};

const RULE: &str = "jsx-pascal-case";
const USE_PASCAL_CASE: &str = "Imported JSX component {{name}} must be in PascalCase";
const USE_PASCAL_OR_SNAKE_CASE: &str =
    "Imported JSX component {{name}} must be in PascalCase or SCREAMING_SNAKE_CASE";

#[derive(Debug)]
struct Options {
    allow_all_caps: bool,
    allow_leading_underscore: bool,
    allow_namespace: bool,
    ignore: Vec<IgnorePattern>,
}

impl Options {
    fn from_value(value: &Value) -> Self {
        let ignore = first_option(value)
            .and_then(|option| option.get("ignore"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(IgnorePattern::compile)
            .collect();
        Self {
            allow_all_caps: option_object_bool(value, "allowAllCaps", false),
            allow_leading_underscore: option_object_bool(value, "allowLeadingUnderscore", false),
            allow_namespace: option_object_bool(value, "allowNamespace", false),
            ignore,
        }
    }

    fn ignores(&self, name: &str) -> bool {
        self.ignore.iter().any(|pattern| pattern.matches(name))
    }
}

#[derive(Debug)]
enum IgnorePattern {
    Match(Regex),
    Negated(Regex),
    Literal(String),
}

impl IgnorePattern {
    fn compile(pattern: &str) -> Self {
        let (negated, body) = pattern
            .strip_prefix('!')
            .map_or((false, pattern), |body| (true, body));
        let translated = glob_to_regex(body);
        match Regex::new(&translated) {
            Ok(regex) if negated => Self::Negated(regex),
            Ok(regex) => Self::Match(regex),
            Err(_) => Self::Literal(pattern.to_owned()),
        }
    }

    fn matches(&self, name: &str) -> bool {
        match self {
            Self::Match(regex) => regex.is_match(name),
            Self::Negated(regex) => !regex.is_match(name),
            Self::Literal(pattern) => pattern == name,
        }
    }
}

pub(crate) fn check_jsx_pascal_case(
    source: &str,
    filename: Option<&str>,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let options = Options::from_value(options);
    if let Some(source_type) = filename.and_then(|path| SourceType::from_path(path).ok())
        && parse_and_check(source, source_type, &options, diagnostics)
    {
        return;
    }

    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        if parse_and_check(source, source_type, &options, diagnostics) {
            return;
        }
    }
}

fn parse_and_check(
    source: &str,
    source_type: SourceType,
    options: &Options,
    diagnostics: &mut Vec<LintDiagnostic>,
) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.errors.is_empty() {
        return false;
    }

    let mut visitor = PascalCaseVisitor {
        options,
        diagnostics,
    };
    visitor.visit_program(&parsed.program);
    true
}

struct PascalCaseVisitor<'options, 'diagnostics> {
    options: &'options Options,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for PascalCaseVisitor<'_, '_> {
    fn visit_jsx_opening_element(&mut self, node: &JSXOpeningElement<'ast>) {
        self.check(node);
        walk::walk_jsx_opening_element(self, node);
    }
}

impl PascalCaseVisitor<'_, '_> {
    fn check(&mut self, node: &JSXOpeningElement<'_>) {
        let name = element_name(&node.name);
        if name.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
            return;
        }

        let segments = if name.contains(':') {
            name.split(':').collect::<Vec<_>>()
        } else if name.contains('.') {
            name.split('.').collect::<Vec<_>>()
        } else {
            std::iter::once(name.as_str()).collect()
        };

        for segment in segments {
            // Upstream returns from the listener, not merely this iteration.
            if segment.encode_utf16().count() == 1 {
                return;
            }

            let checked = if self.options.allow_leading_underscore {
                segment.strip_prefix('_').unwrap_or(segment)
            } else {
                segment
            };
            let valid_pascal = test_pascal_case(checked);
            let valid_all_caps = self.options.allow_all_caps && test_all_caps(checked);
            if !valid_pascal && !valid_all_caps && !self.options.ignores(segment) {
                let (message_id, template) = if self.options.allow_all_caps {
                    ("usePascalOrSnakeCase", USE_PASCAL_OR_SNAKE_CASE)
                } else {
                    ("usePascalCase", USE_PASCAL_CASE)
                };
                self.diagnostics.push(LintDiagnostic {
                    rule_name: RULE.to_owned(),
                    message_id: message_id.to_owned(),
                    message: template.replace("{{name}}", segment),
                    data: BTreeMap::from([("name".to_owned(), segment.to_owned())]),
                    range: TextRange::new(node.span.start, node.span.end),
                    suggestions: Vec::new(),
                });
                return;
            }
            if self.options.allow_namespace {
                return;
            }
        }
    }
}

fn element_name(name: &JSXElementName<'_>) -> String {
    match name {
        JSXElementName::Identifier(identifier) => identifier.name.as_str().to_owned(),
        JSXElementName::IdentifierReference(identifier) => identifier.name.as_str().to_owned(),
        JSXElementName::NamespacedName(namespaced) => {
            let namespace = namespaced.namespace.name.as_str();
            let local = namespaced.name.name.as_str();
            let mut name = String::with_capacity(namespace.len() + local.len() + 1);
            name.push_str(namespace);
            name.push(':');
            name.push_str(local);
            name
        }
        JSXElementName::MemberExpression(member) => member_expression_name(member),
        JSXElementName::ThisExpression(_) => "this".to_owned(),
    }
}

fn member_expression_name(member: &JSXMemberExpression<'_>) -> String {
    let mut object = match &member.object {
        JSXMemberExpressionObject::IdentifierReference(identifier) => {
            identifier.name.as_str().to_owned()
        }
        JSXMemberExpressionObject::MemberExpression(member) => member_expression_name(member),
        JSXMemberExpressionObject::ThisExpression(_) => "this".to_owned(),
    };
    object.reserve(member.property.name.len() + 1);
    object.push('.');
    object.push_str(member.property.name.as_str());
    object
}

fn test_pascal_case(name: &str) -> bool {
    let units = name.encode_utf16().collect::<Vec<_>>();
    if !units.first().copied().is_some_and(test_uppercase) {
        return false;
    }
    if units[1..]
        .iter()
        .copied()
        .any(|unit| same_upper_and_lower(unit) && !test_digit(unit))
    {
        return false;
    }
    units[1..]
        .iter()
        .copied()
        .any(|unit| test_lowercase(unit) || test_digit(unit))
}

fn test_all_caps(name: &str) -> bool {
    let units = name.encode_utf16().collect::<Vec<_>>();
    let Some((&first, rest)) = units.split_first() else {
        return false;
    };
    let Some((&last, middle)) = rest.split_last() else {
        return test_uppercase(first) || test_digit(first);
    };
    (test_uppercase(first) || test_digit(first))
        && middle
            .iter()
            .copied()
            .all(|unit| test_uppercase(unit) || test_digit(unit) || unit == u16::from(b'_'))
        && (test_uppercase(last) || test_digit(last))
}

fn test_digit(unit: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&unit)
}

fn test_uppercase(unit: u16) -> bool {
    let Some(character) = char::from_u32(u32::from(unit)) else {
        return false;
    };
    case_mapping_equals(character.to_uppercase(), character)
        && !case_mapping_equals(character.to_lowercase(), character)
}

fn test_lowercase(unit: u16) -> bool {
    let Some(character) = char::from_u32(u32::from(unit)) else {
        return false;
    };
    case_mapping_equals(character.to_lowercase(), character)
        && !case_mapping_equals(character.to_uppercase(), character)
}

fn same_upper_and_lower(unit: u16) -> bool {
    let Some(character) = char::from_u32(u32::from(unit)) else {
        return true;
    };
    character.to_uppercase().eq(character.to_lowercase())
}

fn case_mapping_equals(iter: impl Iterator<Item = char>, character: char) -> bool {
    let mut iter = iter;
    iter.next() == Some(character) && iter.next().is_none()
}

fn glob_to_regex(pattern: &str) -> String {
    let characters = pattern.chars().collect::<Vec<_>>();
    let mut index = 0;
    let body = translate_glob_sequence(&characters, &mut index, &[]);
    let mut regex = String::with_capacity(body.len() + 6);
    regex.push_str("^(?:");
    regex.push_str(&body);
    regex.push_str(")$");
    regex
}

fn translate_glob_sequence(characters: &[char], index: &mut usize, stops: &[char]) -> String {
    let mut output = String::new();
    while let Some(&character) = characters.get(*index) {
        if stops.contains(&character) {
            break;
        }

        if matches!(character, '+' | '@' | '?' | '*') && characters.get(*index + 1) == Some(&'(') {
            *index += 2;
            let alternatives = translate_glob_alternatives(characters, index, ')');
            if characters.get(*index) == Some(&')') {
                *index += 1;
            }
            output.push_str("(?:");
            output.push_str(&alternatives);
            output.push(')');
            match character {
                '+' => output.push('+'),
                '?' => output.push('?'),
                '*' => output.push('*'),
                '@' => {}
                _ => unreachable!(),
            }
            continue;
        }

        match character {
            '*' => output.push_str(".*"),
            '?' => output.push('.'),
            '[' => translate_character_class(characters, index, &mut output),
            '{' => {
                *index += 1;
                let alternatives = translate_glob_alternatives(characters, index, '}');
                if characters.get(*index) == Some(&'}') {
                    *index += 1;
                    output.push_str("(?:");
                    output.push_str(&alternatives.replace(',', "|"));
                    output.push(')');
                } else {
                    output.push_str(r"\{");
                    output.push_str(&alternatives);
                }
                continue;
            }
            '\\' => {
                *index += 1;
                if let Some(&escaped) = characters.get(*index) {
                    push_escaped(&mut output, escaped);
                } else {
                    output.push_str(r"\\");
                    continue;
                }
            }
            _ => push_escaped(&mut output, character),
        }
        *index += 1;
    }
    output
}

fn push_escaped(output: &mut String, character: char) {
    let mut encoded = [0; 4];
    output.push_str(&regex::escape(character.encode_utf8(&mut encoded)));
}

fn translate_glob_alternatives(characters: &[char], index: &mut usize, end: char) -> String {
    let mut alternatives = String::new();
    loop {
        alternatives.push_str(&translate_glob_sequence(
            characters,
            index,
            &['|', ',', end],
        ));
        match characters.get(*index) {
            Some('|') | Some(',') => {
                alternatives.push('|');
                *index += 1;
            }
            _ => break,
        }
    }
    alternatives
}

fn translate_character_class(characters: &[char], index: &mut usize, output: &mut String) {
    let start = *index;
    *index += 1;
    let mut class = String::from("[");
    if matches!(characters.get(*index), Some('!') | Some('^')) {
        class.push('^');
        *index += 1;
    }
    while let Some(&character) = characters.get(*index) {
        if character == ']' {
            class.push(']');
            output.push_str(&class);
            return;
        }
        if character == '\\' {
            class.push('\\');
        }
        class.push(character);
        *index += 1;
    }
    *index = start;
    output.push_str(r"\[");
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json keeps the compatibility option matrix concise"
)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(source: &str, filename: Option<&str>, options: Value) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_jsx_pascal_case(source, filename, &options, &mut diagnostics);
        diagnostics
    }

    fn ids(source: &str, options: Value) -> Vec<String> {
        run(source, Some("fixture.tsx"), options)
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    fn upstream_fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../npm/stylistic/test/fixtures/jsx-pascal-case-v5.10.0.json"
        ))
        .expect("generated jsx-pascal-case fixture is valid JSON")
    }

    #[test]
    fn covers_default_all_caps_underscore_and_namespace_modes() {
        assert_eq!(ids("<Test_component />", Value::Null), ["usePascalCase"]);
        assert!(ids("<TEST_COMPONENT />", json!([{ "allowAllCaps": true }])).is_empty());
        assert!(
            ids(
                "<_TestComponent />",
                json!([{ "allowLeadingUnderscore": true }])
            )
            .is_empty()
        );
        assert!(ids("<Styled.h1 />", json!([{ "allowNamespace": true }])).is_empty());
        assert_eq!(
            ids("<STYLED.h1 />", json!([{ "allowNamespace": true }])),
            ["usePascalCase"]
        );
    }

    #[test]
    fn matches_exact_wildcard_extglob_brace_and_character_class_ignores() {
        for (source, pattern) in [
            ("<IGNORED />", "IGNORED"),
            ("<Foo_DEPRECATED />", "*_D*D"),
            ("<Foo_DEPRECATED />", "*_+(DEPRECATED|IGNORED)"),
            ("<Foo_IGNORED />", "*_+(DEPRECATED|IGNORED)"),
            ("<Foo_DEPRECATED />", "Foo_{DEPRECATED,IGNORED}"),
            ("<Foo_IGNORED />", "Foo_{DEPRECATED,IGNORED}"),
            ("<Foo_DEPRECATED />", "Foo_[A-Z]*"),
        ] {
            assert!(
                ids(source, json!([{ "ignore": [pattern] }])).is_empty(),
                "{pattern} should ignore {source}"
            );
        }
        assert_eq!(
            ids("<Foo_DEPRECATED />", json!([{ "ignore": ["*_FOO"] }])),
            ["usePascalCase"]
        );
    }

    #[test]
    fn preserves_dom_single_segment_and_nested_member_shortcuts() {
        for source in [
            "<testComponent />",
            "<qualification.bad />",
            "<T.bad />",
            "<this.bad />",
            "<$ />",
            "<_ />",
        ] {
            assert!(ids(source, Value::Null).is_empty(), "{source}");
        }
        assert_eq!(
            ids("<Modal.Bad_name.Deep />", Value::Null),
            ["usePascalCase"]
        );
    }

    #[test]
    fn uses_utf16_compatible_unicode_case_predicates_and_byte_ranges() {
        let source = "const emoji = '😀'; const view = <Éurströmming_日本 />;";
        let diagnostics = run(source, Some("fixture.tsx"), Value::Null);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].data["name"], "Éurströmming_日本");
        let start = source.find("<É").expect("opening element");
        let end = source.find("/>").expect("opening end") + 2;
        assert_eq!(
            diagnostics[0].range,
            TextRange::new(start as u32, end as u32)
        );
        assert!(ids("<Éurströmming />", Value::Null).is_empty());
        assert_eq!(ids("<𝔘nicode />", Value::Null), ["usePascalCase"]);
    }

    #[test]
    fn supports_tsx_generics_crlf_fragments_namespaces_and_invalid_inputs() {
        let source = [
            "type Item = { id: string };\r\n",
            "const view = <>\r\n",
            "  <Bad_name<Item> />\r\n",
            "</>;\r\n",
        ]
        .concat();
        assert_eq!(ids(&source, Value::Null), ["usePascalCase"]);
        assert!(run("<Modal:Header />", Some("fixture.jsx"), Value::Null).is_empty());
        assert!(
            run(
                "const broken = <Bad_name>",
                Some("fixture.tsx"),
                Value::Null
            )
            .is_empty()
        );
        assert!(run("const value = 1;", Some("fixture.js"), Value::Null).is_empty());
    }

    #[test]
    fn malformed_options_fall_back_without_panicking() {
        let source = "<TEST_COMPONENT />";
        for options in [
            Value::Null,
            json!([42]),
            json!([{ "allowAllCaps": "yes" }]),
            json!([{ "ignore": [42, null] }]),
        ] {
            assert_eq!(ids(source, options), ["usePascalCase"]);
        }
    }

    #[test]
    fn accepts_all_57_parser_expanded_stable_v5_10_0_valid_fixtures() {
        let fixture = upstream_fixture();
        let generated = &fixture["__generated"];
        assert_eq!(generated["version"], "5.10.0");
        assert_eq!(
            generated["sourceCommit"],
            "efbb1bc0e5aaedc4695c44a03f46f4fcbbe58712"
        );
        assert_eq!(generated["inventory"]["logicalValid"], 29);
        assert_eq!(generated["inventory"]["logicalInvalid"], 14);
        assert_eq!(generated["inventory"]["valid"], 57);
        assert_eq!(generated["inventory"]["invalid"], 28);
        assert_eq!(generated["inventory"]["diagnostics"], 28);
        assert_eq!(generated["inventory"]["fixableInvalid"], 0);
        assert_eq!(generated["inventory"]["unfixableInvalid"], 28);
        assert_eq!(generated["inventory"]["total"], 85);

        for (index, test) in fixture["valid"]
            .as_array()
            .expect("valid fixture array")
            .iter()
            .enumerate()
        {
            let source = test["code"].as_str().expect("valid fixture code");
            let filename = if test["parser"] == "typescript" {
                "fixture.tsx"
            } else {
                "fixture.jsx"
            };
            let diagnostics = run(source, Some(filename), test["options"].clone());
            assert!(
                diagnostics.is_empty(),
                "upstream valid fixture {index} reported {diagnostics:#?}:\n{source}"
            );
        }
    }

    #[test]
    fn reproduces_all_28_invalid_diagnostics_ranges_data_and_nonfixable_output() {
        let fixture = upstream_fixture();
        for (index, test) in fixture["invalid"]
            .as_array()
            .expect("invalid fixture array")
            .iter()
            .enumerate()
        {
            let source = test["code"].as_str().expect("invalid fixture code");
            let filename = if test["parser"] == "typescript" {
                "fixture.tsx"
            } else {
                "fixture.jsx"
            };
            let diagnostics = run(source, Some(filename), test["options"].clone());
            let expected = test["expectedDiagnostics"]
                .as_array()
                .expect("expected diagnostics");
            assert_eq!(
                diagnostics.len(),
                expected.len(),
                "invalid fixture {index} diagnostic count:\n{source}"
            );
            for (diagnostic_index, (actual, expected)) in
                diagnostics.iter().zip(expected).enumerate()
            {
                assert_eq!(
                    actual.message_id,
                    expected["messageId"].as_str().expect("message ID"),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.message,
                    expected["message"].as_str().expect("message"),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                assert_eq!(
                    actual.data["name"],
                    expected["data"]["name"].as_str().expect("name data"),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                let range = expected["range"].as_array().expect("expected range");
                assert_eq!(
                    actual.range,
                    TextRange::new(
                        range[0].as_u64().expect("range start") as u32,
                        range[1].as_u64().expect("range end") as u32,
                    ),
                    "invalid fixture {index}, diagnostic {diagnostic_index}"
                );
                assert!(
                    actual.suggestions.is_empty(),
                    "invalid fixture {index}, diagnostic {diagnostic_index} unexpectedly fixed"
                );
                assert!(
                    expected["fix"].is_null(),
                    "upstream invalid fixture {index} unexpectedly fixes"
                );
            }
            assert!(test["output"].is_null());
            assert!(test["recursiveOutput"].is_null());
        }
    }
}
