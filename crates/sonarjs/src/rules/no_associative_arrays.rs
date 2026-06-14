//! Rule `no-associative-arrays` (SonarJS key S3579).
//!
//! Clean-room port. JavaScript arrays are meant for sequential, numeric-indexed
//! data. Because an array is also an object, you *can* assign to a string- or
//! otherwise non-numeric property (`arr['name'] = 'bob'`), turning it into a
//! so-called "associative array". That is an object misuse: such named members
//! are not part of the array's indexed contents, are skipped by `length` and by
//! most iteration, and signal that a plain object (or a `Map`) was the right
//! container. The rule flags a WRITE to a non-numeric key of a value that is an
//! array.
//!
//! The upstream rule is type-aware (it knows the receiver's type is an array).
//! We have NO TypeScript type information, so this port CONSERVATIVELY flags only
//! when the receiver is *provably* an array: a direct array-literal receiver
//! (`[].foo = 1`) or an identifier that resolves via semantic analysis to a
//! never-reassigned binding whose initializer is an array literal
//! (`const a = []; a.foo = 1`). When the receiver's array-ness cannot be proven
//! (parameters, imports, reassigned variables, member access, call results) the
//! write is deliberately NOT flagged, so we under-report relative to upstream
//! rather than risk false positives. This under-reporting is a known parity risk.
//!
//! Only the assignment LHS is inspected (a WRITE), never a read. Both static
//! member writes (`arr.foo = 1`) and computed writes (`arr['foo'] = 1`) are
//! covered, for ANY assignment operator (`=`, `+=`, …) since each still writes a
//! named member. Exemptions:
//! - numeric index writes — `arr[0] = x`, `arr['0'] = x` (canonical index keys);
//! - the `length` property — `arr.length = 0` / `arr['length'] = 0` is legitimate
//!   truncation, not an associative write;
//! - computed keys that are NOT clearly numeric/string literals — `arr[i] = x`
//!   with a variable index is ambiguous (it may be numeric) and is conservatively
//!   skipped to avoid false positives.
//!
//! Reported at the offending member-expression span (`arr['key']`).
//!
//! Behaviour is reproduced from the public SonarSource rule documentation only;
//! no upstream source, tests, fixtures, or message strings were consulted or
//! copied.
//!
//! ## Flagged
//! - `const a = []; a['key'] = 1;` — computed non-numeric string-literal key
//! - `const a = []; a.foo = 1;` — static non-numeric key
//!
//! ## Not flagged
//! - `const a = []; a[0] = 1;` — numeric index
//! - `const a = []; a['0'] = 1;` — canonical numeric-index string key
//! - `const a = []; a.length = 0;` — `length` is exempt (truncation)
//! - `const a = []; a[i] = 1;` — variable index, ambiguous, conservatively skipped
//! - `const o = {}; o.foo = 1;` — receiver is not a provable array
//! - `function f(p) { p.foo = 1; }` — `p` is not a provable array

use oxc_ast::ast::{AssignmentExpression, AssignmentTarget, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-associative-arrays";

/// Returns `true` when `name` is an array property that may legitimately be
/// written without making the array associative. Only `length` qualifies
/// (truncation/extension); other built-ins are methods that are not assigned to.
fn is_exempt_property(name: &str) -> bool {
    name == "length"
}

/// Returns `true` when `s` is a canonical numeric array-index string — a
/// non-empty run of ASCII digits (`"0"`, `"12"`). Such keys are real array
/// indices, not associative members. Slightly over-exempts non-canonical forms
/// (`"00"`, `"007"`), which only causes under-reporting, never a false positive.
fn is_numeric_index_string(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

impl<'a> Scanner<'a> {
    /// Reports a write to a non-numeric, non-exempt key of a provable array.
    pub(crate) fn check_no_associative_arrays(&mut self, assign: &AssignmentExpression<'a>) {
        match &assign.left {
            AssignmentTarget::StaticMemberExpression(member) => {
                let name = member.property.name.as_str();
                if is_exempt_property(name) {
                    return;
                }
                if !self.assoc_receiver_is_array(member.object.get_inner_expression()) {
                    return;
                }
                self.report(RULE_NAME, "noAssociativeArray", member.span);
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                if !computed_key_is_associative(&member.expression) {
                    return;
                }
                if !self.assoc_receiver_is_array(member.object.get_inner_expression()) {
                    return;
                }
                self.report(RULE_NAME, "noAssociativeArray", member.span);
            }
            _ => (),
        }
    }

    /// Conservatively decides whether `receiver` is a provable array: a direct
    /// array literal, or an identifier resolving to an array-literal initializer.
    fn assoc_receiver_is_array(&self, receiver: &Expression<'a>) -> bool {
        match receiver {
            Expression::ArrayExpression(_) => true,
            Expression::Identifier(ident) => match self.resolve_identifier_initializer(ident) {
                Some(init) => matches!(init.get_inner_expression(), Expression::ArrayExpression(_)),
                None => false,
            },
            _ => false,
        }
    }
}

/// Decides whether a computed member key denotes an associative (non-numeric)
/// write. Only string literals are judged: a non-numeric, non-`length` string is
/// associative. Numeric literals and canonical numeric-index strings are real
/// indices. Any other key expression (variable, call, …) is ambiguous and
/// conservatively treated as NOT associative to avoid false positives.
fn computed_key_is_associative(key: &Expression<'_>) -> bool {
    match key.get_inner_expression() {
        Expression::StringLiteral(lit) => {
            let value = lit.value.as_str();
            !is_numeric_index_string(value) && !is_exempt_property(value)
        }
        _ => false,
    }
}
