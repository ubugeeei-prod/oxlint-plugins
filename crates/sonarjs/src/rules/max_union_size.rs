//! Rule `max-union-size` (SonarJS key S4622).
//!
//! Clean-room port. Reports a TypeScript union type (`A | B | C | D`) whose
//! number of direct member types exceeds the threshold, because a union with
//! too many members is hard to read and often signals a missing abstraction
//! that should be expressed as a named type alias or interface.
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! ## Threshold
//!
//! The threshold mirrors SonarJS's configurable `threshold` option
//! (`self.options.max_union_size_threshold`); when no option is supplied the
//! SonarJS default of **3** is used.
//!
//! ## Counting
//!
//! Each `TSUnionType` node is checked independently. `union.types.len()`
//! counts the direct members of that union node. Nested parenthesized unions
//! are counted per-node (i.e. each `TSUnionType` node is evaluated on its own
//! direct children, without flattening through parentheses).

use oxc_ast::ast::TSUnionType;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "max-union-size";

impl Scanner<'_> {
    pub(crate) fn check_max_union_size(&mut self, union: &TSUnionType<'_>) {
        if union.types.len() <= self.options.max_union_size_threshold as usize {
            return;
        }
        self.report(RULE_NAME, "maxUnionSize", union.span);
    }
}
