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
