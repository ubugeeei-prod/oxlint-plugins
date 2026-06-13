//! Helpers for sorting and rewriting `{ … }` import/export specifier lists.

use oxc_ast::ast::{
    ExportSpecifier, ImportDeclaration, ImportDeclarationSpecifier, ModuleExportName,
};
use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::items::kind_rank;
use crate::scanner::span_text;

pub(crate) fn sort_import_specifiers_in_code(
    source_text: &str,
    original: &str,
    declaration: &ImportDeclaration<'_>,
) -> CompactString {
    let Some(specifiers) = &declaration.specifiers else {
        return CompactString::from(original);
    };
    let mut named: SmallVec<[(CompactString, CompactString, u8, CompactString); 8]> =
        SmallVec::new();
    for specifier in specifiers {
        if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier {
            named.push((
                module_name(&specifier.imported),
                CompactString::from(specifier.local.name.as_str()),
                kind_rank(specifier.import_kind),
                CompactString::from(span_text(source_text, specifier.span)),
            ));
        }
    }
    if named.len() <= 1 {
        return CompactString::from(original);
    }
    named.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    replace_braced_specifiers(original, named.iter().map(|item| item.3.as_str()))
}

pub(crate) fn sort_export_specifiers_in_code(
    source_text: &str,
    original: &str,
    specifiers: &[ExportSpecifier<'_>],
) -> CompactString {
    if specifiers.len() <= 1 {
        return CompactString::from(original);
    }
    let mut named: SmallVec<[(CompactString, CompactString, u8, CompactString); 8]> =
        SmallVec::new();
    for specifier in specifiers {
        named.push((
            module_name(&specifier.exported),
            module_name(&specifier.local),
            kind_rank(specifier.export_kind),
            CompactString::from(span_text(source_text, specifier.span)),
        ));
    }
    named.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    replace_braced_specifiers(original, named.iter().map(|item| item.3.as_str()))
}

pub(crate) fn replace_braced_specifiers<'a>(
    original: &str,
    sorted_specifiers: impl Iterator<Item = &'a str>,
) -> CompactString {
    let Some(open) = original.find('{') else {
        return CompactString::from(original);
    };
    let Some(close) = original.rfind('}') else {
        return CompactString::from(original);
    };
    if close <= open {
        return CompactString::from(original);
    }
    let mut out = CompactString::new("");
    out.push_str(&original[..open + 1]);
    out.push(' ');
    for (index, specifier) in sorted_specifiers.enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(specifier.trim());
    }
    out.push(' ');
    out.push_str(&original[close..]);
    out
}

pub(crate) fn module_name(name: &ModuleExportName<'_>) -> CompactString {
    match name {
        ModuleExportName::IdentifierName(identifier) => {
            CompactString::from(identifier.name.as_str())
        }
        ModuleExportName::IdentifierReference(identifier) => {
            CompactString::from(identifier.name.as_str())
        }
        ModuleExportName::StringLiteral(literal) => CompactString::from(literal.value.as_str()),
    }
}
