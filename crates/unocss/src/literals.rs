//! Source-text scanners that find class-string literals and identify
//! UnoCSS-relevant call sites.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::types::LiteralSpan;

pub(crate) fn collect_literals(source_text: &str) -> SmallVec<[LiteralSpan<'_>; 16]> {
    let mut literals = SmallVec::new();
    let bytes = source_text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[index];
                let content_start = index + 1;
                let mut cursor = content_start;
                let mut has_template_expr = false;
                while cursor < bytes.len() {
                    if bytes[cursor] == b'\\' {
                        cursor = (cursor + 2).min(bytes.len());
                        continue;
                    }
                    if quote == b'`'
                        && cursor + 1 < bytes.len()
                        && bytes[cursor] == b'$'
                        && bytes[cursor + 1] == b'{'
                    {
                        has_template_expr = true;
                    }
                    if bytes[cursor] == quote {
                        if !has_template_expr {
                            literals.push(LiteralSpan {
                                full_start: index,
                                content_start,
                                content_end: cursor,
                                content: &source_text[content_start..cursor],
                            });
                        }
                        index = cursor + 1;
                        break;
                    }
                    cursor += 1;
                }
                if cursor >= bytes.len() {
                    break;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            _ => index += 1,
        }
    }
    literals
}

pub(crate) fn is_jsx_class_literal(source_text: &str, literal: LiteralSpan<'_>) -> bool {
    let prefix_start = literal.full_start.saturating_sub(80);
    let prefix = source_text[prefix_start..literal.full_start].trim_end();
    let candidates = [
        "class=",
        "className=",
        "class={",
        "className={",
        "classname=",
        "classname={",
    ];
    candidates
        .iter()
        .any(|candidate| prefix.ends_with(candidate))
}

pub(crate) fn is_uno_call_literal(
    source_text: &str,
    start: usize,
    uno_functions: &[CompactString],
) -> bool {
    let prefix_start = start.saturating_sub(800);
    let prefix = &source_text[prefix_start..start];
    let last_statement = prefix
        .rfind(';')
        .map_or(prefix, |index| &prefix[index + 1..]);
    uno_functions.iter().any(|function| {
        let Some(call_index) = rfind_function_call(last_statement, function.as_str()) else {
            return false;
        };
        let after_call = &last_statement[call_index + function.len() + 1..];
        after_call.bytes().filter(|byte| *byte == b'(').count()
            <= after_call.bytes().filter(|byte| *byte == b')').count()
    }) || uno_functions.iter().any(|function| {
        let Some(call_index) = rfind_function_call(last_statement, function.as_str()) else {
            return false;
        };
        let after_call = &last_statement[call_index..];
        let opens = after_call.bytes().filter(|byte| *byte == b'(').count();
        let closes = after_call.bytes().filter(|byte| *byte == b')').count();
        opens > closes
    })
}

fn rfind_function_call(statement: &str, function_name: &str) -> Option<usize> {
    let mut end = statement.len();
    while let Some(index) = statement[..end].rfind(function_name) {
        let before = index
            .checked_sub(1)
            .and_then(|previous| statement.as_bytes().get(previous));
        if before.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'$')
        {
            end = index;
            continue;
        }
        let Some(after) = statement.as_bytes().get(index + function_name.len()) else {
            end = index;
            continue;
        };
        if *after == b'(' {
            return Some(index);
        }
        end = index;
    }
    None
}

pub(crate) fn variable_name_in_statement(statement: &str) -> Option<&str> {
    for keyword in ["const", "let", "var"] {
        let Some(index) = statement.rfind(keyword) else {
            continue;
        };
        let after_keyword = statement[index + keyword.len()..].trim_start();
        let name_end = after_keyword
            .find(|ch: char| !(ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()))
            .unwrap_or(after_keyword.len());
        if name_end == 0 {
            continue;
        }
        let name = &after_keyword[..name_end];
        let after_name = after_keyword[name_end..].trim_start();
        if after_name.starts_with('=') {
            return Some(name);
        }
    }
    None
}
