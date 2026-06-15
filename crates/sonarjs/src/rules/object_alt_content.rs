//! Rule `object-alt-content` (SonarJS key S5264).
//!
//! `<object>` JSX elements must provide a text alternative for assistive
//! technologies. Screen readers and other AT rely on text content or an ARIA
//! label to describe the embedded object; an element with neither is
//! inaccessible.
//!
//! Behaviour derived from public RSPEC S5264 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Flagged
//!
//! ```jsx
//! // No children, no labeling attribute
//! <object data="video.swf" />
//! <object data="video.swf"></object>
//!
//! // Whitespace-only text child is not meaningful
//! <object data="video.swf">   </object>
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! // Has text child content
//! <object data="video.swf">Your browser does not support embedded objects.</object>
//!
//! // Has child element
//! <object data="video.swf"><img src="fallback.png" alt="Video" /></object>
//!
//! // Has child expression
//! <object data={src}>{fallback}</object>
//!
//! // Has aria-label
//! <object data="video.swf" aria-label="Embedded video" />
//!
//! // Has aria-labelledby
//! <object data="video.swf" aria-labelledby="label-id" />
//!
//! // Has title attribute
//! <object data="video.swf" title="Embedded video" />
//!
//! // Explicitly hidden from AT
//! <object data="video.swf" aria-hidden="true" />
//!
//! // Spread attribute — labeling may come from props, cannot verify statically
//! <object {...props} />
//! ```

use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElement, JSXElementName,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "object-alt-content";

impl Scanner<'_> {
    pub(crate) fn check_object_alt_content(&mut self, it: &JSXElement<'_>) {
        // Only consider lowercase `object` HTML elements.
        let tag_name = match &it.opening_element.name {
            JSXElementName::Identifier(ident) => ident.name.as_str(),
            _ => return,
        };
        if tag_name != "object" {
            return;
        }

        // Any spread attribute means we cannot statically determine the full set
        // of props — bail to avoid false positives.
        for attr_item in &it.opening_element.attributes {
            if matches!(attr_item, JSXAttributeItem::SpreadAttribute(_)) {
                return;
            }
        }

        // Check for labeling or hiding attributes.
        let mut has_label_attr = false;
        let mut is_aria_hidden = false;

        for attr_item in &it.opening_element.attributes {
            let attr = match attr_item {
                JSXAttributeItem::Attribute(a) => a,
                JSXAttributeItem::SpreadAttribute(_) => return,
            };
            let name = match &attr.name {
                JSXAttributeName::Identifier(ident) => ident.name.as_str(),
                _ => continue,
            };
            match name {
                "aria-label" | "aria-labelledby" | "title" => {
                    has_label_attr = true;
                }
                "aria-hidden" => match &attr.value {
                    Some(JSXAttributeValue::StringLiteral(lit)) if lit.value.as_str() == "true" => {
                        is_aria_hidden = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if has_label_attr || is_aria_hidden {
            return;
        }

        // Check whether any child provides meaningful accessible content.
        // A JSXText child that is only whitespace is not meaningful.
        let has_meaningful_child = it.children.iter().any(|child| match child {
            JSXChild::Text(text) => !text.value.as_str().trim().is_empty(),
            _ => true,
        });

        if has_meaningful_child {
            return;
        }

        self.report(RULE_NAME, "objectAltContent", it.opening_element.span);
    }
}
