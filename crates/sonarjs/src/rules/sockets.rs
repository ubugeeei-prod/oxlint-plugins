//! Rule `sockets` (SonarJS key S4818).
//!
//! Clean-room port. Opening a raw network socket gives code low-level control
//! over a communication channel and is therefore a *security hotspot*: it is
//! not a bug in itself, but every such usage should be reviewed to confirm the
//! endpoint, protocol, and data exchanged are safe. The public RSPEC (S4818)
//! demonstrates the Node.js `net` module API as the sensitive shapes:
//!
//! ```js
//! const net = require('net');
//! var socket = new net.Socket();                  // Sensitive
//! net.createConnection({ port: port }, () => {});  // Sensitive
//! net.connect({ port: port }, () => {});           // Sensitive
//! ```
//!
//! **Narrowing (zero-false-positive subset)**:
//! The method names `connect` and `createConnection` are extremely generic —
//! they collide with database clients (`db.connect`), state libraries (Redux
//! `connect`), and countless other APIs. Likewise a bare `new Socket()` says
//! nothing about which library is in use. To stay false-positive free this port
//! is *receiver-gated on the `net` module*: it only flags member expressions
//! whose object is the identifier `net`. Concretely it reports
//!
//!   * a `NewExpression` whose callee (after `get_inner_expression`) is the
//!     static member `net.Socket` → `new net.Socket()`, and
//!   * a `CallExpression` whose callee (after `get_inner_expression`) is the
//!     static member `net.createConnection` or `net.connect` →
//!     `net.createConnection(...)` / `net.connect(...)`.
//!
//! Anything on a non-`net` receiver (`db.connect(...)`, `store.connect(...)`)
//! and a bare `new Socket()` without the `net.` qualifier are intentionally
//! left alone, since those names are not specific enough to be raw-socket use.
//!
//! This is a security hotspot (review-only), reproduced from the public RSPEC
//! description (S4818) only. No upstream source, tests, fixtures, helper code,
//! or message strings were consulted or copied.
//!
//! ## Flagged
//! ```js
//! new net.Socket()
//! net.createConnection({ port: port }, () => {})
//! net.connect({ port: port }, () => {})
//! ```
//!
//! ## Not flagged
//! ```js
//! db.connect()                 // non-`net` receiver
//! store.connect(mapState)      // non-`net` receiver
//! new Socket()                 // no `net.` receiver
//! net.isIP(addr)               // not a socket-opening member
//! ```

use oxc_ast::ast::{CallExpression, Expression, NewExpression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "sockets";

/// Returns `true` when `object` is exactly the identifier `net`.
fn is_net_receiver(object: &Expression<'_>) -> bool {
    matches!(object.get_inner_expression(), Expression::Identifier(ident) if ident.name == "net")
}

impl Scanner<'_> {
    /// Reports `new net.Socket()` — instantiating a raw socket on the Node
    /// `net` module (security hotspot S4818).
    pub(crate) fn check_sockets_new(&mut self, it: &NewExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        if member.property.name == "Socket" && is_net_receiver(&member.object) {
            self.report(RULE_NAME, "sockets", it.span);
        }
    }

    /// Reports `net.createConnection(...)` / `net.connect(...)` — opening a raw
    /// socket connection on the Node `net` module (security hotspot S4818).
    pub(crate) fn check_sockets_call(&mut self, it: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = it.callee.get_inner_expression() else {
            return;
        };
        let name = member.property.name.as_str();
        if (name == "createConnection" || name == "connect") && is_net_receiver(&member.object) {
            self.report(RULE_NAME, "sockets", it.span);
        }
    }
}
