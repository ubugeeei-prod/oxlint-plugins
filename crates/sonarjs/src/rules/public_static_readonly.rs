//! Rule `public-static-readonly` (SonarJS key S1444).
//!
//! Clean-room port. A `public static` class field that is NOT `readonly` is
//! shared mutable state and is error-prone. This rule flags any class field
//! declaration that is:
//!   - `static`, and
//!   - publicly accessible (explicit `public`, or no accessibility modifier,
//!     which is public by default), and
//!   - NOT `readonly`.
//!
//! Fields that are `private`, `protected`, `readonly`, non-static, declared
//! with a `#private` key, or declared `declare` (ambient, no runtime storage)
//! are NOT flagged. The flag is purely modifier-based and
//! does not depend on whether the field has an initializer. The rule applies to
//! both JavaScript and TypeScript source (in plain JS the `readonly` modifier is
//! unavailable, so a public static JS field is always reported).
//!
//! Behaviour reproduced from the public RSPEC S1444 description and the
//! eslint-plugin-sonarjs docs only; no upstream source, tests, fixtures, or
//! message strings were consulted or copied.

use oxc_ast::ast::{PropertyDefinition, PropertyKey, TSAccessibility};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "public-static-readonly";

impl Scanner<'_> {
    /// Reports a `public static` class field that is not `readonly`.
    pub(crate) fn check_public_static_readonly(&mut self, prop: &PropertyDefinition<'_>) {
        if !prop.r#static {
            return;
        }
        if prop.readonly {
            return;
        }
        // A `declare` field is an ambient (type-only) declaration with no runtime
        // storage, so the shared-mutable-state concern does not apply.
        if prop.declare {
            return;
        }
        match prop.accessibility {
            None | Some(TSAccessibility::Public) => {}
            Some(TSAccessibility::Private | TSAccessibility::Protected) => return,
        }
        if matches!(prop.key, PropertyKey::PrivateIdentifier(_)) {
            return;
        }
        self.report(RULE_NAME, "publicStaticReadonly", prop.key.span());
    }
}
