//! Rust unit tests for the sonarjs core. All test inputs are independently
//! authored (clean-room); no upstream SonarJS fixtures or expectations are used.

use oxlint_plugins_carton::{CompactString, SmallVec};

use crate::{Diagnostic, SonarjsOptions, scan_sonarjs};

fn scan(rule_name: &str, source: &str) -> SmallVec<[Diagnostic; 32]> {
    let options = SonarjsOptions {
        rule_names: [CompactString::from(rule_name)].into_iter().collect(),
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
    // 31 case clauses (indices 0..=30) — strictly greater than MAX_CASES (30) → 1 diagnostic
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
    // 30 cases (indices 0..=29) — equal to MAX_CASES, not strictly greater → 0 diagnostics
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
