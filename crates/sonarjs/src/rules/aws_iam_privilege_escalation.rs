//! Rule `aws-iam-privilege-escalation` (SonarJS key S6317).
//!
//! Clean-room port from public RSPEC S6317 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.
//!
//! An AWS IAM policy that grants a principal one of a known set of sensitive
//! IAM/STS actions (for example `iam:CreatePolicyVersion`,
//! `iam:AttachUserPolicy`, `iam:PutRolePolicy`, `sts:AssumeRole`) on *every*
//! resource (`"*"`) enables privilege escalation: the principal can rewrite or
//! attach policies, mint credentials, or assume other roles and thereby grant
//! itself administrator-level access. Such statements should target only the
//! specific resources actually required, never the `"*"` wildcard.
//!
//! ## Zero-FP sibling-gated subset
//!
//! A bare sensitive action string is not enough to conclude privilege
//! escalation, and an unconstrained scan of action strings would be noisy. To
//! stay false-positive-free without dataflow or type analysis, this port flags
//! an `ObjectExpression` (an IAM `PolicyStatement` properties literal) ONLY
//! when ALL of the following hold in the same object literal:
//!
//! 1. an `actions` property whose value is an array literal containing a
//!    string-literal element equal (ASCII-case-insensitively) to one of the
//!    curated privilege-escalation actions, AND
//! 2. a `resources` property whose value is an array literal containing the
//!    string-literal element `"*"` (all resources), AND
//! 3. no `effect` property explicitly set to `Deny` (CDK's default effect is
//!    `Allow`, so an absent `effect` still grants the permission).
//!
//! The combination "privilege-escalation action + all resources + not Deny" is
//! distinctive to dangerous IAM `PolicyStatement` configuration objects, so
//! flagging only this exact shape is effectively zero-false-positive. The span
//! of the `actions` property is reported. Statements that restrict the resource
//! list, or that are explicitly `Deny`, are deliberately not flagged
//! (under-report rather than over-report).
//!
//! ## Flagged
//! ```js
//! new PolicyStatement({
//!   actions: ["iam:CreatePolicyVersion"],
//!   resources: ["*"],
//! });
//! ```
//!
//! ## Not Flagged
//! ```js
//! new PolicyStatement({ actions: ["iam:CreatePolicyVersion"], resources: [arn] }); // scoped
//! new PolicyStatement({ actions: ["s3:GetObject"], resources: ["*"] });            // benign action
//! new PolicyStatement({ actions: ["iam:PassRole"], resources: ["*"], effect: Effect.DENY });
//! ```

use oxc_ast::ast::{
    ArrayExpressionElement, Expression, ObjectExpression, ObjectPropertyKind, PropertyKey,
};
use oxc_span::GetSpan;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "aws-iam-privilege-escalation";

/// Curated set of AWS IAM/STS actions that, when granted on all resources,
/// allow a principal to escalate its own privileges. Compared
/// ASCII-case-insensitively against literal action strings.
const ESCALATION_ACTIONS: [&str; 22] = [
    "iam:CreatePolicyVersion",
    "iam:SetDefaultPolicyVersion",
    "iam:CreateAccessKey",
    "iam:CreateLoginProfile",
    "iam:UpdateLoginProfile",
    "iam:AttachUserPolicy",
    "iam:AttachGroupPolicy",
    "iam:AttachRolePolicy",
    "iam:PutUserPolicy",
    "iam:PutGroupPolicy",
    "iam:PutRolePolicy",
    "iam:AddUserToGroup",
    "iam:UpdateAssumeRolePolicy",
    "iam:PassRole",
    "sts:AssumeRole",
    "lambda:CreateFunction",
    "lambda:InvokeFunction",
    "lambda:CreateEventSourceMapping",
    "glue:CreateDevEndpoint",
    "glue:UpdateDevEndpoint",
    "cloudformation:CreateStack",
    "datapipeline:CreatePipeline",
];

/// Returns the static key name of a property (static identifier or string
/// literal key), or `None` for computed/other keys.
fn property_key_name<'a>(key: &'a PropertyKey<'_>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.as_str()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.as_str()),
        _ => None,
    }
}

/// True when `action` matches one of the curated escalation actions,
/// case-insensitively (AWS treats IAM action names case-insensitively).
fn is_escalation_action(action: &str) -> bool {
    ESCALATION_ACTIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(action))
}

/// True when `array` contains a string-literal element equal to `needle`
/// (case-sensitive; the `"*"` wildcard is exact).
fn array_contains_exact(array: &oxc_ast::ast::ArrayExpression<'_>, needle: &str) -> bool {
    array.elements.iter().any(|element| {
        matches!(element, ArrayExpressionElement::StringLiteral(lit) if lit.value == needle)
    })
}

impl Scanner<'_> {
    pub(crate) fn check_aws_iam_privilege_escalation(&mut self, it: &ObjectExpression<'_>) {
        let mut escalation_actions_span = None;
        let mut grants_all_resources = false;
        let mut explicit_deny = false;

        for prop in &it.properties {
            let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                continue;
            };
            let Some(name) = property_key_name(&prop.key) else {
                continue;
            };
            match name {
                "actions" => {
                    if let Expression::ArrayExpression(array) = &prop.value {
                        let has_escalation = array.elements.iter().any(|element| {
                            matches!(
                                element,
                                ArrayExpressionElement::StringLiteral(lit)
                                    if is_escalation_action(lit.value.as_str())
                            )
                        });
                        if has_escalation {
                            escalation_actions_span = Some(prop.span);
                        }
                    }
                }
                "resources" => {
                    if let Expression::ArrayExpression(array) = &prop.value
                        && array_contains_exact(array, "*")
                    {
                        grants_all_resources = true;
                    }
                }
                "effect" => {
                    // CDK's default effect is `Allow`, so only an explicit
                    // `Deny` clears the finding. Match `Effect.DENY`,
                    // `"Deny"`, etc. via the value's source text.
                    let text = self.text(prop.value.span());
                    if text.contains("DENY") || text.contains("Deny") || text.contains("deny") {
                        explicit_deny = true;
                    }
                }
                _ => {}
            }
        }

        if explicit_deny || !grants_all_resources {
            return;
        }
        let Some(span) = escalation_actions_span else {
            return;
        };
        self.report(RULE_NAME, "iamPrivilegeEscalation", span);
    }
}
