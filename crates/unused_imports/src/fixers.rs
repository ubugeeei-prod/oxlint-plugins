//! Autofix builders and JSDoc usage detection for unused-imports.

use std::fmt::Write as _;

use oxc_span::Span;
use oxlint_plugins_carton::CompactString;
use regex::Regex;

use crate::types::{DiagnosticFix, ImportBinding, ImportSpecifierKind, LineIndex};

pub(crate) fn unused_message(name: &str) -> CompactString {
    let mut message = CompactString::new("'");
    let _ = write!(&mut message, "{name}' is defined but never used.");
    message
}

pub(crate) fn fix_remove_declaration(
    source_text: &str,
    line_index: &LineIndex,
    declaration_span: Span,
) -> DiagnosticFix {
    let mut end = declaration_span.end;
    let mut replacement = CompactString::new("");
    let next_start = next_non_whitespace(source_text, end as usize);
    if next_start < source_text.len() {
        let import_line = line_index.line_for_offset(declaration_span.start);
        let next_line = line_index.line_for_offset(next_start as u32);
        let count = next_line.saturating_sub(import_line + 1);
        replacement = CompactString::from("\n".repeat(count as usize));
        end = next_start as u32;
    }
    DiagnosticFix {
        start: declaration_span.start,
        end,
        replacement,
    }
}

pub(crate) fn fix_remove_specifier(source_text: &str, binding: ImportBinding<'_>) -> DiagnosticFix {
    let start;
    let end;
    if binding.specifier_index + 1 < binding.specifier_count {
        start = token_before_end(source_text, binding.specifier_span.start as usize);
        end = comma_after_end(source_text, binding.specifier_span.end as usize);
    } else if binding.kind == ImportSpecifierKind::Named && binding.named_specifier_count == 1 {
        start = comma_before_named_group(source_text, binding.specifier_span.start as usize);
        end = brace_after_end(source_text, binding.specifier_span.end as usize);
    } else {
        start = comma_before_start(source_text, binding.specifier_span.start as usize);
        end = binding.specifier_span.end as usize;
    }
    DiagnosticFix {
        start: start as u32,
        end: end as u32,
        replacement: CompactString::new(""),
    }
}

fn next_non_whitespace(source_text: &str, from: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = from;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn token_before_end(source_text: &str, before: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = before.min(bytes.len());
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    index
}

fn comma_after_end(source_text: &str, after: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = after.min(bytes.len());
    while index < bytes.len() {
        match bytes[index] {
            b',' => return index + 1,
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => return after.min(bytes.len()),
        }
    }
    after.min(bytes.len())
}

fn comma_before_start(source_text: &str, before: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = before.min(bytes.len());
    while index > 0 {
        match bytes[index - 1] {
            b',' => return index - 1,
            byte if byte.is_ascii_whitespace() => index -= 1,
            _ => return before.min(bytes.len()),
        }
    }
    before.min(bytes.len())
}

fn comma_before_named_group(source_text: &str, before: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = before.min(bytes.len());
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    if index > 0 && bytes[index - 1] == b'{' {
        index -= 1;
    }
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    if index > 0 && bytes[index - 1] == b',' {
        return index - 1;
    }
    comma_before_start(source_text, before)
}

fn brace_after_end(source_text: &str, after: usize) -> usize {
    let bytes = source_text.as_bytes();
    let mut index = after.min(bytes.len());
    while index < bytes.len() {
        match bytes[index] {
            b'}' => return index + 1,
            byte if byte.is_ascii_whitespace() => index += 1,
            _ => return after.min(bytes.len()),
        }
    }
    after.min(bytes.len())
}

pub(crate) fn is_used_in_jsdoc(identifier_name: &str, source_text: &str) -> bool {
    let escaped_name = regex::escape(identifier_name);
    let mut pattern_text = CompactString::new("");
    let _ = write!(
        &mut pattern_text,
        r"(?:(?:@(?:link|linkcode|linkplain|see)\s+{escaped_name}\b)|(?:\{{@(?:link|linkcode|linkplain)\s+{escaped_name}\b\}})|(?:[@\{{](?:type|typedef|param|returns?|template|augments|extends|implements)\s+[^}}]*\b{escaped_name}\b))"
    );
    let Ok(pattern) = Regex::new(pattern_text.as_str()) else {
        return false;
    };
    let bytes = source_text.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'*' {
            let comment_start = index + 2;
            let mut end = comment_start;
            while end + 1 < bytes.len() && !(bytes[end] == b'*' && bytes[end + 1] == b'/') {
                end += 1;
            }
            if pattern.is_match(&source_text[comment_start..end]) {
                return true;
            }
            index = end.saturating_add(2);
        } else {
            index += 1;
        }
    }
    false
}
