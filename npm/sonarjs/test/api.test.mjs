import { describe, expect, it } from 'vitest';

import { implementedSonarjsRuleNames, scanSonarjs } from '../api.js';

const expectedRuleNames = [
  'no-nested-template-literals',
  'no-nested-switch',
  'no-nested-conditional',
  'no-collapsible-if',
  'no-redundant-boolean',
  'comma-or-logical-or-case',
  'no-duplicate-in-composite',
  'non-existent-operator',
  'no-identical-conditions',
  'no-all-duplicated-branches',
  'no-identical-expressions',
];

function scan(ruleName, sourceText, filename = 'sample.ts') {
  return scanSonarjs(sourceText, filename, { ruleNames: [ruleName] });
}

describe('sonarjs native API', () => {
  it('exposes all implemented sonarjs rule names', () => {
    expect(implementedSonarjsRuleNames()).toEqual(expectedRuleNames);
  });

  it('reports a template literal nested inside another', () => {
    const diagnostics = scan('no-nested-template-literals', 'const x = `outer ${`inner`} end`;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-nested-template-literals');
    expect(diagnostics[0].messageId).toBe('nestedTemplateLiteral');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('does not report a flat template literal', () => {
    const diagnostics = scan('no-nested-template-literals', 'const x = `value ${y}`;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports each nested level independently', () => {
    const diagnostics = scan('no-nested-template-literals', 'const x = `a ${`b ${`c`}`}`;');
    expect(diagnostics).toHaveLength(2);
  });

  it('reports a switch nested inside another switch', () => {
    const diagnostics = scan(
      'no-nested-switch',
      'switch (a) {\n  case 1:\n    switch (b) {\n      case 2:\n        break;\n    }\n    break;\n}',
    );
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-nested-switch');
    expect(diagnostics[0].messageId).toBe('nestedSwitch');
    expect(diagnostics[0].loc.startLine).toBe(3);
  });

  it('does not report a single switch', () => {
    const diagnostics = scan('no-nested-switch', 'switch (a) {\n  case 1:\n    break;\n}');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports a conditional nested in the alternate of another conditional', () => {
    const diagnostics = scan('no-nested-conditional', 'const x = a ? b : (c ? d : e);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-nested-conditional');
    expect(diagnostics[0].messageId).toBe('nestedConditional');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('does not report a single (non-nested) conditional expression', () => {
    const diagnostics = scan('no-nested-conditional', 'const x = a ? b : c;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports each nested level of a doubly-nested conditional', () => {
    const diagnostics = scan('no-nested-conditional', 'const x = a ? (b ? c : d) : (e ? f : g);');
    expect(diagnostics).toHaveLength(2);
  });

  it('ignores rules that are not enabled', () => {
    const diagnostics = scanSonarjs('const x = `outer ${`inner`}`;', 'sample.ts', {
      ruleNames: [],
    });
    expect(diagnostics).toHaveLength(0);
  });

  it('reports a collapsible if when the outer if contains only a nested if', () => {
    const diagnostics = scan('no-collapsible-if', 'if (a) { if (b) {} }');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-collapsible-if');
    expect(diagnostics[0].messageId).toBe('collapsibleIf');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('does not report when the outer if has an else clause', () => {
    const diagnostics = scan('no-collapsible-if', 'if (a) { if (b) {} } else {}');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the inner if has an else clause', () => {
    const diagnostics = scan('no-collapsible-if', 'if (a) { if (b) {} else {} }');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the block contains more than one statement', () => {
    const diagnostics = scan('no-collapsible-if', 'if (a) { if (b) {} doSomething(); }');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports a boolean literal on the right side of a strict equality', () => {
    const diagnostics = scan('no-redundant-boolean', 'x === true');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantBoolean');
  });

  it('reports a boolean literal on the left side of a strict inequality', () => {
    const diagnostics = scan('no-redundant-boolean', 'false !== y');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantBoolean');
  });

  it('reports negation of a boolean literal', () => {
    const diagnostics = scan('no-redundant-boolean', '!false');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantBoolean');
  });

  it('reports a ternary whose both branches are boolean literals', () => {
    const diagnostics = scan('no-redundant-boolean', 'c ? true : false');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantBoolean');
  });

  it('does not report a regular equality comparison without boolean literals', () => {
    const diagnostics = scan('no-redundant-boolean', 'x === y');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report negation of a non-boolean expression', () => {
    const diagnostics = scan('no-redundant-boolean', '!x');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a ternary with non-boolean branches', () => {
    const diagnostics = scan('no-redundant-boolean', 'c ? a : b');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports a logical-OR expression as a case label', () => {
    const source = 'switch (x) { case 1 || 2: break; }';
    const diagnostics = scan('comma-or-logical-or-case', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('commaOrLogicalOrInCase');
  });

  it('reports a comma/sequence expression as a case label', () => {
    const source = 'switch (x) { case (1, 2): break; }';
    const diagnostics = scan('comma-or-logical-or-case', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('commaOrLogicalOrInCase');
  });

  it('does not report a plain case label or default', () => {
    const source = 'switch (x) { case 1: break; default: break; }';
    const diagnostics = scan('comma-or-logical-or-case', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a logical-AND expression as a case label', () => {
    const source = 'switch (x) { case 1 && 2: break; }';
    const diagnostics = scan('comma-or-logical-or-case', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports a duplicate type in a union type', () => {
    const source = 'type T = A | B | A;';
    const diagnostics = scan('no-duplicate-in-composite', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('duplicateType');
  });

  it('reports a duplicate type in an intersection type', () => {
    const source = 'type T = A & B & A;';
    const diagnostics = scan('no-duplicate-in-composite', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('duplicateType');
  });

  it('does not report a union type with all unique members', () => {
    const source = 'type T = A | B | C;';
    const diagnostics = scan('no-duplicate-in-composite', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an intersection type with all unique members', () => {
    const source = 'type T = A & B;';
    const diagnostics = scan('no-duplicate-in-composite', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports one diagnostic for a union with a repeated primitive type', () => {
    const source = 'type T = string | string | number;';
    const diagnostics = scan('no-duplicate-in-composite', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('duplicateType');
  });

  it('reports non-existent-operator for x =- 1 (negation adjacent to assign)', () => {
    const source = 'let x = 0; x =- 1;';
    const diagnostics = scan('non-existent-operator', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('non-existent-operator');
    expect(diagnostics[0].messageId).toBe('nonExistentOperator');
  });

  it('reports non-existent-operator for x =+ 1 (unary plus adjacent to assign)', () => {
    const source = 'let x = 0; x =+ 1;';
    const diagnostics = scan('non-existent-operator', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('nonExistentOperator');
  });

  it('reports non-existent-operator for x =! y (logical not adjacent to assign)', () => {
    const source = 'let x = false; let y = true; x =! y;';
    const diagnostics = scan('non-existent-operator', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('nonExistentOperator');
  });

  it('does not report non-existent-operator for x = -1 (space before unary)', () => {
    const source = 'let x = 0; x = -1;';
    const diagnostics = scan('non-existent-operator', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report non-existent-operator for x -= 1 (compound assignment)', () => {
    const source = 'let x = 0; x -= 1;';
    const diagnostics = scan('non-existent-operator', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report non-existent-operator for plain assignment x = y', () => {
    const source = 'let x = 0; let y = 1; x = y;';
    const diagnostics = scan('non-existent-operator', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports one identical condition in an if/else-if/else-if chain', () => {
    const source = 'if (a) {} else if (b) {} else if (a) {}';
    const diagnostics = scan('no-identical-conditions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-identical-conditions');
    expect(diagnostics[0].messageId).toBe('identicalConditions');
  });

  it('reports one identical condition in a two-branch if/else-if chain', () => {
    const source = 'if (a) {} else if (a) {}';
    const diagnostics = scan('no-identical-conditions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalConditions');
  });

  it('does not report when the else branch is a plain block (no condition)', () => {
    const source = 'if (a) {} else if (b) {} else {}';
    const diagnostics = scan('no-identical-conditions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a standalone if with no else-if', () => {
    const source = 'if (a) {}';
    const diagnostics = scan('no-identical-conditions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports one identical condition in a four-branch chain', () => {
    const source = 'if (a) {} else if (b) {} else if (c) {} else if (b) {}';
    const diagnostics = scan('no-identical-conditions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalConditions');
  });

  it('reports no-all-duplicated-branches for an if/else where both branches are identical', () => {
    const source = 'if (a) { f(); } else { f(); }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-all-duplicated-branches');
    expect(diagnostics[0].messageId).toBe('allDuplicatedBranches');
  });

  it('reports no-all-duplicated-branches for an if/else-if/else chain where all branches are identical', () => {
    const source = 'if (a) { f(); } else if (b) { f(); } else { f(); }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('allDuplicatedBranches');
  });

  it('does not report no-all-duplicated-branches when branches differ', () => {
    const source = 'if (a) { f(); } else { g(); }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-all-duplicated-branches when there is no terminal else', () => {
    const source = 'if (a) { f(); } else if (b) { f(); }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-all-duplicated-branches for a switch where all cases are identical', () => {
    const source = 'switch (x) { case 1: f(); break; default: f(); break; }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('allDuplicatedBranches');
  });

  it('does not report no-all-duplicated-branches for a switch where cases differ', () => {
    const source = 'switch (x) { case 1: f(); break; default: g(); break; }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-all-duplicated-branches for a switch without a default case', () => {
    const source = 'switch (x) { case 1: f(); break; case 2: f(); break; }';
    const diagnostics = scan('no-all-duplicated-branches', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-identical-expressions for a === a', () => {
    const source = 'a === a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-identical-expressions');
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for b !== b', () => {
    const source = 'b !== b';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for x < x', () => {
    const source = 'x < x';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for a && a', () => {
    const source = 'a && a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for a || a', () => {
    const source = 'a || a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for a & a (bitwise AND)', () => {
    const source = 'a & a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for a - a (subtraction)', () => {
    const source = 'a - a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('reports no-identical-expressions for a / a (division)', () => {
    const source = 'a / a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('identicalExpressions');
  });

  it('does not report no-identical-expressions for a === b (different operands)', () => {
    const source = 'a === b';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-identical-expressions for a + a (addition is excluded)', () => {
    const source = 'a + a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-identical-expressions for a * a (multiplication is excluded)', () => {
    const source = 'a * a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-identical-expressions for a << a (left-shift is excluded)', () => {
    const source = 'a << a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-identical-expressions for a ?? a (nullish coalescing is excluded)', () => {
    const source = 'a ?? a';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-identical-expressions for a.b === a.c (different member access)', () => {
    const source = 'a.b === a.c';
    const diagnostics = scan('no-identical-expressions', source);
    expect(diagnostics).toHaveLength(0);
  });
});
