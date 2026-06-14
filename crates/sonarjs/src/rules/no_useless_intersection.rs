//! Rule `no-useless-intersection` (SonarJS key S6571).
//!
//! Clean-room port (syntactic subset). Flags a TypeScript intersection type
//! (`A & B & ...`) that contains a keyword member which makes the intersection
//! pointless:
//!   - `any`     — the intersection collapses to `any`   (e.g. `string & any`)
//!   - `never`   — the intersection collapses to `never` (e.g. `string & never`)
//!   - `unknown` — `unknown` is absorbed: `A & unknown` is just `A`
//!
//! The type-aware subtype/supertype cases handled by the upstream rule (e.g.
//! `string & 'literal'`, where one constituent is a subtype of another) require
//! a type checker and are out of scope for this syntactic port. Duplicate
//! members are already handled by `no-duplicate-in-composite`, so they are not
//! re-flagged here.
//!
//! Behaviour is reproduced from the public RSPEC S6571 description and the
//! typescript-eslint `no-redundant-type-constituents` docs it mirrors; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{TSIntersectionType, TSType};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-useless-intersection";

impl<'a> Scanner<'a> {
    pub(crate) fn check_no_useless_intersection(&mut self, it: &TSIntersectionType<'a>) {
        for member in &it.types {
            let is_useless = matches!(
                member,
                TSType::TSAnyKeyword(_) | TSType::TSNeverKeyword(_) | TSType::TSUnknownKeyword(_)
            );
            if is_useless {
                self.report(RULE_NAME, "uselessIntersection", member.span());
            }
        }
    }
}
