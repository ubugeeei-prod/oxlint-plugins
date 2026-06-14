//! Rule `shorthand-property-grouping` (SonarJS key S3499).
//!
//! Clean-room port. ES6 object literals allow a shorthand form for properties
//! whose key matches an in-scope variable name (`{ a, b }` instead of
//! `{ a: a, b: b }`). When such shorthand properties are mixed with regular
//! `key: value` properties, the literal is easier to read if every shorthand
//! property is kept together as a single contiguous block at the *start* or the
//! *end* of the literal, rather than scattered between regular entries.
//!
//! **Flagged** — an object literal whose shorthand properties do not form one
//! contiguous block anchored to the first or the last property:
//! - `{ a, x: 1, b }` — the shorthand `a` and `b` are split by `x: 1`.
//! - `{ x: 1, a, y: 2 }` — the lone shorthand `a` sits in the middle.
//! - `{ a, ...rest, b }` — a spread element breaks the shorthand block apart.
//!
//! **Not flagged**:
//! - `{ a, b, x: 1 }` — shorthand grouped at the beginning.
//! - `{ x: 1, a, b }` — shorthand grouped at the end.
//! - `{ a, b }` — every property is shorthand.
//! - `{ x: 1, y: 2 }` — no shorthand property at all.
//! - `{ a }` — a single property can never be misgrouped.
//!
//! A spread element (`...x`) is treated as a non-shorthand entry, so a spread
//! sitting between two shorthand properties splits the block and is flagged.
//! The report is anchored to the whole object expression.
//!
//! Behaviour is reproduced from the public RSPEC description (S3499) only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::{ObjectExpression, ObjectPropertyKind};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "shorthand-property-grouping";

impl Scanner<'_> {
    pub(crate) fn check_shorthand_property_grouping(&mut self, obj: &ObjectExpression<'_>) {
        let properties = &obj.properties;
        if properties.len() < 2 {
            return;
        }

        let mut first_shorthand: Option<usize> = None;
        let mut last_shorthand: Option<usize> = None;
        let mut shorthand_count: usize = 0;
        for (index, property) in properties.iter().enumerate() {
            let ObjectPropertyKind::ObjectProperty(p) = property else {
                continue;
            };
            if !p.shorthand {
                continue;
            }
            if first_shorthand.is_none() {
                first_shorthand = Some(index);
            }
            last_shorthand = Some(index);
            shorthand_count += 1;
        }

        let Some(first) = first_shorthand else {
            return;
        };
        let Some(last) = last_shorthand else {
            return;
        };

        let contiguous = last - first + 1 == shorthand_count;
        let anchored = first == 0 || last == properties.len() - 1;
        if contiguous && anchored {
            return;
        }
        self.report(RULE_NAME, "groupShorthand", obj.span);
    }
}
