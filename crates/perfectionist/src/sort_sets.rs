//! Oxc AST target matching for Perfectionist's `sort-sets` v5.10.0 contract.

use oxc_ast::{Comment, ast::Program};
use oxlint_plugins_carton::SmallVec;
use serde_json::Value;

use crate::{
    sort_array_includes::{ArrayRuleTarget, check_with_target},
    sort_named_specifiers::RuleContract,
    types::RuleDiagnostic,
};

const CONTRACT: RuleContract = RuleContract {
    rule: "sort-sets",
    selector: "literal",
    order_message_id: "unexpectedSetsOrder",
    group_order_message_id: "unexpectedSetsGroupOrder",
    extra_spacing_message_id: "extraSpacingBetweenSetsMembers",
    missed_spacing_message_id: "missedSpacingBetweenSetsMembers",
    missed_comment_above_message_id: None,
};

pub(crate) fn check<'ast>(
    source_text: &'ast str,
    program: &Program<'ast>,
    comments: &[Comment],
    raw_options: &Value,
) -> SmallVec<[RuleDiagnostic; 8]> {
    check_with_target(
        source_text,
        program,
        comments,
        raw_options,
        CONTRACT,
        ArrayRuleTarget::Sets,
    )
}
