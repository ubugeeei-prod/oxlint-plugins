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
