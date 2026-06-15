//! Rule `link-with-target-blank` (SonarJS key S5148).
//!
//! Clean-room port. Flags `<a>` and `<area>` JSX elements that carry a literal
//! `target="_blank"` attribute but do not have a `rel` attribute containing
//! `"noopener"` or `"noreferrer"` in its space-separated token list.  Opening a
//! link in a new tab or window without these values leaves `window.opener` exposed
//! to the destination page, enabling reverse-tabnabbing attacks.
//!
//! Behaviour derived from public RSPEC S5148 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//!
//! ```jsx
//! // No rel at all
//! <a target="_blank">link</a>
//!
//! // rel present but missing noopener/noreferrer
//! <a target="_blank" rel="nofollow">link</a>
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! // rel contains noopener
//! <a target="_blank" rel="noopener">link</a>
//!
//! // rel contains noreferrer
//! <a target="_blank" rel="noreferrer">link</a>
//!
//! // target is not _blank
//! <a href="/x">link</a>
//! <a target="_self">link</a>
//!
//! // spread attribute — cannot statically verify props
//! <a {...props} target="_blank">link</a>
//!
//! // dynamic rel — cannot statically verify value
//! <a target="_blank" rel={dyn}>link</a>
//! ```

use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElementName, JSXOpeningElement,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "link-with-target-blank";

impl Scanner<'_> {
    pub(crate) fn check_link_with_target_blank(&mut self, it: &JSXOpeningElement<'_>) {
        // Only consider lowercase `a` or `area` HTML elements.
        let tag_name = match &it.name {
            JSXElementName::Identifier(ident) => ident.name.as_str(),
            _ => return,
        };
        if tag_name != "a" && tag_name != "area" {
            return;
        }

        // Any spread attribute means we cannot statically determine the full set
        // of props — bail to avoid false positives.
        for attr_item in &it.attributes {
            if matches!(attr_item, JSXAttributeItem::SpreadAttribute(_)) {
                return;
            }
        }

        let mut has_blank_target = false;
        let mut rel_is_safe = false;
        let mut rel_is_dynamic = false;

        for attr_item in &it.attributes {
            let attr = match attr_item {
                JSXAttributeItem::Attribute(a) => a,
                JSXAttributeItem::SpreadAttribute(_) => return,
            };
            let name = match &attr.name {
                JSXAttributeName::Identifier(ident) => ident.name.as_str(),
                _ => continue,
            };
            if name == "target" {
                match &attr.value {
                    Some(JSXAttributeValue::StringLiteral(lit)) => {
                        if lit.value.as_str() == "_blank" {
                            has_blank_target = true;
                        }
                    }
                    Some(_) => {
                        // Dynamic target value — cannot statically determine; skip element.
                        return;
                    }
                    None => {
                        // Bare `target` attribute without a value — not `_blank`.
                    }
                }
            } else if name == "rel" {
                match &attr.value {
                    Some(JSXAttributeValue::StringLiteral(lit)) => {
                        let val = lit.value.as_str();
                        if val
                            .split_whitespace()
                            .any(|t| t == "noopener" || t == "noreferrer")
                        {
                            rel_is_safe = true;
                        }
                    }
                    Some(JSXAttributeValue::ExpressionContainer(_)) => {
                        // Dynamic rel value — cannot verify statically; skip.
                        rel_is_dynamic = true;
                    }
                    _ => {}
                }
            }
        }

        if !has_blank_target {
            return;
        }
        if rel_is_dynamic {
            return;
        }
        if rel_is_safe {
            return;
        }
        self.report(RULE_NAME, "targetBlankNoOpener", it.span);
    }
}
