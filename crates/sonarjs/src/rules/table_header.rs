//! Rule `table-header` (SonarJS key S5256).
//!
//! Clean-room port. Flags a JSX `<table>` data table that contains at least
//! one `<td>` data cell but no `<th>` header cell. A data table without any
//! header row is inaccessible to screen-reader users: assistive technologies
//! rely on `<th>` cells to describe what each column or row contains, so a
//! table built only from `<tr>`/`<td>` rows cannot be navigated meaningfully.
//! The accessible fix is to mark the header row with `<th scope="col">` cells
//! instead of `<td>`.
//!
//! Behaviour derived from public RSPEC S5256 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied. The
//! authoritative public RSPEC Noncompliant example is a `<table>` containing
//! only `<tr>`/`<td>` rows with NO `<th>`; the Compliant version replaces the
//! header-row cells with `<th scope="col">`.
//!
//! ## Zero-false-positive narrowing (conservative static-only subset)
//!
//! This port flags only a fully *static* table: a `<table>` element whose
//! entire subtree contains no dynamic JSX expression container (`{…}`) child or
//! spread child. The dynamic-content skip is critical for correctness — a table
//! that renders its rows via `{rows.map(r => <tr>…</tr>)}` may well contain
//! `<th>` cells that are produced dynamically and are therefore invisible to a
//! static analysis. Reporting such a table would be a false positive, so any
//! table whose subtree contains *any* dynamic content is skipped entirely.
//!
//! A single recursive pass over the table's children collects three facts:
//! whether any descendant element is named `td` (`has_td`), whether any is
//! named `th` (`has_th`), and whether any dynamic child was seen anywhere in
//! the subtree (`has_dynamic`). The table is reported only when
//! `has_td && !has_th && !has_dynamic`.
//!
//! ### Nested tables
//!
//! The recursive scan descends through *all* descendant elements, including any
//! nested `<table>`. This is deliberate and safe: an inner table is also
//! visited on its own (the rule fires per `<table>` element), and counting an
//! inner table's `<th>` toward the outer table can only *suppress* an outer
//! report — it never creates a false positive. The behaviour is therefore
//! conservative by construction.
//!
//! ## Flagged
//!
//! ```jsx
//! // Static table with data cells but no header cell.
//! <table><tr><td>a</td></tr></table>
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! // A header cell is present.
//! <table><tr><th>a</th></tr><tr><td>b</td></tr></table>
//!
//! // Dynamic content — rows (and any <th>) may be produced at runtime.
//! <table>{rows.map(r => <tr><td>{r}</td></tr>)}</table>
//!
//! // No data cell at all.
//! <table></table>
//!
//! // Not a <table> element.
//! <div><td>x</td></div>
//! ```

use oxc_ast::ast::{JSXChild, JSXElement, JSXElementName};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "table-header";

/// Facts collected while statically scanning a `<table>` subtree.
#[derive(Default)]
struct TableScan {
    /// At least one descendant element is named `td`.
    has_td: bool,
    /// At least one descendant element is named `th`.
    has_th: bool,
    /// At least one dynamic child (expression container or spread) was seen
    /// anywhere in the subtree, so the table cannot be analysed statically.
    has_dynamic: bool,
}

/// Returns the lowercase identifier name of a JSX element, or `None` when the
/// element name is not a plain identifier (e.g. a member or namespaced name).
fn element_identifier_name<'a>(element: &'a JSXElement<'a>) -> Option<&'a str> {
    match &element.opening_element.name {
        JSXElementName::Identifier(ident) => Some(ident.name.as_str()),
        _ => None,
    }
}

/// Recursively walks `children`, accumulating `td`/`th`/dynamic facts into
/// `scan`. Descends into every child element (including nested tables); any
/// expression container or spread child marks the whole subtree as dynamic.
fn scan_children<'a>(children: &'a [JSXChild<'a>], scan: &mut TableScan) {
    for child in children {
        match child {
            JSXChild::Element(element) => {
                if let Some(name) = element_identifier_name(element) {
                    if name == "td" {
                        scan.has_td = true;
                    } else if name == "th" {
                        scan.has_th = true;
                    }
                }
                scan_children(&element.children, scan);
            }
            // Any dynamic child means rows/cells may be produced at runtime, so
            // the table cannot be analysed statically. Bail conservatively.
            JSXChild::ExpressionContainer(_) | JSXChild::Spread(_) | JSXChild::Fragment(_) => {
                scan.has_dynamic = true;
            }
            JSXChild::Text(_) => {}
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_table_header(&mut self, it: &JSXElement<'_>) {
        // Only consider the lowercase `table` HTML element.
        let Some(name) = element_identifier_name(it) else {
            return;
        };
        if name != "table" {
            return;
        }

        let mut scan = TableScan::default();
        scan_children(&it.children, &mut scan);

        // Report only a fully-static table that has data cells but no header
        // cell. Any dynamic content suppresses the report (zero false positive).
        if scan.has_td && !scan.has_th && !scan.has_dynamic {
            self.report(RULE_NAME, "tableHeader", it.opening_element.span);
        }
    }
}
