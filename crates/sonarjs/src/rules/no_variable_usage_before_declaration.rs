//! Rule `no-variable-usage-before-declaration` (SonarJS key S1526).
//!
//! Clean-room port. A variable should be declared before it is first used.
//! Relying on JavaScript `var` hoisting to reference a variable before its
//! textual declaration is confusing and error-prone.
//!
//! ## Conservative scope (zero-false-positive design)
//!
//! Only `var`/`let`/`const` variable declarators are flagged. Function
//! declarations, function parameters, class declarations, import bindings, and
//! catch bindings are deliberately excluded — hoisting of function declarations
//! is a widely-used, intentional idiom.
//!
//! References inside a nested function or arrow expression are excluded even
//! when the nested function is textually before the declaration. Such closures
//! may be invoked after the declaration has been evaluated, so they do not
//! actually observe the variable in an uninitialized state.
//!
//! The rule is conservative: it compares byte offsets of the reference span
//! start against the variable declarator span start. If the reference appears
//! anywhere after the declarator start it is not flagged.
//!
//! The diagnostic is reported at the identifier reference (use) span.
//!
//! Behaviour is reproduced from the public RSPEC S1526 documentation and
//! observed behaviour only; no upstream source, tests, fixtures, or message
//! strings were consulted or copied.
//!
//! ## Flagged
//! - `console.log(x); var x = 5;` — `x` referenced at module level before
//!   its `var` declaration at the same level
//! - `function f() { console.log(y); var y = 1; }` — `y` referenced inside
//!   `f` before its `var` declaration also inside `f`
//!
//! ## Not flagged
//! - `foo(); function foo() {}` — `foo` is a function declaration; hoisting
//!   of function declarations is intentional and excluded
//! - `function outer() { function cb() { console.log(z); } var z = 3; cb(); }` —
//!   `z` reference is inside `cb`, a nested function; even though `cb` is
//!   defined before `var z`, it is called after, so this is a safe closure
//! - `const x = 5; console.log(x);` — reference is after the declaration

use oxc_ast::AstKind;
use oxc_ast::ast::IdentifierReference;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-variable-usage-before-declaration";

impl<'a> Scanner<'a> {
    /// Flags an identifier reference whose byte offset is before the byte
    /// offset of the variable declarator that introduces the symbol. See
    /// module doc for the zero-FP scope.
    pub(crate) fn check_no_variable_usage_before_declaration(
        &mut self,
        ident: &IdentifierReference<'a>,
    ) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let scoping = match self.scoping {
            Some(s) => s,
            None => return,
        };
        let nodes = match self.nodes {
            Some(n) => n,
            None => return,
        };
        let reference_id = match ident.reference_id.get() {
            Some(id) => id,
            None => return,
        };
        let symbol_id = match scoping.get_reference(reference_id).symbol_id() {
            Some(id) => id,
            None => return,
        };
        // Only flag variable declarators — excludes function declarations,
        // parameters, class declarations, imports, and catch bindings.
        let decl_node_id = scoping.symbol_declaration(symbol_id);
        let declarator = match nodes.get_node(decl_node_id).kind() {
            AstKind::VariableDeclarator(d) => d,
            _ => return,
        };
        let decl_start = declarator.span.start;
        // Not a use-before-declaration if the reference is at or after the
        // declarator.
        if ident.span.start >= decl_start {
            return;
        }
        // Determine the nesting depth of the innermost function that contains
        // the declaration. If the reference is nested deeper (i.e. inside a
        // nested function/arrow that is defined before the declaration), it is
        // a safe closure — do not flag.
        let ref_fn_depth = self.fn_span_stack.len();
        let mut decl_fn_depth = 0usize;
        for (i, span) in self.fn_span_stack.iter().enumerate().rev() {
            if span.start <= decl_start && decl_start < span.end {
                decl_fn_depth = i + 1;
                break;
            }
        }
        if ref_fn_depth != decl_fn_depth {
            return;
        }
        self.report(RULE_NAME, "usedBeforeDeclaration", ident.span);
    }
}
