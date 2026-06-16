//! Rust unit tests for the sonarjs core. All test inputs are independently
//! authored (clean-room); no upstream SonarJS fixtures or expectations are used.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Diagnostic, SonarjsOptions, scan_sonarjs};

fn scan(rule_name: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
    let options = SonarjsOptions {
        rule_names: [CompactString::from(rule_name)].into_iter().collect(),
        ..SonarjsOptions::default()
    };
    scan_sonarjs(source, "sample.ts", &options)
}

fn scan_with_file(rule_name: &str, source: &str, filename: &str) -> SmallVec<[Diagnostic; 32]> {
    let options = SonarjsOptions {
        rule_names: [CompactString::from(rule_name)].into_iter().collect(),
        ..SonarjsOptions::default()
    };
    scan_sonarjs(source, filename, &options)
}

#[test]
fn reports_template_literal_nested_in_another() {
    let diagnostics = scan(
        "no-nested-template-literals",
        "const x = `outer ${`inner`} end`;",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-template-literals");
    assert_eq!(diagnostics[0].message_id, "nestedTemplateLiteral");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_flat_template_literal() {
    let diagnostics = scan("no-nested-template-literals", "const x = `value ${y}`;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_adjacent_template_literals() {
    let diagnostics = scan(
        "no-nested-template-literals",
        "const a = `x ${y}`;\nconst b = `z ${w}`;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_each_nested_level() {
    let diagnostics = scan(
        "no-nested-template-literals",
        "const x = `a ${`b ${`c`}`}`;",
    );
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn reports_switch_nested_in_another_switch() {
    let source = "switch (a) {\n  case 1:\n    switch (b) {\n      case 2:\n        break;\n    }\n    break;\n}";
    let diagnostics = scan("no-nested-switch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-switch");
    assert_eq!(diagnostics[0].message_id, "nestedSwitch");
    assert_eq!(diagnostics[0].loc.start_line, 3);
}

#[test]
fn does_not_report_single_switch() {
    let diagnostics = scan("no-nested-switch", "switch (a) {\n  case 1:\n    break;\n}");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_sequential_switches() {
    let source = "switch (a) {\n  default:\n    break;\n}\nswitch (b) {\n  default:\n    break;\n}";
    let diagnostics = scan("no-nested-switch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_each_inner_switch_of_doubly_nested() {
    let source = "switch (a) {\n  case 1:\n    switch (b) {\n      case 2:\n        switch (c) {\n          case 3:\n            break;\n        }\n    }\n}";
    let diagnostics = scan("no-nested-switch", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn reports_conditional_nested_in_alternate() {
    let diagnostics = scan("no-nested-conditional", "const x = a ? b : (c ? d : e);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-conditional");
    assert_eq!(diagnostics[0].message_id, "nestedConditional");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_conditional_nested_in_consequent() {
    let diagnostics = scan("no-nested-conditional", "const x = a ? (b ? c : d) : e;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-conditional");
    assert_eq!(diagnostics[0].message_id, "nestedConditional");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_flat_conditional() {
    let diagnostics = scan("no-nested-conditional", "const x = a ? b : c;");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_two_diagnostics_for_doubly_nested_conditional() {
    let diagnostics = scan(
        "no-nested-conditional",
        "const x = a ? (b ? c : d) : (e ? f : g);",
    );
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn disabled_rule_reports_nothing() {
    let options = SonarjsOptions {
        rule_names: SmallVec::new(),
        ..SonarjsOptions::default()
    };
    let diagnostics = scan_sonarjs("const x = `outer ${`inner`}`;", "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_collapsible_if_direct_inner() {
    let source = "if (a) if (b) {}";
    let diagnostics = scan("no-collapsible-if", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-collapsible-if");
    assert_eq!(diagnostics[0].message_id, "collapsibleIf");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_collapsible_if_block_with_single_inner() {
    let source = "if (a) { if (b) {} }";
    let diagnostics = scan("no-collapsible-if", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-collapsible-if");
    assert_eq!(diagnostics[0].message_id, "collapsibleIf");
}

#[test]
fn does_not_report_collapsible_if_outer_has_else() {
    let source = "if (a) { if (b) {} } else {}";
    let diagnostics = scan("no-collapsible-if", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_collapsible_if_inner_has_else() {
    let source = "if (a) { if (b) {} else {} }";
    let diagnostics = scan("no-collapsible-if", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_collapsible_if_block_has_two_statements() {
    let source = "if (a) { if (b) {} doSomething(); }";
    let diagnostics = scan("no-collapsible-if", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_boolean_literal_in_strict_equality() {
    let source = "x === true";
    let diagnostics = scan("no-redundant-boolean", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-redundant-boolean");
    assert_eq!(diagnostics[0].message_id, "redundantBoolean");
}

#[test]
fn reports_boolean_literal_on_left_of_strict_inequality() {
    let source = "false !== y";
    let diagnostics = scan("no-redundant-boolean", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantBoolean");
}

#[test]
fn reports_negation_of_boolean_literal() {
    let source = "!true";
    let diagnostics = scan("no-redundant-boolean", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantBoolean");
}

#[test]
fn reports_ternary_true_false() {
    let source = "cond ? true : false";
    let diagnostics = scan("no-redundant-boolean", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantBoolean");
}

#[test]
fn reports_ternary_false_true() {
    let source = "cond ? false : true";
    let diagnostics = scan("no-redundant-boolean", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantBoolean");
}

#[test]
fn does_not_report_equality_without_boolean_literal() {
    let source = "x === y";
    let diagnostics = scan("no-redundant-boolean", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_logical_not_of_non_boolean() {
    let source = "!x";
    let diagnostics = scan("no-redundant-boolean", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_ternary_with_non_boolean_branches() {
    let source = "cond ? a : b";
    let diagnostics = scan("no-redundant-boolean", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_logical_or_as_case_label() {
    let source = "switch (x) { case 1 || 2: break; }";
    let diagnostics = scan("comma-or-logical-or-case", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "comma-or-logical-or-case");
    assert_eq!(diagnostics[0].message_id, "commaOrLogicalOrInCase");
}

#[test]
fn reports_sequence_expression_as_case_label() {
    let source = "switch (x) { case (1, 2): break; }";
    let diagnostics = scan("comma-or-logical-or-case", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "comma-or-logical-or-case");
    assert_eq!(diagnostics[0].message_id, "commaOrLogicalOrInCase");
}

#[test]
fn does_not_report_plain_case_or_default() {
    let source = "switch (x) { case 1: break; default: break; }";
    let diagnostics = scan("comma-or-logical-or-case", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_logical_and_as_case_label() {
    let source = "switch (x) { case 1 && 2: break; }";
    let diagnostics = scan("comma-or-logical-or-case", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_duplicate_type_in_union() {
    let source = "type T = A | B | A;";
    let diagnostics = scan("no-duplicate-in-composite", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-duplicate-in-composite");
    assert_eq!(diagnostics[0].message_id, "duplicateType");
}

#[test]
fn reports_duplicate_type_in_intersection() {
    let source = "type T = A & B & A;";
    let diagnostics = scan("no-duplicate-in-composite", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "duplicateType");
}

#[test]
fn does_not_report_union_with_all_unique_members() {
    let source = "type T = A | B | C;";
    let diagnostics = scan("no-duplicate-in-composite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_intersection_with_all_unique_members() {
    let source = "type T = A & B;";
    let diagnostics = scan("no-duplicate-in-composite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_two_diagnostics_for_triple_duplicate_in_union() {
    let source = "type T = A | A | A;";
    let diagnostics = scan("no-duplicate-in-composite", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn reports_non_existent_operator_for_equals_minus() {
    let source = "let x = 0; x =- 1;";
    let diagnostics = scan("non-existent-operator", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "non-existent-operator");
    assert_eq!(diagnostics[0].message_id, "nonExistentOperator");
}

#[test]
fn reports_non_existent_operator_for_equals_plus() {
    let source = "let x = 0; x =+ 1;";
    let diagnostics = scan("non-existent-operator", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "nonExistentOperator");
}

#[test]
fn reports_non_existent_operator_for_equals_not() {
    let source = "let x = false; let y = true; x =! y;";
    let diagnostics = scan("non-existent-operator", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "nonExistentOperator");
}

#[test]
fn does_not_report_non_existent_operator_when_space_before_unary() {
    let source = "let x = 0; x = -1;";
    let diagnostics = scan("non-existent-operator", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_non_existent_operator_for_compound_assignment() {
    let source = "let x = 0; x -= 1;";
    let diagnostics = scan("non-existent-operator", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_non_existent_operator_for_plain_assign_non_unary() {
    let source = "let x = 0; let y = 1; x = y;";
    let diagnostics = scan("non-existent-operator", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_identical_condition_in_three_branch_chain() {
    let source = "if (a) {} else if (b) {} else if (a) {}";
    let diagnostics = scan("no-identical-conditions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-identical-conditions");
    assert_eq!(diagnostics[0].message_id, "identicalConditions");
}

#[test]
fn reports_identical_condition_in_two_branch_chain() {
    let source = "if (a) {} else if (a) {}";
    let diagnostics = scan("no-identical-conditions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalConditions");
}

#[test]
fn does_not_report_when_else_is_a_plain_block() {
    let source = "if (a) {} else if (b) {} else {}";
    let diagnostics = scan("no-identical-conditions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_standalone_if_with_no_else_if() {
    let source = "if (a) {}";
    let diagnostics = scan("no-identical-conditions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_one_identical_condition_in_four_branch_chain() {
    let source = "if (a) {} else if (b) {} else if (c) {} else if (b) {}";
    let diagnostics = scan("no-identical-conditions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalConditions");
}

#[test]
fn does_not_report_identical_condition_in_nested_separate_chain() {
    // The inner `if (a)` is a separate chain; identical condition across
    // different chains must NOT be reported.
    let source = "if (a) { if (a) {} }";
    let diagnostics = scan("no-identical-conditions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_all_duplicated_branches_if_else_identical() {
    let source = "if (a) { f(); } else { f(); }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-all-duplicated-branches");
    assert_eq!(diagnostics[0].message_id, "allDuplicatedBranches");
}

#[test]
fn reports_all_duplicated_branches_if_else_if_else_identical() {
    let source = "if (a) { f(); } else if (b) { f(); } else { f(); }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "allDuplicatedBranches");
}

#[test]
fn does_not_report_all_duplicated_branches_if_else_differ() {
    let source = "if (a) { f(); } else { g(); }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_all_duplicated_branches_no_terminal_else() {
    let source = "if (a) { f(); } else if (b) { f(); }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_all_duplicated_branches_switch_all_identical() {
    let source = "switch (x) { case 1: f(); break; default: f(); break; }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "allDuplicatedBranches");
}

#[test]
fn does_not_report_all_duplicated_branches_switch_cases_differ() {
    let source = "switch (x) { case 1: f(); break; default: g(); break; }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_all_duplicated_branches_switch_no_default() {
    let source = "switch (x) { case 1: f(); break; case 2: f(); break; }";
    let diagnostics = scan("no-all-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_identical_expressions_strict_equality() {
    let source = "a === a";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-identical-expressions");
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_strict_inequality() {
    let source = "b !== b";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_less_than() {
    let source = "x < x";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_logical_and() {
    let source = "a && a";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_logical_or() {
    let source = "a || a";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_bitwise_and() {
    let source = "a & a";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_subtraction() {
    let source = "a - a";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn reports_identical_expressions_division() {
    let source = "a / a";
    let diagnostics = scan("no-identical-expressions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "identicalExpressions");
}

#[test]
fn does_not_report_identical_expressions_different_operands() {
    let source = "a === b";
    let diagnostics = scan("no-identical-expressions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_identical_expressions_addition_excluded() {
    let source = "a + a";
    let diagnostics = scan("no-identical-expressions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_identical_expressions_multiplication_excluded() {
    let source = "a * a";
    let diagnostics = scan("no-identical-expressions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_identical_expressions_left_shift_excluded() {
    let source = "a << a";
    let diagnostics = scan("no-identical-expressions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_identical_expressions_nullish_coalescing_excluded() {
    let source = "a ?? a";
    let diagnostics = scan("no-identical-expressions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_identical_expressions_different_member_access() {
    let source = "a.b === a.c";
    let diagnostics = scan("no-identical-expressions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_arguments_usage_inside_function() {
    let source = "function f() { return arguments[0]; }";
    let diagnostics = scan("arguments-usage", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "arguments-usage");
    assert_eq!(diagnostics[0].message_id, "argumentsUsage");
}

#[test]
fn reports_arguments_usage_for_arguments_length() {
    let source = "function f() { console.log(arguments.length); }";
    let diagnostics = scan("arguments-usage", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "argumentsUsage");
}

#[test]
fn does_not_report_arguments_usage_with_rest_params() {
    let source = "function f(...args) { return args[0]; }";
    let diagnostics = scan("arguments-usage", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_arguments_usage_for_property_name() {
    let source = "const o = { arguments: 1 }; o.arguments;";
    let diagnostics = scan("arguments-usage", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_arguments_usage_for_plain_function() {
    let source = "function f() { return 1; }";
    let diagnostics = scan("arguments-usage", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_labels_for_labeled_loop() {
    let source = "loop: for (;;) { break loop; }";
    let diagnostics = scan("no-labels", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-labels");
    assert_eq!(diagnostics[0].message_id, "noLabels");
}

#[test]
fn reports_no_labels_for_two_nested_labeled_loops() {
    let source = "outer: for (;;) { inner: for (;;) { break outer; } }";
    let diagnostics = scan("no-labels", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn does_not_report_no_labels_for_unlabeled_loop() {
    let source = "for (;;) { break; }";
    let diagnostics = scan("no-labels", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_labels_for_plain_variable_declaration() {
    let source = "const x = 1;";
    let diagnostics = scan("no-labels", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_label_position_for_expression_label() {
    let source = "unused: doWork();";
    let diagnostics = scan("label-position", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "label-position");
    assert_eq!(diagnostics[0].message_id, "removeLabel");
}

#[test]
fn reports_label_position_for_block_and_if_labels() {
    let source = "block: { doWork(); }\nconditional: if (ready) { doWork(); }";
    let diagnostics = scan("label-position", source);
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message_id == "removeLabel")
    );
}

#[test]
fn does_not_report_label_position_for_directly_labelled_breakable_statements() {
    let source = "
        labelled_for: for (;;) { break labelled_for; }
        labelled_for_in: for (const key in object) { break labelled_for_in; }
        labelled_for_of: for (const value of values) { break labelled_for_of; }
        labelled_while: while (condition) { break labelled_while; }
        labelled_do: do { break labelled_do; } while (condition);
        labelled_switch: switch (value) { case 1: break labelled_switch; }
    ";
    let diagnostics = scan("label-position", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_label_position_for_outer_nested_label() {
    let source = "outer: inner: for (;;) { break outer; }";
    let diagnostics = scan("label-position", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "removeLabel");
    assert_eq!(diagnostics[0].loc.start_column, 0);
}

#[test]
fn reports_no_delete_var_for_bare_variable() {
    let source = "delete x;";
    let diagnostics = scan("no-delete-var", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-delete-var");
    assert_eq!(diagnostics[0].message_id, "noDeleteVar");
}

#[test]
fn reports_no_delete_var_for_parenthesised_variable() {
    let source = "delete (y);";
    let diagnostics = scan("no-delete-var", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noDeleteVar");
}

#[test]
fn does_not_report_no_delete_var_for_member_expression_dot() {
    let source = "delete obj.prop;";
    let diagnostics = scan("no-delete-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_delete_var_for_member_expression_bracket() {
    let source = "delete obj[key];";
    let diagnostics = scan("no-delete-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_delete_var_for_plain_variable_declaration() {
    let source = "const z = 1;";
    let diagnostics = scan("no-delete-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_constructor_for_side_effects_new_with_parens() {
    let source = "new Foo();";
    let diagnostics = scan("constructor-for-side-effects", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "constructor-for-side-effects");
    assert_eq!(diagnostics[0].message_id, "constructorForSideEffects");
}

#[test]
fn reports_constructor_for_side_effects_new_without_parens() {
    let source = "new Foo;";
    let diagnostics = scan("constructor-for-side-effects", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "constructorForSideEffects");
}

#[test]
fn does_not_report_constructor_for_side_effects_when_result_assigned() {
    let source = "const x = new Foo();";
    let diagnostics = scan("constructor-for-side-effects", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_constructor_for_side_effects_when_result_used_as_receiver() {
    let source = "new Foo().bar();";
    let diagnostics = scan("constructor-for-side-effects", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_constructor_for_side_effects_for_plain_call_statement() {
    let source = "foo();";
    let diagnostics = scan("constructor-for-side-effects", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_empty_character_class_between_other_chars() {
    let source = "const r = /a[]b/;";
    let diagnostics = scan("no-empty-character-class", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-empty-character-class");
    assert_eq!(diagnostics[0].message_id, "emptyCharacterClass");
}

#[test]
fn reports_no_empty_character_class_whole_pattern() {
    let source = "const r = /[]/;";
    let diagnostics = scan("no-empty-character-class", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyCharacterClass");
}

#[test]
fn does_not_report_no_empty_character_class_for_non_empty_class() {
    let source = "const r = /[abc]/;";
    let diagnostics = scan("no-empty-character-class", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_character_class_for_negated_empty_class() {
    // [^] is a valid JS regex that matches any single character — NOT empty
    let source = "const r = /[^]/;";
    let diagnostics = scan("no-empty-character-class", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_character_class_for_escaped_brackets() {
    // /a\[\]b/ in source: the pattern is a\[\]b — escaped brackets, no class
    let source = "const r = /a\\[\\]b/;";
    let diagnostics = scan("no-empty-character-class", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_character_class_for_literal_bracket_in_class() {
    // /[a[]/ — class content is `a[`, closed by the first `]`; no empty class
    let source = "const r = /[a[]/;";
    let diagnostics = scan("no-empty-character-class", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_generator_without_yield_for_generator_that_only_returns() {
    let source = "function* g() { return 1; }";
    let diagnostics = scan("generator-without-yield", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "generator-without-yield");
    assert_eq!(diagnostics[0].message_id, "generatorWithoutYield");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_generator_without_yield_for_empty_body_generator() {
    let source = "function* g() {}";
    let diagnostics = scan("generator-without-yield", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "generatorWithoutYield");
}

#[test]
fn does_not_report_generator_without_yield_when_generator_yields() {
    let source = "function* g() { yield 1; }";
    let diagnostics = scan("generator-without-yield", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_generator_without_yield_for_regular_function() {
    let source = "function g() { return 1; }";
    let diagnostics = scan("generator-without-yield", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_generator_without_yield_for_outer_only_when_inner_yields() {
    // outer has no direct yield; inner has yield 1 → only outer is flagged
    let source = "function* outer() { function* inner() { yield 1; } }";
    let diagnostics = scan("generator-without-yield", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "generatorWithoutYield");
    // outer starts at column 0
    assert_eq!(diagnostics[0].loc.start_column, 0);
}

#[test]
fn reports_generator_without_yield_for_inner_only_when_outer_yields() {
    // outer yields directly; inner has no yield → only inner is flagged
    let source = "function* outer() { yield 1; function* inner() {} }";
    let diagnostics = scan("generator-without-yield", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "generatorWithoutYield");
    // inner starts at column > 0 (it is not at the start of the line)
    assert!(diagnostics[0].loc.start_column > 0);
}

#[test]
fn reports_no_exclusive_tests_for_describe_only() {
    let source = "describe.only('x', () => {});";
    let diagnostics = scan("no-exclusive-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-exclusive-tests");
    assert_eq!(diagnostics[0].message_id, "noExclusiveTests");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_exclusive_tests_for_it_only() {
    let source = "it.only('x', () => {});";
    let diagnostics = scan("no-exclusive-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noExclusiveTests");
}

#[test]
fn reports_no_exclusive_tests_for_test_only() {
    let source = "test.only('x', () => {});";
    let diagnostics = scan("no-exclusive-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noExclusiveTests");
}

#[test]
fn does_not_report_no_exclusive_tests_for_it_without_only() {
    let source = "it('x', () => {});";
    let diagnostics = scan("no-exclusive-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_exclusive_tests_for_unknown_function_with_only() {
    let source = "foo.only();";
    let diagnostics = scan("no-exclusive-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_exclusive_tests_for_describe_without_only() {
    let source = "describe('x', () => {});";
    let diagnostics = scan("no-exclusive-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_built_in_override_for_let_declaration_shadowing_object() {
    let source = "let Object = 1;";
    let diagnostics = scan("no-built-in-override", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-built-in-override");
    assert_eq!(diagnostics[0].message_id, "noBuiltInOverride");
}

#[test]
fn reports_no_built_in_override_for_simple_assignment_to_array() {
    let source = "Array = 2;";
    let diagnostics = scan("no-built-in-override", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noBuiltInOverride");
}

#[test]
fn reports_no_built_in_override_for_function_declaration_named_map() {
    let source = "function Map() {}";
    let diagnostics = scan("no-built-in-override", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noBuiltInOverride");
}

#[test]
fn does_not_report_no_built_in_override_for_member_expression_assignment() {
    let source = "Math.PI = 3;";
    let diagnostics = scan("no-built-in-override", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_built_in_override_for_non_builtin_variable() {
    let source = "let obj = 1;";
    let diagnostics = scan("no-built-in-override", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_built_in_override_for_member_assignment_foo_object() {
    let source = "foo.Object = 1;";
    let diagnostics = scan("no-built-in-override", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_class_prototype_for_method_assignment() {
    let source = "Foo.prototype.bar = function () {};";
    let diagnostics = scan("class-prototype", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "class-prototype");
    assert_eq!(diagnostics[0].message_id, "classPrototype");
}

#[test]
fn reports_class_prototype_for_property_assignment() {
    let source = "Foo.prototype.baz = 1;";
    let diagnostics = scan("class-prototype", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "classPrototype");
}

#[test]
fn reports_class_prototype_for_chained_prototype_assignment() {
    let source = "a.b.prototype.c = x;";
    let diagnostics = scan("class-prototype", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "classPrototype");
}

#[test]
fn does_not_report_class_prototype_for_prototype_itself_assignment() {
    // LHS is Foo.prototype — the property IS prototype; no .member after it
    let source = "Foo.prototype = {};";
    let diagnostics = scan("class-prototype", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_class_prototype_for_plain_member_assignment() {
    let source = "foo.bar = 1;";
    let diagnostics = scan("class-prototype", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_class_prototype_for_read_expression() {
    // Reading Foo.prototype (not an assignment) — no AssignmentExpression
    let source = "obj.prototype;";
    let diagnostics = scan("class-prototype", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_max_switch_cases_for_switch_with_31_cases() {
    // 31 case clauses (indices 0..=30) — strictly greater than the default threshold (30) → 1 diagnostic
    let source = "switch (x) {case 0: break;case 1: break;case 2: break;case 3: break;case 4: break;case 5: break;case 6: break;case 7: break;case 8: break;case 9: break;case 10: break;case 11: break;case 12: break;case 13: break;case 14: break;case 15: break;case 16: break;case 17: break;case 18: break;case 19: break;case 20: break;case 21: break;case 22: break;case 23: break;case 24: break;case 25: break;case 26: break;case 27: break;case 28: break;case 29: break;case 30: break;}";
    let diagnostics = scan("max-switch-cases", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "max-switch-cases");
    assert_eq!(diagnostics[0].message_id, "maxSwitchCases");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_max_switch_cases_for_small_switch() {
    let source = "switch (x) { case 1: break; default: break; }";
    let diagnostics = scan("max-switch-cases", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_max_switch_cases_for_exactly_30_cases() {
    // 30 cases (indices 0..=29) — equal to the default threshold, not strictly greater → 0 diagnostics
    let source = "switch (x) {case 0: break;case 1: break;case 2: break;case 3: break;case 4: break;case 5: break;case 6: break;case 7: break;case 8: break;case 9: break;case 10: break;case 11: break;case 12: break;case 13: break;case 14: break;case 15: break;case 16: break;case 17: break;case 18: break;case 19: break;case 20: break;case 21: break;case 22: break;case 23: break;case 24: break;case 25: break;case 26: break;case 27: break;case 28: break;case 29: break;}";
    let diagnostics = scan("max-switch-cases", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_max_union_size_for_four_member_union() {
    let source = "type T = A | B | C | D;";
    let diagnostics = scan("max-union-size", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "max-union-size");
    assert_eq!(diagnostics[0].message_id, "maxUnionSize");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_max_union_size_for_three_member_union_at_threshold() {
    let source = "type T = A | B | C;";
    let diagnostics = scan("max-union-size", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_max_union_size_for_two_member_union() {
    let source = "type T = A | B;";
    let diagnostics = scan("max-union-size", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_max_union_size_for_single_type_alias() {
    let source = "type T = A;";
    let diagnostics = scan("max-union-size", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_max_union_size_for_union_in_variable_annotation() {
    let source = "let x: A | B | C | D | E;";
    let diagnostics = scan("max-union-size", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "maxUnionSize");
}

#[test]
fn reports_elseif_without_else_for_chain_with_one_else_if() {
    let source = "if (a) {} else if (b) {}";
    let diagnostics = scan("elseif-without-else", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "elseif-without-else");
    assert_eq!(diagnostics[0].message_id, "elseifWithoutElse");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_elseif_without_else_for_chain_with_two_else_ifs() {
    let source = "if (a) {} else if (b) {} else if (c) {}";
    let diagnostics = scan("elseif-without-else", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "elseifWithoutElse");
}

#[test]
fn does_not_report_elseif_without_else_when_chain_ends_with_else() {
    let source = "if (a) {} else if (b) {} else {}";
    let diagnostics = scan("elseif-without-else", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_elseif_without_else_for_lone_if() {
    let source = "if (a) {}";
    let diagnostics = scan("elseif-without-else", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_elseif_without_else_for_if_with_only_else() {
    let source = "if (a) {} else {}";
    let diagnostics = scan("elseif-without-else", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_elseif_without_else_exactly_once_for_inner_chain() {
    let source = "if (a) { if (x) {} else if (y) {} }";
    let diagnostics = scan("elseif-without-else", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "elseifWithoutElse");
}

#[test]
fn reports_no_case_label_in_switch_for_label_directly_in_case() {
    let source = "switch (x) { case 1: foo(); lbl: bar(); break; }";
    let diagnostics = scan("no-case-label-in-switch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-case-label-in-switch");
    assert_eq!(diagnostics[0].message_id, "caseLabelInSwitch");
}

#[test]
fn does_not_report_no_case_label_in_switch_for_switch_without_labels() {
    let source = "switch (x) { case 1: break; default: break; }";
    let diagnostics = scan("no-case-label-in-switch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_case_label_in_switch_for_label_nested_in_block() {
    // The label is inside a block statement, not a direct child of the case consequent.
    let source = "switch (x) { case 1: { lbl: bar(); } break; }";
    let diagnostics = scan("no-case-label-in-switch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_case_label_in_switch_for_label_outside_switch() {
    let source = "lbl: for (;;) {}";
    let diagnostics = scan("no-case-label-in-switch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_for_in_when_body_is_block_with_non_if_statement() {
    let source = "for (const k in o) { doStuff(k); }";
    let diagnostics = scan("for-in", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "for-in");
    assert_eq!(diagnostics[0].message_id, "forIn");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_for_in_when_body_is_single_non_if_statement_no_block() {
    let source = "for (const k in o) doStuff(k);";
    let diagnostics = scan("for-in", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "forIn");
}

#[test]
fn reports_for_in_when_body_is_empty_block() {
    let source = "for (const k in o) {}";
    let diagnostics = scan("for-in", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "forIn");
}

#[test]
fn reports_for_in_when_block_has_two_statements() {
    let source = "for (const k in o) { if (a) {} doStuff(); }";
    let diagnostics = scan("for-in", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "forIn");
}

#[test]
fn does_not_report_for_in_when_body_block_contains_single_if() {
    let source = "for (const k in o) { if (o.hasOwnProperty(k)) { doStuff(k); } }";
    let diagnostics = scan("for-in", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_in_when_body_is_directly_an_if_statement() {
    let source = "for (const k in o) if (cond) doStuff();";
    let diagnostics = scan("for-in", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_prefer_while_when_for_has_no_init_and_no_update() {
    let source = "for (; i < 10;) { i++; }";
    let diagnostics = scan("prefer-while", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-while");
    assert_eq!(diagnostics[0].message_id, "preferWhile");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_prefer_while_when_for_has_no_init_no_test_no_update() {
    let source = "for (;;) {}";
    let diagnostics = scan("prefer-while", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferWhile");
}

#[test]
fn does_not_report_prefer_while_when_for_has_init() {
    let source = "for (let i = 0; i < 10;) {}";
    let diagnostics = scan("prefer-while", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_while_when_for_has_update() {
    let source = "for (; i < 10; i++) {}";
    let diagnostics = scan("prefer-while", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_while_when_for_has_init_and_update() {
    let source = "for (let i = 0; i < 10; i++) {}";
    let diagnostics = scan("prefer-while", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_small_switch_for_switch_with_one_case() {
    let source = "switch (x) { case 1: break; }";
    let diagnostics = scan("no-small-switch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-small-switch");
    assert_eq!(diagnostics[0].message_id, "smallSwitch");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_small_switch_for_switch_with_one_case_and_default() {
    let source = "switch (x) { case 1: break; default: break; }";
    let diagnostics = scan("no-small-switch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "smallSwitch");
}

#[test]
fn reports_no_small_switch_for_switch_with_only_default() {
    let source = "switch (x) { default: break; }";
    let diagnostics = scan("no-small-switch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "smallSwitch");
}

#[test]
fn reports_no_small_switch_for_empty_switch() {
    let source = "switch (x) {}";
    let diagnostics = scan("no-small-switch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "smallSwitch");
}

#[test]
fn does_not_report_no_small_switch_for_switch_with_two_cases() {
    let source = "switch (x) { case 1: break; case 2: break; }";
    let diagnostics = scan("no-small-switch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_small_switch_for_switch_with_two_cases_and_default() {
    let source = "switch (x) { case 1: break; case 2: break; default: break; }";
    let diagnostics = scan("no-small-switch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_prefer_default_last_when_default_is_first() {
    let source = "switch (x) { default: break; case 1: break; }";
    let diagnostics = scan("prefer-default-last", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-default-last");
    assert_eq!(diagnostics[0].message_id, "defaultLast");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_prefer_default_last_when_default_is_in_the_middle() {
    let source = "switch (x) { case 1: break; default: break; case 2: break; }";
    let diagnostics = scan("prefer-default-last", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "defaultLast");
}

#[test]
fn does_not_report_prefer_default_last_when_default_is_last() {
    let source = "switch (x) { case 1: break; default: break; }";
    let diagnostics = scan("prefer-default-last", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_default_last_when_there_is_no_default() {
    let source = "switch (x) { case 1: break; case 2: break; }";
    let diagnostics = scan("prefer-default-last", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_inverted_boolean_check_for_negated_strict_equality() {
    let source = "const r = !(a === b);";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-inverted-boolean-check");
    assert_eq!(diagnostics[0].message_id, "invertedBooleanCheck");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_inverted_boolean_check_for_negated_less_than() {
    let source = "const r = !(a < b);";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "invertedBooleanCheck");
}

#[test]
fn reports_inverted_boolean_check_for_negated_strict_inequality() {
    let source = "const r = !(x !== y);";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "invertedBooleanCheck");
}

#[test]
fn reports_inverted_boolean_check_for_negated_greater_equal() {
    let source = "const r = !(a >= b);";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "invertedBooleanCheck");
}

#[test]
fn does_not_report_inverted_boolean_check_for_negated_logical_and() {
    let source = "const r = !(a && b);";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_boolean_check_for_plain_negation() {
    let source = "const r = !a;";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_boolean_check_for_negated_arithmetic() {
    let source = "const r = !(a + b);";
    let diagnostics = scan("no-inverted-boolean-check", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_useless_catch_for_catch_that_only_rethrows() {
    let source = "try { f(); } catch (e) { throw e; }";
    let diagnostics = scan("no-useless-catch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-useless-catch");
    assert_eq!(diagnostics[0].message_id, "uselessCatch");
}

#[test]
fn reports_no_useless_catch_when_finally_is_present() {
    let source = "try { f(); } catch (err) { throw err; } finally { g(); }";
    let diagnostics = scan("no-useless-catch", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "uselessCatch");
}

#[test]
fn does_not_report_no_useless_catch_when_body_has_two_statements() {
    let source = "try { f(); } catch (e) { log(e); throw e; }";
    let diagnostics = scan("no-useless-catch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_useless_catch_when_throw_is_new_expression() {
    let source = "try { f(); } catch (e) { throw new Error(); }";
    let diagnostics = scan("no-useless-catch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_useless_catch_when_throw_is_member_expression() {
    let source = "try { f(); } catch (e) { throw e.cause; }";
    let diagnostics = scan("no-useless-catch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_useless_catch_when_no_throw_in_body() {
    let source = "try { f(); } catch (e) { handle(e); }";
    let diagnostics = scan("no-useless-catch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_useless_catch_for_destructured_param() {
    let source = "try { f(); } catch ({ message }) { throw message; }";
    let diagnostics = scan("no-useless-catch", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_redundant_optional_for_union_with_undefined() {
    let source = "interface I { a?: string | undefined; }";
    let diagnostics = scan("no-redundant-optional", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-redundant-optional");
    assert_eq!(diagnostics[0].message_id, "redundantOptional");
}

#[test]
fn reports_no_redundant_optional_for_undefined_type_directly() {
    let source = "interface I { b?: undefined; }";
    let diagnostics = scan("no-redundant-optional", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantOptional");
}

#[test]
fn reports_no_redundant_optional_for_multi_member_union_with_undefined() {
    let source = "interface I { c?: number | string | undefined; }";
    let diagnostics = scan("no-redundant-optional", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantOptional");
}

#[test]
fn does_not_report_no_redundant_optional_when_no_undefined_in_type() {
    let source = "interface I { a?: string; }";
    let diagnostics = scan("no-redundant-optional", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_redundant_optional_for_non_optional_property_with_undefined() {
    let source = "interface I { b: string | undefined; }";
    let diagnostics = scan("no-redundant-optional", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_redundant_optional_for_optional_property_with_null_not_undefined() {
    let source = "interface I { c?: string | null; }";
    let diagnostics = scan("no-redundant-optional", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_prefer_immediate_return_for_const_declared_then_returned() {
    let source = "function f() { const x = compute(); return x; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-immediate-return");
    assert_eq!(diagnostics[0].message_id, "preferImmediateReturn");
}

#[test]
fn reports_prefer_immediate_return_for_const_declared_then_thrown() {
    let source = "function f() { const e = new Error(); throw e; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferImmediateReturn");
}

#[test]
fn reports_prefer_immediate_return_for_arrow_function_block_body() {
    let source = "const g = () => { const x = 1; return x; };";
    let diagnostics = scan("prefer-immediate-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferImmediateReturn");
}

#[test]
fn does_not_report_prefer_immediate_return_for_direct_return() {
    let source = "function f() { return compute(); }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_immediate_return_when_statement_between_decl_and_return() {
    let source = "function f() { const x = 1; doStuff(); return x; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_immediate_return_when_return_uses_different_identifier() {
    let source = "function f() { const x = 1; return y; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_immediate_return_when_return_is_not_bare_identifier() {
    let source = "function f() { const x = 1; return x + 1; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_immediate_return_when_declaration_has_two_declarators() {
    let source = "function f() { const x = 1, y = 2; return x; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_immediate_return_when_declarator_has_no_init() {
    // `let x;` has no initializer — there is nothing to inline
    let source = "function f() { let x; return x; }";
    let diagnostics = scan("prefer-immediate-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_redundant_jump_for_trailing_continue_in_for_loop() {
    let source = "for (;;) { foo(); continue; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-redundant-jump");
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn reports_redundant_jump_for_trailing_continue_in_while_loop() {
    let source = "while (x) { foo(); continue; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn reports_redundant_jump_for_trailing_continue_in_do_while_loop() {
    let source = "do { foo(); continue; } while (x);";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn reports_redundant_jump_for_trailing_continue_in_for_of_loop() {
    let source = "for (const a of b) { foo(); continue; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn reports_redundant_jump_for_trailing_continue_in_for_in_loop() {
    let source = "for (k in o) { foo(); continue; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn reports_redundant_jump_for_trailing_return_in_function() {
    let source = "function f() { foo(); return; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn reports_redundant_jump_for_trailing_return_in_arrow_function() {
    let source = "const g = () => { foo(); return; };";
    let diagnostics = scan("no-redundant-jump", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantJump");
}

#[test]
fn does_not_report_redundant_jump_when_continue_is_not_last() {
    let source = "for (;;) { if (x) continue; foo(); }";
    let diagnostics = scan("no-redundant-jump", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_redundant_jump_for_return_with_value() {
    let source = "function f() { foo(); return x; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_redundant_jump_for_labeled_continue() {
    let source = "outer: for (;;) { foo(); continue outer; }";
    let diagnostics = scan("no-redundant-jump", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_redundant_jump_for_non_block_loop_body() {
    let source = "while (x) foo();";
    let diagnostics = scan("no-redundant-jump", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_primitive_wrappers_for_new_number() {
    let source = "const n = new Number(1);";
    let diagnostics = scan("no-primitive-wrappers", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-primitive-wrappers");
    assert_eq!(diagnostics[0].message_id, "primitiveWrapper");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_primitive_wrappers_for_new_string() {
    let source = "const s = new String('x');";
    let diagnostics = scan("no-primitive-wrappers", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "primitiveWrapper");
}

#[test]
fn reports_no_primitive_wrappers_for_new_boolean() {
    let source = "const b = new Boolean(false);";
    let diagnostics = scan("no-primitive-wrappers", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "primitiveWrapper");
}

#[test]
fn does_not_report_no_primitive_wrappers_for_call_without_new() {
    let source = "const n = Number(1);";
    let diagnostics = scan("no-primitive-wrappers", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_primitive_wrappers_for_new_array() {
    let source = "const a = new Array(3);";
    let diagnostics = scan("no-primitive-wrappers", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_primitive_wrappers_for_unknown_constructor() {
    let source = "const f = new Foo();";
    let diagnostics = scan("no-primitive-wrappers", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_skipped_tests_for_describe_skip() {
    let source = "describe.skip('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-skipped-tests");
    assert_eq!(diagnostics[0].message_id, "skippedTest");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_skipped_tests_for_it_skip() {
    let source = "it.skip('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "skippedTest");
}

#[test]
fn reports_no_skipped_tests_for_test_skip() {
    let source = "test.skip('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "skippedTest");
}

#[test]
fn reports_no_skipped_tests_for_xit() {
    let source = "xit('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "skippedTest");
}

#[test]
fn reports_no_skipped_tests_for_xdescribe() {
    let source = "xdescribe('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "skippedTest");
}

#[test]
fn does_not_report_no_skipped_tests_for_it_without_skip() {
    let source = "it('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_skipped_tests_for_describe_without_skip() {
    let source = "describe('x', () => {});";
    let diagnostics = scan("no-skipped-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_skipped_tests_for_unknown_runner_with_skip() {
    let source = "foo.skip();";
    let diagnostics = scan("no-skipped-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_skipped_tests_for_xfoo_not_in_x_set() {
    let source = "xfoo();";
    let diagnostics = scan("no-skipped-tests", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_prefer_single_boolean_return_block_form() {
    let source = "function f() { if (c) { return true; } else { return false; } }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-single-boolean-return");
    assert_eq!(diagnostics[0].message_id, "preferSingleBooleanReturn");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_prefer_single_boolean_return_bare_form() {
    let source = "function f() { if (c) return true; else return false; }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferSingleBooleanReturn");
}

#[test]
fn reports_prefer_single_boolean_return_inverted() {
    let source = "function f() { if (c) { return false; } else { return true; } }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferSingleBooleanReturn");
}

#[test]
fn does_not_report_prefer_single_boolean_return_no_else() {
    let source = "function f() { if (c) { return true; } }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_single_boolean_return_non_literal_consequent() {
    let source = "function f() { if (c) { return x; } else { return false; } }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_single_boolean_return_else_if_chain() {
    let source = "function f() { if (c) return true; else if (d) return x; }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_single_boolean_return_block_has_two_statements() {
    let source = "function f() { if (c) { return true; bar(); } else { return false; } }";
    let diagnostics = scan("prefer-single-boolean-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_unthrown_error_for_new_error_bare_statement() {
    let source = "new Error('boom');";
    let diagnostics = scan("no-unthrown-error", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unthrown-error");
    assert_eq!(diagnostics[0].message_id, "unthrownError");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_unthrown_error_for_new_type_error_bare_statement() {
    let source = "new TypeError('x');";
    let diagnostics = scan("no-unthrown-error", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "unthrownError");
}

#[test]
fn reports_no_unthrown_error_for_user_defined_error_subtype_bare_statement() {
    let source = "new MyError();";
    let diagnostics = scan("no-unthrown-error", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "unthrownError");
}

#[test]
fn does_not_report_no_unthrown_error_when_error_is_thrown() {
    let source = "throw new Error('boom');";
    let diagnostics = scan("no-unthrown-error", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_unthrown_error_when_error_is_assigned() {
    let source = "const e = new Error();";
    let diagnostics = scan("no-unthrown-error", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_unthrown_error_for_non_error_constructor() {
    let source = "new Foo();";
    let diagnostics = scan("no-unthrown-error", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_unthrown_error_when_error_passed_as_argument() {
    let source = "foo(new Error());";
    let diagnostics = scan("no-unthrown-error", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_tab_for_leading_tab() {
    let source = "\tconst x = 1;";
    let diagnostics = scan("no-tab", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-tab");
    assert_eq!(diagnostics[0].message_id, "noTab");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_tab_for_tab_in_middle_of_line() {
    let source = "const x\t= 1;";
    let diagnostics = scan("no-tab", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noTab");
}

#[test]
fn reports_no_tab_once_when_only_second_line_has_tab() {
    let source = "a();\n\tb();";
    let diagnostics = scan("no-tab", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noTab");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn reports_no_tab_twice_when_both_lines_have_tabs() {
    let source = "\ta();\n\tb();";
    let diagnostics = scan("no-tab", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn does_not_report_no_tab_for_source_without_tabs() {
    let source = "const x = 1;";
    let diagnostics = scan("no-tab", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_fixme_tag_for_line_comment_containing_fixme() {
    let source = "// FIXME do x";
    let diagnostics = scan("fixme-tag", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "fixme-tag");
    assert_eq!(diagnostics[0].message_id, "fixmeTag");
}

#[test]
fn reports_fixme_tag_for_block_comment_containing_fixme() {
    let source = "/* FIXME: broken */";
    let diagnostics = scan("fixme-tag", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "fixmeTag");
}

#[test]
fn reports_fixme_tag_for_trailing_comment_containing_fixme() {
    let source = "const a = 1; // FIXME later";
    let diagnostics = scan("fixme-tag", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "fixmeTag");
}

#[test]
fn does_not_report_fixme_tag_for_todo_comment() {
    let source = "// TODO do x";
    let diagnostics = scan("fixme-tag", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_fixme_tag_for_lowercase_fixme() {
    let source = "// fixme";
    let diagnostics = scan("fixme-tag", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_fixme_tag_for_source_with_no_comments() {
    let source = "const a = 1;";
    let diagnostics = scan("fixme-tag", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_todo_tag_for_line_comment_containing_todo() {
    let source = "// TODO do x";
    let diagnostics = scan("todo-tag", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "todo-tag");
    assert_eq!(diagnostics[0].message_id, "todoTag");
}

#[test]
fn reports_todo_tag_for_block_comment_containing_todo() {
    let source = "/* TODO: later */";
    let diagnostics = scan("todo-tag", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_todo_tag_for_fixme_comment() {
    let source = "// FIXME do x";
    let diagnostics = scan("todo-tag", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_todo_tag_for_lowercase_todo() {
    let source = "// todo";
    let diagnostics = scan("todo-tag", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_todo_tag_for_source_with_no_comments() {
    let source = "const a = 1;";
    let diagnostics = scan("todo-tag", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_sonar_comments_for_comment_containing_nosonar() {
    let source = "// NOSONAR suppress this";
    let diagnostics = scan("no-sonar-comments", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-sonar-comments");
    assert_eq!(diagnostics[0].message_id, "noSonarComments");
}

#[test]
fn reports_no_sonar_comments_for_block_comment_containing_nosonar() {
    let source = "/* NOSONAR */";
    let diagnostics = scan("no-sonar-comments", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_sonar_comments_for_plain_comment() {
    let source = "// just a comment";
    let diagnostics = scan("no-sonar-comments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_sonar_comments_for_source_with_no_comments() {
    let source = "const a = 1;";
    let diagnostics = scan("no-sonar-comments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_array_constructor_for_multi_argument_call() {
    let source = "const a = Array(1, 2, 3);";
    let diagnostics = scan("array-constructor", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "array-constructor");
    assert_eq!(diagnostics[0].message_id, "arrayConstructor");
}

#[test]
fn reports_array_constructor_for_multi_argument_new_expression() {
    let source = "const a = new Array(1, 2, 3);";
    let diagnostics = scan("array-constructor", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_array_constructor_for_zero_argument_new_expression() {
    let source = "const a = new Array();";
    let diagnostics = scan("array-constructor", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_array_constructor_for_single_argument_length_form() {
    let source = "const a = new Array(500);";
    let diagnostics = scan("array-constructor", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_constructor_when_type_arguments_present() {
    let source = "const a = Array<number>(1, 2, 3);";
    let diagnostics = scan("array-constructor", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_constructor_for_array_literal() {
    let source = "const a = [1, 2, 3];";
    let diagnostics = scan("array-constructor", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_constructor_for_unrelated_member_call() {
    let source = "const a = foo.Array(1, 2, 3);";
    let diagnostics = scan("array-constructor", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_function_declaration_in_block_for_if_block() {
    let source = "if (cond) { function f() {} }";
    let diagnostics = scan("no-function-declaration-in-block", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noFunctionDeclarationInBlock");
}

#[test]
fn reports_no_function_declaration_in_block_for_bare_block() {
    let source = "{ function f() {} }";
    let diagnostics = scan("no-function-declaration-in-block", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_function_declaration_in_block_for_top_level() {
    let source = "function f() {}";
    let diagnostics = scan("no-function-declaration-in-block", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_function_declaration_in_block_for_nested_function_body() {
    let source = "function outer() { function inner() {} }";
    let diagnostics = scan("no-function-declaration-in-block", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_function_declaration_in_block_for_function_expression() {
    let source = "if (cond) { const f = function () {}; }";
    let diagnostics = scan("no-function-declaration-in-block", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_inconsistent_returns_for_mixed_returns() {
    let source = "function f(x) { if (!x) return; return x.value; }";
    let diagnostics = scan("no-inconsistent-returns", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-inconsistent-returns");
    assert_eq!(diagnostics[0].message_id, "inconsistentReturns");
}

#[test]
fn reports_no_inconsistent_returns_for_mixed_returns_in_arrow() {
    let source = "const f = (x) => { if (!x) return; return 1; };";
    let diagnostics = scan("no-inconsistent-returns", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_inconsistent_returns_when_all_returns_have_values() {
    let source = "function f(x) { if (!x) return 0; return x.value; }";
    let diagnostics = scan("no-inconsistent-returns", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_inconsistent_returns_when_all_returns_are_bare() {
    let source = "function f(x) { if (!x) return; doWork(); return; }";
    let diagnostics = scan("no-inconsistent-returns", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_inconsistent_returns_only_for_inner_scope() {
    let source = "function outer() { return 1; function inner() { if (a) return; return 2; } }";
    let diagnostics = scan("no-inconsistent-returns", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_invariant_returns_for_function_always_returning_same_value() {
    let source = "function f(x) { if (x > 0) return 42; return 42; }";
    let diagnostics = scan("no-invariant-returns", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-invariant-returns");
    assert_eq!(diagnostics[0].message_id, "invariantReturn");
}

#[test]
fn does_not_report_no_invariant_returns_when_values_differ() {
    let source = "function f(x) { if (x > 0) return 1; return 2; }";
    let diagnostics = scan("no-invariant-returns", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_invariant_returns_with_only_one_value_return() {
    let source = "function f(x) { if (x) return 42; }";
    let diagnostics = scan("no-invariant-returns", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_invariant_returns_when_bare_return_present() {
    let source = "function f(x) { if (!x) return; return 42; }";
    let diagnostics = scan("no-invariant-returns", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_same_line_conditional_for_if_on_closing_brace_line() {
    let source = "if (a) {\n  doA();\n} if (b) {\n  doB();\n}";
    let diagnostics = scan("no-same-line-conditional", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-same-line-conditional");
    assert_eq!(diagnostics[0].message_id, "sameLineConditional");
}

#[test]
fn does_not_report_no_same_line_conditional_for_if_on_new_line() {
    let source = "if (a) {\n  doA();\n}\nif (b) {\n  doB();\n}";
    let diagnostics = scan("no-same-line-conditional", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_same_line_conditional_for_else_if_chain() {
    let source = "if (a) {\n  doA();\n} else if (b) {\n  doB();\n}";
    let diagnostics = scan("no-same-line-conditional", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_same_line_conditional_when_preceding_is_not_if() {
    let source = "doA(); if (b) {\n  doB();\n}";
    let diagnostics = scan("no-same-line-conditional", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_nested_assignment_in_if_condition() {
    let source = "if (x = compute()) { use(x); }";
    let diagnostics = scan("no-nested-assignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-assignment");
    assert_eq!(diagnostics[0].message_id, "nestedAssignment");
}

#[test]
fn reports_no_nested_assignment_in_while_condition() {
    let source = "while (node = node.next) { visit(node); }";
    let diagnostics = scan("no-nested-assignment", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_nested_assignment_for_chained_assignment() {
    let source = "a = b = c;";
    let diagnostics = scan("no-nested-assignment", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_nested_assignment_for_plain_statement() {
    let source = "x = compute();";
    let diagnostics = scan("no-nested-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_nested_assignment_for_for_loop_init_and_update() {
    let source = "for (i = 0; i < 10; i = i + 1) { use(i); }";
    let diagnostics = scan("no-nested-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_nested_assignment_for_equality_in_condition() {
    let source = "if (x === compute()) { use(x); }";
    let diagnostics = scan("no-nested-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_nested_assignment_for_compound_assignment_in_condition() {
    let source = "while (x += 1) { use(x); }";
    let diagnostics = scan("no-nested-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_nested_incdec_for_increment_call_argument() {
    let source = "foo(i++);";
    let diagnostics = scan("no-nested-incdec", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-incdec");
    assert_eq!(diagnostics[0].message_id, "nestedIncDec");
}

#[test]
fn reports_no_nested_incdec_for_decrement_method_argument() {
    let source = "arr.push(--count);";
    let diagnostics = scan("no-nested-incdec", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_nested_incdec_for_constructor_argument() {
    let source = "new Widget(n++);";
    let diagnostics = scan("no-nested-incdec", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_nested_incdec_for_standalone_statement() {
    let source = "i++;";
    let diagnostics = scan("no-nested-incdec", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_nested_incdec_for_for_loop_update() {
    let source = "for (let i = 0; i < n; i++) { use(i); }";
    let diagnostics = scan("no-nested-incdec", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_useless_increment_for_postfix_self_increment() {
    let source = "i = i++;";
    let diagnostics = scan("no-useless-increment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-useless-increment");
    assert_eq!(diagnostics[0].message_id, "uselessIncrement");
}

#[test]
fn reports_no_useless_increment_for_postfix_self_decrement() {
    let source = "j = j--;";
    let diagnostics = scan("no-useless-increment", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_useless_increment_for_prefix_increment() {
    let source = "i = ++i;";
    let diagnostics = scan("no-useless-increment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_useless_increment_for_different_variable() {
    let source = "i = j++;";
    let diagnostics = scan("no-useless-increment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_useless_increment_for_standalone_increment() {
    let source = "i++;";
    let diagnostics = scan("no-useless-increment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_class_name_for_lowercase_class() {
    let source = "class myClass {}";
    let diagnostics = scan("class-name", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "class-name");
    assert_eq!(diagnostics[0].message_id, "className");
}

#[test]
fn reports_class_name_for_underscore_class() {
    let source = "class _Helper {}";
    let diagnostics = scan("class-name", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_class_name_for_pascal_case_class() {
    let source = "class MyClass {}";
    let diagnostics = scan("class-name", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_class_name_for_anonymous_class() {
    let source = "export default class {}";
    let diagnostics = scan("class-name", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_class_name_for_lowercase_class_expression() {
    let source = "const C = class widget {};";
    let diagnostics = scan("class-name", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_function_name_for_declarations_not_matching_default_format() {
    let source = "function Bad_name() {} function goodName() {} function _ok1() {}";
    let diagnostics = scan("function-name", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "function-name");
    assert_eq!(diagnostics[0].message_id, "renameFunction");
    assert_eq!(diagnostics[0].data.value.as_deref(), Some("Bad_name"));
    assert_eq!(
        diagnostics[0].data.format.as_deref(),
        Some("^[_a-z][a-zA-Z0-9]*$")
    );
}

#[test]
fn reports_function_name_for_variable_initialized_with_function_or_arrow() {
    let source = "const Bad_name = function() {}; const Bad_name2 = () => {}; const goodName = function Bad_name() {};";
    let diagnostics = scan("function-name", source);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].data.value.as_deref(), Some("Bad_name"));
    assert_eq!(diagnostics[1].data.value.as_deref(), Some("Bad_name2"));
}

#[test]
fn reports_function_name_for_class_and_object_method_keys() {
    let source = r#"
class C {
  Bad_name() {}
  async Bad_name2() {}
  get Bad_name3() { return 1; }
  #Bad_name4() {}
}
const obj = { Bad_name5() {}, goodName() {} };
"#;
    let diagnostics = scan("function-name", source);
    let names: SmallVec<[&str; 4]> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.data.value.as_deref())
        .collect();
    assert_eq!(
        &names[..],
        ["Bad_name", "Bad_name2", "Bad_name3", "Bad_name5"]
    );
}

#[test]
fn reports_function_name_for_object_property_function_values() {
    let source = r#"const obj = {
  Bad_name: function() {},
  Bad_name2: () => {},
  goodName: function Bad_name3() {},
  ["Bad_name4"]: function() {},
};"#;
    let diagnostics = scan("function-name", source);
    let names: SmallVec<[&str; 2]> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.data.value.as_deref())
        .collect();
    assert_eq!(&names[..], ["Bad_name", "Bad_name2"]);
}

#[test]
fn does_not_report_function_name_for_assignments_iifes_or_private_names() {
    let source = r#"
let Bad_name;
Bad_name = function() {};
Bad_name = () => {};
(function Bad_name2() {})();
class C { #Bad_name3() {} }
"#;
    let diagnostics = scan("function-name", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn function_name_respects_custom_format() {
    let mut options = options_for("function-name");
    options.function_name_format = CompactString::from("^[A-Z][A-Za-z0-9]*$");
    let diagnostics = scan_sonarjs(
        "function goodName() {} function GoodName() {} const goodName2 = () => {}; class C { goodName3() {} GoodName2() {} }",
        "sample.ts",
        &options,
    );
    let names: SmallVec<[&str; 3]> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.data.value.as_deref())
        .collect();
    assert_eq!(&names[..], ["goodName", "goodName2", "goodName3"]);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.data.format.as_deref() == Some("^[A-Z][A-Za-z0-9]*$"))
    );
}

fn options_for(rule_name: &str) -> SonarjsOptions {
    SonarjsOptions {
        rule_names: [CompactString::from(rule_name)].into_iter().collect(),
        ..SonarjsOptions::default()
    }
}

#[test]
fn max_switch_cases_respects_custom_threshold() {
    let mut options = options_for("max-switch-cases");
    options.max_switch_cases_threshold = 2;
    let three = "switch (x) { case 1: break; case 2: break; case 3: break; }";
    assert_eq!(scan_sonarjs(three, "sample.ts", &options).len(), 1);
    let two = "switch (x) { case 1: break; case 2: break; }";
    assert!(scan_sonarjs(two, "sample.ts", &options).is_empty());
}

#[test]
fn max_switch_cases_uses_default_threshold_when_unset() {
    // The default threshold is 30, so a 3-case switch is not flagged by default.
    let source = "switch (x) { case 1: break; case 2: break; case 3: break; }";
    let diagnostics = scan("max-switch-cases", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn max_union_size_respects_custom_threshold() {
    let mut options = options_for("max-union-size");
    options.max_union_size_threshold = 2;
    let three = "type T = A | B | C;";
    assert_eq!(scan_sonarjs(three, "sample.ts", &options).len(), 1);
    let two = "type T = A | B;";
    assert!(scan_sonarjs(two, "sample.ts", &options).is_empty());
}

#[test]
fn max_lines_reports_when_code_lines_exceed_threshold() {
    let mut options = options_for("max-lines");
    options.max_lines_threshold = 2;
    let source = "const a = 1;\nconst b = 2;\nconst c = 3;";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "max-lines");
    assert_eq!(diagnostics[0].message_id, "maxLines");
    assert_eq!(diagnostics[0].loc.start_line, 1);
    assert_eq!(diagnostics[0].loc.start_column, 0);
}

#[test]
fn max_lines_does_not_report_when_code_lines_equal_threshold() {
    let mut options = options_for("max-lines");
    options.max_lines_threshold = 3;
    let source = "const a = 1;\nconst b = 2;\nconst c = 3;";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn max_lines_excludes_blank_and_comment_only_lines() {
    // 2 code lines + 1 blank line + 1 comment-only line = 2 code lines total;
    // with a threshold of 2, no diagnostic should be emitted.
    let mut options = options_for("max-lines");
    options.max_lines_threshold = 2;
    let source = "const a = 1;\n\n// only a comment\nconst b = 2;";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn max_lines_uses_default_threshold_when_unset() {
    // Default threshold is 1000; a small file must not be flagged.
    let source = "const x = 1;\nconst y = 2;";
    let diagnostics = scan("max-lines", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_nested_control_flow_for_four_levels_deep() {
    // Default threshold is 3; a 4th-level statement is flagged.
    let source = "if (a) { for (let i = 0; i < 10; i++) { while (b) { if (c) {} } } }";
    let diagnostics = scan("nested-control-flow", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "nested-control-flow");
    assert_eq!(diagnostics[0].message_id, "nestedControlFlow");
}

#[test]
fn does_not_report_nested_control_flow_for_exactly_three_levels() {
    // Exactly 3 levels: at the threshold, not exceeding it.
    let source = "if (a) { for (let i = 0; i < 10; i++) { while (b) {} } }";
    let diagnostics = scan("nested-control-flow", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_nested_control_flow_for_else_if_chain() {
    // An else-if chain at the top level must not accumulate depth.
    let source = "if (a) {} else if (b) {} else if (c) {} else if (d) {} else {}";
    let diagnostics = scan("nested-control-flow", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_nested_control_flow_when_try_counts_as_a_level() {
    // try(1) + if(2) + for(3) + while(4) = 4 levels → 1 diagnostic
    let source = "try { if (a) { for (let i = 0; i < 10; i++) { while (b) {} } } } catch (e) {}";
    let diagnostics = scan("nested-control-flow", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "nested-control-flow");
    assert_eq!(diagnostics[0].message_id, "nestedControlFlow");
}

#[test]
fn nested_control_flow_respects_custom_threshold() {
    // With threshold 2, a 3-level nest should fire exactly once.
    let mut options = options_for("nested-control-flow");
    options.nested_control_flow_threshold = 2;
    let source = "if (a) { for (let i = 0; i < 10; i++) { while (b) {} } }";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "nested-control-flow");
    assert_eq!(diagnostics[0].message_id, "nestedControlFlow");
}

#[test]
fn reports_max_lines_per_function_for_function_over_threshold() {
    let mut options = options_for("max-lines-per-function");
    options.max_lines_per_function_threshold = 5;
    // 6 code lines (signature + 4 body + closing brace) → strictly above 5 → 1 diagnostic
    let source = "function f() {\n  a;\n  b;\n  c;\n  d;\n}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "max-lines-per-function");
    assert_eq!(diagnostics[0].message_id, "maxLinesPerFunction");
}

#[test]
fn does_not_report_max_lines_per_function_at_exactly_threshold() {
    let mut options = options_for("max-lines-per-function");
    options.max_lines_per_function_threshold = 5;
    // 5 code lines (signature + 3 body + closing brace), exactly at the threshold → 0
    let source = "function f() {\n  a;\n  b;\n  c;\n}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_count_blank_and_comment_lines_in_function() {
    let mut options = options_for("max-lines-per-function");
    options.max_lines_per_function_threshold = 5;
    // 7 physical lines, but the comment-only and blank lines are excluded →
    // 5 code lines, exactly at the threshold → 0 diagnostics
    let source = "function f() {\n  // c\n  a;\n\n  b;\n  c;\n}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_max_lines_per_function_for_iife() {
    let mut options = options_for("max-lines-per-function");
    options.max_lines_per_function_threshold = 5;
    // 6 code lines (above the threshold) but it's an IIFE → 0 diagnostics
    let source = "(function() {\n  a;\n  b;\n  c;\n  d;\n})();";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_max_lines_per_function_for_jsx_containing_arrow() {
    let mut options = options_for("max-lines-per-function");
    options.max_lines_per_function_threshold = 5;
    // 6 code lines (above the threshold) but the body contains JSX → 0 (parsed as TSX)
    let source = "const f = () => {\n  a;\n  b;\n  c;\n  return <div />;\n};";
    let diagnostics = scan_sonarjs(source, "sample.tsx", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_duplicate_string_at_default_threshold() {
    // "hello wrld" = 10 chars, has a space → qualifies; appears 3×; default threshold 3
    let source = "const a = \"hello wrld\"; const b = \"hello wrld\"; const c = \"hello wrld\";";
    let diagnostics = scan("no-duplicate-string", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-duplicate-string");
    assert_eq!(diagnostics[0].message_id, "duplicateString");
}

#[test]
fn does_not_report_no_duplicate_string_below_threshold() {
    // "hello wrld" appears only twice; default threshold 3 → no report
    let source = "const a = \"hello wrld\"; const b = \"hello wrld\";";
    let diagnostics = scan("no-duplicate-string", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_duplicate_string_for_short_value() {
    // "hi there" = 8 chars (< 10) → not counted; appears 3× but too short
    let source = "const a = \"hi there\"; const b = \"hi there\"; const c = \"hi there\";";
    let diagnostics = scan("no-duplicate-string", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_duplicate_string_for_all_word_chars() {
    // "helloWorld1" = 11 chars but all word chars ([A-Za-z0-9_]) → not counted
    let source = "const a = \"helloWorld1\"; const b = \"helloWorld1\"; const c = \"helloWorld1\";";
    let diagnostics = scan("no-duplicate-string", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_duplicate_string_for_import_sources() {
    // "some/module" = 11 chars, has '/' → would qualify if not excluded;
    // appears 2× as import sources with custom threshold 2 → still 0
    let mut options = options_for("no-duplicate-string");
    options.no_duplicate_string_threshold = 2;
    let source = "import a from \"some/module\";\nimport b from \"some/module\";";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_duplicate_string_respects_custom_threshold() {
    let mut options = options_for("no-duplicate-string");
    options.no_duplicate_string_threshold = 2;
    let source = "const a = \"hello wrld\"; const b = \"hello wrld\";";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-duplicate-string");
    assert_eq!(diagnostics[0].message_id, "duplicateString");
}

#[test]
fn reports_no_empty_group_for_empty_capturing_group() {
    let source = "const r = /foo()bar/;";
    let diagnostics = scan("no-empty-group", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-empty-group");
    assert_eq!(diagnostics[0].message_id, "emptyGroup");
}

#[test]
fn reports_no_empty_group_for_empty_non_capturing_group() {
    let source = "const r = /(?:)/;";
    let diagnostics = scan("no-empty-group", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyGroup");
}

#[test]
fn reports_no_empty_group_for_second_group_only() {
    let source = "const r = /(a)()/;";
    let diagnostics = scan("no-empty-group", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyGroup");
}

#[test]
fn does_not_report_no_empty_group_for_non_empty_group() {
    let source = "const r = /foo(bar)/;";
    let diagnostics = scan("no-empty-group", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_group_for_empty_alternative() {
    // (a|) has two alternatives; the group itself is not empty
    let source = "const r = /(a|)/;";
    let diagnostics = scan("no-empty-group", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_group_for_quantified_non_empty_group() {
    let source = "const r = /(a)?/;";
    let diagnostics = scan("no-empty-group", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_empty_alternatives_for_trailing_empty_alternative() {
    let source = "const r = /a|/;";
    let diagnostics = scan("no-empty-alternatives", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-empty-alternatives");
    assert_eq!(diagnostics[0].message_id, "emptyAlternative");
}

#[test]
fn reports_no_empty_alternatives_for_leading_empty_alternative() {
    let source = "const r = /|a/;";
    let diagnostics = scan("no-empty-alternatives", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_empty_alternatives_for_empty_alternative_in_group() {
    let source = "const r = /(?:a|)/;";
    let diagnostics = scan("no-empty-alternatives", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_empty_alternatives_when_all_have_content() {
    let source = "const r = /a|b|c/;";
    let diagnostics = scan("no-empty-alternatives", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_alternatives_for_empty_group() {
    // An empty group has a single empty alternative (no `|`), which is the
    // no-empty-group rule's concern, not an empty alternative.
    let source = "const r = /(?:)/;";
    let diagnostics = scan("no-empty-alternatives", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_regex_spaces_for_two_consecutive_spaces() {
    let source = "const r = /a  b/;";
    let diagnostics = scan("no-regex-spaces", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-regex-spaces");
    assert_eq!(diagnostics[0].message_id, "multipleSpaces");
}

#[test]
fn reports_no_regex_spaces_for_three_consecutive_spaces() {
    let source = "const r = /foo   bar/;";
    let diagnostics = scan("no-regex-spaces", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-regex-spaces");
    assert_eq!(diagnostics[0].message_id, "multipleSpaces");
}

#[test]
fn does_not_report_no_regex_spaces_for_single_space() {
    let source = "const r = /a b/;";
    let diagnostics = scan("no-regex-spaces", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_regex_spaces_for_space_with_quantifier() {
    let source = "const r = /a {2}b/;";
    let diagnostics = scan("no-regex-spaces", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_regex_spaces_for_spaces_inside_character_class() {
    let source = "const r = /[  ]{2}/;";
    let diagnostics = scan("no-regex-spaces", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_control_regex_for_hex_escape() {
    let source = "const r = /\\x1f/;";
    let diagnostics = scan("no-control-regex", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-control-regex");
    assert_eq!(diagnostics[0].message_id, "controlCharacter");
}

#[test]
fn reports_no_control_regex_for_unicode_escape() {
    let source = "const r = /\\u001f/;";
    let diagnostics = scan("no-control-regex", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-control-regex");
    assert_eq!(diagnostics[0].message_id, "controlCharacter");
}

#[test]
fn reports_no_control_regex_for_control_letter_escape() {
    let source = "const r = /\\cA/;";
    let diagnostics = scan("no-control-regex", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-control-regex");
    assert_eq!(diagnostics[0].message_id, "controlCharacter");
}

#[test]
fn reports_no_control_regex_for_range_in_character_class() {
    let source = "const r = /[\\x00-\\x1f]/;";
    let diagnostics = scan("no-control-regex", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn does_not_report_no_control_regex_for_named_escape_tab() {
    let source = "const r = /\\t/;";
    let diagnostics = scan("no-control-regex", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_control_regex_for_hex_above_control_range() {
    let source = "const r = /\\x20/;";
    let diagnostics = scan("no-control-regex", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_single_char_in_character_classes_for_one_char_class() {
    let source = "const r = /[a]/;";
    let diagnostics = scan("single-char-in-character-classes", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "singleCharInCharacterClass");
}

#[test]
fn reports_single_char_in_character_classes_for_dot_in_class() {
    let source = "const r = /[.]/;";
    let diagnostics = scan("single-char-in-character-classes", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_single_char_in_character_classes_for_two_chars() {
    let source = "const r = /[ab]/;";
    let diagnostics = scan("single-char-in-character-classes", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_char_in_character_classes_for_range() {
    let source = "const r = /[a-z]/;";
    let diagnostics = scan("single-char-in-character-classes", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_char_in_character_classes_for_negated_class() {
    let source = "const r = /[^a]/;";
    let diagnostics = scan("single-char-in-character-classes", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_duplicates_in_character_class_for_repeated_char() {
    let source = "const r = /[aa]/;";
    let diagnostics = scan("duplicates-in-character-class", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "duplicates-in-character-class");
    assert_eq!(diagnostics[0].message_id, "duplicateCharacter");
}

#[test]
fn reports_duplicates_in_character_class_for_non_adjacent_repeat() {
    let source = "const r = /[abca]/;";
    let diagnostics = scan("duplicates-in-character-class", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_duplicates_in_character_class_for_distinct_chars() {
    let source = "const r = /[abc]/;";
    let diagnostics = scan("duplicates-in-character-class", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_duplicates_in_character_class_for_range() {
    let source = "const r = /[a-z]/;";
    let diagnostics = scan("duplicates-in-character-class", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_anchor_precedence_for_caret_on_first_of_three_alts() {
    let source = "const r = /^a|b|c$/;";
    let diagnostics = scan("anchor-precedence", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "anchor-precedence");
    assert_eq!(diagnostics[0].message_id, "anchorPrecedence");
}

#[test]
fn reports_anchor_precedence_for_caret_only_on_first_of_two_alts() {
    let source = "const r = /^a|b/;";
    let diagnostics = scan("anchor-precedence", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "anchorPrecedence");
}

#[test]
fn reports_anchor_precedence_for_dollar_only_on_last_of_two_alts() {
    let source = "const r = /a|b$/;";
    let diagnostics = scan("anchor-precedence", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "anchorPrecedence");
}

#[test]
fn does_not_report_anchor_precedence_for_grouped_alts() {
    let source = "const r = /^(a|b|c)$/;";
    let diagnostics = scan("anchor-precedence", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_anchor_precedence_for_no_anchors() {
    let source = "const r = /a|b|c/;";
    let diagnostics = scan("anchor-precedence", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_anchor_precedence_for_trim_idiom() {
    let source = "const r = /^\\s+|\\s+$/;";
    let diagnostics = scan("anchor-precedence", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_anchor_precedence_when_all_branches_fully_anchored() {
    let source = "const r = /^a$|^b$|^c$/;";
    let diagnostics = scan("anchor-precedence", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_anchor_precedence_when_middle_alt_is_anchored() {
    let source = "const r = /^a|^b|c$/;";
    let diagnostics = scan("anchor-precedence", source);
    assert!(diagnostics.is_empty());
}

// --- cyclomatic-complexity (S1541) ---

#[test]
fn cyclomatic_complexity_exceeds_threshold_reports() {
    // base 1 + 4 if statements = 5, threshold 3: 5 > 3 → 1 diagnostic
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 3;
    let source = "function f(a,b,c,d){if(a){}if(b){}if(c){}if(d){}}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "cyclomatic-complexity");
    assert_eq!(diagnostics[0].message_id, "cyclomaticComplexity");
}

#[test]
fn cyclomatic_complexity_at_threshold_no_report() {
    // base 1 + 3 ifs = 4, threshold 4: 4 is NOT > 4 → 0 diagnostics
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 4;
    let source = "function f(a,b,c){if(a){}if(b){}if(c){}}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn cyclomatic_complexity_logical_operators_count() {
    // "a&&b&&c" → 2 LogicalExpression nodes; base 1 + 2 = 3 > threshold 2 → 1 diagnostic
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 2;
    let source = "function f(a,b,c){return a&&b&&c;}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "cyclomatic-complexity");
}

#[test]
fn cyclomatic_complexity_default_case_not_counted() {
    // switch with only a default clause: base 1 + 0 case clauses = 1, threshold 1: not > 1 → 0
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 1;
    let source = "function f(x){switch(x){default:break;}}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn cyclomatic_complexity_toplevel_decision_points_not_counted() {
    // ifs at top level (no enclosing function) → no frame → complexity never reported
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 1;
    let source = "if(a){}if(b){}if(c){}if(d){}if(e){}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert!(diagnostics.is_empty());
}

#[test]
fn cyclomatic_complexity_case_clause_counts() {
    // switch with 2 case clauses + 1 default; base 1 + 2 cases = 3 > threshold 2 → 1 diagnostic
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 2;
    let source = "function f(x){switch(x){case 1:break;case 2:break;default:break;}}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn cyclomatic_complexity_catch_clause_counts() {
    // base 1 + 1 catch = 2 > threshold 1 → 1 diagnostic
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 1;
    let source = "function f(){try{}catch(e){}}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn cyclomatic_complexity_nested_functions_independent() {
    // outer: base 1 (no decision points in outer body itself) → not reported at threshold 1
    // inner: base 1 + 2 ifs = 3 > threshold 1 → inner reported; outer not reported
    let mut options = options_for("cyclomatic-complexity");
    options.cyclomatic_complexity_threshold = 1;
    let source = "function outer(){function inner(a,b){if(a){}if(b){}}}";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "cyclomaticComplexity");
}

#[test]
fn cyclomatic_complexity_uses_default_threshold_when_unset() {
    // default threshold is 10; a function with base 1 + 5 ifs = 6 must not be flagged
    let source = "function f(a,b,c,d,e){if(a){}if(b){}if(c){}if(d){}if(e){}}";
    let diagnostics = scan("cyclomatic-complexity", source);
    assert!(diagnostics.is_empty());
}

// no-collection-size-mischeck

#[test]
fn collection_size_mischeck_length_less_than_zero() {
    let source = "const b = x.length < 0;";
    let diagnostics = scan("no-collection-size-mischeck", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-collection-size-mischeck");
    assert_eq!(diagnostics[0].message_id, "collectionSizeMischeck");
}

#[test]
fn collection_size_mischeck_length_gte_zero() {
    let source = "const b = x.length >= 0;";
    let diagnostics = scan("no-collection-size-mischeck", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-collection-size-mischeck");
    assert_eq!(diagnostics[0].message_id, "collectionSizeMischeck");
}

#[test]
fn collection_size_mischeck_size_mirrored_gt() {
    let source = "const b = 0 > arr.size;";
    let diagnostics = scan("no-collection-size-mischeck", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-collection-size-mischeck");
    assert_eq!(diagnostics[0].message_id, "collectionSizeMischeck");
}

#[test]
fn collection_size_mischeck_no_report_gt_zero() {
    let source = "const b = x.length > 0;";
    let diagnostics = scan("no-collection-size-mischeck", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn collection_size_mischeck_no_report_strict_eq_zero() {
    let source = "const b = x.length === 0;";
    let diagnostics = scan("no-collection-size-mischeck", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn collection_size_mischeck_no_report_lte_zero() {
    let source = "const b = x.length <= 0;";
    let diagnostics = scan("no-collection-size-mischeck", source);
    assert!(diagnostics.is_empty());
}

// index-of-compare-to-positive-number

#[test]
fn index_of_compare_gt_zero() {
    let source = "const b = a.indexOf(x) > 0;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "index-of-compare-to-positive-number"
    );
    assert_eq!(diagnostics[0].message_id, "indexOfPositive");
}

#[test]
fn index_of_compare_gt_positive_number() {
    let source = "const b = a.indexOf(x) > 2;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "index-of-compare-to-positive-number"
    );
    assert_eq!(diagnostics[0].message_id, "indexOfPositive");
}

#[test]
fn last_index_of_compare_gt_zero() {
    let source = "const b = a.lastIndexOf(x) > 0;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "index-of-compare-to-positive-number"
    );
    assert_eq!(diagnostics[0].message_id, "indexOfPositive");
}

#[test]
fn index_of_compare_reversed_lt() {
    let source = "const b = 0 < a.indexOf(x);";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "index-of-compare-to-positive-number"
    );
    assert_eq!(diagnostics[0].message_id, "indexOfPositive");
}

#[test]
fn index_of_compare_gte_one() {
    let source = "const b = a.indexOf(x) >= 1;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "index-of-compare-to-positive-number"
    );
    assert_eq!(diagnostics[0].message_id, "indexOfPositive");
}

#[test]
fn index_of_compare_gte_zero_no_report() {
    let source = "const b = a.indexOf(x) >= 0;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn index_of_compare_strict_eq_no_report() {
    let source = "const b = a.indexOf(x) === -1;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn index_of_compare_gt_neg_one_no_report() {
    let source = "const b = a.indexOf(x) > -1;";
    let diagnostics = scan("index-of-compare-to-positive-number", source);
    assert!(diagnostics.is_empty());
}

// --- no-nested-functions (S2004) ---

#[test]
fn no_nested_functions_reports_depth_five_with_default_threshold() {
    // Default threshold is 4; depth 5 is the first flagged level.
    let source = concat!(
        "function a() {",
        "function b() {",
        "function c() {",
        "function d() {",
        "function e() {}",
        "}}}}",
    );
    let diagnostics = scan("no-nested-functions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-functions");
    assert_eq!(diagnostics[0].message_id, "noNestedFunctions");
}

#[test]
fn no_nested_functions_does_not_report_at_exactly_threshold_depth() {
    // Exactly 4 levels deep — at threshold, not exceeding it.
    let source = concat!(
        "function a() {",
        "function b() {",
        "function c() {",
        "function d() {}",
        "}}}",
    );
    let diagnostics = scan("no-nested-functions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_nested_functions_arrow_functions_count_toward_depth() {
    // Arrow functions count the same as regular functions.
    let source = concat!(
        "const a = () => {",
        "const b = () => {",
        "const c = () => {",
        "const d = () => {",
        "const e = () => {};",
        "}}}};",
    );
    let diagnostics = scan("no-nested-functions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-functions");
    assert_eq!(diagnostics[0].message_id, "noNestedFunctions");
}

#[test]
fn no_nested_functions_mixed_kinds_count_toward_depth() {
    // Mixing function declarations and arrow functions still tracks depth correctly.
    let source = concat!(
        "function a() {",
        "const b = () => {",
        "function c() {",
        "const d = () => {",
        "function e() {}",
        "}}}};",
    );
    let diagnostics = scan("no-nested-functions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-functions");
}

#[test]
fn no_nested_functions_respects_custom_threshold() {
    // With threshold 2, a function at depth 3 is the first flagged.
    let mut options = options_for("no-nested-functions");
    options.no_nested_functions_threshold = 2;
    let source = concat!("function a() {", "function b() {", "function c() {}", "}}",);
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-nested-functions");
    assert_eq!(diagnostics[0].message_id, "noNestedFunctions");
}

#[test]
fn no_nested_functions_sibling_functions_do_not_accumulate() {
    // Two sibling functions at depth 2 must each be at depth 2, not accumulate.
    let source = concat!(
        "function outer() {",
        "function sibling_a() {}",
        "function sibling_b() {}",
        "}",
    );
    let diagnostics = scan("no-nested-functions", source);
    assert!(diagnostics.is_empty());
}

// --- too-many-break-or-continue-in-loop tests ---

#[test]
fn too_many_break_two_breaks_in_while_flagged() {
    // Two breaks targeting the while loop → flagged once at the loop span.
    let source = "while (a) { if (b) break; if (c) break; }";
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "too-many-break-or-continue-in-loop"
    );
    assert_eq!(diagnostics[0].message_id, "tooManyBreakContinue");
}

#[test]
fn too_many_break_one_break_one_continue_flagged() {
    // One break plus one continue in the same loop → count = 2 → flagged.
    let source = "for (;;) { if (a) break; if (b) continue; }";
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "tooManyBreakContinue");
}

#[test]
fn too_many_break_single_break_not_flagged() {
    // Only one jump → not flagged.
    let source = "while (a) { if (b) break; }";
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn too_many_break_single_continue_not_flagged() {
    // Only one continue → not flagged.
    let source = "for (let i = 0; i < 10; i++) { if (a) continue; }";
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn too_many_break_zero_jumps_not_flagged() {
    // No break or continue → not flagged.
    let source = "while (a) { doWork(); }";
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn too_many_break_two_breaks_inside_nested_switch_not_flagged() {
    // Both breaks target the inner switch → loop count = 0 → not flagged.
    let source = concat!(
        "while (a) {",
        "  switch (x) {",
        "    case 1: break;",
        "    case 2: break;",
        "  }",
        "}",
    );
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn too_many_break_one_own_break_plus_switch_break_not_flagged() {
    // One break targets the loop, one targets the nested switch → loop count = 1 → not flagged.
    let source = concat!(
        "while (a) {",
        "  switch (x) { case 1: break; }",
        "  if (b) break;",
        "}",
    );
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn too_many_break_continue_inside_switch_inside_loop_flagged() {
    // An unlabeled continue skips the switch frame and targets the loop.
    // With two such continues (one direct, one via switch) the loop count >= 2 → flagged.
    let source = concat!(
        "while (a) {",
        "  if (c) continue;",
        "  switch (x) { case 1: continue; }",
        "}",
    );
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "tooManyBreakContinue");
}

#[test]
fn too_many_break_nested_loops_inner_flagged_outer_not() {
    // Inner loop has two breaks → flagged. Outer loop has one break → not flagged.
    let source = concat!(
        "for (let i = 0; i < 10; i++) {",
        "  for (let j = 0; j < 10; j++) {",
        "    if (a) break;",
        "    if (b) break;",
        "  }",
        "  if (c) break;",
        "}",
    );
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert_eq!(diagnostics.len(), 1);
    // The diagnostic is on the inner loop, which starts later in the source.
    assert!(diagnostics[0].loc.start_column > 0 || diagnostics[0].loc.start_line > 1);
}

#[test]
fn too_many_break_inner_loop_break_does_not_count_for_outer() {
    // An unlabeled break inside the inner loop targets the inner loop, not the outer.
    // The outer loop therefore has zero jumps → not flagged.
    let source = concat!("while (a) {", "  while (b) { if (c) break; }", "}",);
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn too_many_break_labeled_jump_targets_outer_loop() {
    // `continue outer` and `break outer` both target the outer loop → count 2 → flagged.
    let source = concat!(
        "outer: for (;;) {",
        "  for (;;) {",
        "    if (a) continue outer;",
        "    if (b) break outer;",
        "  }",
        "}",
    );
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    // Outer loop should be flagged; inner loop has zero of its own jumps.
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message_id == "tooManyBreakContinue")
    );
    let outer_reports = diagnostics
        .iter()
        .filter(|d| d.message_id == "tooManyBreakContinue")
        .count();
    assert_eq!(outer_reports, 1);
}

#[test]
fn too_many_break_sibling_loops_each_with_one_break_not_flagged() {
    // Each sibling loop has exactly one break → neither is flagged.
    let source = concat!("while (a) { if (b) break; }", "while (c) { if (d) break; }",);
    let diagnostics = scan("too-many-break-or-continue-in-loop", source);
    assert!(diagnostics.is_empty());
}

// code-eval tests

#[test]
fn reports_code_eval_for_bare_eval_call() {
    let source = r#"eval("x + 1");"#;
    let diagnostics = scan("code-eval", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "code-eval");
    assert_eq!(diagnostics[0].message_id, "codeEval");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_code_eval_for_new_function_constructor() {
    let source = r#"const f = new Function("a", "return a");"#;
    let diagnostics = scan("code-eval", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "code-eval");
    assert_eq!(diagnostics[0].message_id, "codeEval");
}

#[test]
fn reports_code_eval_for_function_call_without_new() {
    let source = r#"const f = Function("return 42");"#;
    let diagnostics = scan("code-eval", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "codeEval");
}

#[test]
fn does_not_report_code_eval_for_member_access_eval() {
    let source = r#"window.eval("x");"#;
    let diagnostics = scan("code-eval", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_code_eval_for_member_access_eval_foo() {
    let source = r#"foo.eval(x);"#;
    let diagnostics = scan("code-eval", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_code_eval_for_unrelated_call() {
    let source = r#"foo("x");"#;
    let diagnostics = scan("code-eval", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_code_eval_for_function_declaration() {
    let source = "function eval() {}";
    let diagnostics = scan("code-eval", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_code_eval_for_eval_with_variable_argument() {
    let source = "eval(userInput);";
    let diagnostics = scan("code-eval", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "codeEval");
}

#[test]
fn reports_void_use_for_function_call() {
    let diagnostics = scan("void-use", "void foo();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "void-use");
    assert_eq!(diagnostics[0].message_id, "voidUse");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_void_use_for_variable() {
    let diagnostics = scan("void-use", "void x;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "void-use");
    assert_eq!(diagnostics[0].message_id, "voidUse");
}

#[test]
fn reports_void_use_for_nonzero_numeric_literal() {
    let diagnostics = scan("void-use", "void 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "voidUse");
}

#[test]
fn does_not_report_void_use_for_void_zero() {
    let diagnostics = scan("void-use", "void 0;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_void_use_for_void_parenthesised_zero() {
    let diagnostics = scan("void-use", "void (0);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_void_use_for_logical_not() {
    let diagnostics = scan("void-use", "const b = !x;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_void_use_for_typeof() {
    let diagnostics = scan("void-use", "typeof x;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_void_use_for_unary_minus() {
    let diagnostics = scan("void-use", "const n = -x;");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_prefer_promise_shorthand_arrow_expression_resolve() {
    let source = "const p = new Promise((resolve) => resolve(42));";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-promise-shorthand");
    assert_eq!(diagnostics[0].message_id, "preferShorthand");
}

#[test]
fn reports_prefer_promise_shorthand_arrow_expression_resolve_no_arg() {
    let source = "const p = new Promise((resolve) => resolve());";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferShorthand");
}

#[test]
fn reports_prefer_promise_shorthand_two_params_reject() {
    let source = "const p = new Promise((resolve, reject) => reject(err));";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferShorthand");
}

#[test]
fn reports_prefer_promise_shorthand_function_expression() {
    let source = "const p = new Promise(function (resolve) { resolve(1); });";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferShorthand");
}

#[test]
fn reports_prefer_promise_shorthand_arrow_block_body() {
    let source = "const p = new Promise((resolve) => { resolve(1); });";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferShorthand");
}

#[test]
fn reports_prefer_promise_shorthand_arrow_block_body_return() {
    let source = "const p = new Promise((resolve) => { return resolve(1); });";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferShorthand");
}

#[test]
fn does_not_report_prefer_promise_shorthand_multiple_statements() {
    let source = "const p = new Promise((resolve, reject) => { doStuff(); resolve(1); });";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_promise_shorthand_call_not_resolve() {
    let source = "const p = new Promise((resolve) => setTimeout(resolve, 100));";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_promise_shorthand_two_args_to_resolve() {
    let source = "const p = new Promise((resolve) => resolve(1, 2));";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_promise_shorthand_executor_is_identifier() {
    let source = "const p = new Promise(executor);";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_prefer_promise_shorthand_arg_is_other_param() {
    let source = "const p = new Promise((resolve, reject) => resolve(reject));";
    let diagnostics = scan("prefer-promise-shorthand", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_pseudo_random_for_math_random_call() {
    let diagnostics = scan("pseudo-random", "const x = Math.random();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "pseudo-random");
    assert_eq!(diagnostics[0].message_id, "pseudoRandom");
}

#[test]
fn reports_pseudo_random_for_bare_math_random_call() {
    let diagnostics = scan("pseudo-random", "Math.random();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "pseudoRandom");
}

#[test]
fn does_not_report_pseudo_random_for_different_property() {
    let diagnostics = scan("pseudo-random", "Math.floor(1.5);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_pseudo_random_for_different_object() {
    let diagnostics = scan("pseudo-random", "foo.random();");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_pseudo_random_for_bare_identifier_call() {
    let diagnostics = scan("pseudo-random", "random();");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_pseudo_random_for_bare_reference() {
    let diagnostics = scan("pseudo-random", "const f = Math.random;");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_process_argv_for_direct_access() {
    let source = "const a = process.argv;";
    let diagnostics = scan("process-argv", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "process-argv");
    assert_eq!(diagnostics[0].message_id, "processArgv");
}

#[test]
fn reports_process_argv_for_indexed_access() {
    let source = "process.argv[2];";
    let diagnostics = scan("process-argv", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "processArgv");
}

#[test]
fn reports_process_argv_for_slice_access() {
    let source = "process.argv.slice(2);";
    let diagnostics = scan("process-argv", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "processArgv");
}

#[test]
fn does_not_report_process_argv_for_different_property() {
    let source = "process.env.PATH;";
    let diagnostics = scan("process-argv", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_process_argv_for_different_object() {
    let source = "foo.argv;";
    let diagnostics = scan("process-argv", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_process_argv_for_bare_identifier() {
    let source = "argv;";
    let diagnostics = scan("process-argv", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_standard_input_for_direct_access() {
    let source = "const x = process.stdin;";
    let diagnostics = scan("standard-input", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "standard-input");
    assert_eq!(diagnostics[0].message_id, "standardInput");
}

#[test]
fn reports_standard_input_for_on_call() {
    let source = "process.stdin.on('data', cb);";
    let diagnostics = scan("standard-input", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "standardInput");
}

#[test]
fn does_not_report_standard_input_for_different_property() {
    let source = "process.stdout;";
    let diagnostics = scan("standard-input", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_standard_input_for_different_object() {
    let source = "foo.stdin;";
    let diagnostics = scan("standard-input", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_standard_input_for_bare_identifier() {
    let source = "stdin;";
    let diagnostics = scan("standard-input", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_hardcoded_ip_for_private_ipv4() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "192.168.1.1";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-hardcoded-ip");
    assert_eq!(diagnostics[0].message_id, "hardcodedIp");
}

#[test]
fn reports_no_hardcoded_ip_for_class_a_private_ipv4() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "10.0.0.1";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedIp");
}

#[test]
fn reports_no_hardcoded_ip_for_ipv4_in_url_string() {
    let diagnostics = scan("no-hardcoded-ip", r#"const u = "http://10.20.30.40/api";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedIp");
}

#[test]
fn reports_no_hardcoded_ip_for_non_documentation_ipv6() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "fe80::1";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedIp");
}

#[test]
fn reports_no_hardcoded_ip_for_full_ipv6_address() {
    let source = r#"const ip = "2001:0001:85a3::8a2e:370:7334";"#;
    let diagnostics = scan("no-hardcoded-ip", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedIp");
}

#[test]
fn does_not_report_no_hardcoded_ip_for_loopback_127_0_0_1() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "127.0.0.1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_loopback_127_x_x_x_range() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "127.1.2.3";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_broadcast() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "255.255.255.255";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_unspecified_0_0_0_0() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "0.0.0.0";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_ipv6_loopback() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "::1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_ipv6_documentation_range() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "2001:db8::1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_ipv4_mapped_loopback() {
    let diagnostics = scan("no-hardcoded-ip", r#"const ip = "::ffff:127.0.0.1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_plain_string() {
    let diagnostics = scan("no-hardcoded-ip", r#"const s = "hello world";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_partial_ipv4() {
    let diagnostics = scan("no-hardcoded-ip", r#"const s = "192.168.1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_for_version_string_with_invalid_octet() {
    let diagnostics = scan("no-hardcoded-ip", r#"const v = "256.0.0.1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_hardcoded_ip_when_rule_not_enabled() {
    let diagnostics = scan(
        "no-nested-template-literals",
        r#"const ip = "192.168.1.1";"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_global_this_at_top_level() {
    let diagnostics = scan("no-global-this", "this.foo = 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-global-this");
    assert_eq!(diagnostics[0].message_id, "noGlobalThis");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_global_this_inside_top_level_arrow() {
    let diagnostics = scan("no-global-this", "const f = () => this.x;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-global-this");
    assert_eq!(diagnostics[0].message_id, "noGlobalThis");
}

#[test]
fn reports_no_global_this_inside_nested_top_level_arrows() {
    let diagnostics = scan("no-global-this", "const f = () => () => this;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noGlobalThis");
}

#[test]
fn does_not_report_no_global_this_inside_regular_function() {
    let diagnostics = scan("no-global-this", "function f() { return this.x; }");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_global_this_inside_object_method() {
    let diagnostics = scan("no-global-this", "const o = { m() { return this.x; } };");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_global_this_inside_class_method() {
    let diagnostics = scan("no-global-this", "class C { m() { return this.x; } }");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_global_this_inside_class_field_initializer() {
    let diagnostics = scan("no-global-this", "class C { x = this.y; }");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_global_this_inside_static_block() {
    let diagnostics = scan("no-global-this", "class C { static { this.z(); } }");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_global_this_for_arrow_inside_function() {
    let diagnostics = scan("no-global-this", "function f() { const g = () => this.x; }");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_single_character_alternation_simple() {
    let diagnostics = scan("single-character-alternation", "const re = /a|b|c/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "single-character-alternation");
    assert_eq!(diagnostics[0].message_id, "singleCharAlternation");
}

#[test]
fn reports_single_character_alternation_two_alternatives() {
    let diagnostics = scan("single-character-alternation", "const re = /x|y/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "singleCharAlternation");
}

#[test]
fn reports_single_character_alternation_inside_capturing_group() {
    let diagnostics = scan("single-character-alternation", "const re = /(a|b|c)/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "singleCharAlternation");
}

#[test]
fn reports_single_character_alternation_nested_group() {
    let diagnostics = scan("single-character-alternation", "const re = /x(1|2|3)y/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "singleCharAlternation");
}

#[test]
fn reports_single_character_alternation_escaped_chars() {
    let diagnostics = scan("single-character-alternation", "const re = /\\.|,/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "singleCharAlternation");
}

#[test]
fn does_not_report_single_character_alternation_multi_char_alt() {
    let diagnostics = scan("single-character-alternation", "const re = /ab|c/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_character_alternation_multi_char_alt2() {
    let diagnostics = scan("single-character-alternation", "const re = /a|bc/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_character_alternation_class_escape() {
    let diagnostics = scan("single-character-alternation", "const re = /\\d|x/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_character_alternation_no_disjunction() {
    let diagnostics = scan("single-character-alternation", "const re = /abc/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_character_alternation_empty_alt() {
    let diagnostics = scan("single-character-alternation", "const re = /a|/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_character_alternation_quantified_term() {
    let diagnostics = scan("single-character-alternation", "const re = /a+|b/;");
    assert!(diagnostics.is_empty());
}

// empty-string-repetition tests

#[test]
fn reports_empty_string_repetition_star_on_star_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /(a*)*/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "empty-string-repetition");
    assert_eq!(diagnostics[0].message_id, "emptyStringRepetition");
}

#[test]
fn reports_empty_string_repetition_plus_on_optional_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /(a?)+/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyStringRepetition");
}

#[test]
fn reports_empty_string_repetition_star_on_empty_ignore_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /(?:)*/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyStringRepetition");
}

#[test]
fn reports_empty_string_repetition_plus_on_empty_capturing_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /()+/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyStringRepetition");
}

#[test]
fn reports_empty_string_repetition_star_on_disjunction_with_empty_alt() {
    let diagnostics = scan("empty-string-repetition", "const re = /(?:|a)*/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyStringRepetition");
}

#[test]
fn reports_empty_string_repetition_plus_on_group_with_star() {
    let diagnostics = scan("empty-string-repetition", "const re = /(a*)+/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyStringRepetition");
}

#[test]
fn does_not_report_empty_string_repetition_star_on_literal() {
    let diagnostics = scan("empty-string-repetition", "const re = /a*/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_empty_string_repetition_star_on_nonempty_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /(a+)*/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_empty_string_repetition_optional_on_literal() {
    let diagnostics = scan("empty-string-repetition", "const re = /a?/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_empty_string_repetition_optional_on_empty_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /(?:)?/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_empty_string_repetition_plus_on_multichar_group() {
    let diagnostics = scan("empty-string-repetition", "const re = /(abc)+/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_empty_string_repetition_plus_on_char_class() {
    let diagnostics = scan("empty-string-repetition", "const re = /[a-z]+/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_misleading_array_reverse_assigned_from_array_variable() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "const a = [1, 2, 3];\nconst b = a.reverse();",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-misleading-array-reverse");
    assert_eq!(diagnostics[0].message_id, "misleadingReverse");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn reports_misleading_array_sort_assigned_from_array_variable() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "const a = [3, 1, 2];\nlet b;\nb = a.sort();",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "misleadingReverse");
}

#[test]
fn does_not_report_misleading_array_reverse_bare_statement() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "const a = [1, 2, 3];\na.reverse();",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misleading_array_reverse_on_fresh_array_literal() {
    let diagnostics = scan("no-misleading-array-reverse", "const c = [1, 2].reverse();");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misleading_array_reverse_on_spread_copy() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "const a = [1, 2, 3];\nconst b = [...a].reverse();",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misleading_array_reverse_on_function_parameter() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "function f(a) {\n  const b = a.reverse();\n  return b;\n}",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misleading_array_reverse_on_non_array_variable() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "const obj = { reverse() {} };\nconst b = obj.reverse();",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misleading_array_reverse_when_variable_reassigned() {
    let diagnostics = scan(
        "no-misleading-array-reverse",
        "let a = [1, 2, 3];\na = a.reverse();",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_alphabetical_sort_on_array_literal() {
    let diagnostics = scan("no-alphabetical-sort", "[3, 1, 2].sort();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-alphabetical-sort");
    assert_eq!(diagnostics[0].message_id, "provideCompareFunction");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_alphabetical_sort_on_resolved_array_variable() {
    let diagnostics = scan("no-alphabetical-sort", "const a = [3, 1, 2];\na.sort();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "provideCompareFunction");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn reports_alphabetical_to_sorted_on_array_literal() {
    let diagnostics = scan("no-alphabetical-sort", "[3, 1, 2].toSorted();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "provideCompareFunction");
}

#[test]
fn does_not_report_alphabetical_sort_with_compare_function() {
    let diagnostics = scan("no-alphabetical-sort", "[3, 1, 2].sort((x, y) => x - y);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_alphabetical_sort_on_non_array_receiver() {
    let diagnostics = scan(
        "no-alphabetical-sort",
        "const obj = { sort() {} };\nobj.sort();",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_alphabetical_sort_on_unresolvable_receiver() {
    let diagnostics = scan("no-alphabetical-sort", "function f(a) {\n  a.sort();\n}");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_alphabetical_sort_on_non_sort_call() {
    let diagnostics = scan("no-alphabetical-sort", "[3, 1, 2].map((x) => x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_alphabetical_sort_on_string_literal_array() {
    // Alphabetical order is the expected sort for strings, so a string-only
    // array literal is exempt (matches the type-aware upstream behaviour).
    let diagnostics = scan("no-alphabetical-sort", "['b', 'a', 'c'].sort();");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_alphabetical_sort_on_mixed_string_and_number_array() {
    // A non-string element keeps the missing comparator a likely defect.
    let diagnostics = scan("no-alphabetical-sort", "['b', 1].sort();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "provideCompareFunction");
}

#[test]
fn reports_for_in_iterable_on_array_literal() {
    let diagnostics = scan("no-for-in-iterable", "for (const i in [1, 2, 3]) {}");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-for-in-iterable");
    assert_eq!(diagnostics[0].message_id, "noForInIterable");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_for_in_iterable_on_resolved_array_variable() {
    let diagnostics = scan(
        "no-for-in-iterable",
        "const a = [1, 2, 3];\nfor (const i in a) {}",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noForInIterable");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn does_not_report_for_in_iterable_on_object_literal() {
    let diagnostics = scan(
        "no-for-in-iterable",
        "const obj = { a: 1 };\nfor (const k in obj) {}",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_in_iterable_on_unresolvable_parameter() {
    let diagnostics = scan(
        "no-for-in-iterable",
        "function f(p) {\n  for (const k in p) {}\n}",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_in_iterable_on_for_of_loop() {
    let diagnostics = scan("no-for-in-iterable", "for (const x of [1, 2, 3]) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_reduce_initial_value_on_array_literal() {
    let source = "[1, 2, 3].reduce((a, b) => a + b);";
    let diagnostics = scan("reduce-initial-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "reduce-initial-value");
    assert_eq!(diagnostics[0].message_id, "provideInitialValue");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_reduce_initial_value_on_resolved_array_variable() {
    let source = "const a = [1, 2];\na.reduce(fn);";
    let diagnostics = scan("reduce-initial-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "provideInitialValue");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn reports_reduce_right_initial_value_on_array_literal() {
    let source = "[1, 2, 3].reduceRight((a, b) => a + b);";
    let diagnostics = scan("reduce-initial-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "provideInitialValue");
}

#[test]
fn does_not_report_reduce_initial_value_with_initial_value() {
    let source = "[1, 2].reduce((a, b) => a + b, 0);";
    let diagnostics = scan("reduce-initial-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_reduce_initial_value_on_non_array_receiver() {
    let source = "const obj = { reduce() {} };\nobj.reduce(fn);";
    let diagnostics = scan("reduce-initial-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_reduce_initial_value_on_spread_argument() {
    let source = "[1, 2, 3].reduce(...args);";
    let diagnostics = scan("reduce-initial-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_reduce_initial_value_on_non_reduce_call() {
    let source = "foo.bar();";
    let diagnostics = scan("reduce-initial-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_associative_array_computed_string_key() {
    let diagnostics = scan("no-associative-arrays", "const a = [];\na['key'] = 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-associative-arrays");
    assert_eq!(diagnostics[0].message_id, "noAssociativeArray");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn reports_associative_array_static_key() {
    let diagnostics = scan("no-associative-arrays", "const a = [];\na.foo = 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noAssociativeArray");
}

#[test]
fn reports_associative_array_on_direct_array_literal() {
    let diagnostics = scan("no-associative-arrays", "[].foo = 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noAssociativeArray");
}

#[test]
fn reports_associative_array_for_compound_assignment() {
    let diagnostics = scan("no-associative-arrays", "const a = [];\na.foo += 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noAssociativeArray");
}

#[test]
fn does_not_report_associative_array_numeric_index() {
    let diagnostics = scan("no-associative-arrays", "const a = [];\na[0] = 1;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_associative_array_numeric_string_index() {
    let diagnostics = scan("no-associative-arrays", "const a = [];\na['0'] = 1;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_associative_array_length_write() {
    let diagnostics = scan("no-associative-arrays", "const a = [];\na.length = 0;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_associative_array_variable_index() {
    let diagnostics = scan(
        "no-associative-arrays",
        "const a = [];\nlet i = 0;\na[i] = 1;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_associative_array_on_object_literal() {
    let diagnostics = scan("no-associative-arrays", "const o = {};\no.foo = 1;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_associative_array_on_unresolvable_parameter() {
    let diagnostics = scan("no-associative-arrays", "function f(p) {\n  p.foo = 1;\n}");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_bitwise_and_with_comparison_operands() {
    let diagnostics = scan("bitwise-operators", "if (a < 1 & b > 2) {\n}");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "bitwise-operators");
    assert_eq!(diagnostics[0].message_id, "bitwiseOperator");
}

#[test]
fn reports_bitwise_or_with_one_equality_operand() {
    let diagnostics = scan("bitwise-operators", "const x = (a === b) | c;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "bitwiseOperator");
}

#[test]
fn reports_bitwise_and_with_logical_not_operand() {
    let diagnostics = scan("bitwise-operators", "const x = !a & b;");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_bitwise_or_with_logical_operand() {
    let diagnostics = scan("bitwise-operators", "const x = (a && b) | c;");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_bitwise_and_with_boolean_literal_operand() {
    let diagnostics = scan("bitwise-operators", "const x = a & true;");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_bitwise_and_on_numeric_operands() {
    let diagnostics = scan("bitwise-operators", "const y = flags & MASK;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_bitwise_or_on_identifiers() {
    let diagnostics = scan("bitwise-operators", "const y = a | b;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_logical_and() {
    let diagnostics = scan("bitwise-operators", "if (a < 1 && b > 2) {\n}");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_bitwise_xor_with_comparison_operand() {
    let diagnostics = scan("bitwise-operators", "const z = (a === b) ^ c;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_compound_bitwise_assignment() {
    let diagnostics = scan("bitwise-operators", "let a = 0;\na &= b;");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_same_argument_assert_equal() {
    let diagnostics = scan("no-same-argument-assert", "assert.equal(x, x);");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_same_argument_assert_strict_equal_member() {
    let diagnostics = scan(
        "no-same-argument-assert",
        "assert.strictEqual(foo.bar, foo.bar);",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_same_argument_assert_with_different_args() {
    let diagnostics = scan("no-same-argument-assert", "assert.equal(x, y);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_same_argument_assert_for_non_assert_call() {
    let diagnostics = scan("no-same-argument-assert", "foo(x, x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_same_argument_assert_with_single_argument() {
    let diagnostics = scan("no-same-argument-assert", "assert.ok(x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_inverted_assertion_arguments_numeric_first() {
    let diagnostics = scan("inverted-assertion-arguments", "assert.equal(42, x);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "inverted-assertion-arguments");
    assert_eq!(diagnostics[0].message_id, "invertedArguments");
}

#[test]
fn reports_inverted_assertion_arguments_string_first() {
    let diagnostics = scan(
        "inverted-assertion-arguments",
        "assert.strictEqual('foo', bar);",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "invertedArguments");
}

#[test]
fn does_not_report_inverted_assertion_arguments_in_correct_order() {
    let diagnostics = scan("inverted-assertion-arguments", "assert.equal(x, 42);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_assertion_arguments_both_literals() {
    let diagnostics = scan("inverted-assertion-arguments", "assert.equal(1, 2);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_assertion_arguments_neither_literal() {
    let diagnostics = scan("inverted-assertion-arguments", "assert.equal(x, y);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_assertion_arguments_for_non_assert_call() {
    let diagnostics = scan("inverted-assertion-arguments", "foo(42, x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_assertion_arguments_single_argument() {
    let diagnostics = scan("inverted-assertion-arguments", "assert.ok(x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inverted_assertion_arguments_for_non_equality_method() {
    // `include` is not an equality method: a literal first argument (the
    // haystack) is legitimate, so it must not be flagged.
    let diagnostics = scan(
        "inverted-assertion-arguments",
        "assert.include('foobar', x);",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_incomplete_assertions_bare_expect_call() {
    let diagnostics = scan("no-incomplete-assertions", "expect(x);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-incomplete-assertions");
    assert_eq!(diagnostics[0].message_id, "incompleteAssertion");
}

#[test]
fn reports_no_incomplete_assertions_expect_dot_to() {
    let diagnostics = scan("no-incomplete-assertions", "expect(x).to;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "incompleteAssertion");
}

#[test]
fn reports_no_incomplete_assertions_expect_to_be() {
    let diagnostics = scan("no-incomplete-assertions", "expect(x).to.be;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "incompleteAssertion");
}

#[test]
fn reports_no_incomplete_assertions_expect_not() {
    let diagnostics = scan("no-incomplete-assertions", "expect(x).not;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "incompleteAssertion");
}

#[test]
fn does_not_report_no_incomplete_assertions_expect_true_terminal() {
    let diagnostics = scan("no-incomplete-assertions", "expect(x).to.be.true;");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_incomplete_assertions_expect_equal_call() {
    let diagnostics = scan("no-incomplete-assertions", "expect(x).to.equal(42);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_incomplete_assertions_non_expect_call() {
    let diagnostics = scan("no-incomplete-assertions", "foo(x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_incomplete_assertions_namespaced_expect() {
    let diagnostics = scan("no-incomplete-assertions", "chai.expect(x).to;");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_for_loop_increment_sign_increasing_cond_decrements() {
    let source = "for (let i = 0; i < 10; i--) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "for-loop-increment-sign");
    assert_eq!(diagnostics[0].message_id, "wrongDirection");
}

#[test]
fn reports_for_loop_increment_sign_decreasing_cond_increments() {
    let source = "for (let i = 10; i > 0; i++) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "wrongDirection");
}

#[test]
fn reports_for_loop_increment_sign_compound_minus_assign() {
    let source = "for (let i = 0; i <= 10; i -= 1) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "wrongDirection");
}

#[test]
fn reports_for_loop_increment_sign_counter_on_right() {
    let source = "for (let i = 0; 10 > i; i--) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "wrongDirection");
}

#[test]
fn does_not_report_for_loop_increment_sign_correct_increasing() {
    let source = "for (let i = 0; i < 10; i++) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_loop_increment_sign_correct_decreasing() {
    let source = "for (let i = 10; i > 0; i--) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_loop_increment_sign_equality_condition() {
    let source = "for (let i = 0; i != 10; i++) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_loop_increment_sign_update_var_differs() {
    let source = "for (let i = 0, j = 0; i < 10; j++) {}";
    let diagnostics = scan("for-loop-increment-sign", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_for_loop_increment_sign_no_test_or_update() {
    let diagnostics = scan("for-loop-increment-sign", "for (;;) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_equals_in_for_termination_non_unit_compound_add() {
    let source = "for (let i = 0; i != 10; i += 2) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-equals-in-for-termination");
    assert_eq!(diagnostics[0].message_id, "noEqualsInForTermination");
}

#[test]
fn reports_no_equals_in_for_termination_non_unit_compound_subtract() {
    let source = "for (let i = 10; i !== 0; i -= 3) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noEqualsInForTermination");
}

#[test]
fn reports_no_equals_in_for_termination_non_unit_plain_assign() {
    let source = "for (let i = 0; i !== 10; i = i + 2) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noEqualsInForTermination");
}

#[test]
fn reports_no_equals_in_for_termination_counter_on_right() {
    let source = "for (let i = 0; 10 != i; i += 2) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_no_equals_in_for_termination_unit_increment() {
    let source = "for (let i = 0; i != 10; i++) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_equals_in_for_termination_unit_compound_add() {
    let source = "for (let i = 0; i !== 10; i += 1) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_equals_in_for_termination_relational_condition() {
    let source = "for (let i = 0; i < 10; i += 2) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_equals_in_for_termination_update_var_differs() {
    let source = "for (let i = 0, j = 0; i != 10; j += 2) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_equals_in_for_termination_no_update() {
    let source = "for (let i = 0; i != 10; ) {}";
    let diagnostics = scan("no-equals-in-for-termination", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_equals_in_for_termination_empty_header() {
    let diagnostics = scan("no-equals-in-for-termination", "for (;;) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_parameter_reassignment_plain_assign() {
    let source = "function f(p) { p = 1; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-parameter-reassignment");
    assert_eq!(diagnostics[0].message_id, "noParameterReassignment");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_parameter_reassignment_increment() {
    let source = "function f(p) { p++; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noParameterReassignment");
}

#[test]
fn reports_no_parameter_reassignment_arrow_compound() {
    let source = "const g = (a) => { a += 2; };";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noParameterReassignment");
}

#[test]
fn reports_no_parameter_reassignment_catch_clause() {
    let source = "try {} catch (e) { e = null; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noParameterReassignment");
}

#[test]
fn reports_no_parameter_reassignment_for_of_variable() {
    let source = "for (const x of xs) { x = 0; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noParameterReassignment");
}

#[test]
fn does_not_report_no_parameter_reassignment_property_write() {
    let source = "function f(p) { p.x = 1; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_parameter_reassignment_local_variable() {
    let source = "function f(p) { let q = p; q = 2; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_parameter_reassignment_module_scope_var() {
    let source = "let x = 1; x = 2;";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_parameter_reassignment_no_reassignment() {
    let source = "function f(p) { return p; }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_parameter_reassignment_classic_for_counter() {
    let source = "function f() { for (let i = 0; i < 3; i++) { i = 2; } }";
    let diagnostics = scan("no-parameter-reassignment", source);
    assert!(diagnostics.is_empty());
}

// updated-loop-counter tests

#[test]
fn reports_updated_loop_counter_plain_assign() {
    let source = "for (let i = 0; i < 10; i++) { i = 5; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "updated-loop-counter");
    assert_eq!(diagnostics[0].message_id, "noCounterUpdate");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_updated_loop_counter_compound_assign() {
    let source = "for (let i = 0; i < 10; i++) { i += 2; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noCounterUpdate");
}

#[test]
fn reports_updated_loop_counter_decrement_in_branch() {
    let source = "for (let i = 0; i < 10; i++) { if (x) i--; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noCounterUpdate");
}

#[test]
fn reports_updated_loop_counter_for_sequence_update() {
    let source = "for (let i = 0, j = 0; i < 10; i++, j++) { j = 5; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noCounterUpdate");
}

#[test]
fn does_not_report_updated_loop_counter_only_in_update_clause() {
    let source = "for (let i = 0; i < 10; i++) {}";
    let diagnostics = scan("updated-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_updated_loop_counter_shadowing_local() {
    let source = "for (let i = 0; i < 10; i++) { let i = 0; i = 5; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_updated_loop_counter_different_variable() {
    let source = "for (let i = 0; i < 10; i++) { j = 5; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_updated_loop_counter_property_write() {
    let source = "for (let i = 0; i < 10; i++) { i.x = 5; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_updated_loop_counter_for_of_variable() {
    let source = "for (const x of xs) { x = 1; }";
    let diagnostics = scan("updated-loop-counter", source);
    assert!(diagnostics.is_empty());
}

// array-callback-without-return tests

#[test]
fn reports_array_callback_without_return_map_function_no_return() {
    let source = "[1, 2].map(function (x) { console.log(x); });";
    let diagnostics = scan("array-callback-without-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "array-callback-without-return");
    assert_eq!(diagnostics[0].message_id, "addReturn");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_array_callback_without_return_filter_arrow_block_no_return() {
    let source = "arr.filter((x) => { doStuff(x); });";
    let diagnostics = scan("array-callback-without-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "addReturn");
}

#[test]
fn reports_array_callback_without_return_bare_return_does_not_count() {
    let source = "arr.map((x) => { if (x) { return; } });";
    let diagnostics = scan("array-callback-without-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "addReturn");
}

#[test]
fn reports_array_callback_without_return_ignores_nested_function_return() {
    let source = "arr.map((x) => { function inner() { return x; } });";
    let diagnostics = scan("array-callback-without-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "addReturn");
}

#[test]
fn does_not_report_array_callback_without_return_arrow_expression_body() {
    let source = "[1, 2].map((x) => x + 1);";
    let diagnostics = scan("array-callback-without-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_callback_without_return_function_with_return() {
    let source = "arr.filter(function (x) { return x > 0; });";
    let diagnostics = scan("array-callback-without-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_callback_without_return_value_return_in_control_flow() {
    let source = "arr.map((x) => { if (x) { return x; } });";
    let diagnostics = scan("array-callback-without-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_callback_without_return_value_return_in_try() {
    let source = "arr.map(function (x) { try { return x; } catch (e) {} });";
    let diagnostics = scan("array-callback-without-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_callback_without_return_for_each() {
    let source = "arr.forEach((x) => { log(x); });";
    let diagnostics = scan("array-callback-without-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_array_callback_without_return_identifier_callback() {
    let source = "arr.map(fn);";
    let diagnostics = scan("array-callback-without-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_wildcard_namespace_import() {
    let source = "import * as ns from 'mod';";
    let diagnostics = scan("no-wildcard-import", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noWildcardImport");
}

#[test]
fn reports_combined_default_and_wildcard_import() {
    let source = "import def, * as ns from 'mod';";
    let diagnostics = scan("no-wildcard-import", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noWildcardImport");
}

#[test]
fn does_not_report_named_import() {
    let source = "import { a, b } from 'mod';";
    let diagnostics = scan("no-wildcard-import", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_default_import() {
    let source = "import def from 'mod';";
    let diagnostics = scan("no-wildcard-import", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_side_effect_import() {
    let source = "import 'mod';";
    let diagnostics = scan("no-wildcard-import", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_export_all_reexport() {
    let source = "export * from 'mod';";
    let diagnostics = scan("no-wildcard-import", source);
    assert!(diagnostics.is_empty());
}

// misplaced-loop-counter tests

#[test]
fn reports_misplaced_loop_counter_increment() {
    let source = "for (let i = 0; i < 10; j++) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "misplaced-loop-counter");
    assert_eq!(diagnostics[0].message_id, "misplacedCounter");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_misplaced_loop_counter_compound_assign() {
    let source = "for (let i = 0; i < 10; k += 1) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "misplacedCounter");
}

#[test]
fn reports_misplaced_loop_counter_sequence_all_disjoint() {
    let source = "for (let i = 0; i < 10; j++, k++) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "misplacedCounter");
}

#[test]
fn does_not_report_misplaced_loop_counter_matching_update() {
    let source = "for (let i = 0; i < 10; i++) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misplaced_loop_counter_sequence_overlap() {
    let source = "for (let i = 0, j = 0; i < 10 && j < 5; i++, j++) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misplaced_loop_counter_member_condition() {
    let source = "for (let i = 0; arr[i] < 10; i++) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misplaced_loop_counter_no_test_or_update() {
    let source = "for (;;) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_misplaced_loop_counter_call_condition_no_identifier() {
    let source = "for (let i = 0; cond(); i++) {}";
    let diagnostics = scan("misplaced-loop-counter", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_array_delete_on_resolved_array_variable() {
    let source = "const a = [1, 2, 3];\ndelete a[0];";
    let diagnostics = scan("no-array-delete", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-array-delete");
    assert_eq!(diagnostics[0].message_id, "noArrayDelete");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn reports_no_array_delete_on_array_literal() {
    let source = "delete [1, 2][0];";
    let diagnostics = scan("no-array-delete", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noArrayDelete");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_no_array_delete_on_object_property() {
    let source = "const o = { x: 1 };\ndelete o.x;";
    let diagnostics = scan("no-array-delete", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_array_delete_on_non_array_computed() {
    let source = "const o = {};\ndelete o['x'];";
    let diagnostics = scan("no-array-delete", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_array_delete_on_static_member() {
    let source = "const a = [1, 2, 3];\ndelete a.foo;";
    let diagnostics = scan("no-array-delete", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_array_delete_on_unprovable_receiver() {
    let source = "function f(p) {\n  delete p[0];\n}";
    let diagnostics = scan("no-array-delete", source);
    assert!(diagnostics.is_empty());
}

// no-literal-call tests

#[test]
fn reports_no_literal_call_for_boolean_literal() {
    let source = "true();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-literal-call");
    assert_eq!(diagnostics[0].message_id, "noLiteralCall");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_literal_call_for_number_literal() {
    let source = "(42)();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_literal_call_for_string_literal() {
    let source = "(\"foo\")();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_literal_call_for_null_literal() {
    let source = "(null)();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_literal_call_for_regex_literal() {
    let source = "(/re/)();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_literal_call_for_bigint_literal() {
    let source = "(1n)();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_literal_call_for_template_literal_callee() {
    let source = "`foo`();";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_no_literal_call_for_tagged_template_literal_tag() {
    let source = "true`text`;";
    let diagnostics = scan("no-literal-call", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noLiteralCall");
}

#[test]
fn does_not_report_no_literal_call_for_identifier_call() {
    let source = "foo();";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_literal_call_for_member_call() {
    let source = "obj.method();";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_literal_call_for_iife() {
    let source = "(() => {})();";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_literal_call_for_function_expression_call() {
    let source = "(function f() {})();";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_literal_call_for_object_or_array_callee() {
    let source = "({})();\n[]();";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_literal_call_for_callable_tagged_template() {
    let source = "foo`text`;";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_literal_call_for_plain_template_literal() {
    let source = "const x = `foo`;";
    let diagnostics = scan("no-literal-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_shorthand_property_grouping_split_by_regular_property() {
    let source = "const o = { a, x: 1, b };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "shorthand-property-grouping");
    assert_eq!(diagnostics[0].message_id, "groupShorthand");
}

#[test]
fn reports_shorthand_property_grouping_lone_middle_shorthand() {
    let source = "const o = { x: 1, a, y: 2 };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "groupShorthand");
}

#[test]
fn reports_shorthand_property_grouping_split_by_spread() {
    let source = "const o = { a, ...rest, b };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_shorthand_property_grouping_two_blocks() {
    let source = "const o = { a, x: 1, y: 2, b };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_shorthand_property_grouping_grouped_at_start() {
    let source = "const o = { a, b, x: 1 };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_shorthand_property_grouping_grouped_at_end() {
    let source = "const o = { x: 1, a, b };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_shorthand_property_grouping_all_shorthand() {
    let source = "const o = { a, b, c };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_shorthand_property_grouping_no_shorthand() {
    let source = "const o = { x: 1, y: 2 };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_shorthand_property_grouping_single_shorthand() {
    let source = "const o = { a };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_shorthand_property_grouping_spread_then_shorthand() {
    let source = "const o = { ...rest, a, b };";
    let diagnostics = scan("shorthand-property-grouping", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_code_after_done_for_statement_after_done() {
    let source = "it('t', function (done) { done(); foo(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-code-after-done");
    assert_eq!(diagnostics[0].message_id, "noCodeAfterDone");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_no_code_after_done_for_arrow_hook_callback() {
    let source = "beforeEach((done) => { done(); cleanup(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noCodeAfterDone");
}

#[test]
fn reports_no_code_after_done_for_it_only_member_call() {
    let source = "it.only('t', function (done) { done(); foo(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noCodeAfterDone");
}

#[test]
fn does_not_report_no_code_after_done_when_done_is_last() {
    let source = "it('t', function (done) { foo(); done(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_code_after_done_when_nothing_after_done() {
    let source = "it('t', function (done) { done(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_code_after_done_for_trailing_bare_return() {
    let source = "it('t', function (done) { done(); return; });";
    let diagnostics = scan("no-code-after-done", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_code_after_done_without_done_param() {
    let source = "it('t', function () { foo(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_code_after_done_for_non_mocha_call() {
    let source = "register(function (done) { done(); foo(); });";
    let diagnostics = scan("no-code-after-done", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_code_after_done_for_nested_done_call() {
    let source = "it('t', function (done) { if (x) { done(); foo(); } });";
    let diagnostics = scan("no-code-after-done", source);
    assert!(diagnostics.is_empty());
}

// --- function-inside-loop tests ---

#[test]
fn function_inside_loop_reports_arrow_in_for_loop() {
    let source = "for (let i = 0; i < 10; i++) { const f = () => i; }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "function-inside-loop");
    assert_eq!(diagnostics[0].message_id, "noFunctionInLoop");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn function_inside_loop_reports_function_declaration_in_while() {
    let source = "while (x) { function g() {} }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noFunctionInLoop");
}

#[test]
fn function_inside_loop_reports_function_expression_in_for_of() {
    let source = "for (const x of xs) { const h = function () {}; }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn function_inside_loop_reports_in_for_in_and_do_while() {
    let source = "for (const k in obj) { const f = () => k; }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
    let source = "do { const g = () => {}; } while (cond);";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn function_inside_loop_does_not_report_top_level_function() {
    let source = "function h() {}";
    let diagnostics = scan("function-inside-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn function_inside_loop_does_not_report_loop_without_function() {
    let source = "for (const x of xs) { use(x); }";
    let diagnostics = scan("function-inside-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn function_inside_loop_reports_function_in_loop_inside_outer_function() {
    let source = "function outer() { for (;;) { const f = () => {}; } }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn function_inside_loop_resets_context_at_function_boundary() {
    // `a` is directly in the loop and flagged; `b` is inside `a`, not the loop.
    let source = "for (;;) { const a = () => { const b = () => {}; }; }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn function_inside_loop_does_not_report_nested_loop_in_function_outside_loop() {
    // The inner function `f` is in a loop that lives inside `inner`, which is
    // itself inside the outer loop: `f` IS in a loop, so it is flagged. The
    // function `inner` is directly in the outer loop, so it is flagged too.
    let source = "for (;;) { const inner = () => { for (;;) { const f = () => {}; } }; }";
    let diagnostics = scan("function-inside-loop", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn function_inside_loop_does_not_report_iife() {
    let source = "for (let i = 0; i < 10; i++) { (() => use(i))(); }";
    let diagnostics = scan("function-inside-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn function_inside_loop_does_not_report_function_outside_any_loop() {
    let source = "const f = () => { const g = () => {}; };";
    let diagnostics = scan("function-inside-loop", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_useless_intersection_reports_any_member() {
    let source = "type T = string & any;";
    let diagnostics = scan("no-useless-intersection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-useless-intersection");
    assert_eq!(diagnostics[0].message_id, "uselessIntersection");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn no_useless_intersection_reports_never_member() {
    let source = "type U = number & never;";
    let diagnostics = scan("no-useless-intersection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "uselessIntersection");
}

#[test]
fn no_useless_intersection_reports_unknown_member() {
    let source = "type V = string & unknown;";
    let diagnostics = scan("no-useless-intersection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "uselessIntersection");
}

#[test]
fn no_useless_intersection_reports_each_keyword_member() {
    let source = "type W = any & string & never;";
    let diagnostics = scan("no-useless-intersection", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn no_useless_intersection_does_not_report_without_keyword_member() {
    let source = "type X = A & B;";
    let diagnostics = scan("no-useless-intersection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_useless_intersection_does_not_report_union_with_any() {
    let source = "type Y = string | any;";
    let diagnostics = scan("no-useless-intersection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn use_type_alias_reports_repeated_union() {
    // "string | number" appears 3× (default threshold) → one report on the first
    let source = "let a: string | number;\nlet b: string | number;\nlet c: string | number;";
    let diagnostics = scan("use-type-alias", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "use-type-alias");
    assert_eq!(diagnostics[0].message_id, "useTypeAlias");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn use_type_alias_reports_repeated_intersection() {
    let source = "let a: A & B;\nlet b: A & B;\nlet c: A & B;";
    let diagnostics = scan("use-type-alias", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useTypeAlias");
}

#[test]
fn use_type_alias_does_not_report_below_threshold() {
    // "string | number" appears only twice; default threshold 3 → no report
    let source = "let a: string | number;\nlet b: string | number;";
    let diagnostics = scan("use-type-alias", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn use_type_alias_does_not_report_distinct_unions() {
    // Each distinct union appears once → no report
    let source = "let a: string | number;\nlet b: boolean | null;\nlet c: number | boolean;";
    let diagnostics = scan("use-type-alias", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn use_type_alias_is_order_sensitive() {
    // "string | number" and "number | string" are distinct as written text
    let source = "let a: string | number;\nlet b: number | string;\nlet c: string | number;";
    let diagnostics = scan("use-type-alias", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn use_type_alias_reports_each_distinct_repeated_type() {
    let source = "let a: string | number;\nlet b: string | number;\nlet c: string | number;\n\
let d: A & B;\nlet e: A & B;\nlet f: A & B;";
    let diagnostics = scan("use-type-alias", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn reports_public_default_static_field() {
    let source = "class C { static x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "public-static-readonly");
}

#[test]
fn reports_explicit_public_static_field() {
    let source = "class C { public static x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn reports_uninitialized_public_static_field() {
    let source = "class C { static x: number; }";
    let diagnostics = scan("public-static-readonly", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_declare_public_static_field() {
    // An ambient `declare` field has no runtime storage, so it is exempt.
    let source = "class C { declare static x: number; }";
    let diagnostics = scan("public-static-readonly", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_static_readonly_field() {
    let source = "class C { static readonly x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_private_static_field() {
    let source = "class C { private static x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_protected_static_field() {
    let source = "class C { protected static x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_non_static_field() {
    let source = "class C { x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_static_private_key_field() {
    let source = "class C { static #x = 1; }";
    let diagnostics = scan("public-static-readonly", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_call_with_paren_on_next_line() {
    let source = "foo\n(arg);";
    let diagnostics = scan("call-argument-line", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "call-argument-line");
    assert_eq!(diagnostics[0].message_id, "sameLineAsCallee");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn does_not_report_call_with_paren_on_same_line() {
    let diagnostics = scan("call-argument-line", "foo(arg);");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_call_with_wrapped_arguments() {
    let source = "foo(\n  a,\n  b\n);";
    let diagnostics = scan("call-argument-line", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_member_call() {
    let diagnostics = scan("call-argument-line", "obj.method(x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_member_call_with_paren_on_next_line() {
    let source = "obj.method\n(x);";
    let diagnostics = scan("call-argument-line", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_zero_argument_call_with_paren_on_next_line() {
    let source = "foo\n();";
    let diagnostics = scan("call-argument-line", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_call_with_comment_before_same_line_paren() {
    let source = "foo /* c */ (x);";
    let diagnostics = scan("call-argument-line", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_generic_call_with_paren_on_type_args_line() {
    let source = "foo<number>(x);";
    let diagnostics = scan("call-argument-line", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_empty_object_decl_followed_by_property_assignment() {
    let source = "let p = {};\np.name = \"John\";";
    let diagnostics = scan("prefer-object-literal", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-object-literal");
    assert_eq!(diagnostics[0].message_id, "preferObjectLiteral");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_empty_object_decl_followed_by_computed_property_assignment() {
    let source = "let p = {};\np[\"name\"] = \"John\";";
    let diagnostics = scan("prefer-object-literal", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn does_not_report_non_empty_object_literal_declaration() {
    let source = "let p = { name: \"John\" };\np.age = 42;";
    let diagnostics = scan("prefer-object-literal", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_when_next_statement_reads_the_variable() {
    let source = "let p = {};\nfoo(p);";
    let diagnostics = scan("prefer-object-literal", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_empty_object_decl_with_no_following_statement() {
    let source = "let p = {};";
    let diagnostics = scan("prefer-object-literal", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_when_initializer_is_not_an_empty_literal() {
    let source = "let p = getObj();\np.x = 1;";
    let diagnostics = scan("prefer-object-literal", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_property_assignment_to_a_different_variable() {
    let source = "let p = {};\nq.name = \"John\";";
    let diagnostics = scan("prefer-object-literal", source);
    assert!(diagnostics.is_empty());
}

// --- no-undefined-argument (S4623) ---

#[test]
fn no_undefined_argument_reports_sole_undefined() {
    let diagnostics = scan("no-undefined-argument", "foo(undefined);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-undefined-argument");
    assert_eq!(diagnostics[0].message_id, "removeUndefined");
}

#[test]
fn no_undefined_argument_reports_trailing_undefined() {
    let diagnostics = scan("no-undefined-argument", "foo(1, undefined);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-undefined-argument");
    assert_eq!(diagnostics[0].message_id, "removeUndefined");
}

#[test]
fn no_undefined_argument_reports_new_expression() {
    let diagnostics = scan("no-undefined-argument", "new Foo(undefined);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-undefined-argument");
    assert_eq!(diagnostics[0].message_id, "removeUndefined");
}

#[test]
fn no_undefined_argument_no_report_undefined_not_last() {
    let diagnostics = scan("no-undefined-argument", "foo(undefined, 1);");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_undefined_argument_no_report_no_args() {
    let diagnostics = scan("no-undefined-argument", "foo();");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_undefined_argument_no_report_non_undefined_args() {
    let diagnostics = scan("no-undefined-argument", "foo(1, 2);");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_undefined_argument_no_report_spread() {
    let source = "foo(...args);";
    let diagnostics = scan("no-undefined-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_identical_functions_reports_second_of_two_identical() {
    let source = r#"function a(x) {
  if (x > 0) return x;
  return -x;
}
function b(x) {
  if (x > 0) return x;
  return -x;
}"#;
    let diagnostics = scan("no-identical-functions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-identical-functions");
    assert_eq!(diagnostics[0].message_id, "identicalFunctions");
    assert_eq!(diagnostics[0].data.value.as_deref(), Some("1"));
}

#[test]
fn no_identical_functions_reports_two_duplicates_of_three() {
    let source = r#"function a(x) {
  if (x > 0) return x;
  return -x;
}
function b(x) {
  if (x > 0) return x;
  return -x;
}
function c(x) {
  if (x > 0) return x;
  return -x;
}"#;
    let diagnostics = scan("no-identical-functions", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn no_identical_functions_no_report_below_threshold() {
    // Single-line functions: below the 3-line threshold — never flagged.
    let source = "function a(x) { return x; }\nfunction b(x) { return x; }";
    let diagnostics = scan("no-identical-functions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_identical_functions_no_report_different_bodies() {
    let source = r#"function a(x) {
  if (x > 0) return x;
  return -x;
}
function b(x) {
  if (x > 0) return x + 1;
  return -x;
}"#;
    let diagnostics = scan("no-identical-functions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_identical_functions_no_report_whitespace_difference() {
    // Bodies differ only in whitespace — conservative port does NOT flag these.
    let source = "function a(x) {\n  return x;\n  }\nfunction b(x) {\n return x;\n}";
    let diagnostics = scan("no-identical-functions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_identical_functions_no_report_expression_bodied_arrow() {
    // Expression-bodied arrows (no block body) are never flagged.
    let source = "const f = x => x + 1;\nconst g = x => x + 1;";
    let diagnostics = scan("no-identical-functions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_identical_functions_reports_block_bodied_arrows() {
    let source = r#"const f = (x) => {
  if (x > 0) return x;
  return -x;
};
const g = (x) => {
  if (x > 0) return x;
  return -x;
};"#;
    let diagnostics = scan("no-identical-functions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-identical-functions");
    assert_eq!(diagnostics[0].message_id, "identicalFunctions");
}

#[test]
fn no_in_misuse_reports_string_value_in_array_literal() {
    let diagnostics = scan(
        "no-in-misuse",
        r#"const found = "apple" in ["apple", "banana"];"#,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-in-misuse");
    assert_eq!(diagnostics[0].message_id, "inMisuse");
}

#[test]
fn no_in_misuse_reports_string_value_in_const_array_identifier() {
    let diagnostics = scan(
        "no-in-misuse",
        r#"const fruits = ["apple", "banana"]; const found = "apple" in fruits;"#,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-in-misuse");
    assert_eq!(diagnostics[0].message_id, "inMisuse");
}

#[test]
fn no_in_misuse_does_not_report_numeric_index_string() {
    // "0" is a valid array-index key — not a value-membership check.
    let diagnostics = scan(
        "no-in-misuse",
        r#"const found = "0" in ["apple", "banana"];"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn no_in_misuse_does_not_report_array_prototype_member() {
    // "length" is a legitimate property probe.
    let diagnostics = scan("no-in-misuse", r#"const has = "length" in [1, 2, 3];"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_in_misuse_does_not_report_push_prototype_member() {
    let diagnostics = scan("no-in-misuse", r#"const has = "push" in [1, 2, 3];"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_in_misuse_does_not_report_non_string_literal_left() {
    // Left operand is a variable — not a string literal, conservatively skipped.
    let diagnostics = scan(
        "no-in-misuse",
        r#"const k = "apple"; const found = k in [1, 2];"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn no_in_misuse_does_not_report_right_is_not_array() {
    // Right operand is an identifier that doesn't resolve to an array literal.
    let diagnostics = scan("no-in-misuse", r#"const found = "apple" in someObject;"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_in_misuse_does_not_report_object_right_operand() {
    // Right operand is an object literal, not an array.
    let diagnostics = scan("no-in-misuse", r#"const found = "apple" in { apple: 1 };"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_require_or_define_flags_require_call() {
    let source = "require('fs');";
    let diagnostics = scan("no-require-or-define", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-require-or-define");
    assert_eq!(diagnostics[0].message_id, "noRequireOrDefine");
}

#[test]
fn no_require_or_define_flags_require_in_assignment() {
    let source = "const x = require('fs');";
    let diagnostics = scan("no-require-or-define", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-require-or-define");
    assert_eq!(diagnostics[0].message_id, "noRequireOrDefine");
}

#[test]
fn no_require_or_define_flags_define_call() {
    let source = "define(['dep'], function(dep) {});";
    let diagnostics = scan("no-require-or-define", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-require-or-define");
    assert_eq!(diagnostics[0].message_id, "noRequireOrDefine");
}

#[test]
fn no_require_or_define_no_flag_member_require() {
    let source = "foo.require('x');";
    let diagnostics = scan("no-require-or-define", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_require_or_define_no_flag_es_import() {
    let source = "import x from 'fs';";
    let diagnostics = scan("no-require-or-define", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_require_or_define_no_flag_different_name() {
    let source = "function f() { requireSomething(); }";
    let diagnostics = scan("no-require-or-define", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_invalid_regexp_reports_unclosed_bracket() {
    let source = "new RegExp('[');";
    let diagnostics = scan("no-invalid-regexp", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-invalid-regexp");
    assert_eq!(diagnostics[0].message_id, "invalidRegExp");
}

#[test]
fn no_invalid_regexp_reports_call_form_unclosed_group() {
    let source = "RegExp('(');";
    let diagnostics = scan("no-invalid-regexp", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "invalidRegExp");
}

#[test]
fn no_invalid_regexp_reports_invalid_flag() {
    let source = "new RegExp('a', 'z');";
    let diagnostics = scan("no-invalid-regexp", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "invalidRegExp");
}

#[test]
fn no_invalid_regexp_does_not_report_valid_pattern() {
    let source = "new RegExp('abc');";
    let diagnostics = scan("no-invalid-regexp", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_invalid_regexp_does_not_report_cooked_digit_escape() {
    // JS source: new RegExp('\\d+'); — cooked value is \d+, a valid pattern
    let source = "new RegExp('\\\\d+');";
    let diagnostics = scan("no-invalid-regexp", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_invalid_regexp_does_not_report_dynamic_arg() {
    let source = "new RegExp(somePattern);";
    let diagnostics = scan("no-invalid-regexp", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_invalid_regexp_does_not_report_valid_quantifier() {
    let source = "new RegExp('a{2,3}');";
    let diagnostics = scan("no-invalid-regexp", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_extra_arguments_reports_function_expression_with_extra_args() {
    let source = "const f = function(a){}; f(1, 2);";
    let diagnostics = scan("no-extra-arguments", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-extra-arguments");
    assert_eq!(diagnostics[0].message_id, "extraArguments");
}

#[test]
fn no_extra_arguments_reports_arrow_function_with_extra_args() {
    let source = "const g = (a) => a; g(1, 2, 3);";
    let diagnostics = scan("no-extra-arguments", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-extra-arguments");
    assert_eq!(diagnostics[0].message_id, "extraArguments");
}

#[test]
fn no_extra_arguments_does_not_report_exact_arg_count() {
    let source = "const f = (a, b) => {}; f(1, 2);";
    let diagnostics = scan("no-extra-arguments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_extra_arguments_does_not_report_fewer_args_than_params() {
    let source = "const f = (a) => {}; f();";
    let diagnostics = scan("no-extra-arguments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_extra_arguments_does_not_report_rest_parameter() {
    let source = "const f = (...args) => {}; f(1, 2, 3);";
    let diagnostics = scan("no-extra-arguments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_extra_arguments_does_not_report_arguments_usage() {
    let source = "const f = function(){ return arguments.length; }; f(1, 2);";
    let diagnostics = scan("no-extra-arguments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_extra_arguments_does_not_report_unresolved_callee() {
    let source = "g(1, 2);";
    let diagnostics = scan("no-extra-arguments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_extra_arguments_does_not_report_spread_argument() {
    let source = "const f = (a) => {}; f(...arr);";
    let diagnostics = scan("no-extra-arguments", source);
    assert!(diagnostics.is_empty());
}

fn scan_jsx(rule_name: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
    let options = SonarjsOptions {
        rule_names: [CompactString::from(rule_name)].into_iter().collect(),
        ..SonarjsOptions::default()
    };
    scan_sonarjs(source, "sample.tsx", &options)
}

#[test]
fn link_with_target_blank_reports_anchor_without_rel() {
    let source = r#"<a target="_blank">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "link-with-target-blank");
    assert_eq!(diagnostics[0].message_id, "targetBlankNoOpener");
}

#[test]
fn link_with_target_blank_reports_anchor_with_rel_lacking_noopener() {
    let source = r#"<a target="_blank" rel="nofollow">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "targetBlankNoOpener");
}

#[test]
fn link_with_target_blank_does_not_report_anchor_with_rel_noopener() {
    let source = r#"<a target="_blank" rel="noopener">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn link_with_target_blank_does_not_report_anchor_with_rel_noreferrer() {
    let source = r#"<a target="_blank" rel="noreferrer">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn link_with_target_blank_does_not_report_anchor_without_target() {
    let source = r#"<a href="/x">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn link_with_target_blank_does_not_report_anchor_with_target_self() {
    let source = r#"<a target="_self">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn link_with_target_blank_does_not_report_anchor_with_spread() {
    let source = r#"<a {...props} target="_blank">link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn link_with_target_blank_does_not_report_anchor_with_dynamic_rel() {
    let source = r#"<a target="_blank" rel={dyn}>link</a>"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn link_with_target_blank_reports_area_without_rel() {
    let source = r#"<area target="_blank" />"#;
    let diagnostics = scan_jsx("link-with-target-blank", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "targetBlankNoOpener");
}

#[test]
fn no_hardcoded_passwords_reports_variable_declarator() {
    let source = "const password = \"s3cr3t-value\";";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-hardcoded-passwords");
    assert_eq!(diagnostics[0].message_id, "hardcodedPassword");
}

#[test]
fn no_hardcoded_passwords_reports_object_property() {
    let source = "const config = { password: \"hunter2abc\" };";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedPassword");
}

#[test]
fn no_hardcoded_passwords_reports_member_assignment() {
    let source = "obj.passwd = \"realSecret123\";";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedPassword");
}

#[test]
fn no_hardcoded_passwords_does_not_report_empty_value() {
    let source = "const password = \"\";";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_passwords_does_not_report_placeholder_equals_name() {
    let source = "const password = \"password\";";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_passwords_does_not_report_non_credential_identifier() {
    let source = "const username = \"admin\";";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_passwords_does_not_report_non_literal_init() {
    let source = "const password = getSecret();";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_passwords_does_not_report_partial_name_match() {
    let source = "const passwordHint = \"x\";";
    let diagnostics = scan("no-hardcoded-passwords", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn hashing_reports_crypto_create_hash_md5() {
    let source = "const h = crypto.createHash(\"md5\");";
    let diagnostics = scan("hashing", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "hashing");
    assert_eq!(diagnostics[0].message_id, "weakHash");
}

#[test]
fn hashing_reports_webcrypto_sha1_digest() {
    let source = "crypto.subtle.digest(\"SHA-1\", data);";
    let diagnostics = scan("hashing", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "weakHash");
}

#[test]
fn hashing_does_not_report_sha256() {
    let source = "const h = crypto.createHash(\"sha256\");";
    let diagnostics = scan("hashing", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn hashing_does_not_report_dynamic_algorithm() {
    let source = "const h = crypto.createHash(algorithm);";
    let diagnostics = scan("hashing", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_clear_text_protocols_reports_http_url() {
    let source = "const url = \"http://example.com\";";
    let diagnostics = scan("no-clear-text-protocols", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-clear-text-protocols");
    assert_eq!(diagnostics[0].message_id, "clearTextProtocol");
}

#[test]
fn no_clear_text_protocols_reports_clear_text_websocket_url() {
    let source = "const url = \"ws://example.com/socket\";";
    let diagnostics = scan("no-clear-text-protocols", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "clearTextProtocol");
}

#[test]
fn no_clear_text_protocols_does_not_report_encrypted_protocols() {
    let source = "const a = \"https://example.com\"; const b = \"wss://example.com/socket\";";
    let diagnostics = scan("no-clear-text-protocols", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_clear_text_protocols_does_not_report_protocol_label() {
    let source = "const label = \"http: status\";";
    let diagnostics = scan("no-clear-text-protocols", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_weak_cipher_reports_des_cipher_creation() {
    let source = "const c = crypto.createCipheriv(\"des-cbc\", key, iv);";
    let diagnostics = scan("no-weak-cipher", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-weak-cipher");
    assert_eq!(diagnostics[0].message_id, "weakCipher");
}

#[test]
fn no_weak_cipher_reports_bare_rc4_cipher_factory() {
    let source = "const c = createCipher(\"rc4\", password);";
    let diagnostics = scan("no-weak-cipher", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "weakCipher");
}

#[test]
fn no_weak_cipher_does_not_report_aes_gcm() {
    let source = "const c = crypto.createCipheriv(\"aes-256-gcm\", key, iv);";
    let diagnostics = scan("no-weak-cipher", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_weak_cipher_does_not_report_dynamic_algorithm() {
    let source = "const c = crypto.createCipheriv(algorithm, key, iv);";
    let diagnostics = scan("no-weak-cipher", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_ignored_exceptions_reports_empty_catch_with_binding() {
    let source = "try { foo(); } catch (e) {}";
    let diagnostics = scan("no-ignored-exceptions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-ignored-exceptions");
    assert_eq!(diagnostics[0].message_id, "ignoredException");
}

#[test]
fn no_ignored_exceptions_reports_empty_catch_without_binding() {
    let source = "try { foo(); } catch {}";
    let diagnostics = scan("no-ignored-exceptions", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "ignoredException");
}

#[test]
fn no_ignored_exceptions_does_not_report_non_empty_catch() {
    let source = "try { foo(); } catch (e) { log(e); }";
    let diagnostics = scan("no-ignored-exceptions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_ignored_exceptions_does_not_report_catch_with_block_comment() {
    let source = "try { foo(); } catch (e) { /* intentionally ignored */ }";
    let diagnostics = scan("no-ignored-exceptions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_ignored_exceptions_does_not_report_catch_with_line_comment() {
    let source = "try { foo(); } catch (e) {\n// safe to ignore here\n}";
    let diagnostics = scan("no-ignored-exceptions", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_function_argument_reports_trailing_unused_param() {
    let source = "function f(a, b) { return a; }";
    let diagnostics = scan("no-unused-function-argument", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unused-function-argument");
    assert_eq!(diagnostics[0].message_id, "unusedFunctionArgument");
}

#[test]
fn no_unused_function_argument_reports_trailing_unused_arrow_param() {
    let source = "const g = (x, y, z) => x + y;";
    let diagnostics = scan("no-unused-function-argument", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unused-function-argument");
    assert_eq!(diagnostics[0].message_id, "unusedFunctionArgument");
}

#[test]
fn no_unused_function_argument_does_not_report_all_params_used() {
    let source = "function f(a, b) { return a + b; }";
    let diagnostics = scan("no-unused-function-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_function_argument_does_not_report_early_unused_when_trailing_used() {
    let source = "function f(a, b) { return b; }";
    let diagnostics = scan("no-unused-function-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_function_argument_does_not_report_underscore_prefixed() {
    let source = "function f(_unused) {}";
    let diagnostics = scan("no-unused-function-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_function_argument_does_not_report_param_used_in_nested_fn() {
    let source = "function f(a) { return inner(); function inner() { return a; } }";
    let diagnostics = scan("no-unused-function-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_function_argument_does_not_report_rest_param() {
    let source = "function f(a, ...rest) {}";
    let diagnostics = scan("no-unused-function-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_function_argument_does_not_report_destructuring_param() {
    let source = "function f({ x }) {}";
    let diagnostics = scan("no-unused-function-argument", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_reports_self_closing_object_without_attributes() {
    let source = r#"<object data="video.swf" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "object-alt-content");
    assert_eq!(diagnostics[0].message_id, "objectAltContent");
}

#[test]
fn object_alt_content_reports_object_with_empty_children() {
    let source = r#"<object data="video.swf"></object>"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "objectAltContent");
}

#[test]
fn object_alt_content_reports_object_with_whitespace_only_text_child() {
    let source = r#"<object data="video.swf">   </object>"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "objectAltContent");
}

#[test]
fn object_alt_content_does_not_report_object_with_text_child() {
    let source = r#"<object data="video.swf">Fallback text for assistive technologies.</object>"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_child_element() {
    let source = r#"<object data="video.swf"><img src="fallback.png" alt="Embedded" /></object>"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_child_expression() {
    let source = r#"<object data={src}>{fallback}</object>"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_aria_label() {
    let source = r#"<object data="video.swf" aria-label="Embedded video" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_aria_labelledby() {
    let source = r#"<object data="video.swf" aria-labelledby="label-id" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_title() {
    let source = r#"<object data="video.swf" title="Embedded video" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_aria_hidden_true() {
    let source = r#"<object data="video.swf" aria-hidden="true" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_spread_attribute() {
    let source = r#"<object {...props} />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_non_object_element() {
    let source = r#"<video src="clip.mp4" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn object_alt_content_does_not_report_object_with_aria_hidden_non_true() {
    let source = r#"<object data="video.swf" aria-hidden="false" />"#;
    let diagnostics = scan_jsx("object-alt-content", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "objectAltContent");
}

#[test]
fn no_use_of_empty_return_value_reports_var_decl_with_void_fn_result() {
    let source = "function voidFn() { console.log('x'); } const x = voidFn();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-use-of-empty-return-value");
    assert_eq!(diagnostics[0].message_id, "useOfEmptyReturnValue");
}

#[test]
fn no_use_of_empty_return_value_reports_assignment_with_void_fn_result() {
    let source = "function voidFn() {} let x; x = voidFn();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useOfEmptyReturnValue");
}

#[test]
fn no_use_of_empty_return_value_reports_return_with_void_fn_result() {
    let source = "function voidFn() {} function outer() { return voidFn(); }";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useOfEmptyReturnValue");
}

#[test]
fn no_use_of_empty_return_value_does_not_report_bare_call_statement() {
    let source = "function voidFn() {} voidFn();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_use_of_empty_return_value_does_not_report_valued_function() {
    let source = "function valued() { return 42; } const x = valued();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_use_of_empty_return_value_does_not_report_async_function() {
    let source = "async function asyncFn() {} const p = asyncFn();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_use_of_empty_return_value_does_not_report_generator_function() {
    let source = "function* genFn() {} const g = genFn();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_use_of_empty_return_value_reports_const_arrow_void() {
    let source = "const f = () => { console.log('hi'); }; const x = f();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useOfEmptyReturnValue");
}

#[test]
fn no_use_of_empty_return_value_does_not_report_expression_arrow() {
    let source = "const f = () => 42; const x = f();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_use_of_empty_return_value_does_not_flag_return_in_nested_fn() {
    let source = "function outer() { function inner() { return 1; } } const x = outer();";
    let diagnostics = scan("no-use-of-empty-return-value", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useOfEmptyReturnValue");
}

#[test]
fn no_duplicated_branches_reports_else_identical_to_if() {
    let source = "if (a) { doWork(); } else { doWork(); }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-duplicated-branches");
    assert_eq!(diagnostics[0].message_id, "duplicatedBranch");
}

#[test]
fn no_duplicated_branches_reports_duplicate_else_if_branch() {
    let source = "if (a) { doWork(); } else if (b) { other(); } else if (c) { doWork(); }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "duplicatedBranch");
}

#[test]
fn no_duplicated_branches_does_not_report_when_all_differ() {
    let source = "if (a) { one(); } else if (b) { two(); } else { three(); }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_duplicated_branches_does_not_report_lone_if() {
    let source = "if (a) { doWork(); }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_duplicated_branches_does_not_report_if_else_if_without_else() {
    // Two branches that differ — no duplicate
    let source = "if (a) { doWork(); } else if (b) { other(); }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_duplicated_branches_reports_duplicate_switch_case() {
    let source = "switch (x) { case 1: doWork(); break; case 2: doWork(); break; }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-duplicated-branches");
    assert_eq!(diagnostics[0].message_id, "duplicatedBranch");
}

#[test]
fn no_duplicated_branches_does_not_report_switch_all_differ() {
    let source = "switch (x) { case 1: one(); break; case 2: two(); break; }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_duplicated_branches_does_not_report_switch_fall_through() {
    // Fall-through cases have empty consequents and should be skipped
    let source = "switch (x) { case 1: case 2: doWork(); break; }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_duplicated_branches_does_not_report_if_else_if_same_but_no_all_match() {
    // if and else-if are same, else differs: reports the duplicate else-if
    let source = "if (a) { doWork(); } else if (b) { doWork(); } else { other(); }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "duplicatedBranch");
}

#[test]
fn no_duplicated_branches_switch_duplicate_default_case() {
    let source = "switch (x) { case 1: doWork(); break; default: doWork(); break; }";
    let diagnostics = scan("no-duplicated-branches", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "duplicatedBranch");
}

#[test]
fn block_scoped_var_reports_var_used_after_if_block() {
    let source = "function f(c) { if (c) { var x = 1; } return x; }";
    let diagnostics = scan("block-scoped-var", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "block-scoped-var");
    assert_eq!(diagnostics[0].message_id, "blockScopedVar");
}

#[test]
fn block_scoped_var_reports_for_loop_counter_used_after_loop() {
    let source = "function f(n) { for (var i = 0; i < n; i++) {} return i; }";
    let diagnostics = scan("block-scoped-var", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "block-scoped-var");
    assert_eq!(diagnostics[0].message_id, "blockScopedVar");
}

#[test]
fn block_scoped_var_does_not_report_var_at_function_top_level() {
    let source = "function f() { var x = 1; return x; }";
    let diagnostics = scan("block-scoped-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn declarations_in_global_scope_reports_top_level_function() {
    let diagnostics = scan("declarations-in-global-scope", "function f() {}\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "declarations-in-global-scope");
    assert_eq!(diagnostics[0].message_id, "defineLocally");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn declarations_in_global_scope_reports_exported_named_functions() {
    let source = "export function f() {}\nexport default function g() {}\n";
    let diagnostics = scan("declarations-in-global-scope", source);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].message_id, "defineLocally");
    assert_eq!(diagnostics[1].message_id, "defineLocally");
}

#[test]
fn declarations_in_global_scope_allows_anonymous_default_function() {
    let diagnostics = scan(
        "declarations-in-global-scope",
        "export default function () {}\n",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn declarations_in_global_scope_reports_top_level_var_declarators() {
    let diagnostics = scan("declarations-in-global-scope", "var a = 1, b = 2;\n");
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn declarations_in_global_scope_reports_var_inside_top_level_block() {
    let source = "if (enabled) { var leaked = 1; }\n";
    let diagnostics = scan("declarations-in-global-scope", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "defineLocally");
}

#[test]
fn declarations_in_global_scope_allows_let_const_and_require_var() {
    let source = "let a = 1;\nconst b = 2;\nvar fs = require('fs');\n";
    let diagnostics = scan("declarations-in-global-scope", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn block_scoped_var_does_not_report_var_used_only_inside_block() {
    let source = "function f(c) { if (c) { var x = 1; return x; } }";
    let diagnostics = scan("block-scoped-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn block_scoped_var_does_not_report_let_in_block() {
    let source = "function f(c) { if (c) { let y = 1; return y; } }";
    let diagnostics = scan("block-scoped-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_var_usage_before_declaration_at_module_level() {
    let source = "console.log(x); var x = 5;";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].rule_name,
        "no-variable-usage-before-declaration"
    );
    assert_eq!(diagnostics[0].message_id, "usedBeforeDeclaration");
}

#[test]
fn reports_let_usage_before_declaration_at_module_level() {
    let source = "console.log(y); let y = 10;";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "usedBeforeDeclaration");
}

#[test]
fn reports_const_usage_before_declaration_at_module_level() {
    let source = "console.log(z); const z = 99;";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "usedBeforeDeclaration");
}

#[test]
fn reports_var_usage_before_declaration_inside_function() {
    let source = "function f() { console.log(a); var a = 1; }";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "usedBeforeDeclaration");
}

#[test]
fn does_not_report_variable_usage_after_declaration() {
    let source = "var x = 5; console.log(x);";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_function_declaration_called_before_it_appears() {
    let source = "foo(); function foo() { return 1; }";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_reference_in_nested_function_defined_before_var() {
    // cb is defined before var val, but cb is called after — safe closure.
    let source = "function outer() { function cb() { console.log(val); } var val = 3; cb(); }";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_reference_in_nested_arrow_defined_before_var() {
    let source = "function outer() { const cb = () => console.log(v); var v = 7; cb(); }";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_usage_of_function_parameter() {
    // Function parameters are not variable declarators; must not be flagged.
    let source = "function f(p) { return p; }";
    let diagnostics = scan("no-variable-usage-before-declaration", source);
    assert!(diagnostics.is_empty());
}

// ---- arguments-order -------------------------------------------------------

#[test]
fn reports_swapped_arguments_matching_param_names() {
    let source = "function f(a, b) {} const a = 1, b = 2; f(b, a);";
    let diagnostics = scan("arguments-order", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "arguments-order");
    assert_eq!(diagnostics[0].message_id, "argumentsOrder");
}

#[test]
fn does_not_report_arguments_in_correct_order() {
    let source = "function f(a, b) {} const a = 1, b = 2; f(a, b);";
    let diagnostics = scan("arguments-order", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_when_arg_names_differ_from_param_names() {
    let source = "function f(a, b) {} const x = 1, y = 2; f(x, y);";
    let diagnostics = scan("arguments-order", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_fewer_args_than_params() {
    // N=1 which is less than 2, so exits early before any transposition check.
    let source = "function f(a, b) {} const a = 1; f(a);";
    let diagnostics = scan("arguments-order", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_single_argument_call() {
    let source = "function f(a) {} const a = 1; f(a);";
    let diagnostics = scan("arguments-order", source);
    assert!(diagnostics.is_empty());
}

// ---- updated-const-var -----------------------------------------------------

#[test]
fn updated_const_var_reports_simple_assignment() {
    let diagnostics = scan("updated-const-var", "const x = 1; x = 2;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "updated-const-var");
    assert_eq!(diagnostics[0].message_id, "updateConst");
    assert_eq!(diagnostics[0].data.value.as_deref(), Some("x"));
}

#[test]
fn updated_const_var_reports_compound_assignment() {
    let diagnostics = scan("updated-const-var", "const x = 1; x += 2;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "updated-const-var");
    assert_eq!(diagnostics[0].message_id, "updateConst");
}

#[test]
fn updated_const_var_reports_update_expression() {
    let diagnostics = scan("updated-const-var", "const x = 1; ++x; x++;");
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|d| d.rule_name == "updated-const-var")
    );
    assert!(diagnostics.iter().all(|d| d.message_id == "updateConst"));
}

#[test]
fn updated_const_var_reports_destructuring_assignment_targets() {
    let source = "const x = 1, y = 2; ({ x } = obj); [y] = values;";
    let diagnostics = scan("updated-const-var", source);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].data.value.as_deref(), Some("x"));
    assert_eq!(diagnostics[1].data.value.as_deref(), Some("y"));
}

#[test]
fn updated_const_var_reports_for_in_and_for_of_targets() {
    let source = "const x = 1, y = 2; for (x in obj) {} for (y of values) {}";
    let diagnostics = scan("updated-const-var", source);
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|d| d.rule_name == "updated-const-var")
    );
}

#[test]
fn updated_const_var_does_not_report_let_var_or_property_writes() {
    let source = r#"
let x = 1; x = 2;
var y = 1; y++;
const obj = {}; obj.x = 1;
"#;
    let diagnostics = scan("updated-const-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn updated_const_var_does_not_report_shadowed_assignments() {
    let source = "const x = 1; function f(x) { x = 2; } { let x = 3; x = 4; }";
    let diagnostics = scan("updated-const-var", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn updated_const_var_does_not_report_for_declarations() {
    let source = "for (const x in obj) {} for (const y of values) {}";
    let diagnostics = scan("updated-const-var", source);
    assert!(diagnostics.is_empty());
}

// ---- unicode-aware-regex ---------------------------------------------------

#[test]
fn reports_unicode_aware_regex_for_property_escape_without_u_flag() {
    let source = "const r = /\\p{Letter}/;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "unicode-aware-regex");
    assert_eq!(diagnostics[0].message_id, "unicodeAwareRegex");
}

#[test]
fn reports_unicode_aware_regex_for_negative_property_escape_without_u_flag() {
    let source = "const r = /\\P{ASCII}/;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "unicode-aware-regex");
    assert_eq!(diagnostics[0].message_id, "unicodeAwareRegex");
}

#[test]
fn reports_unicode_aware_regex_with_other_flags_but_not_u() {
    let source = "const r = /\\p{Letter}/gi;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "unicodeAwareRegex");
}

#[test]
fn does_not_report_unicode_aware_regex_with_u_flag() {
    let source = "const r = /\\p{Letter}/u;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_unicode_aware_regex_with_v_flag() {
    let source = "const r = /\\p{Letter}/v;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_unicode_aware_regex_without_property_escape() {
    let source = "const r = /[a-z]+/;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_unicode_aware_regex_for_escaped_backslash_before_p() {
    // \\p{ is a literal backslash followed by p{, not a property escape.
    let source = "const r = /\\\\p{3}/;";
    let diagnostics = scan("unicode-aware-regex", source);
    assert!(diagnostics.is_empty());
}

// ---- no-undefined-assignment -----------------------------------------------

#[test]
fn reports_plain_variable_assigned_undefined() {
    let source = "x = undefined;";
    let diagnostics = scan("no-undefined-assignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-undefined-assignment");
    assert_eq!(diagnostics[0].message_id, "noUndefinedAssignment");
}

#[test]
fn reports_property_assigned_undefined() {
    let source = "obj.prop = undefined;";
    let diagnostics = scan("no-undefined-assignment", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-undefined-assignment");
    assert_eq!(diagnostics[0].message_id, "noUndefinedAssignment");
}

#[test]
fn does_not_report_assignment_of_null() {
    let source = "x = null;";
    let diagnostics = scan("no-undefined-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_assignment_of_void_zero() {
    let source = "x = void 0;";
    let diagnostics = scan("no-undefined-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_strict_equality_comparison_with_undefined() {
    let source = "if (x === undefined) {}";
    let diagnostics = scan("no-undefined-assignment", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_assignment_of_function_call() {
    let source = "x = foo();";
    let diagnostics = scan("no-undefined-assignment", source);
    assert!(diagnostics.is_empty());
}

// no-empty-after-reluctant tests

#[test]
fn reports_no_empty_after_reluctant_lazy_star_no_following() {
    let source = "const r = /a*?/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-empty-after-reluctant");
    assert_eq!(diagnostics[0].message_id, "emptyAfterReluctant");
}

#[test]
fn reports_no_empty_after_reluctant_lazy_star_followed_by_boundary() {
    let source = "const r = /a*?$/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyAfterReluctant");
}

#[test]
fn reports_no_empty_after_reluctant_lazy_optional_no_following() {
    let source = "const r = /a??/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyAfterReluctant");
}

#[test]
fn reports_no_empty_after_reluctant_lazy_star_followed_by_lookahead() {
    let source = "const r = /a*?(?=b)/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyAfterReluctant");
}

#[test]
fn does_not_report_no_empty_after_reluctant_lazy_star_followed_by_char() {
    let source = "const r = /a*?b/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_after_reluctant_lazy_plus_no_following() {
    let source = "const r = /a+?/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_after_reluctant_greedy_star() {
    let source = "const r = /a*/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_after_reluctant_lazy_star_followed_by_non_empty_group() {
    let source = "const r = /a*?(b+)/;";
    let diagnostics = scan("no-empty-after-reluctant", source);
    assert!(diagnostics.is_empty());
}

// ---- no-ignored-return -----------------------------------------------

#[test]
fn reports_string_literal_trim_as_statement() {
    let source = r#""hello".trim();"#;
    let diagnostics = scan("no-ignored-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-ignored-return");
    assert_eq!(diagnostics[0].message_id, "ignoredReturn");
}

#[test]
fn reports_array_literal_map_as_statement() {
    let source = "[1, 2].map(x => x);";
    let diagnostics = scan("no-ignored-return", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-ignored-return");
    assert_eq!(diagnostics[0].message_id, "ignoredReturn");
}

#[test]
fn does_not_report_no_ignored_return_when_value_is_used() {
    let source = r#"const y = "hello".trim();"#;
    let diagnostics = scan("no-ignored-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_ignored_return_for_non_literal_receiver() {
    let source = "foo.trim();";
    let diagnostics = scan("no-ignored-return", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_ignored_return_for_push_on_array_literal() {
    let source = "[1, 2].push(3);";
    let diagnostics = scan("no-ignored-return", source);
    assert!(diagnostics.is_empty());
}

// ---- file-name-differ-from-class -----------------------------------------------

#[test]
fn reports_file_name_differ_from_class_when_names_differ() {
    let source = "export class Foo {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "bar.ts");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "file-name-differ-from-class");
    assert_eq!(diagnostics[0].message_id, "fileNameDifferFromClass");
}

#[test]
fn does_not_report_file_name_differ_from_class_when_exact_match() {
    let source = "export class Foo {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "foo.ts");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_file_name_differ_from_class_pascal_vs_kebab() {
    let source = "export class FooBar {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "foo-bar.ts");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_file_name_differ_from_class_case_insensitive() {
    let source = "export class Foo {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "Foo.ts");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_file_name_differ_from_class_tsx_extension() {
    let source = "export class Foo {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "foo.tsx");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_file_name_differ_from_class_no_exported_class() {
    let source = "class Foo {} export {};";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "bar.ts");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_file_name_differ_from_class_multiple_exports() {
    let source = "export class Foo {} export class Bar {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "baz.ts");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_file_name_differ_from_class_export_default() {
    let source = "export default class Foo {}";
    let diagnostics = scan_with_file("file-name-differ-from-class", source, "bar.ts");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "fileNameDifferFromClass");
}

#[test]
fn declarations_in_global_scope_reports_non_require_in_mixed_var() {
    let source = "var fs = require('fs'), value = 1;\n";
    let diagnostics = scan("declarations-in-global-scope", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "defineLocally");
}

#[test]
fn declarations_in_global_scope_only_reports_outer_function_when_inner_declarations_are_local() {
    let source = "function outer() { var local = 1; function inner() {} }\n";
    let diagnostics = scan("declarations-in-global-scope", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn declarations_in_global_scope_allows_var_inside_class_static_block() {
    let source = "class C { static { var local = 1; } }\n";
    let diagnostics = scan("declarations-in-global-scope", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_unenclosed_multiline_block_for_indented_sibling_after_unbraced_if() {
    let source = "if (c)\n  a();\n  b();";
    let diagnostics = scan("no-unenclosed-multiline-block", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unenclosed-multiline-block");
    assert_eq!(diagnostics[0].message_id, "unenclosedMultilineBlock");
}

#[test]
fn does_not_report_no_unenclosed_multiline_block_when_body_is_braced() {
    let source = "if (c) {\n  a();\n  b();\n}";
    let diagnostics = scan("no-unenclosed-multiline-block", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_unenclosed_multiline_block_when_sibling_at_outer_column() {
    let source = "if (c)\n  a();\nb();";
    let diagnostics = scan("no-unenclosed-multiline-block", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_unenclosed_multiline_block_for_single_line_if() {
    let diagnostics = scan("no-unenclosed-multiline-block", "if (c) a();");
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_inconsistent_function_call_for_function_declaration_called_both_ways() {
    let source = "function f() {} f(); new f();";
    let diagnostics = scan("inconsistent-function-call", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "inconsistent-function-call");
    assert_eq!(diagnostics[0].message_id, "inconsistentFunctionCall");
}

#[test]
fn reports_inconsistent_function_call_for_arrow_called_both_ways() {
    let source = "const f = () => {}; f(); new f();";
    let diagnostics = scan("inconsistent-function-call", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "inconsistent-function-call");
    assert_eq!(diagnostics[0].message_id, "inconsistentFunctionCall");
}

#[test]
fn does_not_report_inconsistent_function_call_when_only_plain_calls() {
    let source = "function f() {} f(); f();";
    let diagnostics = scan("inconsistent-function-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inconsistent_function_call_when_only_constructor_calls() {
    let source = "function f() {} new f(); new f();";
    let diagnostics = scan("inconsistent-function-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inconsistent_function_call_for_different_functions() {
    let source = "function f() {} function g() {} f(); new g();";
    let diagnostics = scan("inconsistent-function-call", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_inconsistent_function_call_for_member_expression_callee() {
    let source = "const obj = { f: function() {} }; obj.f(); new obj.f();";
    let diagnostics = scan("inconsistent-function-call", source);
    assert!(diagnostics.is_empty());
}

// new-operator-misuse tests

#[test]
fn reports_new_operator_misuse_on_inline_arrow() {
    let source = "new (() => {})();";
    let diagnostics = scan("new-operator-misuse", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "new-operator-misuse");
    assert_eq!(diagnostics[0].message_id, "newOperatorMisuse");
    assert_eq!(diagnostics[0].loc.start_line, 1);
}

#[test]
fn reports_new_operator_misuse_on_identifier_resolving_to_arrow() {
    let source = "const f = () => {};\nnew f();";
    let diagnostics = scan("new-operator-misuse", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "new-operator-misuse");
    assert_eq!(diagnostics[0].message_id, "newOperatorMisuse");
    assert_eq!(diagnostics[0].loc.start_line, 2);
}

#[test]
fn does_not_report_new_operator_misuse_on_regular_function() {
    let source = "function F() {}\nnew F();";
    let diagnostics = scan("new-operator-misuse", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_new_operator_misuse_on_class() {
    let source = "class C {}\nnew C();";
    let diagnostics = scan("new-operator-misuse", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_new_operator_misuse_on_unresolved_identifier() {
    let source = "new Foo();";
    let diagnostics = scan("new-operator-misuse", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_new_operator_misuse_on_function_expression() {
    let source = "const g = function() {};\nnew g();";
    let diagnostics = scan("new-operator-misuse", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_no_empty_test_file_for_test_file_with_no_test_calls() {
    let source = "import {x} from './x';";
    let diagnostics = scan_with_file("no-empty-test-file", source, "foo.test.ts");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-empty-test-file");
    assert_eq!(diagnostics[0].message_id, "emptyTestFile");
}

#[test]
fn reports_no_empty_test_file_for_spec_file_with_only_describe() {
    let source = "describe('x', () => {});";
    let diagnostics = scan_with_file("no-empty-test-file", source, "a.spec.ts");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyTestFile");
}

#[test]
fn does_not_report_no_empty_test_file_when_it_call_is_present() {
    let source = "it('works', () => {});";
    let diagnostics = scan_with_file("no-empty-test-file", source, "foo.test.ts");
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_no_empty_test_file_for_non_test_filename() {
    let source = "import {x} from './x';";
    let diagnostics = scan_with_file("no-empty-test-file", source, "foo.ts");
    assert!(diagnostics.is_empty());
}

// deprecation tests

#[test]
fn reports_deprecation_for_call_to_deprecated_function() {
    let source = "/** @deprecated */ function old() {} old();";
    let diagnostics = scan("deprecation", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "deprecation");
    assert_eq!(diagnostics[0].message_id, "deprecatedUse");
}

#[test]
fn reports_deprecation_for_reference_to_deprecated_class() {
    let source = "/** @deprecated */ class OldClass {} new OldClass();";
    let diagnostics = scan("deprecation", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "deprecation");
    assert_eq!(diagnostics[0].message_id, "deprecatedUse");
}

#[test]
fn does_not_report_deprecation_for_non_deprecated_function() {
    let source = "function modern() {} modern();";
    let diagnostics = scan("deprecation", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_deprecation_for_deprecated_function_never_called() {
    let source = "/** @deprecated */ function old() {}";
    let diagnostics = scan("deprecation", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_deprecation_for_line_comment_with_at_deprecated() {
    let source = "// @deprecated\nfunction old() {} old();";
    let diagnostics = scan("deprecation", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn does_not_report_deprecation_when_comment_not_adjacent() {
    let source = "/** @deprecated */ function unrelated() {}\nfunction other() {} other();";
    let diagnostics = scan("deprecation", source);
    assert!(diagnostics.is_empty());
}

// --- cognitive-complexity (S3776) ---

fn scan_cognitive(source: &str, threshold: u32) -> SmallVec<[Diagnostic; 32]> {
    let mut options = options_for("cognitive-complexity");
    options.cognitive_complexity_threshold = threshold;
    scan_sonarjs(source, "sample.ts", &options)
}

#[test]
fn cognitive_complexity_matrix_1_single_if() {
    // function f(a){ if(a){} }  → score 1; threshold 0 → report (1>0)
    let d = scan_cognitive("function f(a){ if(a){} }", 0);
    assert_eq!(d.len(), 1, "case 1: expected 1 report");
    assert_eq!(d[0].rule_name, "cognitive-complexity");
    assert_eq!(d[0].message_id, "cognitiveComplexity");
    // At threshold 1, score 1 is not > 1 → no report
    let d2 = scan_cognitive("function f(a){ if(a){} }", 1);
    assert!(d2.is_empty(), "case 1: at threshold 1 should not report");
}

#[test]
fn cognitive_complexity_matrix_2_nested_ifs() {
    // if(a){ if(b){ if(c){} } }  → 1+2+3=6; threshold 5 → report
    let d = scan_cognitive("function f(a,b,c){ if(a){ if(b){ if(c){} } } }", 5);
    assert_eq!(d.len(), 1, "case 2: expected 1 report at threshold 5");
    // threshold 6: score 6 is not > 6 → no report
    let d2 = scan_cognitive("function f(a,b,c){ if(a){ if(b){ if(c){} } } }", 6);
    assert!(d2.is_empty(), "case 2: at threshold 6 should not report");
}

#[test]
fn cognitive_complexity_matrix_3_else_if_chain() {
    // if(a){} else if(b){} else {}  → 3; threshold 2 → report
    let d = scan_cognitive("function f(a,b){ if(a){} else if(b){} else {} }", 2);
    assert_eq!(d.len(), 1, "case 3: expected 1 report at threshold 2");
    let d2 = scan_cognitive("function f(a,b){ if(a){} else if(b){} else {} }", 3);
    assert!(d2.is_empty(), "case 3: at threshold 3 should not report");
}

#[test]
fn cognitive_complexity_matrix_4_and_operator() {
    // if(a && b){}  → if(1) + &&(1) = 2; threshold 1 → report
    let d = scan_cognitive("function f(a,b){ if(a && b){} }", 1);
    assert_eq!(d.len(), 1, "case 4: expected 1 report at threshold 1");
    let d2 = scan_cognitive("function f(a,b){ if(a && b){} }", 2);
    assert!(d2.is_empty(), "case 4: at threshold 2 should not report");
}

#[test]
fn cognitive_complexity_matrix_5_chained_and() {
    // if(a && b && c){}  → if(1) + one-&&-run(1) = 2
    let d = scan_cognitive("function f(a,b,c){ if(a && b && c){} }", 1);
    assert_eq!(d.len(), 1, "case 5: expected 1 report at threshold 1");
    let d2 = scan_cognitive("function f(a,b,c){ if(a && b && c){} }", 2);
    assert!(d2.is_empty(), "case 5: at threshold 2 should not report");
}

#[test]
fn cognitive_complexity_matrix_6_or_and_mixed() {
    // if(a || b && c){}  → if(1) + ||(1) + &&(1) = 3
    let d = scan_cognitive("function f(a,b,c){ if(a || b && c){} }", 2);
    assert_eq!(d.len(), 1, "case 6: expected 1 report at threshold 2");
    let d2 = scan_cognitive("function f(a,b,c){ if(a || b && c){} }", 3);
    assert!(d2.is_empty(), "case 6: at threshold 3 should not report");
}

#[test]
fn cognitive_complexity_matrix_7_nested_loops() {
    // for(;;){ while(true){} }  → for(1) + while(1+1=2) = 3
    let d = scan_cognitive("function f(){ for(;;){ while(true){} } }", 2);
    assert_eq!(d.len(), 1, "case 7: expected 1 report at threshold 2");
    let d2 = scan_cognitive("function f(){ for(;;){ while(true){} } }", 3);
    assert!(d2.is_empty(), "case 7: at threshold 3 should not report");
}

#[test]
fn cognitive_complexity_matrix_8_catch() {
    // try{}catch(e){}  → catch(1+0=1) = 1
    let d = scan_cognitive("function f(){ try{}catch(e){} }", 0);
    assert_eq!(d.len(), 1, "case 8: expected 1 report");
    let d2 = scan_cognitive("function f(){ try{}catch(e){} }", 1);
    assert!(d2.is_empty(), "case 8: at threshold 1 should not report");
}

#[test]
fn cognitive_complexity_matrix_9_ternary() {
    // return a ? 1 : 2  → ternary(1+0=1) = 1
    let d = scan_cognitive("function f(a){ return a ? 1 : 2; }", 0);
    assert_eq!(d.len(), 1, "case 9: expected 1 report");
    let d2 = scan_cognitive("function f(a){ return a ? 1 : 2; }", 1);
    assert!(d2.is_empty(), "case 9: at threshold 1 should not report");
}

#[test]
fn cognitive_complexity_matrix_10_continue_label() {
    // for(;;){ if(x) continue LBL; }  → for(1)+if(2)+continue-label(1)=4
    let d = scan_cognitive("function f(){ LBL: for(;;){ if(x) continue LBL; } }", 3);
    assert_eq!(d.len(), 1, "case 10: expected 1 report at threshold 3");
    let d2 = scan_cognitive("function f(){ LBL: for(;;){ if(x) continue LBL; } }", 4);
    assert!(d2.is_empty(), "case 10: at threshold 4 should not report");
}

#[test]
fn cognitive_complexity_matrix_11_empty_function() {
    // function f(){}  → 0; threshold 0 → not > 0 → no report
    let d = scan_cognitive("function f(){}", 0);
    assert!(d.is_empty(), "case 11: empty function should have score 0");
}

#[test]
fn cognitive_complexity_default_threshold_is_15() {
    // A function with score 15 should NOT be reported under the default
    // threshold of 15 (15 is not > 15).
    // Build a function with score exactly 15: 5 sequential if(a&&b){} → 5*(1+1)=10
    // plus 5 more ifs at top level: total 10 ifs + 5 operators = 10+5 = 15
    let src = "function f(a,b){ if(a&&b){} if(a&&b){} if(a&&b){} if(a&&b){} if(a&&b){} \
               if(a){} if(a){} if(a){} if(a){} if(a){} }";
    let d = scan("cognitive-complexity", src);
    assert!(
        d.is_empty(),
        "score 15 should not be reported at default threshold 15"
    );
}

#[test]
fn cognitive_complexity_nested_function_accrues_to_outer() {
    // Inner function's body adds to outer total at nesting+1.
    // function outer(){ function inner(a){ if(a){} } }
    // inner is a nested function at nesting=1; if(a) inside at nesting=2 → 1+2=3 > threshold 2
    let d = scan_cognitive(
        "function outer(){ function inner(a){ if(a){ if(a){} } } }",
        2,
    );
    assert_eq!(d.len(), 1, "nested fn complexity accrues to outer");
}

// expression-complexity

#[test]
fn expression_complexity_exceeds_default_threshold() {
    // 4 logical && operators: a&&b&&c&&d&&e → 4 > default threshold 3 → 1 diagnostic
    let source = "const x = a && b && c && d && e;";
    let diagnostics = scan("expression-complexity", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "expression-complexity");
    assert_eq!(diagnostics[0].message_id, "expressionComplexity");
}

#[test]
fn expression_complexity_at_threshold_no_report() {
    // 3 logical && operators: a&&b&&c&&d → 3 is not > default threshold 3 → 0 diagnostics
    let source = "const x = a && b && c && d;";
    let diagnostics = scan("expression-complexity", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn expression_complexity_custom_threshold_reports() {
    // 3 operators, threshold 2: 3 > 2 → 1 diagnostic
    let mut options = options_for("expression-complexity");
    options.expression_complexity_threshold = 2;
    let source = "const x = a && b && c && d;";
    let diagnostics = scan_sonarjs(source, "sample.ts", &options);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "expression-complexity");
}

#[test]
fn expression_complexity_resets_at_function_boundary() {
    // outer expression: "a && <fn_call>" → 1 operator ≤ 3 → no report for outer
    // inner function body: "c && d && e && f && g" → 4 operators > 3 → 1 report
    let source = "const x = a && function() { return c && d && e && f && g; }();";
    let diagnostics = scan("expression-complexity", source);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn expression_complexity_mixes_logical_and_ternary() {
    // a || b && c ? d : e → the ternary (?:) wraps (a || b && c);
    // outer context sees: ternary=1, then inner logical chain: ||=1 (nesting=2), &&=1 (nesting=3)
    // actually the ternary is the outermost (nesting=1 when entered); the || and && push it to 3
    // total count in one chain: 3 operators (ternary + || + &&) → 3 is not > 3 → no report
    let source = "const x = a || b && c ? d : e;";
    let diagnostics = scan("expression-complexity", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn expression_complexity_uses_default_threshold_when_unset() {
    // default threshold is 3; a top-level expression with exactly 3 operators must not fire
    let source = "const x = a && b || c && d;";
    let diagnostics = scan("expression-complexity", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn prefer_regexp_exec_reports_match_with_non_global_regex_literal() {
    let diagnostics = scan("prefer-regexp-exec", "const result = str.match(/foo/u);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "prefer-regexp-exec");
    assert_eq!(diagnostics[0].message_id, "preferRegExpExec");
}

#[test]
fn prefer_regexp_exec_reports_member_receiver_match() {
    let diagnostics = scan(
        "prefer-regexp-exec",
        "const result = object.value.match(/bar/);",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "preferRegExpExec");
}

#[test]
fn prefer_regexp_exec_does_not_report_global_regex_literal() {
    let diagnostics = scan("prefer-regexp-exec", "const result = str.match(/foo/gu);");
    assert!(diagnostics.is_empty());
}

#[test]
fn prefer_regexp_exec_does_not_report_dynamic_pattern() {
    let diagnostics = scan("prefer-regexp-exec", "const result = str.match(pattern);");
    assert!(diagnostics.is_empty());
}

#[test]
fn prefer_regexp_exec_does_not_report_unrelated_method() {
    let diagnostics = scan(
        "prefer-regexp-exec",
        "const result = str.replace(/foo/u, 'bar');",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn prefer_regexp_exec_does_not_report_extra_match_arguments() {
    let diagnostics = scan(
        "prefer-regexp-exec",
        "const result = str.match(/foo/u, extra);",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn no_fallthrough_reports_case_without_terminating_jump() {
    let source = "switch (x) { case 1: doWork(); case 2: done(); break; }";
    let diagnostics = scan("no-fallthrough", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-fallthrough");
    assert_eq!(diagnostics[0].message_id, "noFallthrough");
}

#[test]
fn no_fallthrough_allows_break_return_throw_and_continue() {
    let source = "while (ok) { switch (x) { case 1: break; case 2: return; case 3: throw err; case 4: continue; case 5: done(); } }";
    let diagnostics = scan("no-fallthrough", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_fallthrough_allows_empty_grouped_case() {
    let source = "switch (x) { case 1: case 2: doWork(); break; }";
    let diagnostics = scan("no-fallthrough", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_fallthrough_allows_intentional_comment() {
    let source = "switch (x) { case 1: doWork(); // falls through\ncase 2: done(); break; }";
    let diagnostics = scan("no-fallthrough", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_fallthrough_allows_if_else_when_both_paths_terminate() {
    let source = "switch (x) { case 1: if (ok) { return; } else { throw err; } case 2: done(); }";
    let diagnostics = scan("no-fallthrough", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_fallthrough_reports_if_without_else() {
    let source = "switch (x) { case 1: if (ok) { break; } case 2: done(); }";
    let diagnostics = scan("no-fallthrough", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noFallthrough");
}

#[test]
fn no_fallthrough_reports_labeled_break_conservatively() {
    let source = "switch (x) { case 1: block: { break block; } case 2: done(); }";
    let diagnostics = scan("no-fallthrough", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noFallthrough");
}

// no-commented-code

#[test]
fn no_commented_code_flags_line_comment_with_variable_declaration() {
    let source = "// const x = 1;";
    let diagnostics = scan("no-commented-code", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-commented-code");
    assert_eq!(diagnostics[0].message_id, "commentedCode");
}

#[test]
fn no_commented_code_flags_block_comment_with_if_statement() {
    // An if statement with an assignment body is valid at module top level
    // and ends with "}" — a strong code signal.
    let source = "/* if (cond) { doSomething(); } */";
    let diagnostics = scan("no-commented-code", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-commented-code");
    assert_eq!(diagnostics[0].message_id, "commentedCode");
}

#[test]
fn no_commented_code_does_not_flag_prose_comment() {
    let source = "// This returns the user name";
    let diagnostics = scan("no-commented-code", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_commented_code_does_not_flag_jsdoc_comment() {
    let source = "/** @param x */";
    let diagnostics = scan("no-commented-code", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_commented_code_does_not_flag_todo_comment() {
    let source = "// TODO: fix";
    let diagnostics = scan("no-commented-code", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_commented_code_does_not_flag_url_comment() {
    let source = "// see https://example.com";
    let diagnostics = scan("no-commented-code", source);
    assert!(diagnostics.is_empty());
}

// destructuring-assignment-syntax

#[test]
fn destructuring_assignment_syntax_reports_second_consecutive_extraction() {
    let source = "const a = obj.a;\nconst b = obj.b;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "destructuring-assignment-syntax");
    assert_eq!(diagnostics[0].message_id, "useDestructuring");
}

#[test]
fn destructuring_assignment_syntax_reports_each_declaration_after_first_in_group() {
    let source = "const a = obj.a;\nconst b = obj.b;\nconst c = obj.c;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].message_id, "useDestructuring");
    assert_eq!(diagnostics[1].message_id, "useDestructuring");
}

#[test]
fn destructuring_assignment_syntax_does_not_report_lone_extraction() {
    let source = "const a = obj.a;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn destructuring_assignment_syntax_does_not_report_when_binding_differs_from_property() {
    let source = "const x = obj.a;\nconst y = obj.b;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn destructuring_assignment_syntax_does_not_report_chained_source() {
    let source = "const a = foo.bar.a;\nconst b = foo.bar.b;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn destructuring_assignment_syntax_does_not_report_different_base_objects() {
    let source = "const a = foo.a;\nconst b = bar.b;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn destructuring_assignment_syntax_does_not_report_non_consecutive_declarations() {
    let source = "const a = obj.a;\ndoSomething();\nconst b = obj.b;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn destructuring_assignment_syntax_does_not_report_computed_member_access() {
    let source = "const a = obj['a'];\nconst b = obj['b'];";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn destructuring_assignment_syntax_works_in_function_body() {
    let source = "function f() { const a = obj.a;\nconst b = obj.b; }";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useDestructuring");
}

#[test]
fn destructuring_assignment_syntax_works_with_let_keyword() {
    let source = "let a = obj.a;\nlet b = obj.b;";
    let diagnostics = scan("destructuring-assignment-syntax", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "useDestructuring");
}

#[test]
fn no_element_overwrite_reports_consecutive_numeric_index_writes() {
    let source = "var a = [];\na[0] = 1;\na[0] = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-element-overwrite");
    assert_eq!(diagnostics[0].message_id, "elementOverwrite");
}

#[test]
fn no_element_overwrite_reports_consecutive_string_key_writes() {
    let source = "var m = {};\nm[\"x\"] = 1;\nm[\"x\"] = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "elementOverwrite");
}

#[test]
fn no_element_overwrite_reports_consecutive_static_prop_writes() {
    let source = "var obj = {};\nobj.x = 1;\nobj.x = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "elementOverwrite");
}

#[test]
fn no_element_overwrite_does_not_report_different_indices() {
    let source = "var a = [];\na[0] = 1;\na[1] = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_element_overwrite_does_not_report_when_intervening_statement_present() {
    let source = "var a = [];\na[0] = 1;\nfoo();\na[0] = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_element_overwrite_does_not_report_read_modify_write() {
    let source = "var a = [];\na[0] = 1;\na[0] = a[0] + 1;";
    let diagnostics = scan("no-element-overwrite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_element_overwrite_does_not_report_different_static_properties() {
    let source = "var obj = {};\nobj.x = 1;\nobj.y = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_element_overwrite_does_not_report_variable_computed_key() {
    let source = "var a = [];\na[i] = 1;\na[i] = 2;";
    let diagnostics = scan("no-element-overwrite", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_element_overwrite_works_in_function_body() {
    let source = "function f() { var a = []; a[0] = 1; a[0] = 2; }";
    let diagnostics = scan("no-element-overwrite", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "elementOverwrite");
}

#[test]
fn no_redundant_assignments_reports_self_assignment() {
    let source = "x = x;";
    let diagnostics = scan("no-redundant-assignments", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-redundant-assignments");
    assert_eq!(diagnostics[0].message_id, "redundantAssignment");
}

#[test]
fn no_redundant_assignments_reports_adjacent_dead_reassignment() {
    let source = "let y = 0;\ny = 1;\ny = 2;";
    let diagnostics = scan("no-redundant-assignments", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-redundant-assignments");
    assert_eq!(diagnostics[0].message_id, "redundantAssignment");
}

#[test]
fn no_redundant_assignments_does_not_report_different_identifiers() {
    let source = "x = y;";
    let diagnostics = scan("no-redundant-assignments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_redundant_assignments_does_not_report_read_modify_write() {
    let source = "let x = 1;\nx = x + 1;";
    let diagnostics = scan("no-redundant-assignments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_redundant_assignments_does_not_report_when_intervening_statement_present() {
    let source = "let x = 1;\nfoo();\nx = 2;";
    let diagnostics = scan("no-redundant-assignments", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_redundant_assignments_works_in_function_body() {
    let source = "function f() { let x = 0; x = 1; x = 2; }";
    let diagnostics = scan("no-redundant-assignments", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantAssignment");
}

#[test]
fn no_unused_collection_reports_array_only_written_via_push() {
    let source = "const a = [];\na.push(1);\na.push(2);";
    let diagnostics = scan("no-unused-collection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unused-collection");
    assert_eq!(diagnostics[0].message_id, "unusedCollection");
}

#[test]
fn no_unused_collection_reports_map_only_written_via_set() {
    let source = "const m = new Map();\nm.set('k', 1);";
    let diagnostics = scan("no-unused-collection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unused-collection");
    assert_eq!(diagnostics[0].message_id, "unusedCollection");
}

#[test]
fn no_unused_collection_does_not_report_when_array_is_returned() {
    let source = "function f() { const a = [];\na.push(1);\nreturn a; }";
    let diagnostics = scan("no-unused-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_collection_does_not_report_when_array_is_passed_to_function() {
    let source = "const a = [];\na.push(1);\nconsole.log(a);";
    let diagnostics = scan("no-unused-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_collection_does_not_report_when_length_is_read() {
    let source = "const a = [];\na.push(1);\nconst b = a.length;";
    let diagnostics = scan("no-unused-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_collection_does_not_report_when_array_has_initial_elements_and_is_passed() {
    let source = "const a = [1, 2];\nfoo(a);";
    let diagnostics = scan("no-unused-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unused_collection_does_not_report_when_no_write_references() {
    let source = "const a = [];";
    let diagnostics = scan("no-unused-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_reports_array_read_but_never_populated() {
    let source = "const a = [];\nfunction f() { return a.length; }";
    let diagnostics = scan("no-empty-collection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-empty-collection");
    assert_eq!(diagnostics[0].message_id, "emptyCollection");
}

#[test]
fn no_empty_collection_reports_map_queried_but_never_populated() {
    let source = "const m = new Map();\nfunction f(k) { return m.has(k); }";
    let diagnostics = scan("no-empty-collection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyCollection");
}

#[test]
fn no_empty_collection_reports_indexed_read_but_never_populated() {
    let source = "const a = [];\nfunction f() { return a[0]; }";
    let diagnostics = scan("no-empty-collection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyCollection");
}

#[test]
fn no_empty_collection_does_not_report_when_populated_via_push() {
    let source = "const a = [];\na.push(1);\nfunction f() { return a.length; }";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_does_not_report_when_populated_via_index_assignment() {
    let source = "const a = [];\na[0] = 1;\nfunction f() { return a[0]; }";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_does_not_report_when_map_populated_via_set() {
    let source = "const m = new Map();\nm.set('k', 1);\nfunction f(k) { return m.has(k); }";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_does_not_report_when_passed_to_function() {
    let source = "const a = [];\nfill(a);\nfunction f() { return a.length; }";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_does_not_report_when_never_read() {
    let source = "const a = [];";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_does_not_report_when_initially_populated() {
    let source = "const a = [1];\nfunction f() { return a.length; }";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_does_not_report_object_literal() {
    let source = "const o = {};\nfunction f() { return o.x; }";
    let diagnostics = scan("no-empty-collection", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_empty_collection_reports_iterated_via_for_of() {
    let source = "const a = [];\nfor (const x of a) { use(x); }";
    let diagnostics = scan("no-empty-collection", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "emptyCollection");
}

#[test]
fn no_redundant_parentheses_reports_nested_double_parentheses() {
    let source = "const x = ((1));";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-redundant-parentheses");
    assert_eq!(diagnostics[0].message_id, "redundantParentheses");
}

#[test]
fn no_redundant_parentheses_reports_nested_around_identifier() {
    let source = "const y = ((a));";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantParentheses");
}

#[test]
fn no_redundant_parentheses_reports_twice_for_triple_nesting() {
    let source = "const z = (((a)));";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn no_redundant_parentheses_does_not_report_single_pair() {
    let source = "const x = (1);";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_redundant_parentheses_does_not_report_precedence_grouping() {
    let source = "const r = (a + b) * c;";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_redundant_parentheses_does_not_report_distinct_single_pairs() {
    let source = "f((a), (b));";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_redundant_parentheses_reports_nested_inside_call_argument() {
    let source = "f(((a)));";
    let diagnostics = scan("no-redundant-parentheses", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "redundantParentheses");
}

#[test]
fn bool_param_default_reports_optional_boolean_function_param() {
    let diagnostics = scan("bool-param-default", "function f(flag?: boolean) {}");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "bool-param-default");
    assert_eq!(diagnostics[0].message_id, "boolParamDefault");
}

#[test]
fn bool_param_default_does_not_report_required_boolean_param() {
    let diagnostics = scan("bool-param-default", "function f(flag: boolean) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_param_with_default() {
    let diagnostics = scan("bool-param-default", "function f(flag: boolean = false) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_union_annotation() {
    let diagnostics = scan(
        "bool-param-default",
        "function f(flag?: boolean | undefined) {}",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_reports_optional_boolean_arrow_param() {
    let diagnostics = scan("bool-param-default", "const g = (flag?: boolean) => {};");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "boolParamDefault");
}

#[test]
fn bool_param_default_reports_optional_boolean_method_param() {
    let diagnostics = scan("bool-param-default", "class C { m(flag?: boolean) {} }");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "boolParamDefault");
}

#[test]
fn bool_param_default_does_not_report_untyped_optional_param() {
    let diagnostics = scan("bool-param-default", "function f(flag?) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_boolean_array_annotation() {
    let diagnostics = scan("bool-param-default", "function f(flags?: boolean[]) {}");
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_interface_method_signature() {
    let diagnostics = scan(
        "bool-param-default",
        "interface I { m(flag?: boolean): void; }",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_type_alias_function_type() {
    let diagnostics = scan("bool-param-default", "type T = (flag?: boolean) => void;");
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_type_literal_method_signature() {
    let diagnostics = scan(
        "bool-param-default",
        "type O = { m(flag?: boolean): void };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_does_not_report_abstract_method() {
    let diagnostics = scan(
        "bool-param-default",
        "abstract class C { abstract m(flag?: boolean): void; }",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_reports_only_overload_implementation_not_signature() {
    // The bodiless overload signature must NOT be flagged; only the concrete
    // implementation (which has a body) is reported.
    let diagnostics = scan(
        "bool-param-default",
        "function f(flag?: boolean): void; function f(flag?: boolean) { return flag; }",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "boolParamDefault");
}

#[test]
fn bool_param_default_does_not_report_ambient_declare_function() {
    let diagnostics = scan(
        "bool-param-default",
        "declare function f(flag?: boolean): void;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn bool_param_default_reports_concrete_method_with_body() {
    let diagnostics = scan(
        "bool-param-default",
        "class C { m(flag?: boolean) { return flag; } }",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "boolParamDefault");
}

#[test]
fn post_message_reports_wildcard_target_origin() {
    let diagnostics = scan("post-message", r#"win.postMessage(data, "*");"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "post-message");
    assert_eq!(diagnostics[0].message_id, "postMessage");
}

#[test]
fn post_message_reports_wildcard_regardless_of_receiver_type() {
    let diagnostics = scan("post-message", r#"el.postMessage(x, "*");"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "postMessage");
}

#[test]
fn post_message_does_not_report_specific_target_origin() {
    let diagnostics = scan(
        "post-message",
        r#"win.postMessage(data, "https://example.com");"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn post_message_does_not_report_single_argument() {
    let diagnostics = scan("post-message", "worker.postMessage(data);");
    assert!(diagnostics.is_empty());
}

#[test]
fn post_message_does_not_report_variable_target_origin() {
    let diagnostics = scan("post-message", "win.postMessage(data, origin);");
    assert!(diagnostics.is_empty());
}

#[test]
fn post_message_does_not_report_array_second_argument() {
    let diagnostics = scan("post-message", "worker.postMessage(data, [buffer]);");
    assert!(diagnostics.is_empty());
}

#[test]
fn in_operator_type_error_reports_string_right_operand() {
    let diagnostics = scan("in-operator-type-error", r#"const r = "a" in "s";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "in-operator-type-error");
    assert_eq!(diagnostics[0].message_id, "inOperatorTypeError");
}

#[test]
fn in_operator_type_error_reports_numeric_right_operand() {
    let diagnostics = scan("in-operator-type-error", "const r = 0 in 5;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "in-operator-type-error");
    assert_eq!(diagnostics[0].message_id, "inOperatorTypeError");
}

#[test]
fn in_operator_type_error_reports_null_right_operand() {
    let diagnostics = scan("in-operator-type-error", "const r = k in null;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "in-operator-type-error");
    assert_eq!(diagnostics[0].message_id, "inOperatorTypeError");
}

#[test]
fn in_operator_type_error_does_not_report_identifier_right_operand() {
    let diagnostics = scan("in-operator-type-error", r#"const r = "x" in obj;"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn in_operator_type_error_does_not_report_object_literal_right_operand() {
    let diagnostics = scan("in-operator-type-error", r#"const r = "x" in {};"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn in_operator_type_error_does_not_report_array_literal_right_operand() {
    let diagnostics = scan("in-operator-type-error", r#"const r = "x" in [];"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn in_operator_type_error_does_not_report_identifier_both_operands() {
    let diagnostics = scan("in-operator-type-error", "const r = key in foo;");
    assert!(diagnostics.is_empty());
}

#[test]
fn different_types_comparison_reports_string_vs_number() {
    let diagnostics = scan("different-types-comparison", r#""a" === 1;"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "different-types-comparison");
    assert_eq!(diagnostics[0].message_id, "differentTypesComparison");
}

#[test]
fn different_types_comparison_reports_null_vs_number() {
    let diagnostics = scan("different-types-comparison", "null === 0;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "differentTypesComparison");
}

#[test]
fn different_types_comparison_reports_boolean_vs_string() {
    let diagnostics = scan("different-types-comparison", r#"true === "x";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "differentTypesComparison");
}

#[test]
fn different_types_comparison_reports_number_vs_string_strict_inequality() {
    let diagnostics = scan("different-types-comparison", r#"5 !== "5";"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "differentTypesComparison");
}

#[test]
fn different_types_comparison_reports_bigint_vs_number() {
    let diagnostics = scan("different-types-comparison", "1n === 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "differentTypesComparison");
}

#[test]
fn different_types_comparison_does_not_report_same_kind_numbers() {
    let diagnostics = scan("different-types-comparison", "1 === 2;");
    assert!(diagnostics.is_empty());
}

#[test]
fn different_types_comparison_does_not_report_same_kind_strings() {
    let diagnostics = scan("different-types-comparison", r#""a" === "b";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn different_types_comparison_does_not_report_non_literal_operand() {
    let diagnostics = scan("different-types-comparison", "x === 1;");
    assert!(diagnostics.is_empty());
}

#[test]
fn different_types_comparison_does_not_report_loose_equality() {
    let diagnostics = scan("different-types-comparison", r#"1 == "1";"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn operation_returning_nan_reports_arrow_function_operand() {
    let diagnostics = scan("operation-returning-nan", "const x = (() => {}) * 2;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "operation-returning-nan");
    assert_eq!(diagnostics[0].message_id, "operationReturningNan");
}

#[test]
fn operation_returning_nan_reports_function_expression_operand() {
    let diagnostics = scan("operation-returning-nan", "const x = (function(){}) - 1;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "operation-returning-nan");
    assert_eq!(diagnostics[0].message_id, "operationReturningNan");
}

#[test]
fn operation_returning_nan_reports_empty_object_operand() {
    let diagnostics = scan("operation-returning-nan", "const x = ({}) * 2;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "operationReturningNan");
}

#[test]
fn operation_returning_nan_reports_plain_data_object_operand() {
    let diagnostics = scan("operation-returning-nan", "const x = ({a:1}) / 2;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "operationReturningNan");
}

#[test]
fn operation_returning_nan_does_not_report_object_with_value_of() {
    // A custom valueOf can produce a finite number, so the object is not plain.
    let diagnostics = scan(
        "operation-returning-nan",
        "const x = ({valueOf(){return 5}}) * 2;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn operation_returning_nan_does_not_report_object_with_to_string() {
    let diagnostics = scan(
        "operation-returning-nan",
        r#"const x = ({toString(){return "5"}}) * 2;"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn operation_returning_nan_does_not_report_array_operand() {
    // [] * 2 === 0 and [5] * 2 === 10 — arrays are not reliably NaN.
    let diagnostics = scan("operation-returning-nan", "const x = [] * 2;");
    assert!(diagnostics.is_empty());
}

#[test]
fn operation_returning_nan_does_not_report_identifier_operand() {
    let diagnostics = scan("operation-returning-nan", "const x = y * 2;");
    assert!(diagnostics.is_empty());
}

#[test]
fn operation_returning_nan_does_not_report_addition_operator() {
    // The + operator is excluded entirely (string concatenation / coercion).
    let diagnostics = scan("operation-returning-nan", r#"const x = "a" + {};"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn operation_returning_nan_does_not_report_numeric_operands() {
    let diagnostics = scan("operation-returning-nan", "const x = 1 + 2;");
    assert!(diagnostics.is_empty());
}

#[test]
fn production_debug_reports_debugger_in_function() {
    let diagnostics = scan("production-debug", "function f(){ debugger; }");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "production-debug");
    assert_eq!(diagnostics[0].message_id, "productionDebug");
}

#[test]
fn production_debug_reports_debugger_in_if_block() {
    let diagnostics = scan("production-debug", "if (x) { debugger; }");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "productionDebug");
}

#[test]
fn production_debug_reports_top_level_debugger() {
    let diagnostics = scan("production-debug", "debugger;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "productionDebug");
}

#[test]
fn production_debug_does_not_report_console_log() {
    let diagnostics = scan("production-debug", "console.log(1)");
    assert!(diagnostics.is_empty());
}

#[test]
fn production_debug_does_not_report_alert() {
    let diagnostics = scan("production-debug", "alert(1)");
    assert!(diagnostics.is_empty());
}

#[test]
fn production_debug_does_not_report_ordinary_return() {
    let diagnostics = scan("production-debug", "function f(){ return 1; }");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_secrets_reports_apikey_variable_declarator() {
    let source = "const apiKey = \"AKIA1234567890ABCD\";";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-hardcoded-secrets");
    assert_eq!(diagnostics[0].message_id, "hardcodedSecret");
}

#[test]
fn no_hardcoded_secrets_reports_token_variable_declarator() {
    let source = "const token = \"ghp_realLongTokenValue123\";";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedSecret");
}

#[test]
fn no_hardcoded_secrets_reports_object_property() {
    let source = "const x = { secret: \"s3cr3tVal\" };";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "hardcodedSecret");
}

#[test]
fn no_hardcoded_secrets_does_not_report_partial_name_match() {
    let source = "const tokenizer = \"x\";";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_secrets_does_not_report_empty_value() {
    let source = "const apiKey = \"\";";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_secrets_does_not_report_non_literal_init() {
    let source = "const apiKey = process.env.KEY;";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_hardcoded_secrets_does_not_report_placeholder_value() {
    let source = "const apiKey = \"token\";";
    let diagnostics = scan("no-hardcoded-secrets", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn concise_regex_reports_digit_class() {
    let diagnostics = scan("concise-regex", "const r = /[0-9]/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "concise-regex");
    assert_eq!(diagnostics[0].message_id, "conciseRegex");
}

#[test]
fn concise_regex_reports_negated_digit_class() {
    let diagnostics = scan("concise-regex", "const r = /[^0-9]/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "conciseRegex");
}

#[test]
fn concise_regex_reports_word_class() {
    let diagnostics = scan("concise-regex", "const r = /[A-Za-z0-9_]/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "conciseRegex");
}

#[test]
fn concise_regex_reports_word_class_any_order() {
    let diagnostics = scan("concise-regex", "const r = /[_0-9a-zA-Z]/;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "conciseRegex");
}

#[test]
fn concise_regex_does_not_report_extra_member() {
    let diagnostics = scan("concise-regex", "const r = /[0-9a]/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn concise_regex_does_not_report_other_range() {
    let diagnostics = scan("concise-regex", "const r = /[a-z]/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn concise_regex_does_not_report_word_class_missing_underscore() {
    let diagnostics = scan("concise-regex", "const r = /[A-Za-z0-9]/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn concise_regex_does_not_report_already_concise() {
    let diagnostics = scan("concise-regex", "const r = /\\d/;");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_misleading_character_class_reports_astral_char_in_class() {
    let diagnostics = scan("no-misleading-character-class", "/[👍]/");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-misleading-character-class");
    assert_eq!(diagnostics[0].message_id, "misleadingCharacterClass");
}

#[test]
fn no_misleading_character_class_reports_astral_char_among_bmp() {
    let diagnostics = scan("no-misleading-character-class", "/[a👍b]/");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "misleadingCharacterClass");
}

#[test]
fn no_misleading_character_class_does_not_report_with_u_flag() {
    let diagnostics = scan("no-misleading-character-class", "/[👍]/u");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_misleading_character_class_does_not_report_with_v_flag() {
    let diagnostics = scan("no-misleading-character-class", "/[👍]/v");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_misleading_character_class_does_not_report_bmp_only_class() {
    let diagnostics = scan("no-misleading-character-class", "/[abc]/");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_misleading_character_class_does_not_report_astral_outside_class() {
    let diagnostics = scan("no-misleading-character-class", "/👍/");
    assert!(diagnostics.is_empty());
}

#[test]
fn slow_regex_reports_plus_over_plus_group() {
    let diagnostics = scan("slow-regex", "/(a+)+/");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "slow-regex");
    assert_eq!(diagnostics[0].message_id, "slowRegex");
}

#[test]
fn slow_regex_reports_star_over_star_group() {
    let diagnostics = scan("slow-regex", "/(a*)*/");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "slowRegex");
}

#[test]
fn slow_regex_reports_plus_over_dotstar_group() {
    let diagnostics = scan("slow-regex", "/(.*)+$/");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "slowRegex");
}

#[test]
fn slow_regex_reports_plus_over_digit_plus_group() {
    let diagnostics = scan("slow-regex", "/(\\d+)+/");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "slowRegex");
}

#[test]
fn slow_regex_does_not_report_unquantified_inner_group() {
    let diagnostics = scan("slow-regex", "/(ab)+/");
    assert!(diagnostics.is_empty());
}

#[test]
fn slow_regex_does_not_report_single_quantifier() {
    let diagnostics = scan("slow-regex", "/a+/");
    assert!(diagnostics.is_empty());
}

#[test]
fn slow_regex_does_not_report_bounded_inner_quantifier() {
    let diagnostics = scan("slow-regex", "/(a{2,5})+/");
    assert!(diagnostics.is_empty());
}

#[test]
fn slow_regex_does_not_report_bounded_outer_quantifier() {
    let diagnostics = scan("slow-regex", "/(a+){2,3}/");
    assert!(diagnostics.is_empty());
}

#[test]
fn web_sql_database_reports_global_open_database_call() {
    let diagnostics = scan(
        "web-sql-database",
        r#"openDatabase("db", "1.0", "desc", 1024);"#,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "web-sql-database");
    assert_eq!(diagnostics[0].message_id, "webSqlDatabase");
}

#[test]
fn web_sql_database_reports_window_open_database_call() {
    let diagnostics = scan("web-sql-database", r#"window.openDatabase("db");"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "webSqlDatabase");
}

#[test]
fn web_sql_database_reports_member_open_database_regardless_of_receiver() {
    let diagnostics = scan("web-sql-database", "db.openDatabase();");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "webSqlDatabase");
}

#[test]
fn web_sql_database_does_not_report_unrelated_call() {
    let diagnostics = scan("web-sql-database", "foo();");
    assert!(diagnostics.is_empty());
}

#[test]
fn web_sql_database_does_not_report_property_access_without_call() {
    let diagnostics = scan("web-sql-database", "const x = openDatabase;");
    assert!(diagnostics.is_empty());
}

#[test]
fn web_sql_database_does_not_report_unrelated_method_call() {
    let diagnostics = scan("web-sql-database", "obj.query();");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_intrusive_permissions_reports_get_current_position() {
    let diagnostics = scan(
        "no-intrusive-permissions",
        "navigator.geolocation.getCurrentPosition(cb);",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-intrusive-permissions");
}

#[test]
fn no_intrusive_permissions_reports_watch_position() {
    let diagnostics = scan(
        "no-intrusive-permissions",
        "navigator.geolocation.watchPosition(cb);",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn no_intrusive_permissions_reports_notification_request_permission() {
    let diagnostics = scan(
        "no-intrusive-permissions",
        "Notification.requestPermission();",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn no_intrusive_permissions_reports_permissions_query() {
    let diagnostics = scan(
        "no-intrusive-permissions",
        r#"navigator.permissions.query({name:"geolocation"});"#,
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn no_intrusive_permissions_does_not_report_user_agent_access() {
    let diagnostics = scan(
        "no-intrusive-permissions",
        "const ua = navigator.userAgent;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn no_intrusive_permissions_does_not_report_wrong_object_chain() {
    let diagnostics = scan("no-intrusive-permissions", "foo.getCurrentPosition();");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_intrusive_permissions_does_not_report_bare_member_without_call() {
    let diagnostics = scan(
        "no-intrusive-permissions",
        "const g = navigator.geolocation;",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn encryption_secure_mode_reports_cbc_member_call() {
    let diagnostics = scan(
        "encryption-secure-mode",
        r#"crypto.createCipheriv("aes-128-cbc", k, iv);"#,
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn encryption_secure_mode_reports_ecb_identifier_call() {
    let diagnostics = scan(
        "encryption-secure-mode",
        r#"createCipheriv("AES-256-ECB", k, iv);"#,
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn encryption_secure_mode_does_not_report_gcm_mode() {
    let diagnostics = scan(
        "encryption-secure-mode",
        r#"crypto.createCipheriv("aes-256-gcm", k, iv);"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn encryption_secure_mode_does_not_report_wrong_callee() {
    let diagnostics = scan("encryption-secure-mode", r#"foo("aes-128-cbc");"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn encryption_secure_mode_does_not_report_dynamic_algorithm() {
    let diagnostics = scan(
        "encryption-secure-mode",
        "crypto.createCipheriv(algo, k, iv);",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unsafe_unzip_reports_extract_all_to() {
    let diagnostics = scan("no-unsafe-unzip", r#"zip.extractAllTo(".");"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-unsafe-unzip");
    assert_eq!(diagnostics[0].message_id, "unsafeUnzip");
}

#[test]
fn no_unsafe_unzip_reports_extract_all_to_on_new_expression() {
    let diagnostics = scan(
        "no-unsafe-unzip",
        r#"new AdmZip("f.zip").extractAllTo("./out");"#,
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn no_unsafe_unzip_does_not_report_extract_entry_to() {
    let diagnostics = scan("no-unsafe-unzip", r#"zip.extractEntryTo(e, ".");"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unsafe_unzip_does_not_report_generic_tar_x() {
    let diagnostics = scan("no-unsafe-unzip", r#"tar.x({ file: "f" });"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_unsafe_unzip_does_not_report_bare_call() {
    let diagnostics = scan("no-unsafe-unzip", "foo();");
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_timeout_reports_one_past_32_bit_max() {
    let diagnostics = scan("disabled-timeout", "this.timeout(2147483648);");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "disabled-timeout");
    assert_eq!(diagnostics[0].message_id, "disabledTimeout");
}

#[test]
fn disabled_timeout_reports_far_past_max() {
    let diagnostics = scan("disabled-timeout", "this.timeout(9999999999);");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn disabled_timeout_does_not_report_zero() {
    let diagnostics = scan("disabled-timeout", "this.timeout(0);");
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_timeout_does_not_report_value_within_range() {
    let diagnostics = scan("disabled-timeout", "this.timeout(5000);");
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_timeout_does_not_report_non_this_receiver() {
    let diagnostics = scan("disabled-timeout", "foo.timeout(2147483648);");
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_timeout_does_not_report_dynamic_argument() {
    let diagnostics = scan("disabled-timeout", "this.timeout(x);");
    assert!(diagnostics.is_empty());
}

#[test]
fn cookie_no_httponly_reports_direct_false() {
    let diagnostics = scan("cookie-no-httponly", "const c = { httpOnly: false };");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn cookie_no_httponly_reports_nested_cookie_config() {
    let diagnostics = scan(
        "cookie-no-httponly",
        "session({ cookie: { httpOnly: false } });",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn cookie_no_httponly_does_not_report_true() {
    let diagnostics = scan("cookie-no-httponly", "const c = { httpOnly: true };");
    assert!(diagnostics.is_empty());
}

#[test]
fn cookie_no_httponly_does_not_report_dynamic_value() {
    let diagnostics = scan("cookie-no-httponly", "const c = { httpOnly: x };");
    assert!(diagnostics.is_empty());
}

#[test]
fn cookie_no_httponly_does_not_report_other_key() {
    let diagnostics = scan("cookie-no-httponly", "const c = { secure: false };");
    assert!(diagnostics.is_empty());
}

#[test]
fn content_security_policy_reports_helmet_false() {
    let diagnostics = scan(
        "content-security-policy",
        "helmet({ contentSecurityPolicy: false });",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn content_security_policy_reports_direct_false() {
    let diagnostics = scan(
        "content-security-policy",
        "const x = { contentSecurityPolicy: false };",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn content_security_policy_does_not_report_true() {
    let diagnostics = scan(
        "content-security-policy",
        "helmet({ contentSecurityPolicy: true });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn content_security_policy_does_not_report_dynamic_value() {
    let diagnostics = scan(
        "content-security-policy",
        "const x = { contentSecurityPolicy: opts };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn content_security_policy_does_not_report_other_key() {
    let diagnostics = scan("content-security-policy", "const x = { csp: false };");
    assert!(diagnostics.is_empty());
}

#[test]
fn certificate_transparency_reports_helmet_expect_ct_false() {
    let diagnostics = scan("certificate-transparency", "helmet({ expectCt: false })");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "certificate-transparency");
    assert_eq!(diagnostics[0].message_id, "certificateTransparency");
}

#[test]
fn certificate_transparency_reports_direct_false() {
    let diagnostics = scan("certificate-transparency", "const x = { expectCt: false }");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn certificate_transparency_does_not_report_true() {
    let diagnostics = scan("certificate-transparency", "const x = { expectCt: true }");
    assert!(diagnostics.is_empty());
}

#[test]
fn certificate_transparency_does_not_report_dynamic_value() {
    let diagnostics = scan("certificate-transparency", "const x = { expectCt: o }");
    assert!(diagnostics.is_empty());
}

#[test]
fn certificate_transparency_does_not_report_other_key() {
    let diagnostics = scan("certificate-transparency", "const x = { other: false }");
    assert!(diagnostics.is_empty());
}

#[test]
fn csrf_reports_unsafe_method_mixed_with_safe() {
    let diagnostics = scan("csrf", "csrf({ ignoreMethods: [\"POST\",\"GET\"] });");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn csrf_reports_single_unsafe_method() {
    let diagnostics = scan("csrf", "csrf({ ignoreMethods: [\"PUT\"] });");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn csrf_does_not_report_only_safe_methods() {
    let diagnostics = scan(
        "csrf",
        "csrf({ ignoreMethods: [\"GET\",\"HEAD\",\"OPTIONS\"] });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn csrf_does_not_report_bare_call() {
    let diagnostics = scan("csrf", "csrf();");
    assert!(diagnostics.is_empty());
}

#[test]
fn csrf_does_not_report_without_ignore_methods() {
    let diagnostics = scan("csrf", "csrf({ cookie: true });");
    assert!(diagnostics.is_empty());
}

#[test]
fn csrf_does_not_report_wrong_callee() {
    let diagnostics = scan("csrf", "foo({ ignoreMethods: [\"POST\"] });");
    assert!(diagnostics.is_empty());
}

#[test]
fn file_permissions_reports_chmod_sync_others_rwx() {
    let diagnostics = scan("file-permissions", r#"fs.chmodSync("/x", 0o777);"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "file-permissions");
    assert_eq!(diagnostics[0].message_id, "weakFilePermissions");
}

#[test]
fn file_permissions_reports_async_chmod_with_callback() {
    let diagnostics = scan("file-permissions", r#"fs.chmod("/x", 0o666, cb);"#);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn file_permissions_reports_permissive_umask() {
    let diagnostics = scan("file-permissions", "process.umask(0o000);");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn file_permissions_reports_bare_umask_identifier() {
    let diagnostics = scan("file-permissions", "umask(0o022);");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn file_permissions_does_not_report_chmod_without_others_bits() {
    let diagnostics = scan("file-permissions", r#"fs.chmodSync("/x", 0o750);"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn file_permissions_does_not_report_restrictive_umask() {
    let diagnostics = scan("file-permissions", "process.umask(0o077);");
    assert!(diagnostics.is_empty());
}

#[test]
fn file_permissions_does_not_report_dynamic_mode() {
    let diagnostics = scan("file-permissions", r#"fs.chmodSync("/x", mode);"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn file_permissions_reports_any_receiver_chmod() {
    // The chmod-family check keys off the property name only, so an unrelated
    // receiver is still flagged (documented zero-FP trade-off).
    let diagnostics = scan("file-permissions", "foo.chmodSync(0o777);");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn file_uploads_reports_disk_storage_without_destination() {
    let diagnostics = scan("file-uploads", "multer.diskStorage({ filename: fn });");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "file-uploads");
    assert_eq!(diagnostics[0].message_id, "fileUploads");
}

#[test]
fn file_uploads_does_not_report_disk_storage_with_destination() {
    let diagnostics = scan(
        "file-uploads",
        r#"multer.diskStorage({ destination: "/up", filename: fn });"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn file_uploads_does_not_report_string_key_destination() {
    let diagnostics = scan(
        "file-uploads",
        r#"multer.diskStorage({ ["destination"]: d });"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn file_uploads_reports_any_receiver_disk_storage() {
    // The check keys off the distinctive `diskStorage` property name only, so an
    // aliased/unrelated receiver missing a destination is still flagged
    // (documented zero-FP trade-off).
    let diagnostics = scan("file-uploads", "foo.diskStorage({ filename: fn });");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn file_uploads_does_not_report_disk_storage_without_object_argument() {
    let diagnostics = scan("file-uploads", "multer.diskStorage();");
    assert!(diagnostics.is_empty());
}

#[test]
fn file_uploads_does_not_report_unrelated_call() {
    let diagnostics = scan("file-uploads", "bar();");
    assert!(diagnostics.is_empty());
}

#[test]
fn cors_reports_set_header_wildcard_origin() {
    let diagnostics = scan(
        "cors",
        r#"res.setHeader("Access-Control-Allow-Origin", "*");"#,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "cors");
    assert_eq!(diagnostics[0].message_id, "cors");
}

#[test]
fn cors_reports_set_header_wildcard_case_insensitive_name() {
    let diagnostics = scan(
        "cors",
        r#"res.setHeader("access-control-allow-origin", "*");"#,
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn cors_reports_cors_middleware_wildcard_origin() {
    let diagnostics = scan("cors", r#"cors({ origin: "*" });"#);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "cors");
}

#[test]
fn cors_reports_write_head_header_object_wildcard() {
    let diagnostics = scan(
        "cors",
        r#"res.writeHead(200, { "Access-Control-Allow-Origin": "*" });"#,
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn cors_does_not_report_set_header_specific_origin() {
    let diagnostics = scan(
        "cors",
        r#"res.setHeader("Access-Control-Allow-Origin", "https://ex.com");"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn cors_does_not_report_cors_middleware_specific_origin() {
    let diagnostics = scan("cors", r#"cors({ origin: "https://ex.com" });"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn cors_does_not_report_bare_cors_call() {
    let diagnostics = scan("cors", "cors();");
    assert!(diagnostics.is_empty());
}

#[test]
fn cors_does_not_report_unrelated_set_header() {
    let diagnostics = scan("cors", r#"res.setHeader("Content-Type", "x");"#);
    assert!(diagnostics.is_empty());
}

#[test]
fn cors_does_not_report_dynamic_set_header_origin() {
    let diagnostics = scan(
        "cors",
        r#"res.setHeader("Access-Control-Allow-Origin", origin);"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn dns_prefetching_reports_allow_true() {
    let diagnostics = scan(
        "dns-prefetching",
        "helmet.dnsPrefetchControl({ allow: true });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "dns-prefetching");
    assert_eq!(diagnostics[0].message_id, "dnsPrefetching");
}

#[test]
fn dns_prefetching_does_not_report_allow_false() {
    let diagnostics = scan(
        "dns-prefetching",
        "helmet.dnsPrefetchControl({ allow: false });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn dns_prefetching_does_not_report_no_args() {
    let diagnostics = scan("dns-prefetching", "helmet.dnsPrefetchControl();");
    assert!(diagnostics.is_empty());
}

#[test]
fn dns_prefetching_reports_any_receiver() {
    // The check keys off the distinctive `dnsPrefetchControl` method name only,
    // so an unrelated receiver is still flagged (documented zero-FP trade-off).
    let diagnostics = scan(
        "dns-prefetching",
        "foo.dnsPrefetchControl({ allow: true });",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn dns_prefetching_does_not_report_non_literal_allow() {
    let diagnostics = scan(
        "dns-prefetching",
        "helmet.dnsPrefetchControl({ allow: x });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn dns_prefetching_does_not_report_unrelated_callee() {
    let diagnostics = scan("dns-prefetching", "bar();");
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_auto_escaping_reports_handlebars_no_escape_true() {
    let diagnostics = scan(
        "disabled-auto-escaping",
        "Handlebars.compile(src, { noEscape: true });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "disabled-auto-escaping");
}

#[test]
fn disabled_auto_escaping_reports_mustache_escape_override() {
    let diagnostics = scan(
        "disabled-auto-escaping",
        "Mustache.escape = function (t) { return t; };",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "disabled-auto-escaping");
}

#[test]
fn disabled_auto_escaping_does_not_report_no_escape_false() {
    let diagnostics = scan(
        "disabled-auto-escaping",
        "Handlebars.compile(src, { noEscape: false });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_auto_escaping_does_not_report_non_literal_value() {
    let diagnostics = scan(
        "disabled-auto-escaping",
        "Handlebars.compile(src, { noEscape: x });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_auto_escaping_does_not_report_generic_html_key() {
    // `html: true` is too generic to flag without knowing the receiving API.
    let diagnostics = scan("disabled-auto-escaping", "md({ html: true });");
    assert!(diagnostics.is_empty());
}

#[test]
fn disabled_auto_escaping_does_not_report_other_escape_assignment() {
    let diagnostics = scan(
        "disabled-auto-escaping",
        "Other.escape = function (t) { return t; };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn hidden_files_reports_serve_static_allow() {
    let diagnostics = scan(
        "hidden-files",
        "serveStatic('public', { dotfiles: 'allow' });",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn hidden_files_reports_string_literal_key() {
    let diagnostics = scan("hidden-files", "const x = { 'dotfiles': 'allow' };");
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn hidden_files_does_not_report_ignore() {
    let diagnostics = scan("hidden-files", "const x = { dotfiles: 'ignore' };");
    assert!(diagnostics.is_empty());
}

#[test]
fn hidden_files_does_not_report_non_literal_value() {
    let diagnostics = scan("hidden-files", "const x = { dotfiles: x };");
    assert!(diagnostics.is_empty());
}

#[test]
fn hidden_files_does_not_report_other_key() {
    let diagnostics = scan("hidden-files", "const x = { other: 'allow' };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_granted_access_reports_public_read_write() {
    let diagnostics = scan(
        "aws-s3-bucket-granted-access",
        "new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PUBLIC_READ_WRITE });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-s3-bucket-granted-access");
}

#[test]
fn aws_s3_bucket_granted_access_reports_public_read() {
    let diagnostics = scan(
        "aws-s3-bucket-granted-access",
        "new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PUBLIC_READ });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-s3-bucket-granted-access");
}

#[test]
fn aws_s3_bucket_granted_access_reports_authenticated_read() {
    let diagnostics = scan(
        "aws-s3-bucket-granted-access",
        "new s3.Bucket(this, 'b', { accessControl: BucketAccessControl.AUTHENTICATED_READ });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-s3-bucket-granted-access");
}

#[test]
fn aws_s3_bucket_granted_access_does_not_report_private() {
    let diagnostics = scan(
        "aws-s3-bucket-granted-access",
        "new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PRIVATE });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_granted_access_does_not_report_non_member_value() {
    let diagnostics = scan(
        "aws-s3-bucket-granted-access",
        "new s3.Bucket(this, 'b', { accessControl: x });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_granted_access_does_not_report_other_key() {
    let diagnostics = scan(
        "aws-s3-bucket-granted-access",
        "new s3.Bucket(this, 'b', { other: BucketAccessControl.PUBLIC_READ });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_rds_unencrypted_databases_reports_cdk_construct_false() {
    let diagnostics = scan(
        "aws-rds-unencrypted-databases",
        "new DatabaseInstance(this, 'db', { storageEncrypted: false });",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn aws_rds_unencrypted_databases_reports_direct_false() {
    let diagnostics = scan(
        "aws-rds-unencrypted-databases",
        "const x = { storageEncrypted: false };",
    );
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn aws_rds_unencrypted_databases_does_not_report_true() {
    let diagnostics = scan(
        "aws-rds-unencrypted-databases",
        "const x = { storageEncrypted: true };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_rds_unencrypted_databases_does_not_report_dynamic_value() {
    let diagnostics = scan(
        "aws-rds-unencrypted-databases",
        "const x = { storageEncrypted: flag };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_rds_unencrypted_databases_does_not_report_other_key() {
    let diagnostics = scan(
        "aws-rds-unencrypted-databases",
        "const x = { encrypted: false };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_public_access_reports_member_any_principal() {
    let diagnostics = scan("aws-iam-public-access", "new iam.AnyPrincipal()");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-iam-public-access");
    assert_eq!(diagnostics[0].message_id, "iamPublicAccess");
}

#[test]
fn aws_iam_public_access_reports_bare_any_principal() {
    let diagnostics = scan("aws-iam-public-access", "new AnyPrincipal()");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "iamPublicAccess");
}

#[test]
fn aws_iam_public_access_does_not_report_account_root_principal() {
    let diagnostics = scan("aws-iam-public-access", "new iam.AccountRootPrincipal()");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_public_access_does_not_report_arn_principal() {
    let diagnostics = scan("aws-iam-public-access", "new ArnPrincipal(arn)");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_public_access_does_not_report_reference_without_new() {
    let diagnostics = scan("aws-iam-public-access", "const p = iam.AnyPrincipal;");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_sqs_unencrypted_queue_reports_queue_encryption_unencrypted() {
    let diagnostics = scan(
        "aws-sqs-unencrypted-queue",
        "new Queue(this, 'q', { encryption: sqs.QueueEncryption.UNENCRYPTED });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "sqsUnencrypted");
}

#[test]
fn aws_sqs_unencrypted_queue_reports_sqs_managed_sse_disabled() {
    let diagnostics = scan(
        "aws-sqs-unencrypted-queue",
        "const x = { sqsManagedSseEnabled: false };",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "sqsUnencrypted");
}

#[test]
fn aws_sqs_unencrypted_queue_does_not_report_kms_encryption() {
    let diagnostics = scan(
        "aws-sqs-unencrypted-queue",
        "const x = { encryption: QueueEncryption.KMS };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_sqs_unencrypted_queue_does_not_report_sqs_managed_sse_enabled() {
    let diagnostics = scan(
        "aws-sqs-unencrypted-queue",
        "const x = { sqsManagedSseEnabled: true };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_sqs_unencrypted_queue_does_not_report_non_literal_encryption() {
    let diagnostics = scan("aws-sqs-unencrypted-queue", "const x = { encryption: e };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_sqs_unencrypted_queue_does_not_report_other_key() {
    let diagnostics = scan("aws-sqs-unencrypted-queue", "const x = { other: false };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_apigateway_public_api_reports_enum_none() {
    let diagnostics = scan(
        "aws-apigateway-public-api",
        "resource.addMethod('GET', i, { authorizationType: apigateway.AuthorizationType.NONE });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-apigateway-public-api");
}

#[test]
fn aws_apigateway_public_api_reports_string_none() {
    let diagnostics = scan(
        "aws-apigateway-public-api",
        "new apigateway.CfnRoute(this, 'r', { authorizationType: \"NONE\" });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-apigateway-public-api");
}

#[test]
fn aws_apigateway_public_api_does_not_report_iam_enum() {
    let diagnostics = scan(
        "aws-apigateway-public-api",
        "x = { authorizationType: AuthorizationType.IAM };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_apigateway_public_api_does_not_report_aws_iam_string() {
    let diagnostics = scan(
        "aws-apigateway-public-api",
        "x = { authorizationType: \"AWS_IAM\" };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_apigateway_public_api_does_not_report_non_literal_value() {
    let diagnostics = scan(
        "aws-apigateway-public-api",
        "x = { authorizationType: authType };",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_apigateway_public_api_does_not_report_other_key() {
    let diagnostics = scan("aws-apigateway-public-api", "x = { other: \"NONE\" };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_privileges_reports_wildcard_actions() {
    let diagnostics = scan(
        "aws-iam-all-privileges",
        r#"new PolicyStatement({ actions: ["*"], resources: [bucket] });"#,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-iam-all-privileges");
}

#[test]
fn aws_iam_all_privileges_does_not_report_specific_actions() {
    let diagnostics = scan(
        "aws-iam-all-privileges",
        r#"new PolicyStatement({ actions: ["s3:GetObject"] });"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_privileges_does_not_report_empty_actions() {
    let diagnostics = scan(
        "aws-iam-all-privileges",
        "new PolicyStatement({ actions: [] });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_privileges_does_not_report_non_array_value() {
    let diagnostics = scan(
        "aws-iam-all-privileges",
        "new PolicyStatement({ actions: x });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_privileges_does_not_report_other_key() {
    let diagnostics = scan(
        "aws-iam-all-privileges",
        r#"new PolicyStatement({ other: ["*"] });"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_versioning_reports_cdk_bucket_versioned_false() {
    let diagnostics = scan(
        "aws-s3-bucket-versioning",
        "new s3.Bucket(this, 'b', { versioned: false });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-s3-bucket-versioning");
    assert_eq!(diagnostics[0].message_id, "s3BucketVersioning");
}

#[test]
fn aws_s3_bucket_versioning_reports_plain_object_versioned_false() {
    let diagnostics = scan(
        "aws-s3-bucket-versioning",
        "const x = { versioned: false };",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "s3BucketVersioning");
}

#[test]
fn aws_s3_bucket_versioning_does_not_report_versioned_true() {
    let diagnostics = scan("aws-s3-bucket-versioning", "const x = { versioned: true };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_versioning_does_not_report_non_literal_value() {
    let diagnostics = scan("aws-s3-bucket-versioning", "const x = { versioned: flag };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_versioning_does_not_report_other_key() {
    let diagnostics = scan("aws-s3-bucket-versioning", "const x = { other: false };");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_rds_dms_public_reports_publicly_accessible_true() {
    let diagnostics = scan(
        "aws-ec2-rds-dms-public",
        "new ec2.Instance(this,'i',{ publiclyAccessible: true })",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-ec2-rds-dms-public");
}

#[test]
fn aws_ec2_rds_dms_public_reports_associate_public_ip_true() {
    let diagnostics = scan(
        "aws-ec2-rds-dms-public",
        "new ec2.CfnInstance(this,'i',{ networkInterfaces: [{ associatePublicIpAddress: true }] })",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-ec2-rds-dms-public");
}

#[test]
fn aws_ec2_rds_dms_public_does_not_report_false() {
    let diagnostics = scan(
        "aws-ec2-rds-dms-public",
        "new ec2.Instance(this,'i',{ publiclyAccessible: false })",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_rds_dms_public_does_not_report_non_literal() {
    let diagnostics = scan(
        "aws-ec2-rds-dms-public",
        "new ec2.Instance(this,'i',{ publiclyAccessible: x })",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_rds_dms_public_does_not_report_other_key() {
    let diagnostics = scan(
        "aws-ec2-rds-dms-public",
        "new ec2.Instance(this,'i',{ other: true })",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_public_access_reports_block_public_acls_false() {
    let diagnostics = scan(
        "aws-s3-bucket-public-access",
        "new s3.BlockPublicAccess({ blockPublicAcls: false });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-s3-bucket-public-access");
}

#[test]
fn aws_s3_bucket_public_access_reports_restrict_public_buckets_false() {
    let diagnostics = scan(
        "aws-s3-bucket-public-access",
        "new s3.BlockPublicAccess({ restrictPublicBuckets: false });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-s3-bucket-public-access");
}

#[test]
fn aws_s3_bucket_public_access_does_not_report_true() {
    let diagnostics = scan(
        "aws-s3-bucket-public-access",
        "new s3.BlockPublicAccess({ blockPublicAcls: true });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_public_access_does_not_report_non_literal_value() {
    let diagnostics = scan(
        "aws-s3-bucket-public-access",
        "new s3.BlockPublicAccess({ blockPublicAcls: x });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_s3_bucket_public_access_does_not_report_other_key() {
    let diagnostics = scan(
        "aws-s3-bucket-public-access",
        "new s3.BlockPublicAccess({ other: false });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn confidential_information_logging_reports_empty_secrets() {
    let diagnostics = scan(
        "confidential-information-logging",
        "new Signale({ secrets: [] })",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "confidential-information-logging");
}

#[test]
fn confidential_information_logging_does_not_report_non_empty_secrets() {
    let diagnostics = scan(
        "confidential-information-logging",
        r#"new Signale({ secrets: ["pw"] })"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn confidential_information_logging_does_not_report_missing_secrets() {
    let diagnostics = scan("confidential-information-logging", "new Signale({})");
    assert!(diagnostics.is_empty());
}

#[test]
fn confidential_information_logging_does_not_report_other_callee() {
    let diagnostics = scan(
        "confidential-information-logging",
        "new Other({ secrets: [] })",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_resources_accessible_reports_wildcard_resources() {
    let diagnostics = scan(
        "aws-iam-all-resources-accessible",
        r#"new PolicyStatement({ resources: ["*"] });"#,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-iam-all-resources-accessible");
}

#[test]
fn aws_iam_all_resources_accessible_does_not_report_specific_resource() {
    let diagnostics = scan(
        "aws-iam-all-resources-accessible",
        r#"new PolicyStatement({ resources: ["arn:aws:s3:::x"] });"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_resources_accessible_does_not_report_empty_resources() {
    let diagnostics = scan(
        "aws-iam-all-resources-accessible",
        "new PolicyStatement({ resources: [] });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_resources_accessible_does_not_report_non_array_value() {
    let diagnostics = scan(
        "aws-iam-all-resources-accessible",
        "new PolicyStatement({ resources: x });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_iam_all_resources_accessible_does_not_report_other_key() {
    let diagnostics = scan(
        "aws-iam-all-resources-accessible",
        r#"new PolicyStatement({ other: ["*"] });"#,
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_unencrypted_ebs_volume_reports_member_callee_encrypted_false() {
    let diagnostics = scan(
        "aws-ec2-unencrypted-ebs-volume",
        "new ec2.Volume(this, 'v', { encrypted: false });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-ec2-unencrypted-ebs-volume");
    assert_eq!(diagnostics[0].message_id, "ebsUnencrypted");
}

#[test]
fn aws_ec2_unencrypted_ebs_volume_reports_identifier_callee_with_extra_prop() {
    let diagnostics = scan(
        "aws-ec2-unencrypted-ebs-volume",
        "new Volume(this, 'v', { encrypted: false, size: x });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-ec2-unencrypted-ebs-volume");
}

#[test]
fn aws_ec2_unencrypted_ebs_volume_does_not_report_encrypted_true() {
    let diagnostics = scan(
        "aws-ec2-unencrypted-ebs-volume",
        "new ec2.Volume(this, 'v', { encrypted: true });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_unencrypted_ebs_volume_does_not_report_absent_encrypted() {
    let diagnostics = scan(
        "aws-ec2-unencrypted-ebs-volume",
        "new Volume(this, 'v', {});",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_unencrypted_ebs_volume_does_not_report_wrong_construct() {
    let diagnostics = scan(
        "aws-ec2-unencrypted-ebs-volume",
        "new FileSystem(this, 'f', { encrypted: false });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_ec2_unencrypted_ebs_volume_does_not_report_volume_without_options() {
    let diagnostics = scan(
        "aws-ec2-unencrypted-ebs-volume",
        "new ec2.Volume(this, 'v');",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_efs_unencrypted_reports_member_callee_encrypted_false() {
    let diagnostics = scan(
        "aws-efs-unencrypted",
        "new efs.FileSystem(this, 'f', { encrypted: false });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-efs-unencrypted");
}

#[test]
fn aws_efs_unencrypted_reports_identifier_callee_with_other_props() {
    let diagnostics = scan(
        "aws-efs-unencrypted",
        "new FileSystem(this, 'f', { encrypted: false, vpc: v });",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-efs-unencrypted");
}

#[test]
fn aws_efs_unencrypted_does_not_report_encrypted_true() {
    let diagnostics = scan(
        "aws-efs-unencrypted",
        "new efs.FileSystem(this, 'f', { encrypted: true });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_efs_unencrypted_does_not_report_absent_encrypted_prop() {
    let diagnostics = scan("aws-efs-unencrypted", "new FileSystem(this, 'f', {});");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_efs_unencrypted_does_not_report_wrong_construct() {
    let diagnostics = scan(
        "aws-efs-unencrypted",
        "new Volume(this, 'v', { encrypted: false });",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_efs_unencrypted_does_not_report_without_options_object() {
    let diagnostics = scan("aws-efs-unencrypted", "new FileSystem(this, 'f');");
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_restricted_ip_admin_access_reports_any_ipv4_ssh() {
    let diagnostics = scan(
        "aws-restricted-ip-admin-access",
        "sg.addIngressRule(ec2.Peer.anyIpv4(), ec2.Port.tcp(22))",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-restricted-ip-admin-access");
}

#[test]
fn aws_restricted_ip_admin_access_reports_any_ipv6_rdp() {
    let diagnostics = scan(
        "aws-restricted-ip-admin-access",
        "sg.addIngressRule(Peer.anyIpv6(), Port.tcp(3389))",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "aws-restricted-ip-admin-access");
}

#[test]
fn aws_restricted_ip_admin_access_does_not_report_specific_cidr() {
    let diagnostics = scan(
        "aws-restricted-ip-admin-access",
        "sg.addIngressRule(Peer.ipv4(\"10.0.0.0/16\"), Port.tcp(22))",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_restricted_ip_admin_access_does_not_report_non_admin_port() {
    let diagnostics = scan(
        "aws-restricted-ip-admin-access",
        "sg.addIngressRule(Peer.anyIpv4(), Port.tcp(443))",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_restricted_ip_admin_access_does_not_report_single_argument() {
    let diagnostics = scan(
        "aws-restricted-ip-admin-access",
        "sg.addIngressRule(Peer.anyIpv4())",
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn aws_restricted_ip_admin_access_does_not_report_unrelated_call() {
    let diagnostics = scan("aws-restricted-ip-admin-access", "foo(a, b)");
    assert!(diagnostics.is_empty());
}

#[test]
fn redundant_type_aliases_reports_string_keyword() {
    let diagnostics = scan("redundant-type-aliases", "type MyString = string;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "redundant-type-aliases");
    assert_eq!(diagnostics[0].message_id, "redundantTypeAlias");
}

#[test]
fn redundant_type_aliases_reports_boolean_keyword() {
    let diagnostics = scan("redundant-type-aliases", "type B = boolean;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "redundant-type-aliases");
}

#[test]
fn redundant_type_aliases_reports_bare_type_reference() {
    let diagnostics = scan("redundant-type-aliases", "type X = Y;");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "redundant-type-aliases");
}

#[test]
fn redundant_type_aliases_does_not_report_generic_alias_with_type_parameter() {
    let diagnostics = scan("redundant-type-aliases", "type Box<T> = T;");
    assert!(diagnostics.is_empty());
}

#[test]
fn redundant_type_aliases_does_not_report_union() {
    let diagnostics = scan("redundant-type-aliases", "type U = string | number;");
    assert!(diagnostics.is_empty());
}

#[test]
fn redundant_type_aliases_does_not_report_type_reference_with_arguments() {
    let diagnostics = scan("redundant-type-aliases", "type Arr = Array<string>;");
    assert!(diagnostics.is_empty());
}

#[test]
fn redundant_type_aliases_does_not_report_object_type() {
    let diagnostics = scan("redundant-type-aliases", "type O = { a: number };");
    assert!(diagnostics.is_empty());
}

#[test]
fn jsx_no_leaked_render_reports_length_member_before_jsx() {
    let source = "const x = <div>{items.length && <List/>}</div>";
    let diagnostics = scan_jsx("jsx-no-leaked-render", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "jsx-no-leaked-render");
    assert_eq!(diagnostics[0].message_id, "jsxNoLeakedRender");
}

#[test]
fn jsx_no_leaked_render_reports_numeric_literal_before_jsx() {
    let source = "const x = <div>{0 && <X/>}</div>";
    let diagnostics = scan_jsx("jsx-no-leaked-render", source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "jsxNoLeakedRender");
}

#[test]
fn jsx_no_leaked_render_does_not_report_boolean_comparison() {
    let source = "const x = <div>{items.length > 0 && <List/>}</div>";
    let diagnostics = scan_jsx("jsx-no-leaked-render", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn jsx_no_leaked_render_does_not_report_plain_identifier() {
    let source = "const x = <div>{show && <X/>}</div>";
    let diagnostics = scan_jsx("jsx-no-leaked-render", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn jsx_no_leaked_render_does_not_report_or_operator() {
    let source = "const x = <div>{a.length || <X/>}</div>";
    let diagnostics = scan_jsx("jsx-no-leaked-render", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn jsx_no_leaked_render_does_not_report_non_jsx_right() {
    let source = "cond && doThing()";
    let diagnostics = scan_jsx("jsx-no-leaked-render", source);
    assert!(diagnostics.is_empty());
}

#[test]
fn no_uniq_key_reports_math_random() {
    let diagnostics = scan_jsx("no-uniq-key", "<li key={Math.random()}>x</li>");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "no-uniq-key");
    assert_eq!(diagnostics[0].message_id, "noUniqKey");
}

#[test]
fn no_uniq_key_reports_date_now() {
    let diagnostics = scan_jsx("no-uniq-key", "<li key={Date.now()}>x</li>");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "noUniqKey");
}

#[test]
fn no_uniq_key_does_not_report_stable_identifier() {
    let diagnostics = scan_jsx("no-uniq-key", "<li key={item.id}>x</li>");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_uniq_key_does_not_report_index() {
    let diagnostics = scan_jsx("no-uniq-key", "<li key={i}>x</li>");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_uniq_key_does_not_report_string_literal() {
    let diagnostics = scan_jsx("no-uniq-key", "<li key=\"static\">x</li>");
    assert!(diagnostics.is_empty());
}

#[test]
fn no_uniq_key_does_not_report_non_key_attribute() {
    let diagnostics = scan_jsx("no-uniq-key", "<li id={Math.random()}>x</li>");
    assert!(diagnostics.is_empty());
}
