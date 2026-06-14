//! Rule `no-useless-catch` (SonarJS key S2737).
//!
//! Clean-room port. A `catch` clause that does nothing but rethrow the caught
//! exception is useless: removing it leaves error propagation unchanged. The
//! rule fires when ALL of the following hold:
//!
//! 1. The `catch` parameter is a simple binding identifier (`catch (e)`).
//!    Destructured parameters (`catch ({ message })`) are excluded because they
//!    cannot be a straight rethrow.
//! 2. The `catch` body contains exactly ONE statement, and that statement is a
//!    `ThrowStatement`.
//! 3. The thrown expression (after stripping parentheses via
//!    `get_inner_expression`) is an `Identifier` whose name equals the caught
//!    parameter name.
//!
//! ## Flagged
//!
//! ```js
//! try { f(); } catch (e) { throw e; }
//! try { f(); } catch (err) { throw err; } finally { g(); }
//! ```
//!
//! ## Not flagged
//!
//! ```js
//! try { f(); } catch (e) { log(e); throw e; }   // two statements
//! try { f(); } catch (e) { throw new Error(); }  // throws something else
//! try { f(); } catch (e) { throw e.cause; }      // throws a member, not e
//! try { f(); } catch ({ message }) { throw message; }  // destructured param
//! try { f(); } catch (e) { handle(e); }          // no rethrow at all
//! ```
//!
//! Behaviour is reproduced from the public RSPEC description only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use oxc_ast::ast::{BindingPattern, CatchClause, Expression, Statement};
use oxc_span::Span;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-useless-catch";

impl Scanner<'_> {
    pub(crate) fn check_no_useless_catch(&mut self, catch: &CatchClause<'_>) {
        let Some(param) = &catch.param else { return };
        let BindingPattern::BindingIdentifier(id) = &param.pattern else {
            return;
        };
        let caught = id.name.as_str();
        let [only] = catch.body.body.as_slice() else {
            return;
        };
        let Statement::ThrowStatement(throw) = only else {
            return;
        };
        let Expression::Identifier(thrown) = throw.argument.get_inner_expression() else {
            return;
        };
        if thrown.name.as_str() != caught {
            return;
        }
        let start = catch.span.start;
        self.report(RULE_NAME, "uselessCatch", Span::new(start, start + 5));
    }
}
