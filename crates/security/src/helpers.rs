//! Free helper functions and additional constants used by the security scanner.

#![allow(
    unused_imports,
    reason = "Helpers share the security AST import surface; not every helper uses every type."
)]

use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BindingPattern, CallExpression, Expression,
    ImportDeclarationSpecifier, ModuleExportName, ObjectPropertyKind, PropertyKey, Statement,
    StaticMemberExpression,
};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{AccessPath, Binding, TIMING_KEYWORDS};

pub(crate) const INTERESTING_PACKAGES: [&str; 13] = [
    "child_process",
    "node:child_process",
    "fs",
    "node:fs",
    "fs/promises",
    "node:fs/promises",
    "fs-extra",
    "path",
    "node:path",
    "path/posix",
    "node:path/posix",
    "url",
    "node:url",
];

pub(crate) const BUFFER_READ_METHODS: [&str; 14] = [
    "readUInt8",
    "readUInt16LE",
    "readUInt16BE",
    "readUInt32LE",
    "readUInt32BE",
    "readInt8",
    "readInt16LE",
    "readInt16BE",
    "readInt32LE",
    "readInt32BE",
    "readFloatLE",
    "readFloatBE",
    "readDoubleLE",
    "readDoubleBE",
];

pub(crate) const BUFFER_WRITE_METHODS: [&str; 14] = [
    "writeUInt8",
    "writeUInt16LE",
    "writeUInt16BE",
    "writeUInt32LE",
    "writeUInt32BE",
    "writeInt8",
    "writeInt16LE",
    "writeInt16BE",
    "writeInt32LE",
    "writeInt32BE",
    "writeFloatLE",
    "writeFloatBE",
    "writeDoubleLE",
    "writeDoubleBE",
];

pub(crate) fn small_path<const N: usize>(values: [&str; N]) -> SmallVec<[CompactString; 4]> {
    values.into_iter().map(CompactString::from).collect()
}

pub(crate) fn is_interesting_package(package_name: &str) -> bool {
    INTERESTING_PACKAGES.contains(&package_name)
}

pub(crate) fn module_export_name<'a>(name: &'a ModuleExportName<'a>) -> Option<&'a str> {
    match name {
        ModuleExportName::IdentifierName(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        ModuleExportName::StringLiteral(literal) => Some(literal.value.as_str()),
    }
}

pub(crate) fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(literal) => Some(literal.value.as_str()),
        PropertyKey::Identifier(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

pub(crate) fn static_member_property<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    let Expression::StaticMemberExpression(member) = expression.get_inner_expression() else {
        return None;
    };
    Some(member.property.name.as_str())
}

pub(crate) fn argument_is_literal(argument: &Argument<'_>) -> bool {
    argument.as_expression().is_some_and(Expression::is_literal)
}

pub(crate) fn array_element_expression<'a>(
    element: &'a ArrayExpressionElement<'a>,
) -> Option<&'a Expression<'a>> {
    element.as_expression()
}

pub(crate) fn string_literal_value<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Some(literal.value.as_str()),
        _ => None,
    }
}

pub(crate) fn contains_timing_keyword(expression: &Expression<'_>) -> bool {
    let Expression::Identifier(identifier) = expression.get_inner_expression() else {
        return false;
    };
    let name = identifier.name.as_str();
    TIMING_KEYWORDS
        .iter()
        .any(|keyword| name.eq_ignore_ascii_case(keyword))
}

pub(crate) fn is_import_meta_url(expression: &Expression<'_>) -> bool {
    let Expression::StaticMemberExpression(member) = expression.get_inner_expression() else {
        return false;
    };
    if member.property.name != "url" {
        return false;
    }
    matches!(
        member.object.get_inner_expression(),
        Expression::MetaProperty(meta)
            if meta.meta.name == "import" && meta.property.name == "meta"
    )
}

pub(crate) fn source_line_at(source_text: &str, offset: usize) -> &str {
    let start = source_text[..offset]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let end = source_text[offset..]
        .find('\n')
        .map_or(source_text.len(), |index| offset + index);
    &source_text[start..end]
}

pub(crate) fn is_dangerous_bidi(ch: char) -> bool {
    matches!(
        ch,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'
            | '\u{202b}'
            | '\u{202c}'
            | '\u{202d}'
            | '\u{202e}'
            | '\u{2066}'
            | '\u{2067}'
            | '\u{2068}'
            | '\u{2069}'
    )
}

pub(crate) fn expression_type(expression: &Expression<'_>) -> &'static str {
    match expression.get_inner_expression() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::StringLiteral(_) => "Literal",
        Expression::TemplateLiteral(_) => "TemplateLiteral",
        Expression::Identifier(_) => "Identifier",
        Expression::CallExpression(_) => "CallExpression",
        Expression::NewExpression(_) => "NewExpression",
        Expression::StaticMemberExpression(_) => "MemberExpression",
        Expression::ComputedMemberExpression(_) => "MemberExpression",
        Expression::BinaryExpression(_) => "BinaryExpression",
        Expression::ObjectExpression(_) => "ObjectExpression",
        Expression::ArrayExpression(_) => "ArrayExpression",
        Expression::ArrowFunctionExpression(_) => "ArrowFunctionExpression",
        Expression::FunctionExpression(_) => "FunctionExpression",
        _ => "Expression",
    }
}

pub(crate) fn fs_argument_indices(fn_name: &str) -> Option<&'static [usize]> {
    match fn_name {
        "appendFile" | "appendFileSync" | "chmod" | "chmodSync" | "chown" | "chownSync"
        | "createReadStream" | "createWriteStream" | "exists" | "existsSync" | "lchmod"
        | "lchmodSync" | "lchown" | "lchownSync" | "lstat" | "lstatSync" | "mkdir"
        | "mkdirSync" | "open" | "openSync" | "readdir" | "readdirSync" | "readFile"
        | "readFileSync" | "readlink" | "readlinkSync" | "realpath" | "realpathSync" | "rmdir"
        | "rmdirSync" | "stat" | "statSync" | "truncate" | "truncateSync" | "unlink"
        | "unlinkSync" | "unwatchFile" | "utimes" | "utimesSync" | "watch" | "watchFile"
        | "writeFile" | "writeFileSync" => Some(&[0]),
        "link" | "linkSync" | "rename" | "renameSync" | "symlink" | "symlinkSync" => Some(&[0, 1]),
        _ => None,
    }
}

pub(crate) fn join_usize(values: &[usize]) -> CompactString {
    let mut out = CompactString::new("");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(match value {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            _ => "?",
        });
    }
    out
}

pub fn is_unsafe_regex(pattern: &str) -> bool {
    let chars: SmallVec<[char; 64]> = pattern.chars().collect();
    let mut stack: SmallVec<[bool; 8]> = SmallVec::new();
    let mut escaped = false;

    for (index, ch) in chars.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match ch {
            '(' => stack.push(false),
            '*' | '+' => {
                if let Some(in_group) = stack.last_mut() {
                    *in_group = true;
                }
            }
            '{' => {
                if let Some(in_group) = stack.last_mut() {
                    *in_group = true;
                }
            }
            ')' => {
                let group_has_quantifier = stack.pop().unwrap_or_else(|| {
                    index > 0 && is_regex_quantifier(chars[index.saturating_sub(1)])
                });
                if group_has_quantifier
                    && chars
                        .get(index + 1)
                        .copied()
                        .is_some_and(is_regex_quantifier)
                {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

pub(crate) fn is_regex_quantifier(ch: char) -> bool {
    matches!(ch, '*' | '+' | '?' | '{')
}
