//! Rule `table-header-reference` (SonarJS key S5260).
//!
//! Clean-room port. Flags a JSX `<table>` data cell whose `headers` attribute
//! references a header `id` that does not exist anywhere in that table. The
//! `headers` attribute on a `<td>`/`<th>` is a space-separated list of the
//! `id`s of the header cells that describe the cell; assistive technologies
//! follow those references to announce the relevant headers. A `headers` token
//! that points at a non-existent `id` is a dangling reference that breaks this
//! association for screen-reader users.
//!
//! Behaviour derived from public RSPEC S5260 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied. The
//! authoritative public RSPEC Noncompliant example is a cell such as
//! `<td headers="berlin day1 big">` placed in a table that has no header whose
//! `id` is `berlin` — "there is no header with id berlin".
//!
//! ## Zero-false-positive narrowing (conservative static-only subset)
//!
//! This port flags only the unambiguous "referenced id is absent from the same
//! static table" case. A single recursive pass over the `<table>` subtree
//! collects three facts:
//!
//! - `has_dynamic`: set if any descendant child is a JSX expression container
//!   (`{…}`), spread child, or fragment, OR if any element carries a spread
//!   attribute (`{...props}`), OR if any `id`/`headers` attribute value is a
//!   non-string (a JSX expression rather than a string literal). Any of these
//!   means an `id` could be produced at runtime that a static scan cannot see,
//!   so reporting could be a false positive — the entire table is skipped.
//! - `ids`: every static string-literal `id` attribute value found on any
//!   element in the subtree (including the `<table>` itself).
//! - `headers`: the value and span of every static string-literal `headers`
//!   attribute found on any element in the subtree.
//!
//! After the pass (and only when `!has_dynamic`), each `headers` value is split
//! on whitespace; if any referenced token is absent from the collected `id`
//! set, that one `headers` attribute is reported once.
//!
//! The row/column-correctness check from the RSPEC (verifying that the
//! referenced header actually governs the cell's row or column) requires table
//! layout analysis and is deliberately out of scope: this is a conservative
//! under-report, never a false positive.
//!
//! ## Flagged
//!
//! ```jsx
//! // `headers="b"` references an id `b` that no element in the table declares.
//! <table><tr><th id="a">A</th></tr><tr><td headers="b">x</td></tr></table>
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! // Every referenced id exists in the table.
//! <table><tr><th id="a">A</th></tr><tr><td headers="a">x</td></tr></table>
//!
//! // Dynamic content — ids may be produced at runtime.
//! <table>{rows.map(r => <tr><td headers="a">{r}</td></tr>)}</table>
//!
//! // Non-literal headers value — cannot statically verify.
//! <table><tr><td headers={dyn}>x</td></tr></table>
//!
//! // Not inside a <table>, or no headers attribute at all.
//! <td headers="x">y</td>
//! ```

use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElement, JSXElementName,
};
use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "table-header-reference";

/// Facts collected while statically scanning a `<table>` subtree.
#[derive(Default)]
struct TableReferenceScan<'a> {
    /// Set when any dynamic construct is seen anywhere in the subtree (an
    /// expression-container/spread/fragment child, a spread attribute, or a
    /// non-string `id`/`headers` value). The table cannot be analysed
    /// statically, so it is skipped entirely.
    has_dynamic: bool,
    /// Every static string-literal `id` value found on any element.
    ids: SmallVec<[&'a str; 8]>,
    /// `(value, headers-attribute-span)` for every static string-literal
    /// `headers` attribute found on any element.
    headers: SmallVec<[(&'a str, Span); 8]>,
}

/// Returns the lowercase identifier name of a JSX element, or `None` when the
/// element name is not a plain identifier (e.g. a member or namespaced name).
fn element_identifier_name<'a>(element: &'a JSXElement<'a>) -> Option<&'a str> {
    match &element.opening_element.name {
        JSXElementName::Identifier(ident) => Some(ident.name.as_str()),
        _ => None,
    }
}

/// Collects this element's `id`/`headers` facts, then recurses into its
/// children.
fn scan_element<'a>(element: &'a JSXElement<'a>, scan: &mut TableReferenceScan<'a>) {
    for attr_item in &element.opening_element.attributes {
        let attr = match attr_item {
            JSXAttributeItem::Attribute(a) => a,
            // A spread attribute may inject an `id` (or `headers`) we cannot
            // see; bail conservatively.
            JSXAttributeItem::SpreadAttribute(_) => {
                scan.has_dynamic = true;
                continue;
            }
        };
        let name = match &attr.name {
            JSXAttributeName::Identifier(ident) => ident.name.as_str(),
            JSXAttributeName::NamespacedName(_) => continue,
        };
        if name == "id" {
            match &attr.value {
                Some(JSXAttributeValue::StringLiteral(lit)) => scan.ids.push(lit.value.as_str()),
                // Non-string (dynamic) id — cannot statically resolve.
                Some(_) => scan.has_dynamic = true,
                None => {}
            }
        } else if name == "headers" {
            match &attr.value {
                Some(JSXAttributeValue::StringLiteral(lit)) => {
                    scan.headers.push((lit.value.as_str(), attr.span));
                }
                // Non-string (dynamic) headers value — cannot statically verify.
                Some(_) => scan.has_dynamic = true,
                None => {}
            }
        }
    }
    scan_children(&element.children, scan);
}

/// Recursively walks `children`, accumulating facts into `scan`. Any
/// expression container, spread, or fragment child marks the whole subtree as
/// dynamic.
fn scan_children<'a>(children: &'a [JSXChild<'a>], scan: &mut TableReferenceScan<'a>) {
    for child in children {
        match child {
            JSXChild::Element(element) => scan_element(element, scan),
            JSXChild::ExpressionContainer(_) | JSXChild::Spread(_) | JSXChild::Fragment(_) => {
                scan.has_dynamic = true;
            }
            JSXChild::Text(_) => {}
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn check_table_header_reference<'a>(&mut self, it: &'a JSXElement<'a>) {
        // Only consider the lowercase `table` HTML element.
        let Some(name) = element_identifier_name(it) else {
            return;
        };
        if name != "table" {
            return;
        }

        let mut scan = TableReferenceScan::default();
        scan_element(it, &mut scan);

        // Any dynamic content suppresses the report (zero false positive).
        if scan.has_dynamic {
            return;
        }

        for (value, span) in &scan.headers {
            let has_missing_reference = value
                .split_whitespace()
                .any(|token| !scan.ids.contains(&token));
            if has_missing_reference {
                self.report(RULE_NAME, "tableHeaderReference", *span);
            }
        }
    }
}
