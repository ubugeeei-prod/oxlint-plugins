//! Driver that walks the parsed program and reports unused-imports diagnostics.

use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier, Program, Statement};
use oxc_semantic::SymbolFlags;
use oxlint_plugins_carton::{FastHashMap, SmallVec};

use crate::fixers::{fix_remove_declaration, fix_remove_specifier, unused_message};
use crate::types::{
    DeclarationKey, DeclarationUsage, Diagnostic, ImportBinding, ImportSpecifierKind, LineIndex,
};

pub(crate) fn should_report_unused_symbol(flags: SymbolFlags) -> bool {
    flags.intersects(
        SymbolFlags::Variable
            | SymbolFlags::Function
            | SymbolFlags::Class
            | SymbolFlags::TypeAlias
            | SymbolFlags::Interface
            | SymbolFlags::Enum
            | SymbolFlags::TypeParameter
            | SymbolFlags::CatchVariable,
    )
}

pub(crate) fn collect_import_bindings<'a>(
    program: &'a Program<'a>,
) -> SmallVec<[ImportBinding<'a>; 16]> {
    let mut bindings = SmallVec::new();
    for statement in &program.body {
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };
        bindings.extend(bindings_for_declaration(declaration));
    }
    bindings
}

fn bindings_for_declaration<'a>(
    declaration: &'a ImportDeclaration<'a>,
) -> SmallVec<[ImportBinding<'a>; 4]> {
    let Some(specifiers) = &declaration.specifiers else {
        return SmallVec::new();
    };
    let specifier_count = specifiers.len();
    let named_specifier_count = specifiers
        .iter()
        .filter(|specifier| matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(_)))
        .count();
    let mut bindings = SmallVec::new();
    for (specifier_index, specifier) in specifiers.iter().enumerate() {
        let (name, local_span, specifier_span, kind) = match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(specifier) => (
                specifier.local.name.as_str(),
                specifier.local.span,
                specifier.span,
                ImportSpecifierKind::Named,
            ),
            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => (
                specifier.local.name.as_str(),
                specifier.local.span,
                specifier.span,
                ImportSpecifierKind::Default,
            ),
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => (
                specifier.local.name.as_str(),
                specifier.local.span,
                specifier.span,
                ImportSpecifierKind::Namespace,
            ),
        };
        bindings.push(ImportBinding {
            name,
            local_span,
            specifier_span,
            declaration_span: declaration.span,
            specifier_index,
            specifier_count,
            named_specifier_count,
            kind,
        });
    }
    bindings
}

pub(crate) fn report_unused_imports(
    source_text: &str,
    line_index: &LineIndex,
    unused_imports: &[ImportBinding<'_>],
) -> SmallVec<[Diagnostic; 16]> {
    let mut declarations: FastHashMap<DeclarationKey, DeclarationUsage> = FastHashMap::default();
    for binding in unused_imports {
        let key = DeclarationKey::from(binding.declaration_span);
        declarations
            .entry(key)
            .and_modify(|usage| {
                usage.unused_count += 1;
                usage.first_unused_index = usage.first_unused_index.min(binding.specifier_index);
            })
            .or_insert(DeclarationUsage {
                specifier_count: binding.specifier_count,
                unused_count: 1,
                first_unused_index: binding.specifier_index,
            });
    }

    let mut diagnostics = SmallVec::new();
    for binding in unused_imports {
        let usage = declarations
            .get(&DeclarationKey::from(binding.declaration_span))
            .expect("declaration usage is inserted before reporting");
        let remove_declaration = usage.unused_count == usage.specifier_count;
        if remove_declaration && binding.specifier_index != usage.first_unused_index {
            continue;
        }
        diagnostics.push(Diagnostic {
            rule_name: "no-unused-imports",
            message: unused_message(binding.name),
            loc: line_index.loc_for_span(source_text, binding.local_span),
            fix: Some(if remove_declaration {
                fix_remove_declaration(source_text, line_index, binding.declaration_span)
            } else {
                fix_remove_specifier(source_text, *binding)
            }),
        });
    }
    diagnostics
}
