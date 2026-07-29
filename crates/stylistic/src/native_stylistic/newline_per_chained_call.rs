//! Native implementation of `@stylistic/newline-per-chained-call`.
//!
//! Chain depth is an AST property, while the upstream fixer inserts before the
//! first non-comment, non-closing-parenthesis token after the member object.
//! This implementation combines Oxc's AST with the shared stylistic lexer so
//! both decisions match the stable rule without heuristic member parsing.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    CallExpression, ChainElement, ComputedMemberExpression, Expression, MemberExpression,
    PrivateFieldExpression, StaticMemberExpression, match_member_expression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use serde_json::Value;

use crate::{LintDiagnostic, LintFix, LintSuggestion, TextRange};

use super::context::Scan;
use super::lexer::TokenKind;

const RULE_NAME: &str = "newline-per-chained-call";
const MESSAGE_ID: &str = "expected";
const DEFAULT_IGNORED_DEPTH: usize = 2;

pub(crate) fn check_newline_per_chained_call(
    source_text: &str,
    options: &Value,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    let allocator = Allocator::default();
    let ignored_depth = option_depth(options);
    let scan = Scan::new(source_text);
    for source_type in [
        SourceType::tsx(),
        SourceType::jsx().with_unambiguous(true),
        SourceType::jsx().with_script(true),
    ] {
        let parsed = Parser::new(&allocator, source_text, source_type).parse();
        if parsed.errors.is_empty() {
            let mut visitor = NewlinePerChainedCallVisitor {
                source_text,
                scan: &scan,
                ignored_depth,
                diagnostics,
            };
            visitor.visit_program(&parsed.program);
            return;
        }
    }
}

fn option_depth(options: &Value) -> usize {
    match options {
        Value::Array(items) => items.first(),
        Value::Null => None,
        other => Some(other),
    }
    .and_then(|option| option.get("ignoreChainWithDepth"))
    .and_then(Value::as_u64)
    .and_then(|depth| usize::try_from(depth).ok())
    .filter(|depth| (1..=10).contains(depth))
    .unwrap_or(DEFAULT_IGNORED_DEPTH)
}

struct NewlinePerChainedCallVisitor<'source, 'scan, 'diagnostics> {
    source_text: &'source str,
    scan: &'scan Scan<'source>,
    ignored_depth: usize,
    diagnostics: &'diagnostics mut Vec<LintDiagnostic>,
}

impl<'ast> Visit<'ast> for NewlinePerChainedCallVisitor<'_, '_, '_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'ast>) {
        // The upstream listener is `CallExpression:exit`, so nested calls must
        // be checked before their containing call to preserve report order.
        walk::walk_call_expression(self, call);
        self.check_call(call);
    }
}

impl NewlinePerChainedCallVisitor<'_, '_, '_> {
    fn check_call(&mut self, call: &CallExpression<'_>) {
        let Some(member) = member_from_expression(&call.callee) else {
            return;
        };
        if chain_depth(member) <= self.ignored_depth {
            return;
        }

        let parts = member_parts(member);
        let object = skip_parentheses(parts.object);
        if !same_line_between(
            self.source_text,
            object.span().end,
            parts.property_span.start,
        ) {
            return;
        }

        let Some(insert_at) =
            first_fix_token_after_object(self.scan, object.span().end, parts.member_span.end)
        else {
            return;
        };
        let callee = property_text(self.source_text, parts);
        let message = rendered_message(&callee);
        let mut data = BTreeMap::new();
        data.insert("callee".to_owned(), callee);
        let fix_range = TextRange::new(insert_at, insert_at);

        self.diagnostics.push(LintDiagnostic {
            rule_name: RULE_NAME.to_owned(),
            message_id: MESSAGE_ID.to_owned(),
            message: message.clone(),
            data,
            range: TextRange::new(insert_at, parts.member_span.end),
            suggestions: std::iter::once(LintSuggestion {
                message_id: MESSAGE_ID.to_owned(),
                message,
                fixes: std::iter::once(LintFix::replace_range(fix_range, "\n")).collect(),
            })
            .collect(),
        });
    }
}

fn chain_depth(member: &MemberExpression<'_>) -> usize {
    let mut depth = 1usize;
    let mut parent = skip_parentheses(member.object());

    while let Some(callee) = call_like_callee(parent) {
        depth = depth.saturating_add(1);
        let Some(parent_member) = member_from_expression(callee) else {
            break;
        };
        parent = skip_parentheses(parent_member.object());
    }

    depth
}

fn call_like_callee<'ast>(expression: &'ast Expression<'ast>) -> Option<&'ast Expression<'ast>> {
    match expression {
        Expression::CallExpression(call) => Some(&call.callee),
        Expression::NewExpression(new) => Some(&new.callee),
        Expression::ParenthesizedExpression(parenthesized) => {
            call_like_callee(&parenthesized.expression)
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => Some(&call.callee),
            _ => None,
        },
        _ => None,
    }
}

fn member_from_expression<'ast>(
    expression: &'ast Expression<'ast>,
) -> Option<&'ast MemberExpression<'ast>> {
    match expression {
        member @ match_member_expression!(Expression) => Some(member.to_member_expression()),
        Expression::ParenthesizedExpression(parenthesized) => {
            member_from_expression(&parenthesized.expression)
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            member @ match_member_expression!(ChainElement) => Some(member.to_member_expression()),
            _ => None,
        },
        _ => None,
    }
}

fn skip_parentheses<'ast>(mut expression: &'ast Expression<'ast>) -> &'ast Expression<'ast> {
    loop {
        match expression {
            Expression::ParenthesizedExpression(parenthesized) => {
                expression = &parenthesized.expression;
            }
            _ => return expression,
        }
    }
}

#[derive(Clone, Copy)]
struct MemberParts<'ast> {
    object: &'ast Expression<'ast>,
    property_span: Span,
    member_span: Span,
    computed: bool,
    optional: bool,
}

fn member_parts<'ast>(member: &'ast MemberExpression<'ast>) -> MemberParts<'ast> {
    match member {
        MemberExpression::ComputedMemberExpression(member) => computed_parts(member),
        MemberExpression::StaticMemberExpression(member) => static_parts(member),
        MemberExpression::PrivateFieldExpression(member) => private_parts(member),
    }
}

fn computed_parts<'ast>(member: &'ast ComputedMemberExpression<'ast>) -> MemberParts<'ast> {
    MemberParts {
        object: &member.object,
        property_span: skip_parentheses(&member.expression).span(),
        member_span: member.span,
        computed: true,
        optional: member.optional,
    }
}

fn static_parts<'ast>(member: &'ast StaticMemberExpression<'ast>) -> MemberParts<'ast> {
    MemberParts {
        object: &member.object,
        property_span: member.property.span,
        member_span: member.span,
        computed: false,
        optional: member.optional,
    }
}

fn private_parts<'ast>(member: &'ast PrivateFieldExpression<'ast>) -> MemberParts<'ast> {
    MemberParts {
        object: &member.object,
        property_span: member.field.span,
        member_span: member.span,
        computed: false,
        optional: member.optional,
    }
}

fn same_line_between(source_text: &str, start: u32, end: u32) -> bool {
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return false;
    };
    let Some(between) = source_text.get(start..end) else {
        return false;
    };
    !between
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn first_fix_token_after_object(scan: &Scan<'_>, object_end: u32, member_end: u32) -> Option<u32> {
    let object_end = usize::try_from(object_end).ok()?;
    let member_end = usize::try_from(member_end).ok()?;
    scan.tokens()
        .iter()
        .enumerate()
        .find(|(index, token)| {
            token.start >= object_end
                && token.start < member_end
                && !token.kind.is_comment()
                && !(token.kind == TokenKind::Punctuator && scan.token_text(*index) == ")")
        })
        .and_then(|(_, token)| u32::try_from(token.start).ok())
}

fn property_text(source_text: &str, parts: MemberParts<'_>) -> String {
    let prefix = match (parts.computed, parts.optional) {
        (true, true) => "?.[",
        (true, false) => "[",
        (false, true) => "?.",
        (false, false) => ".",
    };
    let property = span_text(source_text, parts.property_span).unwrap_or_default();
    let (first_line, multiline) = first_line(property);
    let suffix = if parts.computed && !multiline {
        "]"
    } else {
        ""
    };
    let mut text = String::with_capacity(
        prefix
            .len()
            .saturating_add(first_line.len())
            .saturating_add(suffix.len()),
    );
    text.push_str(prefix);
    text.push_str(first_line);
    text.push_str(suffix);
    text
}

fn span_text(source_text: &str, span: Span) -> Option<&str> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    source_text.get(start..end)
}

fn first_line(text: &str) -> (&str, bool) {
    for (index, character) in text.char_indices() {
        if matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            return (&text[..index], true);
        }
    }
    (text, false)
}

fn rendered_message(callee: &str) -> String {
    let mut message = String::with_capacity(
        "Expected line break before ``."
            .len()
            .saturating_add(callee.len()),
    );
    message.push_str("Expected line break before `");
    message.push_str(callee);
    message.push_str("`.");
    message
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    reason = "serde_json::json and vec keep the verbatim upstream fixture matrix readable"
)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Clone, Copy)]
    struct Expected<'a> {
        callee: &'a str,
        range_text: &'a str,
        occurrence: usize,
    }

    struct InvalidCase<'a> {
        source: &'a str,
        output: &'a str,
        depth: usize,
        expected: &'a [Expected<'a>],
    }

    fn options(depth: usize) -> Value {
        json!([{ "ignoreChainWithDepth": depth }])
    }

    fn run(source: &str, depth: usize) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        check_newline_per_chained_call(source, &options(depth), &mut diagnostics);
        diagnostics
    }

    fn occurrence_range(source: &str, needle: &str, occurrence: usize) -> TextRange {
        let start = source
            .match_indices(needle)
            .nth(occurrence)
            .map(|(start, _)| start)
            .unwrap_or_else(|| panic!("missing occurrence {occurrence} of {needle:?}"));
        TextRange::new(start as u32, start.saturating_add(needle.len()) as u32)
    }

    fn apply_fixes(source: &str, diagnostics: &[LintDiagnostic]) -> String {
        let mut fixed = source.to_owned();
        let mut fixes = diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.suggestions)
            .flat_map(|suggestion| &suggestion.fixes)
            .collect::<Vec<_>>();
        fixes.sort_by_key(|fix| std::cmp::Reverse(fix.range.start));
        for fix in fixes {
            fixed.replace_range(
                fix.range.start as usize..fix.range.end as usize,
                &fix.replacement_text,
            );
        }
        fixed
    }

    fn assert_invalid(case: InvalidCase<'_>) {
        let diagnostics = run(case.source, case.depth);
        assert_eq!(
            diagnostics.len(),
            case.expected.len(),
            "wrong diagnostic count for {:?}",
            case.source
        );
        for (diagnostic, expected) in diagnostics.iter().zip(case.expected) {
            assert_eq!(diagnostic.rule_name, RULE_NAME);
            assert_eq!(diagnostic.message_id, MESSAGE_ID);
            assert_eq!(
                diagnostic.data.get("callee").map(String::as_str),
                Some(expected.callee)
            );
            assert_eq!(
                diagnostic.range,
                occurrence_range(case.source, expected.range_text, expected.occurrence)
            );
            let fix = &diagnostic.suggestions[0].fixes[0];
            assert_eq!(
                fix.range,
                TextRange::new(diagnostic.range.start, diagnostic.range.start)
            );
            assert_eq!(fix.replacement_text, "\n");
            assert_eq!(diagnostic.message, rendered_message(expected.callee));
        }
        assert_eq!(apply_fixes(case.source, &diagnostics), case.output);
    }

    #[test]
    fn accepts_every_upstream_v5_10_0_valid_fixture() {
        let cases = [
            ("_\n.chain({})\n.map(foo)\n.filter(bar)\n.value();", 2),
            ("a.b.c.d.e.f", 2),
            ("a()\n.b()\n.c\n.e", 2),
            ("var a = m1.m2(); var b = m1.m2();\nvar c = m1.m2()", 2),
            ("var a = m1()\n.m2();", 2),
            ("var a = m1();", 2),
            ("a()\n.b().c.e.d()", 2),
            ("a().b().c.e.d()", 2),
            ("a.b.c.e.d()", 2),
            (
                "var a = window\n.location\n.href\n.match(/(^[^#]*)/)[0];",
                2,
            ),
            (
                "var a = window['location']\n.href\n.match(/(^[^#]*)/)[0];",
                2,
            ),
            ("var a = window['location'].href.match(/(^[^#]*)/)[0];", 2),
            ("var a = m1().m2.m3();", 3),
            ("var a = m1().m2.m3().m4.m5().m6.m7().m8;", 8),
        ];

        for (source, depth) in cases {
            assert!(
                run(source, depth).is_empty(),
                "upstream valid fixture reported: {source:?}"
            );
        }
    }

    #[test]
    fn matches_upstream_basic_invalid_fixtures_and_exact_fixes() {
        let cases = [
            InvalidCase {
                source: "_\n.chain({}).map(foo).filter(bar).value();",
                output: "_\n.chain({}).map(foo)\n.filter(bar)\n.value();",
                depth: 2,
                expected: &[
                    Expected {
                        callee: ".filter",
                        range_text: ".filter",
                        occurrence: 0,
                    },
                    Expected {
                        callee: ".value",
                        range_text: ".value",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "_\n.chain({})\n.map(foo)\n.filter(bar).value();",
                output: "_\n.chain({})\n.map(foo)\n.filter(bar)\n.value();",
                depth: 2,
                expected: &[Expected {
                    callee: ".value",
                    range_text: ".value",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "a().b().c().e.d()",
                output: "a().b()\n.c().e.d()",
                depth: 2,
                expected: &[Expected {
                    callee: ".c",
                    range_text: ".c",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "a.b.c().e().d()",
                output: "a.b.c().e()\n.d()",
                depth: 2,
                expected: &[Expected {
                    callee: ".d",
                    range_text: ".d",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "_.chain({}).map(a).value(); ",
                output: "_.chain({}).map(a)\n.value(); ",
                depth: 2,
                expected: &[Expected {
                    callee: ".value",
                    range_text: ".value",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "var a = m1.m2();\n var b = m1.m2().m3().m4().m5();",
                output: "var a = m1.m2();\n var b = m1.m2().m3()\n.m4()\n.m5();",
                depth: 2,
                expected: &[
                    Expected {
                        callee: ".m4",
                        range_text: ".m4",
                        occurrence: 0,
                    },
                    Expected {
                        callee: ".m5",
                        range_text: ".m5",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "var a = m1.m2();\n var b = m1.m2().m3()\n.m4().m5();",
                output: "var a = m1.m2();\n var b = m1.m2().m3()\n.m4()\n.m5();",
                depth: 2,
                expected: &[Expected {
                    callee: ".m5",
                    range_text: ".m5",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "var a = m1().m2\n.m3().m4().m5().m6().m7();",
                output: "var a = m1().m2\n.m3().m4().m5()\n.m6()\n.m7();",
                depth: 3,
                expected: &[
                    Expected {
                        callee: ".m6",
                        range_text: ".m6",
                        occurrence: 0,
                    },
                    Expected {
                        callee: ".m7",
                        range_text: ".m7",
                        occurrence: 0,
                    },
                ],
            },
        ];

        for case in cases {
            assert_invalid(case);
        }
    }

    #[test]
    fn matches_upstream_comment_parenthesis_and_computed_fixtures() {
        let cases = [
            InvalidCase {
                source: "foo.bar()['foo' + \u{2029} + 'bar']()",
                output: "foo.bar()\n['foo' + \u{2029} + 'bar']()",
                depth: 1,
                expected: &[Expected {
                    callee: "['foo' + ",
                    range_text: "['foo' + \u{2029} + 'bar']",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "foo.bar()[(biz)]()",
                output: "foo.bar()\n[(biz)]()",
                depth: 1,
                expected: &[Expected {
                    callee: "[biz]",
                    range_text: "[(biz)]",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "(foo).bar().biz()",
                output: "(foo).bar()\n.biz()",
                depth: 1,
                expected: &[Expected {
                    callee: ".biz",
                    range_text: ".biz",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "foo.bar(). /* comment */ biz()",
                output: "foo.bar()\n. /* comment */ biz()",
                depth: 1,
                expected: &[Expected {
                    callee: ".biz",
                    range_text: ". /* comment */ biz",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "foo.bar() /* comment */ .biz()",
                output: "foo.bar() /* comment */ \n.biz()",
                depth: 1,
                expected: &[Expected {
                    callee: ".biz",
                    range_text: ".biz",
                    occurrence: 0,
                }],
            },
            InvalidCase {
                source: "((foo.bar()) . baz()).quux();",
                output: "((foo.bar()) \n. baz())\n.quux();",
                depth: 1,
                expected: &[
                    Expected {
                        callee: ".baz",
                        range_text: ". baz",
                        occurrence: 0,
                    },
                    Expected {
                        callee: ".quux",
                        range_text: ".quux",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "((foo.bar()) [a + b] ()) [(c + d)]()",
                output: "((foo.bar()) \n[a + b] ()) \n[(c + d)]()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "[a + b]",
                        range_text: "[a + b]",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "[c + d]",
                        range_text: "[(c + d)]",
                        occurrence: 0,
                    },
                ],
            },
        ];

        for case in cases {
            assert_invalid(case);
        }
    }

    #[test]
    fn matches_upstream_multiline_computed_fixture() {
        let source = concat!(
            "anObject.method1().method2()['method' + n]()[aCondition ?\n",
            "    'method3' :\n",
            "    'method4']()"
        );
        let output = concat!(
            "anObject.method1().method2()\n",
            "['method' + n]()\n",
            "[aCondition ?\n",
            "    'method3' :\n",
            "    'method4']()"
        );
        assert_invalid(InvalidCase {
            source,
            output,
            depth: 2,
            expected: &[
                Expected {
                    callee: "['method' + n]",
                    range_text: "['method' + n]",
                    occurrence: 0,
                },
                Expected {
                    callee: "[aCondition ?",
                    range_text: "[aCondition ?\n    'method3' :\n    'method4']",
                    occurrence: 0,
                },
            ],
        });
    }

    #[test]
    fn matches_upstream_long_multiline_call_fixture() {
        let source = concat!(
            "http.request({\n",
            "    // Param\n",
            "    // Param\n",
            "    // Param\n",
            "}).on('response', function(response) {\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "}).on('error', function(error) {\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "}).end();"
        );
        let output = concat!(
            "http.request({\n",
            "    // Param\n",
            "    // Param\n",
            "    // Param\n",
            "}).on('response', function(response) {\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "    // Do something with response.\n",
            "})\n",
            ".on('error', function(error) {\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "    // Do something with error.\n",
            "})\n",
            ".end();"
        );
        assert_invalid(InvalidCase {
            source,
            output,
            depth: 2,
            expected: &[
                Expected {
                    callee: ".on",
                    range_text: ".on",
                    occurrence: 1,
                },
                Expected {
                    callee: ".end",
                    range_text: ".end",
                    occurrence: 0,
                },
            ],
        });
    }

    #[test]
    fn matches_all_upstream_optional_chain_fixtures() {
        let cases = [
            InvalidCase {
                source: "obj?.foo1()?.foo2()?.foo3()",
                output: "obj?.foo1()\n?.foo2()\n?.foo3()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "?.foo2",
                        range_text: "?.foo2",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "?.foo3",
                        range_text: "?.foo3",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "(obj?.foo1()?.foo2)()?.foo3()",
                output: "(obj?.foo1()\n?.foo2)()\n?.foo3()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "?.foo2",
                        range_text: "?.foo2",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "?.foo3",
                        range_text: "?.foo3",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "(obj?.foo1())?.foo2()?.foo3()",
                output: "(obj?.foo1())\n?.foo2()\n?.foo3()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "?.foo2",
                        range_text: "?.foo2",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "?.foo3",
                        range_text: "?.foo3",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "obj?.[foo1]()?.[foo2]()?.[foo3]()",
                output: "obj?.[foo1]()\n?.[foo2]()\n?.[foo3]()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "?.[foo2]",
                        range_text: "?.[foo2]",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "?.[foo3]",
                        range_text: "?.[foo3]",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "(obj?.[foo1]()?.[foo2])()?.[foo3]()",
                output: "(obj?.[foo1]()\n?.[foo2])()\n?.[foo3]()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "?.[foo2]",
                        range_text: "?.[foo2]",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "?.[foo3]",
                        range_text: "?.[foo3]",
                        occurrence: 0,
                    },
                ],
            },
            InvalidCase {
                source: "(obj?.[foo1]())?.[foo2]()?.[foo3]()",
                output: "(obj?.[foo1]())\n?.[foo2]()\n?.[foo3]()",
                depth: 1,
                expected: &[
                    Expected {
                        callee: "?.[foo2]",
                        range_text: "?.[foo2]",
                        occurrence: 0,
                    },
                    Expected {
                        callee: "?.[foo3]",
                        range_text: "?.[foo3]",
                        occurrence: 0,
                    },
                ],
            },
        ];

        for case in cases {
            assert_invalid(case);
        }
    }

    #[test]
    fn handles_typescript_jsx_new_calls_and_private_members_without_false_positives() {
        for source in [
            "const plain = object.deep.property.only;",
            "const tagged = first().second`template`;",
            "const splitParen = (first()\n).second();",
        ] {
            assert!(
                run(source, 1).is_empty(),
                "false positive for non-violating source: {source}"
            );
        }

        // Independent depth-two chains must not be combined into one deeper
        // chain merely because they share a containing expression.
        for source in [
            "const separate = outer(first().second(), third().fourth());",
            "const nested = () => first().second();",
            "const jsx = <View value={first().second()} />;",
            "const existing = first().second()\n.third();",
        ] {
            assert!(
                run(source, 2).is_empty(),
                "independent chains must retain their own depth: {source}"
            );
        }

        for source in [
            "const typed = service.first<T>().second<U>();",
            "const jsx = <View value={service.first().second()} />;",
            "class Box { #value() {} run() { return this.#value().toString(); } }",
        ] {
            assert_eq!(
                run(source, 1).len(),
                1,
                "expected one chained-call violation: {source}"
            );
        }

        assert_eq!(
            run("const constructed = new Factory().first().second();", 1).len(),
            2,
            "a NewExpression participates in the upstream callee-depth walk"
        );
        assert_eq!(
            run("with (service) { first().second().third(); }", 2).len(),
            1,
            "classic JavaScript scripts must use the script parser fallback"
        );
    }

    #[test]
    fn invalid_options_fall_back_to_the_stable_default_depth() {
        for options in [
            Value::Null,
            json!([]),
            json!([{}]),
            json!([{ "ignoreChainWithDepth": 0 }]),
            json!([{ "ignoreChainWithDepth": 11 }]),
            json!([{ "ignoreChainWithDepth": "2" }]),
        ] {
            let mut diagnostics = Vec::new();
            check_newline_per_chained_call("first().second().third()", &options, &mut diagnostics);
            assert_eq!(diagnostics.len(), 1);
        }
    }

    #[test]
    fn malformed_sources_and_text_lookalikes_do_not_report() {
        for source in [
            "const text = 'first().second().third()';",
            "// first().second().third()\nconst value = 1;",
            "const broken = first().second().",
            "const regex = /first\\(\\)\\.second\\(\\)/;",
        ] {
            assert!(run(source, 1).is_empty(), "false positive: {source}");
        }
    }
}
