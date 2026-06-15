#![doc = "Rust core for the eslint-plugin-postgresql oxlint plugin."]
//!
//! SQL is parsed once (libpg_query, PostgreSQL 17) into the enriched JSON AST
//! produced by [`parse`], then every enabled rule walks that shared tree. The
//! public entry point is [`scan_postgresql`]; the NAPI wrapper in
//! `npm/postgresql` exposes it to the JavaScript Oxlint adapter.

// Keep the statically linked libpg_query (and its build-script link directives)
// in the dependency graph even though we reach its C entry point through `ffi`.
use pg_query as _;

mod ast;
mod embedded_code;
mod eslint;
mod ffi;
mod manipulate;
mod parse;
mod rules;
mod text;
mod tokenize;

use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::embedded_code::attach_embedded_code_to_stmts;
use crate::parse::{ParseError, parse};
use crate::text::Source;

/// The parser entry points consumed by `npm/postgresql-eslint-parser`:
/// `parse_for_eslint` returns the `{ ast, visitorKeys, scopeManager }` value,
/// `parse_for_eslint_json` serializes it for the NAPI boundary, and `parse_ast`
/// returns only the AST (upstream's `parse`).
pub use crate::eslint::{parse_ast, parse_for_eslint, parse_for_eslint_json};

/// Every upstream rule name, in the order listed in the port inventory.
pub use crate::rules::RULE_NAMES;

/// Options for a single `scan_postgresql` invocation. The JS adapter calls the
/// scanner once per rule, passing that rule's name and its raw ESLint options
/// array (verbatim JSON) so each rule reads its own option shape.
#[derive(Clone, Debug, Default)]
pub struct ScanOptions {
    pub rule_names: SmallVec<[CompactString; 4]>,
    pub options: Value,
}

impl ScanOptions {
    pub fn is_enabled(&self, name: &str) -> bool {
        self.rule_names.iter().any(|r| r == name)
    }
}

/// A single key/value interpolation pair for a message template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticDatum {
    pub key: CompactString,
    pub value: CompactString,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub data: SmallVec<[DiagnosticDatum; 2]>,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

/// The names of every rule this crate implements (a subset of [`RULE_NAMES`]
/// while the port is in progress). Drives the JS adapter's rule registry.
pub fn implemented_postgresql_rule_names() -> &'static [&'static str] {
    crate::rules::IMPLEMENTED_RULE_NAMES
}

/// Lint `source_text` (raw SQL) with the rules enabled in `options`.
pub fn scan_postgresql(source_text: &str, options: &ScanOptions) -> SmallVec<[Diagnostic; 8]> {
    let mut parsed = parse(source_text);
    // Attach EmbeddedCode nodes to CreateFunctionStmt so that rules like
    // `plpgsql-keyword-case` can visit them, mirroring the attachment that
    // `parse_for_eslint` performs for the ESLint adapter.
    attach_embedded_code_to_stmts(&mut parsed.statements, &parsed.tokens, &parsed.source);
    let mut ctx = RuleContext {
        source: &parsed.source,
        options: &options.options,
        error: parsed.error.as_ref(),
        statements: &parsed.statements,
        tokens: &parsed.tokens,
        diagnostics: SmallVec::new(),
        rule_name: "",
    };

    for rule in crate::rules::REGISTRY {
        if !options.is_enabled(rule.name) {
            continue;
        }
        ctx.rule_name = rule.name;
        if rule.uses_parse_error {
            (rule.run)(&Value::Null, &[], &mut ctx);
        }
        let mut ancestors: SmallVec<[&Value; 32]> = SmallVec::new();
        for stmt in &parsed.statements {
            visit(stmt, &mut ancestors, &mut ctx, rule.run);
        }
    }

    // ESLint reports diagnostics sorted by source position.
    ctx.diagnostics.sort_by_key(|d| {
        (
            d.loc.start_line,
            d.loc.start_column,
            d.loc.end_line,
            d.loc.end_column,
        )
    });
    ctx.diagnostics
}

/// Per-scan context handed to each rule's `run` function.
pub struct RuleContext<'a> {
    pub source: &'a Source,
    pub options: &'a Value,
    pub error: Option<&'a ParseError>,
    /// All top-level statement nodes for the parsed SQL file.  Rules that need
    /// program-exit behaviour (e.g. `require-index-on-fk-column`) walk this
    /// slice directly instead of relying on the visitor dispatch.
    pub statements: &'a [Value],
    /// The token stream produced by the single tokenization of the source file.
    /// Rules that need to walk the token stream (e.g. `prefer-keyword-case`)
    /// use this shared slice instead of calling `tokenize(ctx.source)` again.
    pub tokens: &'a [crate::tokenize::Token],
    diagnostics: SmallVec<[Diagnostic; 8]>,
    rule_name: &'static str,
}

impl RuleContext<'_> {
    /// Report against a node's own `loc`, with no message data.
    pub fn report(&mut self, node: &Value, message_id: &'static str) {
        if let Some(loc) = loc_of(node) {
            self.push(loc, message_id, SmallVec::new(), None);
        }
    }

    /// Report against a node's own `loc`, with interpolation data.
    pub fn report_data(
        &mut self,
        node: &Value,
        message_id: &'static str,
        data: SmallVec<[DiagnosticDatum; 2]>,
    ) {
        if let Some(loc) = loc_of(node) {
            self.push(loc, message_id, data, None);
        }
    }

    /// Report at an explicit location (e.g. the whole program for a parse error).
    pub fn report_loc(
        &mut self,
        loc: DiagnosticLoc,
        message_id: &'static str,
        data: SmallVec<[DiagnosticDatum; 2]>,
        fix: Option<DiagnosticFix>,
    ) {
        self.push(loc, message_id, data, fix);
    }

    fn push(
        &mut self,
        loc: DiagnosticLoc,
        message_id: &'static str,
        data: SmallVec<[DiagnosticDatum; 2]>,
        fix: Option<DiagnosticFix>,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name: self.rule_name,
            message_id,
            data,
            loc,
            fix,
        });
    }
}

/// A rule entry in the dispatch registry.
pub(crate) struct RuleDef {
    pub name: &'static str,
    /// Called for every node in the tree (the rule matches its own node types).
    pub run: fn(&Value, &[&Value], &mut RuleContext),
    /// When true, the rule is also invoked once with `Value::Null` so it can
    /// react to a captured parse error (e.g. `no-syntax-error`).
    pub uses_parse_error: bool,
}

/// Depth-first walk that invokes `run` for every node, tracking the ancestor
/// chain (nearest-last) so rules can inspect their parent/context.
fn visit<'a>(
    node: &'a Value,
    ancestors: &mut SmallVec<[&'a Value; 32]>,
    ctx: &mut RuleContext,
    run: fn(&Value, &[&Value], &mut RuleContext),
) {
    if node.get("type").is_some() {
        run(node, ancestors.as_slice(), ctx);
    }
    ancestors.push(node);
    match node {
        Value::Object(map) => {
            for (key, value) in map {
                if is_recursible(key, value) {
                    visit(value, ancestors, ctx, run);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                visit(item, ancestors, ctx, run);
            }
        }
        _ => {}
    }
    ancestors.pop();
}

fn is_recursible(key: &str, value: &Value) -> bool {
    !matches!(key, "parent" | "type" | "range" | "loc") && (value.is_object() || value.is_array())
}

pub(crate) fn loc_of(node: &Value) -> Option<DiagnosticLoc> {
    let loc = node.get("loc")?;
    let start = loc.get("start")?;
    let end = loc.get("end")?;
    Some(DiagnosticLoc {
        start_line: start.get("line")?.as_u64()? as u32,
        start_column: start.get("column")?.as_u64()? as u32,
        end_line: end.get("line")?.as_u64()? as u32,
        end_column: end.get("column")?.as_u64()? as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(rule: &str, sql: &str) -> SmallVec<[Diagnostic; 8]> {
        let opts = ScanOptions {
            rule_names: SmallVec::from_iter([CompactString::from(rule)]),
            options: Value::Null,
        };
        scan_postgresql(sql, &opts)
    }

    #[test]
    fn no_select_star_flags_star() {
        let diags = scan("no-select-star", "SELECT * FROM users");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message_id, "noSelectStar");
        assert_eq!(diags[0].loc.start_line, 1);
        assert_eq!(diags[0].loc.start_column, 7);
    }

    #[test]
    fn no_select_star_allows_explicit_columns() {
        let diags = scan("no-select-star", "SELECT id, name FROM users");
        assert!(diags.is_empty());
    }
}
