//! Rule `no-table-as-layout` (SonarJS key S5257).
//!
//! Clean-room port. Flags a JSX `<table>` element that carries an explicit
//! `role="presentation"` or `role="none"` attribute. Such a role marks the
//! table as a layout table — a table used purely for visual arrangement rather
//! than tabular data. Layout tables confuse assistive technology such as screen
//! readers, which announce table semantics (rows, columns, headers) that do not
//! reflect real data. CSS should be used for layout instead.
//!
//! Behaviour derived from public RSPEC S5257 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied. The
//! authoritative Noncompliant example from the public RSPEC is:
//!
//! ```html
//! <table role="presentation"><!-- Noncompliant -->
//! </table>
//! ```
//!
//! ## Zero-false-positive narrowing
//!
//! This port matches only the exact documented Noncompliant shape: an element
//! whose name is the lowercase identifier `table` that has a `role` attribute
//! whose value is a string literal equal to `presentation` or `none`. It does
//! not attempt to infer layout intent from any other signal. A `role` whose
//! value is a JSX expression container (dynamic), or any non-string value, is
//! not flagged because the value cannot be statically resolved to the
//! documented literal.
//!
//! ## Flagged
//!
//! ```jsx
//! <table role="presentation"></table>
//! <table role="none"></table>
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! // No role attribute — a normal data table.
//! <table></table>
//!
//! // A different role.
//! <table role="grid"></table>
//!
//! // A non-`table` element.
//! <div role="presentation"></div>
//!
//! // Dynamic / non-string role value — cannot statically verify.
//! <table role={layoutRole}></table>
//! ```

use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElementName, JSXOpeningElement,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-table-as-layout";

impl Scanner<'_> {
    pub(crate) fn check_no_table_as_layout(&mut self, it: &JSXOpeningElement<'_>) {
        // Only consider the lowercase `table` HTML element.
        let tag_name = match &it.name {
            JSXElementName::Identifier(ident) => ident.name.as_str(),
            _ => return,
        };
        if tag_name != "table" {
            return;
        }

        for attr_item in &it.attributes {
            let attr = match attr_item {
                JSXAttributeItem::Attribute(a) => a,
                JSXAttributeItem::SpreadAttribute(_) => continue,
            };
            let name = match &attr.name {
                JSXAttributeName::Identifier(ident) => ident.name.as_str(),
                JSXAttributeName::NamespacedName(_) => continue,
            };
            if name != "role" {
                continue;
            }
            // Only a static string-literal `role` of exactly `presentation`
            // or `none` matches the documented Noncompliant pattern.
            if let Some(JSXAttributeValue::StringLiteral(lit)) = &attr.value {
                let val = lit.value.as_str();
                if val == "presentation" || val == "none" {
                    self.report(RULE_NAME, "noTableAsLayout", it.span);
                    return;
                }
            }
        }
    }
}
