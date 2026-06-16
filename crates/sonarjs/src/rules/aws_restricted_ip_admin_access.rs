//! Rule `aws-restricted-ip-admin-access` (SonarJS key S6321).
//!
//! Clean-room port. AWS CDK security groups that open an administration port to
//! every IP address let anyone on the internet reach sensitive remote-management
//! services. The two administration ports covered here are SSH (TCP `22`) and
//! RDP (TCP `3389`). The distinctive AWS CDK L2 form of this mistake is an
//! ingress rule whose source peer matches *all* addresses combined with an
//! administration port:
//!
//! ```js
//! sg.addIngressRule(ec2.Peer.anyIpv4(), ec2.Port.tcp(22));
//! sg.addIngressRule(Peer.anyIpv6(), Port.tcp(3389));
//! ```
//!
//! **Narrowing (zero-false-positive subset)**:
//! This port flags only the L2 `addIngressRule` form, where *both* distinctive
//! CDK arguments correlate:
//! - the callee is a static member expression named `addIngressRule`, with at
//!   least two arguments;
//! - argument 0 (the peer) is a call whose terminal callee name is `anyIpv4`
//!   or `anyIpv6` (`Peer.anyIpv4()` / `ec2.Peer.anyIpv4()`) — the "any IP"
//!   source; AND
//! - argument 1 (the port) is a call whose terminal callee name is `tcp`
//!   (`Port.tcp(...)`) whose first argument is the numeric literal `22` or
//!   `3389`.
//!
//! Requiring the any-IP peer *and* an administration `Port.tcp(22|3389)`
//! together is essentially unique to this insecure configuration, keeping the
//! check false-positive free. The reported span is the whole `addIngressRule`
//! call.
//!
//! **Not flagged**:
//! - a specific CIDR peer such as `Peer.ipv4("10.0.0.0/16")` (not any-IP);
//! - a non-administration port such as `Port.tcp(443)` or `Port.allTraffic()`;
//! - fewer than two arguments;
//! - a callee that is not `addIngressRule`.
//!
//! The public RSPEC (S6321) also shows lower-level `CfnSecurityGroup` forms
//! (`cidrIp: "0.0.0.0/0"` with `fromPort`/`toPort`) and the
//! `connections.allowFrom(...)` and `Port.tcpRange(...)` variants. Those are
//! intentionally left out of this subset: detecting them without false
//! positives needs more correlation than the L2 form, so this port deliberately
//! under-reports rather than risk noise.
//!
//! Behaviour is reproduced from the public RSPEC description (S6321) only. No
//! upstream source, tests, fixtures, helper code, or message strings were
//! consulted or copied.

use oxc_ast::ast::{Argument, CallExpression, Expression};

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-restricted-ip-admin-access";

/// The administration ports covered by this rule: SSH (22) and RDP (3389).
const ADMIN_PORTS: [f64; 2] = [22.0, 3389.0];

impl Scanner<'_> {
    /// Reports an `addIngressRule(Peer.anyIpv4()/anyIpv6(), Port.tcp(22|3389))`
    /// call, which exposes an administration port to all IP addresses.
    pub(crate) fn check_aws_restricted_ip_admin_access(&mut self, expr: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = expr.callee.get_inner_expression() else {
            return;
        };
        if member.property.name != "addIngressRule" {
            return;
        }
        if expr.arguments.len() < 2 {
            return;
        }
        if !is_any_ip_peer(&expr.arguments[0]) {
            return;
        }
        if !is_admin_tcp_port(&expr.arguments[1]) {
            return;
        }
        self.report(RULE_NAME, "restrictedIpAdminAccess", expr.span);
    }
}

/// True when `callee`'s terminal name (either a bare identifier or the property
/// of a static member expression) is one of `names`.
fn callee_name_is(callee: &Expression<'_>, names: &[&str]) -> bool {
    let name = match callee.get_inner_expression() {
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        Expression::Identifier(ident) => ident.name.as_str(),
        _ => return false,
    };
    names.contains(&name)
}

/// True when `arg` is a call to `anyIpv4()` / `anyIpv6()` (an any-IP peer).
fn is_any_ip_peer(arg: &Argument<'_>) -> bool {
    let Some(expr) = arg.as_expression() else {
        return false;
    };
    let Expression::CallExpression(call) = expr.get_inner_expression() else {
        return false;
    };
    callee_name_is(&call.callee, &["anyIpv4", "anyIpv6"])
}

/// True when `arg` is a call to `tcp(22)` / `tcp(3389)` (an admin TCP port).
fn is_admin_tcp_port(arg: &Argument<'_>) -> bool {
    let Some(expr) = arg.as_expression() else {
        return false;
    };
    let Expression::CallExpression(call) = expr.get_inner_expression() else {
        return false;
    };
    if !callee_name_is(&call.callee, &["tcp"]) {
        return false;
    }
    let Some(first) = call.arguments.first().and_then(Argument::as_expression) else {
        return false;
    };
    let Expression::NumericLiteral(lit) = first.get_inner_expression() else {
        return false;
    };
    ADMIN_PORTS.contains(&lit.value)
}
