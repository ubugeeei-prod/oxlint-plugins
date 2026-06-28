//! Rule `disabled-resource-integrity` (SonarJS key S5725).
//!
//! Clean-room port. Loading executable code or stylesheets from a third-party
//! origin (typically a CDN) without a Subresource Integrity (SRI) check lets a
//! compromised or malicious provider serve altered content that the browser
//! will run with the full privileges of the embedding page. Adding an
//! `integrity` attribute (a cryptographic hash of the expected bytes) makes the
//! browser refuse to use a resource whose content does not match.
//!
//! Behaviour derived from the public RSPEC S5725 description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Zero-FP subset
//!
//! Only JSX is analysed (the syntactic surface available to this port). An
//! element is flagged when ALL of the following hold:
//!
//! * the tag is a lowercase `script`, or a lowercase `link` whose static
//!   `rel` attribute token list contains `stylesheet` (the only `link` relation
//!   for which SRI is meaningful in practice);
//! * its resource URL attribute (`src` for `script`, `href` for `link`) is a
//!   *string literal* whose value is cross-origin — it starts with `https://`,
//!   `http://`, or the protocol-relative `//`;
//! * it has NO `integrity` attribute.
//!
//! Any spread attribute, a dynamic/expression URL, or a missing URL bails out,
//! so same-origin resources (relative URLs) and statically-unknowable elements
//! are never reported. An `integrity` attribute with *any* value (even dynamic)
//! suppresses the report. This keeps the check effectively false-positive free
//! while still catching the canonical "CDN script with no SRI hash" mistake.
//!
//! ## Flagged
//!
//! ```jsx
//! <script src="https://cdn.example.com/lib.js"></script>
//! <script src="//cdn.example.com/lib.js"></script>
//! <link rel="stylesheet" href="https://cdn.example.com/style.css" />
//! ```
//!
//! ## Not flagged
//!
//! ```jsx
//! <script src="https://cdn.example.com/lib.js" integrity="sha384-abc" crossorigin="anonymous"></script>
//! <script src="/local/lib.js"></script>          // same-origin
//! <script src={dynamicUrl}></script>             // dynamic URL
//! <script {...props}></script>                   // spread props
//! <link rel="icon" href="https://cdn.example.com/favicon.ico" /> // SRI not applicable
//! ```

use oxc_ast::ast::{
    JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElementName, JSXOpeningElement,
};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "disabled-resource-integrity";

impl Scanner<'_> {
    pub(crate) fn check_disabled_resource_integrity(&mut self, it: &JSXOpeningElement<'_>) {
        let tag_name = match &it.name {
            JSXElementName::Identifier(ident) => ident.name.as_str(),
            _ => return,
        };
        let is_script = tag_name == "script";
        let is_link = tag_name == "link";
        if !is_script && !is_link {
            return;
        }

        // A spread attribute hides part of the prop set — cannot verify; bail.
        for attr_item in &it.attributes {
            if matches!(attr_item, JSXAttributeItem::SpreadAttribute(_)) {
                return;
            }
        }

        let url_attr = if is_script { "src" } else { "href" };

        let mut url_value: Option<&str> = None;
        let mut url_is_dynamic = false;
        let mut has_integrity = false;
        let mut rel_is_stylesheet = false;

        for attr_item in &it.attributes {
            let attr = match attr_item {
                JSXAttributeItem::Attribute(a) => a,
                JSXAttributeItem::SpreadAttribute(_) => return,
            };
            let name = match &attr.name {
                JSXAttributeName::Identifier(ident) => ident.name.as_str(),
                _ => continue,
            };
            if name == url_attr {
                match &attr.value {
                    Some(JSXAttributeValue::StringLiteral(lit)) => {
                        url_value = Some(lit.value.as_str());
                    }
                    Some(_) => url_is_dynamic = true,
                    None => {}
                }
            } else if name == "integrity" {
                // Any integrity attribute (even dynamic/empty) means the author
                // opted into SRI; do not report.
                has_integrity = true;
            } else if name == "rel"
                && let Some(JSXAttributeValue::StringLiteral(lit)) = &attr.value
                && lit
                    .value
                    .as_str()
                    .split_whitespace()
                    .any(|t| t.eq_ignore_ascii_case("stylesheet"))
            {
                rel_is_stylesheet = true;
            }
        }

        if has_integrity || url_is_dynamic {
            return;
        }
        // For `link`, only stylesheet relations meaningfully need SRI.
        if is_link && !rel_is_stylesheet {
            return;
        }
        let Some(url) = url_value else {
            return;
        };
        let is_cross_origin =
            url.starts_with("https://") || url.starts_with("http://") || url.starts_with("//");
        if !is_cross_origin {
            return;
        }
        self.report(RULE_NAME, "disabledResourceIntegrity", it.span);
    }
}
