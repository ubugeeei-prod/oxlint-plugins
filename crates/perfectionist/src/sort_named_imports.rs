//! Configurable `sort-named-imports` slice pinned to Perfectionist v5.9.1.

use std::cmp::Ordering;

use oxc_ast::{
    Comment,
    ast::{ImportDeclarationSpecifier, ImportSpecifier, ModuleExportName, Statement},
};
use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::sort_options::SortOptions;
use crate::types::{LineIndex, RuleDiagnostic, RuleDiagnosticData, RuleDiagnosticFix};

const RULE: &str = "sort-named-imports";
const MESSAGE_ID: &str = "unexpectedNamedImportsOrder";

struct NamedImport<'a> {
    span: Span,
    name: CompactString,
    source: &'a str,
    source_start: u32,
    source_end: u32,
    size: usize,
}

pub(crate) fn check(
    source_text: &str,
    body: &[Statement<'_>],
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    let options = SortOptions::from_json(raw_options);
    let lines = LineIndex::new(source_text);
    let mut diagnostics = SmallVec::new();

    for statement in body {
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };
        let Some(specifiers) = declaration.specifiers.as_ref() else {
            continue;
        };
        let named_specifiers: SmallVec<[&ImportSpecifier<'_>; 8]> = specifiers
            .iter()
            .filter_map(|specifier| {
                let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                    return None;
                };
                Some(&**specifier)
            })
            .collect();
        let imports: SmallVec<[NamedImport<'_>; 8]> = named_specifiers
            .iter()
            .enumerate()
            .filter_map(|(index, specifier)| {
                let boundary = named_specifiers
                    .get(index + 1)
                    .map_or(declaration.span.end, |next| next.span.start);
                named_import(
                    source_text,
                    comments,
                    specifier,
                    boundary,
                    options.ignore_alias,
                )
            })
            .collect();
        if imports.len() < 2 {
            continue;
        }

        let mut sorted_indices: SmallVec<[usize; 8]> = (0..imports.len()).collect();
        sorted_indices.sort_by(|&left, &right| {
            compare_imports(&options, &imports[left], &imports[right])
                .then_with(|| left.cmp(&right))
        });
        if sorted_indices
            .iter()
            .enumerate()
            .all(|(position, &original)| position == original)
        {
            continue;
        }

        let Some(fix) = build_fix(source_text, &imports, &sorted_indices) else {
            continue;
        };
        for pair in imports.windows(2) {
            let [left, right] = pair else {
                continue;
            };
            if compare_imports(&options, left, right) != Ordering::Greater {
                continue;
            }
            diagnostics.push(RuleDiagnostic {
                rule_name: RULE,
                message_id: MESSAGE_ID,
                data: RuleDiagnosticData {
                    left: left.name.clone(),
                    right: right.name.clone(),
                },
                loc: lines.loc_for_span(source_text, right.span),
                fix: fix.clone(),
            });
        }
    }
    diagnostics
}

fn named_import<'a>(
    source_text: &'a str,
    comments: &[Comment],
    specifier: &ImportSpecifier<'_>,
    boundary: u32,
    ignore_alias: bool,
) -> Option<NamedImport<'a>> {
    let source_start = comments
        .iter()
        .filter(|comment| {
            comment.attached_to == specifier.span.start && comment.span.end <= specifier.span.start
        })
        .map(|comment| comment.span.start)
        .min()
        .unwrap_or(specifier.span.start);
    let source_end = comments
        .iter()
        .filter(|comment| {
            comment.span.start >= specifier.span.end
                && comment.span.end <= boundary
                && is_same_line(source_text, specifier.span.end, comment.span.start)
        })
        .map(|comment| comment.span.end)
        .max()
        .unwrap_or(specifier.span.end);
    let start = usize::try_from(source_start).ok()?;
    let end = usize::try_from(source_end).ok()?;
    let source = source_text.get(start..end)?;
    let node_start = usize::try_from(specifier.span.start).ok()?;
    let node_end = usize::try_from(specifier.span.end).ok()?;
    let size = source_text
        .get(node_start..node_end)?
        .encode_utf16()
        .count();
    let name = if ignore_alias {
        module_export_name(&specifier.imported)
    } else {
        CompactString::from(specifier.local.name.as_str())
    };
    Some(NamedImport {
        span: specifier.span,
        name,
        source,
        source_start,
        source_end,
        size,
    })
}

fn compare_imports(
    options: &SortOptions,
    left: &NamedImport<'_>,
    right: &NamedImport<'_>,
) -> Ordering {
    options.compare(
        left.name.as_str(),
        left.size,
        right.name.as_str(),
        right.size,
    )
}

fn build_fix(
    source_text: &str,
    imports: &[NamedImport<'_>],
    sorted_indices: &[usize],
) -> Option<RuleDiagnosticFix> {
    let first = sorted_indices
        .iter()
        .enumerate()
        .find_map(|(position, &original)| (position != original).then_some(position))?;
    let last = sorted_indices
        .iter()
        .enumerate()
        .rfind(|(position, original)| *position != **original)
        .map(|(position, _)| position)?;
    let start = usize::try_from(imports[first].source_start).ok()?;
    let end = usize::try_from(imports[last].source_end).ok()?;
    let mut replacement = CompactString::new("");
    for position in first..=last {
        if position > first {
            let separator_start = usize::try_from(imports[position - 1].source_end).ok()?;
            let separator_end = usize::try_from(imports[position].source_start).ok()?;
            replacement.push_str(source_text.get(separator_start..separator_end)?);
        }
        replacement.push_str(imports[*sorted_indices.get(position)?].source);
    }
    Some(RuleDiagnosticFix {
        start: LineIndex::utf16_offset(source_text, u32::try_from(start).ok()?),
        end: LineIndex::utf16_offset(source_text, u32::try_from(end).ok()?),
        replacement,
    })
}

fn is_same_line(source_text: &str, left: u32, right: u32) -> bool {
    let Ok(left) = usize::try_from(left) else {
        return false;
    };
    let Ok(right) = usize::try_from(right) else {
        return false;
    };
    source_text.get(left..right).is_some_and(|between| {
        !between
            .chars()
            .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

fn module_export_name(name: &ModuleExportName<'_>) -> CompactString {
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
