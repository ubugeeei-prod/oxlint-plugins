//! Rule `no-in-misuse` (SonarJS key S4619).
//!
//! The `in` operator tests whether a property KEY exists on an object — it
//! does NOT test whether a VALUE is present in a collection. When the
//! right-hand operand is a provable array and the left-hand operand is a
//! string literal that looks like a data value rather than a property name,
//! the usage is almost certainly a bug: the developer likely intended
//! `.includes()` or `.indexOf()` to check for value membership.
//!
//! ## Conservative zero-false-positive design
//!
//! Only flagged when BOTH conditions hold simultaneously:
//!
//! 1. **Right operand is a provable array** — either a direct array literal
//!    (`[1, 2, 3]`) or an identifier that resolves via semantic analysis to a
//!    never-reassigned binding whose initializer is an array literal. When
//!    the array-ness cannot be proven the expression is conservatively not
//!    flagged.
//!
//! 2. **Left operand is a non-exempt string literal** — specifically a
//!    `StringLiteral` whose value:
//!    - is NOT all ASCII digits (a numeric index string like `"0"` or `"12"`
//!      is a legitimate array-index key probe, not a value-membership check), AND
//!    - is NOT a standard `Array.prototype` member name (legitimate feature
//!      probes like `"length" in arr` or `"push" in arr` are exempt).
//!
//! Everything else is conservatively not flagged to avoid false positives.
//! Reported at the span of the entire `BinaryExpression`.
//!
//! ## Flagged
//! - `"apple" in ["apple", "banana"]`
//! - `const fruits = ["apple", "banana"]; "apple" in fruits;`
//!
//! ## Not flagged
//! - `"0" in [1, 2, 3]` — numeric index string, legitimate key check
//! - `"length" in arr` — Array.prototype member name, legitimate probe
//! - `x in arr` — left operand is not a string literal
//! - `"apple" in obj` — right operand is not a provable array
//!
//! Behaviour is reproduced from the public SonarSource RSPEC S4619 only;
//! no upstream source, tests, fixtures, or message strings were consulted.

use oxc_ast::ast::{BinaryExpression, Expression, IdentifierReference};
use oxc_syntax::operator::BinaryOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-in-misuse";

/// Standard `Array.prototype` member names that are legitimate left-hand
/// operands of `in` (feature/property probes rather than value-membership
/// checks). Using `"length" in arr` to test whether `arr` is array-like, for
/// example, is intentional and must not be flagged.
const ARRAY_PROTOTYPE_MEMBERS: &[&str] = &[
    "length",
    "constructor",
    "at",
    "concat",
    "copyWithin",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "lastIndexOf",
    "map",
    "pop",
    "push",
    "reduce",
    "reduceRight",
    "reverse",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "toLocaleString",
    "toString",
    "unshift",
    "values",
];

/// Returns `true` when `s` is a canonical numeric array-index string — a
/// non-empty sequence of ASCII digit characters only. Such values are genuine
/// array-index key probes (`"0" in arr` is asking "does index 0 exist?") and
/// must not be reported.
fn is_numeric_index_string(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Returns `true` when `name` is a known `Array.prototype` member. Using such
/// a name as the left-hand operand of `in` is a legitimate property probe
/// rather than a value-membership check.
fn is_array_prototype_member(name: &str) -> bool {
    ARRAY_PROTOTYPE_MEMBERS.contains(&name)
}

impl<'a> Scanner<'a> {
    /// Reports a `BinaryExpression` of the form `stringLiteral in array` where
    /// the string is not a numeric index and not an `Array.prototype` member,
    /// and the array is a provably array-typed expression.
    pub(crate) fn check_no_in_misuse(&mut self, it: &BinaryExpression<'a>) {
        if it.operator != BinaryOperator::In {
            return;
        }
        let left = it.left.get_inner_expression();
        let right = it.right.get_inner_expression();
        let lit = match left {
            Expression::StringLiteral(lit) => lit,
            _ => return,
        };
        let value = lit.value.as_str();
        if is_numeric_index_string(value) {
            return;
        }
        if is_array_prototype_member(value) {
            return;
        }
        if !self.in_misuse_is_provable_array(right) {
            return;
        }
        self.report(RULE_NAME, "inMisuse", it.span);
    }

    /// Conservatively decides whether `expr` is a provable array: a direct
    /// array literal, or an identifier that resolves to a never-reassigned
    /// array-literal binding.
    fn in_misuse_is_provable_array(&self, expr: &Expression<'a>) -> bool {
        match expr {
            Expression::ArrayExpression(_) => true,
            Expression::Identifier(ident) => self.in_misuse_ident_resolves_to_array(ident),
            _ => false,
        }
    }

    /// Resolves an identifier and checks whether its initializer is an array
    /// literal. Returns `false` when semantic data is absent, the symbol is
    /// mutated, or the initializer is not an array literal.
    fn in_misuse_ident_resolves_to_array(&self, ident: &IdentifierReference<'a>) -> bool {
        match self.resolve_identifier_initializer(ident) {
            Some(init) => matches!(init.get_inner_expression(), Expression::ArrayExpression(_)),
            None => false,
        }
    }
}
