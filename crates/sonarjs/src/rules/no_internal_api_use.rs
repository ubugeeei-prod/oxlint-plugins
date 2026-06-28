//! Rule `no-internal-api-use` (SonarJS key S6627).
//!
//! Clean-room port. Frameworks such as React intentionally hide their internal
//! plumbing behind deliberately frightening property names so that application
//! code cannot reach for it by accident. Reaching past the public surface and
//! reading those internals couples your code to undocumented implementation
//! details that change without notice between releases, so the rule reports any
//! such access.
//!
//! ## Narrow form
//!
//! Every React internal entry point is exposed through a property whose name
//! contains the screaming-snake-case token `DO_NOT_USE`, for example:
//!
//! ```text
//! __SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED
//! __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE
//! __SERVER_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE
//! ```
//!
//! This port reports a static member access (`obj.PROP`) whose property name
//! contains the substring `DO_NOT_USE`:
//!
//! ```js
//! React.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED; // Noncompliant
//! ReactDOM.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED.Events; // Noncompliant
//! React.useState(0); // Compliant
//! ```
//!
//! Matching on the `DO_NOT_USE` token guarantees no false positives regardless
//! of which framework module the access originates from: no legitimate public
//! API names a property that way, and the check needs neither type information
//! nor import resolution. Internal APIs exposed under different conventions (for
//! example a bare `_internalRoot` field, or member names lacking the token) are
//! intentionally out of scope for this port and are a documented follow-up;
//! under-reporting is preferred over guessing.
//!
//! Behaviour is reproduced from the public RSPEC description (S6627) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::StaticMemberExpression;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-internal-api-use";

impl Scanner<'_> {
    pub(crate) fn check_no_internal_api_use(&mut self, member: &StaticMemberExpression<'_>) {
        if member.property.name.as_str().contains("DO_NOT_USE") {
            self.report(RULE_NAME, "noInternalApiUse", member.property.span);
        }
    }
}
