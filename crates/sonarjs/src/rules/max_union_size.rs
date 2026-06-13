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
//! The threshold is fixed at **3** (`MAX_UNION_SIZE`). SonarJS exposes a
//! configurable `threshold` option, but this port has no per-rule options
//! infrastructure yet. Configurability is a follow-up task; for now the
//! default of 3 is hardcoded.
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

/// Maximum number of members allowed in a single union type.
/// A union with more than this many members is flagged.
const MAX_UNION_SIZE: usize = 3;

impl Scanner<'_> {
    pub(crate) fn check_max_union_size(&mut self, union: &TSUnionType<'_>) {
        if union.types.len() <= MAX_UNION_SIZE {
            return;
        }
        self.report(RULE_NAME, "maxUnionSize", union.span);
    }
}
