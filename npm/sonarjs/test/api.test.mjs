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
  'arguments-usage',
  'no-labels',
  'no-delete-var',
  'constructor-for-side-effects',
  'no-empty-character-class',
  'generator-without-yield',
  'no-exclusive-tests',
  'no-built-in-override',
  'class-prototype',
  'max-switch-cases',
  'max-union-size',
  'elseif-without-else',
  'no-case-label-in-switch',
  'for-in',
  'prefer-while',
  'no-small-switch',
  'prefer-default-last',
  'no-inverted-boolean-check',
  'no-useless-catch',
  'no-redundant-optional',
  'prefer-immediate-return',
  'no-redundant-jump',
  'no-primitive-wrappers',
  'no-skipped-tests',
  'prefer-single-boolean-return',
  'no-unthrown-error',
  'no-tab',
  'fixme-tag',
  'todo-tag',
  'no-sonar-comments',
  'array-constructor',
  'no-function-declaration-in-block',
  'no-inconsistent-returns',
  'no-same-line-conditional',
  'no-nested-assignment',
  'no-nested-incdec',
  'no-useless-increment',
  'class-name',
  'max-lines',
  'nested-control-flow',
  'max-lines-per-function',
  'no-duplicate-string',
  'no-empty-group',
  'no-empty-alternatives',
  'no-regex-spaces',
  'no-control-regex',
  'single-char-in-character-classes',
  'duplicates-in-character-class',
  'anchor-precedence',
  'cyclomatic-complexity',
  'no-collection-size-mischeck',
  'index-of-compare-to-positive-number',
  'no-nested-functions',
  'too-many-break-or-continue-in-loop',
  'code-eval',
  'void-use',
  'prefer-promise-shorthand',
  'pseudo-random',
  'no-hardcoded-ip',
  'no-global-this',
  'single-character-alternation',
  'empty-string-repetition',
  'no-misleading-array-reverse',
  'no-alphabetical-sort',
  'no-for-in-iterable',
  'no-associative-arrays',
  'bitwise-operators',
  'no-same-argument-assert',
  'inverted-assertion-arguments',
  'for-loop-increment-sign',
  'no-equals-in-for-termination',
  'reduce-initial-value',
  'no-parameter-reassignment',
  'array-callback-without-return',
  'no-wildcard-import',
  'updated-loop-counter',
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

  it('reports arguments-usage when arguments is used inside a function', () => {
    const source = 'function f() { return arguments[0]; }';
    const diagnostics = scan('arguments-usage', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('arguments-usage');
    expect(diagnostics[0].messageId).toBe('argumentsUsage');
  });

  it('reports arguments-usage when arguments.length is accessed', () => {
    const source = 'function f() { console.log(arguments.length); }';
    const diagnostics = scan('arguments-usage', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('argumentsUsage');
  });

  it('does not report arguments-usage when rest parameters are used', () => {
    const source = 'function f(...args) { return args[0]; }';
    const diagnostics = scan('arguments-usage', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report arguments-usage for object property named arguments', () => {
    const source = 'const o = { arguments: 1 }; o.arguments;';
    const diagnostics = scan('arguments-usage', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report arguments-usage for a function that does not use arguments', () => {
    const source = 'function f() { return 1; }';
    const diagnostics = scan('arguments-usage', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-labels for a labeled loop', () => {
    const source = 'loop: for (;;) { break loop; }';
    const diagnostics = scan('no-labels', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-labels');
    expect(diagnostics[0].messageId).toBe('noLabels');
  });

  it('reports no-labels for two nested labeled loops (one diagnostic per label)', () => {
    const source = 'outer: for (;;) { inner: for (;;) { break outer; } }';
    const diagnostics = scan('no-labels', source);
    expect(diagnostics).toHaveLength(2);
  });

  it('does not report no-labels for an unlabeled loop', () => {
    const source = 'for (;;) { break; }';
    const diagnostics = scan('no-labels', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-labels for a plain variable declaration', () => {
    const source = 'const x = 1;';
    const diagnostics = scan('no-labels', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-delete-var for delete applied to a bare variable', () => {
    const source = 'delete x;';
    const diagnostics = scan('no-delete-var', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-delete-var');
    expect(diagnostics[0].messageId).toBe('noDeleteVar');
  });

  it('reports no-delete-var for delete applied to a parenthesised variable', () => {
    const source = 'delete (y);';
    const diagnostics = scan('no-delete-var', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noDeleteVar');
  });

  it('does not report no-delete-var for delete on a member expression (dot)', () => {
    const source = 'delete obj.prop;';
    const diagnostics = scan('no-delete-var', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-delete-var for delete on a member expression (bracket)', () => {
    const source = 'delete obj[key];';
    const diagnostics = scan('no-delete-var', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-delete-var for a plain variable declaration', () => {
    const source = 'const z = 1;';
    const diagnostics = scan('no-delete-var', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports constructor-for-side-effects for new Foo() as a bare statement', () => {
    const source = 'new Foo();';
    const diagnostics = scan('constructor-for-side-effects', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('constructor-for-side-effects');
    expect(diagnostics[0].messageId).toBe('constructorForSideEffects');
  });

  it('reports constructor-for-side-effects for new Foo (no parens) as a bare statement', () => {
    const source = 'new Foo;';
    const diagnostics = scan('constructor-for-side-effects', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('constructorForSideEffects');
  });

  it('does not report constructor-for-side-effects when result is assigned to a variable', () => {
    const source = 'const x = new Foo();';
    const diagnostics = scan('constructor-for-side-effects', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report constructor-for-side-effects when result is used as a call receiver', () => {
    const source = 'new Foo().bar();';
    const diagnostics = scan('constructor-for-side-effects', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report constructor-for-side-effects for a plain function call statement', () => {
    const source = 'foo();';
    const diagnostics = scan('constructor-for-side-effects', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-empty-character-class for a regex with an empty class between other chars', () => {
    const source = 'const r = /a[]b/;';
    const diagnostics = scan('no-empty-character-class', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-empty-character-class');
    expect(diagnostics[0].messageId).toBe('emptyCharacterClass');
  });

  it('reports no-empty-character-class for a regex that is only an empty class', () => {
    const source = 'const r = /[]/;';
    const diagnostics = scan('no-empty-character-class', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('emptyCharacterClass');
  });

  it('does not report no-empty-character-class for a regex with a non-empty class', () => {
    const source = 'const r = /[abc]/;';
    const diagnostics = scan('no-empty-character-class', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-empty-character-class for a negated empty class [^]', () => {
    const source = 'const r = /[^]/;';
    const diagnostics = scan('no-empty-character-class', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-empty-character-class for escaped brackets that are not a class', () => {
    const source = 'const r = /a\\[\\]b/;';
    const diagnostics = scan('no-empty-character-class', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-empty-character-class when the class content is a literal open bracket', () => {
    const source = 'const r = /[a[]/;';
    const diagnostics = scan('no-empty-character-class', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports generator-without-yield for a generator that only returns', () => {
    const source = 'function* g() { return 1; }';
    const diagnostics = scan('generator-without-yield', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('generator-without-yield');
    expect(diagnostics[0].messageId).toBe('generatorWithoutYield');
  });

  it('reports generator-without-yield for a generator with an empty body', () => {
    const source = 'function* g() {}';
    const diagnostics = scan('generator-without-yield', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('generatorWithoutYield');
  });

  it('does not report generator-without-yield when the generator yields', () => {
    const source = 'function* g() { yield 1; }';
    const diagnostics = scan('generator-without-yield', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report generator-without-yield for a regular function', () => {
    const source = 'function g() { return 1; }';
    const diagnostics = scan('generator-without-yield', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports generator-without-yield for outer only when inner generator yields', () => {
    const source = 'function* outer() { function* inner() { yield 1; } }';
    const diagnostics = scan('generator-without-yield', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('generatorWithoutYield');
    expect(diagnostics[0].loc.startLine).toBe(1);
    expect(diagnostics[0].loc.startColumn).toBe(0);
  });

  it('reports generator-without-yield for inner only when outer generator yields', () => {
    const source = 'function* outer() { yield 1; function* inner() {} }';
    const diagnostics = scan('generator-without-yield', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('generatorWithoutYield');
    expect(diagnostics[0].loc.startColumn).toBeGreaterThan(0);
  });

  it('reports no-exclusive-tests for describe.only(...)', () => {
    const source = "describe.only('x', () => {});";
    const diagnostics = scan('no-exclusive-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-exclusive-tests');
    expect(diagnostics[0].messageId).toBe('noExclusiveTests');
  });

  it('reports no-exclusive-tests for it.only(...)', () => {
    const source = "it.only('x', () => {});";
    const diagnostics = scan('no-exclusive-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noExclusiveTests');
  });

  it('reports no-exclusive-tests for test.only(...)', () => {
    const source = "test.only('x', () => {});";
    const diagnostics = scan('no-exclusive-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noExclusiveTests');
  });

  it('does not report no-exclusive-tests for it without .only', () => {
    const source = "it('x', () => {});";
    const diagnostics = scan('no-exclusive-tests', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-exclusive-tests for foo.only (unknown function)', () => {
    const source = 'foo.only();';
    const diagnostics = scan('no-exclusive-tests', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-exclusive-tests for describe without .only', () => {
    const source = "describe('x', () => {});";
    const diagnostics = scan('no-exclusive-tests', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-built-in-override for a let declaration that shadows Object', () => {
    const source = 'let Object = 1;';
    const diagnostics = scan('no-built-in-override', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-built-in-override');
    expect(diagnostics[0].messageId).toBe('noBuiltInOverride');
  });

  it('reports no-built-in-override for a simple assignment to Array', () => {
    const source = 'Array = 2;';
    const diagnostics = scan('no-built-in-override', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noBuiltInOverride');
  });

  it('reports no-built-in-override for a function declaration named Map', () => {
    const source = 'function Map() {}';
    const diagnostics = scan('no-built-in-override', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noBuiltInOverride');
  });

  it('does not report no-built-in-override for a member-expression assignment to Math.PI', () => {
    const source = 'Math.PI = 3;';
    const diagnostics = scan('no-built-in-override', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-built-in-override for a non-builtin variable declaration', () => {
    const source = 'let obj = 1;';
    const diagnostics = scan('no-built-in-override', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-built-in-override for a member assignment to foo.Object', () => {
    const source = 'foo.Object = 1;';
    const diagnostics = scan('no-built-in-override', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports class-prototype for Foo.prototype.bar = function () {}', () => {
    const source = 'Foo.prototype.bar = function () {};';
    const diagnostics = scan('class-prototype', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('class-prototype');
    expect(diagnostics[0].messageId).toBe('classPrototype');
  });

  it('reports class-prototype for Foo.prototype.baz = 1', () => {
    const source = 'Foo.prototype.baz = 1;';
    const diagnostics = scan('class-prototype', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('classPrototype');
  });

  it('does not report class-prototype for Foo.prototype = {} (no member after prototype)', () => {
    const source = 'Foo.prototype = {};';
    const diagnostics = scan('class-prototype', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report class-prototype for foo.bar = 1 (no prototype in chain)', () => {
    const source = 'foo.bar = 1;';
    const diagnostics = scan('class-prototype', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report class-prototype for obj.prototype (read, not assignment)', () => {
    const source = 'obj.prototype;';
    const diagnostics = scan('class-prototype', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports max-switch-cases for a switch with 31 cases (exceeds 30)', () => {
    const big =
      'switch (x) {' + Array.from({ length: 31 }, (_, i) => `case ${i}: break;`).join('') + '}';
    const diagnostics = scan('max-switch-cases', big);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('max-switch-cases');
    expect(diagnostics[0].messageId).toBe('maxSwitchCases');
  });

  it('does not report max-switch-cases for a small switch with 3 cases', () => {
    const source = 'switch (x) { case 1: break; case 2: break; default: break; }';
    const diagnostics = scan('max-switch-cases', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports max-union-size for a union type with 4 members', () => {
    const source = 'type T = A | B | C | D;';
    const diagnostics = scan('max-union-size', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('max-union-size');
    expect(diagnostics[0].messageId).toBe('maxUnionSize');
  });

  it('does not report max-union-size for a union type with exactly 3 members (at threshold)', () => {
    const source = 'type T = A | B | C;';
    const diagnostics = scan('max-union-size', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report max-union-size for a union type with 2 members', () => {
    const source = 'type T = A | B;';
    const diagnostics = scan('max-union-size', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report max-union-size for a single type alias (not a union)', () => {
    const source = 'type T = A;';
    const diagnostics = scan('max-union-size', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports max-union-size for a union in a variable type annotation with 5 members', () => {
    const source = 'let x: A | B | C | D | E;';
    const diagnostics = scan('max-union-size', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('maxUnionSize');
  });

  it('reports elseif-without-else for a chain with one else-if and no else', () => {
    const source = 'if (a) {} else if (b) {}';
    const diagnostics = scan('elseif-without-else', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('elseif-without-else');
    expect(diagnostics[0].messageId).toBe('elseifWithoutElse');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('reports elseif-without-else for a chain with two else-ifs and no else', () => {
    const source = 'if (a) {} else if (b) {} else if (c) {}';
    const diagnostics = scan('elseif-without-else', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('elseifWithoutElse');
  });

  it('does not report elseif-without-else when the chain ends with else', () => {
    const source = 'if (a) {} else if (b) {} else {}';
    const diagnostics = scan('elseif-without-else', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report elseif-without-else for a lone if with no else-if', () => {
    const source = 'if (a) {}';
    const diagnostics = scan('elseif-without-else', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report elseif-without-else for an if with only an else (no else-if)', () => {
    const source = 'if (a) {} else {}';
    const diagnostics = scan('elseif-without-else', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports elseif-without-else exactly once for an inner chain with else-if but no else', () => {
    const source = 'if (a) { if (x) {} else if (y) {} }';
    const diagnostics = scan('elseif-without-else', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('elseifWithoutElse');
  });

  it('reports no-case-label-in-switch for a label directly in a case consequent', () => {
    const source = 'switch (x) { case 1: foo(); lbl: bar(); break; }';
    const diagnostics = scan('no-case-label-in-switch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-case-label-in-switch');
    expect(diagnostics[0].messageId).toBe('caseLabelInSwitch');
  });

  it('does not report no-case-label-in-switch for a switch with no labels', () => {
    const source = 'switch (x) { case 1: break; default: break; }';
    const diagnostics = scan('no-case-label-in-switch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-case-label-in-switch for a label nested inside a block within a case', () => {
    // Label is inside a block statement, not a direct child of the case consequent.
    const source = 'switch (x) { case 1: { lbl: bar(); } break; }';
    const diagnostics = scan('no-case-label-in-switch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-case-label-in-switch for a label outside any switch', () => {
    const source = 'lbl: for (;;) {}';
    const diagnostics = scan('no-case-label-in-switch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports for-in when body is a block with a non-if statement', () => {
    const source = 'for (const k in o) { doStuff(k); }';
    const diagnostics = scan('for-in', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('for-in');
    expect(diagnostics[0].messageId).toBe('forIn');
  });

  it('reports for-in when body is a single non-if statement (no block)', () => {
    const source = 'for (const k in o) doStuff(k);';
    const diagnostics = scan('for-in', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('forIn');
  });

  it('reports for-in when body is an empty block', () => {
    const source = 'for (const k in o) {}';
    const diagnostics = scan('for-in', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('forIn');
  });

  it('reports for-in when block has two statements (if plus another)', () => {
    const source = 'for (const k in o) { if (a) {} doStuff(); }';
    const diagnostics = scan('for-in', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('forIn');
  });

  it('does not report for-in when body block contains exactly one if statement', () => {
    const source = 'for (const k in o) { if (o.hasOwnProperty(k)) { doStuff(k); } }';
    const diagnostics = scan('for-in', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report for-in when body is directly an if statement (no block)', () => {
    const source = 'for (const k in o) if (cond) doStuff();';
    const diagnostics = scan('for-in', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports prefer-while when for loop has no init and no update', () => {
    const source = 'for (; i < 10;) { i++; }';
    const diagnostics = scan('prefer-while', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('prefer-while');
    expect(diagnostics[0].messageId).toBe('preferWhile');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('reports prefer-while when for loop has no init, no test, and no update', () => {
    const source = 'for (;;) {}';
    const diagnostics = scan('prefer-while', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('preferWhile');
  });

  it('does not report prefer-while when for loop has an init clause', () => {
    const source = 'for (let i = 0; i < 10;) {}';
    const diagnostics = scan('prefer-while', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-while when for loop has an update clause', () => {
    const source = 'for (; i < 10; i++) {}';
    const diagnostics = scan('prefer-while', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-while when for loop has both init and update', () => {
    const source = 'for (let i = 0; i < 10; i++) {}';
    const diagnostics = scan('prefer-while', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-small-switch for a switch with one case clause', () => {
    const source = 'switch (x) { case 1: break; }';
    const diagnostics = scan('no-small-switch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-small-switch');
    expect(diagnostics[0].messageId).toBe('smallSwitch');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('reports no-small-switch for a switch with one case and a default', () => {
    const source = 'switch (x) { case 1: break; default: break; }';
    const diagnostics = scan('no-small-switch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('smallSwitch');
  });

  it('reports no-small-switch for a switch with only a default clause', () => {
    const source = 'switch (x) { default: break; }';
    const diagnostics = scan('no-small-switch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('smallSwitch');
  });

  it('reports no-small-switch for an empty switch', () => {
    const source = 'switch (x) {}';
    const diagnostics = scan('no-small-switch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('smallSwitch');
  });

  it('does not report no-small-switch for a switch with two case clauses', () => {
    const source = 'switch (x) { case 1: break; case 2: break; }';
    const diagnostics = scan('no-small-switch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-small-switch for a switch with two cases and a default', () => {
    const source = 'switch (x) { case 1: break; case 2: break; default: break; }';
    const diagnostics = scan('no-small-switch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports prefer-default-last when default is the first clause', () => {
    const source = 'switch (x) { default: break; case 1: break; }';
    const diagnostics = scan('prefer-default-last', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('prefer-default-last');
    expect(diagnostics[0].messageId).toBe('defaultLast');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('reports prefer-default-last when default is in the middle', () => {
    const source = 'switch (x) { case 1: break; default: break; case 2: break; }';
    const diagnostics = scan('prefer-default-last', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('defaultLast');
  });

  it('does not report prefer-default-last when default is the last clause', () => {
    const source = 'switch (x) { case 1: break; default: break; }';
    const diagnostics = scan('prefer-default-last', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-default-last when there is no default clause', () => {
    const source = 'switch (x) { case 1: break; case 2: break; }';
    const diagnostics = scan('prefer-default-last', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-inverted-boolean-check for !(a === b)', () => {
    const source = 'const r = !(a === b);';
    const diagnostics = scan('no-inverted-boolean-check', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-inverted-boolean-check');
    expect(diagnostics[0].messageId).toBe('invertedBooleanCheck');
  });

  it('reports no-inverted-boolean-check for !(a < b)', () => {
    const source = 'const r = !(a < b);';
    const diagnostics = scan('no-inverted-boolean-check', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('invertedBooleanCheck');
  });

  it('reports no-inverted-boolean-check for !(x !== y)', () => {
    const source = 'const r = !(x !== y);';
    const diagnostics = scan('no-inverted-boolean-check', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('invertedBooleanCheck');
  });

  it('does not report no-inverted-boolean-check for !(a && b) (logical, not comparison)', () => {
    const source = 'const r = !(a && b);';
    const diagnostics = scan('no-inverted-boolean-check', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-inverted-boolean-check for !a (no comparison)', () => {
    const source = 'const r = !a;';
    const diagnostics = scan('no-inverted-boolean-check', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-inverted-boolean-check for !(a + b) (arithmetic, not comparison)', () => {
    const source = 'const r = !(a + b);';
    const diagnostics = scan('no-inverted-boolean-check', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-useless-catch for catch that only rethrows', () => {
    const source = 'try { f(); } catch (e) { throw e; }';
    const diagnostics = scan('no-useless-catch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-useless-catch');
    expect(diagnostics[0].messageId).toBe('uselessCatch');
  });

  it('reports no-useless-catch for catch that only rethrows when finally is present', () => {
    const source = 'try { f(); } catch (err) { throw err; } finally { g(); }';
    const diagnostics = scan('no-useless-catch', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('uselessCatch');
  });

  it('does not report no-useless-catch when catch body has two statements', () => {
    const source = 'try { f(); } catch (e) { log(e); throw e; }';
    const diagnostics = scan('no-useless-catch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-useless-catch when throw argument is a new expression', () => {
    const source = 'try { f(); } catch (e) { throw new Error(); }';
    const diagnostics = scan('no-useless-catch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-useless-catch when throw argument is a member expression', () => {
    const source = 'try { f(); } catch (e) { throw e.cause; }';
    const diagnostics = scan('no-useless-catch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-useless-catch when catch body has no throw', () => {
    const source = 'try { f(); } catch (e) { handle(e); }';
    const diagnostics = scan('no-useless-catch', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-redundant-optional for optional property with union including undefined', () => {
    const source = 'interface I { a?: string | undefined; }';
    const diagnostics = scan('no-redundant-optional', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-redundant-optional');
    expect(diagnostics[0].messageId).toBe('redundantOptional');
  });

  it('reports no-redundant-optional for optional property typed as undefined directly', () => {
    const source = 'interface I { b?: undefined; }';
    const diagnostics = scan('no-redundant-optional', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantOptional');
  });

  it('reports no-redundant-optional for optional property with multi-member union including undefined', () => {
    const source = 'interface I { c?: number | string | undefined; }';
    const diagnostics = scan('no-redundant-optional', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantOptional');
  });

  it('does not report no-redundant-optional for optional property without undefined in type', () => {
    const source = 'interface I { a?: string; }';
    const diagnostics = scan('no-redundant-optional', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-redundant-optional for non-optional property with undefined in type', () => {
    const source = 'interface I { b: string | undefined; }';
    const diagnostics = scan('no-redundant-optional', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-redundant-optional for optional property with null but not undefined', () => {
    const source = 'interface I { c?: string | null; }';
    const diagnostics = scan('no-redundant-optional', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports prefer-immediate-return for const declared then immediately returned', () => {
    const source = 'function f() { const x = compute(); return x; }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('prefer-immediate-return');
    expect(diagnostics[0].messageId).toBe('preferImmediateReturn');
  });

  it('reports prefer-immediate-return for const declared then immediately thrown', () => {
    const source = 'function f() { const e = new Error(); throw e; }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('preferImmediateReturn');
  });

  it('reports prefer-immediate-return for arrow function with block body', () => {
    const source = 'const g = () => { const x = 1; return x; };';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('preferImmediateReturn');
  });

  it('does not report prefer-immediate-return for a direct return (only one statement)', () => {
    const source = 'function f() { return compute(); }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-immediate-return when a statement appears between decl and return', () => {
    const source = 'function f() { const x = 1; doStuff(); return x; }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-immediate-return when return uses a different identifier', () => {
    const source = 'function f() { const x = 1; return y; }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-immediate-return when return is not the bare identifier', () => {
    const source = 'function f() { const x = 1; return x + 1; }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-immediate-return when the declaration has two declarators', () => {
    const source = 'function f() { const x = 1, y = 2; return x; }';
    const diagnostics = scan('prefer-immediate-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-redundant-jump for trailing continue in a for(;;) loop body', () => {
    const source = 'for (;;) { foo(); continue; }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-redundant-jump');
    expect(diagnostics[0].messageId).toBe('redundantJump');
  });

  it('reports no-redundant-jump for trailing continue in a while loop body', () => {
    const source = 'while (x) { foo(); continue; }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantJump');
  });

  it('reports no-redundant-jump for trailing continue in a do-while loop body', () => {
    const source = 'do { foo(); continue; } while (x);';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantJump');
  });

  it('reports no-redundant-jump for trailing continue in a for-of loop body', () => {
    const source = 'for (const a of b) { foo(); continue; }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantJump');
  });

  it('reports no-redundant-jump for trailing return; in a function body', () => {
    const source = 'function f() { foo(); return; }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantJump');
  });

  it('does not report no-redundant-jump when continue is not the last statement', () => {
    const source = 'for (;;) { if (x) continue; foo(); }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-redundant-jump for return with a value', () => {
    const source = 'function f() { foo(); return x; }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-redundant-jump for a labeled continue', () => {
    const source = 'outer: for (;;) { foo(); continue outer; }';
    const diagnostics = scan('no-redundant-jump', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-primitive-wrappers for new Number(1)', () => {
    const source = 'const n = new Number(1);';
    const diagnostics = scan('no-primitive-wrappers', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-primitive-wrappers');
    expect(diagnostics[0].messageId).toBe('primitiveWrapper');
  });

  it('reports no-primitive-wrappers for new String("x")', () => {
    const source = "const s = new String('x');";
    const diagnostics = scan('no-primitive-wrappers', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('primitiveWrapper');
  });

  it('reports no-primitive-wrappers for new Boolean(false)', () => {
    const source = 'const b = new Boolean(false);';
    const diagnostics = scan('no-primitive-wrappers', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('primitiveWrapper');
  });

  it('does not report no-primitive-wrappers for Number(1) (call, no new)', () => {
    const source = 'const n = Number(1);';
    const diagnostics = scan('no-primitive-wrappers', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-primitive-wrappers for new Array(3) (not a primitive wrapper)', () => {
    const source = 'const a = new Array(3);';
    const diagnostics = scan('no-primitive-wrappers', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-primitive-wrappers for new Foo() (unknown constructor)', () => {
    const source = 'const f = new Foo();';
    const diagnostics = scan('no-primitive-wrappers', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-skipped-tests for describe.skip(...)', () => {
    const source = "describe.skip('x', () => {});";
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-skipped-tests');
    expect(diagnostics[0].messageId).toBe('skippedTest');
  });

  it('reports no-skipped-tests for it.skip(...)', () => {
    const source = "it.skip('x', () => {});";
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('skippedTest');
  });

  it('reports no-skipped-tests for xit(...)', () => {
    const source = "xit('x', () => {});";
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('skippedTest');
  });

  it('reports no-skipped-tests for xdescribe(...)', () => {
    const source = "xdescribe('x', () => {});";
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('skippedTest');
  });

  it('does not report no-skipped-tests for it(...) (no skip)', () => {
    const source = "it('x', () => {});";
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-skipped-tests for foo.skip() (unknown runner)', () => {
    const source = 'foo.skip();';
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-skipped-tests for xfoo() (not in x-set)', () => {
    const source = 'xfoo();';
    const diagnostics = scan('no-skipped-tests', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports prefer-single-boolean-return for if/else both returning bool literals (block form)', () => {
    const source = 'function f() { if (c) { return true; } else { return false; } }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('prefer-single-boolean-return');
    expect(diagnostics[0].messageId).toBe('preferSingleBooleanReturn');
  });

  it('reports prefer-single-boolean-return for if/else both returning bool literals (bare form)', () => {
    const source = 'function f() { if (c) return true; else return false; }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('preferSingleBooleanReturn');
  });

  it('reports prefer-single-boolean-return for if/else with inverted bool literals', () => {
    const source = 'function f() { if (c) { return false; } else { return true; } }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('preferSingleBooleanReturn');
  });

  it('does not report prefer-single-boolean-return when there is no else branch', () => {
    const source = 'function f() { if (c) { return true; } }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-single-boolean-return when consequent returns a non-literal', () => {
    const source = 'function f() { if (c) { return x; } else { return false; } }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-single-boolean-return for an else-if chain', () => {
    const source = 'function f() { if (c) return true; else if (d) return x; }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report prefer-single-boolean-return when consequent block has two statements', () => {
    const source = 'function f() { if (c) { return true; bar(); } else { return false; } }';
    const diagnostics = scan('prefer-single-boolean-return', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-unthrown-error for new Error as a bare statement', () => {
    const source = "new Error('boom');";
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-unthrown-error');
    expect(diagnostics[0].messageId).toBe('unthrownError');
  });

  it('reports no-unthrown-error for new TypeError as a bare statement', () => {
    const source = "new TypeError('x');";
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('unthrownError');
  });

  it('reports no-unthrown-error for a user-defined Error subtype as a bare statement', () => {
    const source = 'new MyError();';
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('unthrownError');
  });

  it('does not report no-unthrown-error when the error is thrown', () => {
    const source = "throw new Error('boom');";
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-unthrown-error when the error is assigned to a variable', () => {
    const source = 'const e = new Error();';
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-unthrown-error for new Foo() (callee does not end with Error)', () => {
    const source = 'new Foo();';
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-unthrown-error when the error is passed as a call argument', () => {
    const source = 'foo(new Error());';
    const diagnostics = scan('no-unthrown-error', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-tab for a line with a leading tab', () => {
    const source = '\tconst x = 1;';
    const diagnostics = scan('no-tab', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-tab');
    expect(diagnostics[0].messageId).toBe('noTab');
  });

  it('reports no-tab for a line with a tab in the middle', () => {
    const source = 'const x\t= 1;';
    const diagnostics = scan('no-tab', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noTab');
  });

  it('reports no-tab once for two lines where only the second has a tab', () => {
    const source = 'a();\n\tb();';
    const diagnostics = scan('no-tab', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noTab');
  });

  it('reports no-tab twice when two lines each contain a tab', () => {
    const source = '\ta();\n\tb();';
    const diagnostics = scan('no-tab', source);
    expect(diagnostics).toHaveLength(2);
  });

  it('does not report no-tab for source with no tab characters', () => {
    const source = 'const x = 1;';
    const diagnostics = scan('no-tab', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports fixme-tag for a line comment containing FIXME', () => {
    const source = '// FIXME do x';
    const diagnostics = scan('fixme-tag', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('fixme-tag');
    expect(diagnostics[0].messageId).toBe('fixmeTag');
  });

  it('reports fixme-tag for a block comment containing FIXME', () => {
    const source = '/* FIXME: broken */';
    const diagnostics = scan('fixme-tag', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('fixmeTag');
  });

  it('reports fixme-tag for a trailing line comment containing FIXME', () => {
    const source = 'const a = 1; // FIXME later';
    const diagnostics = scan('fixme-tag', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('fixmeTag');
  });

  it('does not report fixme-tag for a comment containing TODO but not FIXME', () => {
    const source = '// TODO do x';
    const diagnostics = scan('fixme-tag', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report fixme-tag for lowercase fixme (case-sensitive match only)', () => {
    const source = '// fixme';
    const diagnostics = scan('fixme-tag', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report fixme-tag for source with no comments', () => {
    const source = 'const a = 1;';
    const diagnostics = scan('fixme-tag', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports todo-tag for a line comment containing TODO', () => {
    const source = '// TODO do x';
    const diagnostics = scan('todo-tag', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('todo-tag');
    expect(diagnostics[0].messageId).toBe('todoTag');
  });

  it('reports todo-tag for a block comment containing TODO', () => {
    const source = '/* TODO: later */';
    const diagnostics = scan('todo-tag', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('todoTag');
  });

  it('does not report todo-tag for a comment containing FIXME but not TODO', () => {
    const source = '// FIXME do x';
    const diagnostics = scan('todo-tag', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report todo-tag for lowercase todo (case-sensitive match only)', () => {
    const source = '// todo';
    const diagnostics = scan('todo-tag', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report todo-tag for source with no comments', () => {
    const source = 'const a = 1;';
    const diagnostics = scan('todo-tag', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-sonar-comments for a comment containing NOSONAR', () => {
    const source = '// NOSONAR suppress this';
    const diagnostics = scan('no-sonar-comments', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-sonar-comments');
    expect(diagnostics[0].messageId).toBe('noSonarComments');
  });

  it('reports no-sonar-comments for a block comment containing NOSONAR', () => {
    const source = '/* NOSONAR */';
    const diagnostics = scan('no-sonar-comments', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report no-sonar-comments for a plain comment', () => {
    const source = '// just a comment';
    const diagnostics = scan('no-sonar-comments', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-sonar-comments for source with no comments', () => {
    const source = 'const a = 1;';
    const diagnostics = scan('no-sonar-comments', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports array-constructor for a call with multiple arguments', () => {
    const source = 'const a = Array(1, 2, 3);';
    const diagnostics = scan('array-constructor', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('array-constructor');
    expect(diagnostics[0].messageId).toBe('arrayConstructor');
  });

  it('reports array-constructor for a new expression with multiple arguments', () => {
    const source = 'const a = new Array(1, 2, 3);';
    const diagnostics = scan('array-constructor', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('reports array-constructor for a call with no arguments', () => {
    const source = 'const a = new Array();';
    const diagnostics = scan('array-constructor', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report array-constructor for a single-argument length form', () => {
    const source = 'const a = new Array(500);';
    const diagnostics = scan('array-constructor', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report array-constructor when type arguments are present', () => {
    const source = 'const a = Array<number>(1, 2, 3);';
    const diagnostics = scan('array-constructor', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report array-constructor for an array literal', () => {
    const source = 'const a = [1, 2, 3];';
    const diagnostics = scan('array-constructor', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-function-declaration-in-block for a function declared in an if block', () => {
    const source = 'if (cond) {\n  function f() {}\n}';
    const diagnostics = scan('no-function-declaration-in-block', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-function-declaration-in-block');
    expect(diagnostics[0].messageId).toBe('noFunctionDeclarationInBlock');
  });

  it('reports no-function-declaration-in-block for a function declared in a bare block', () => {
    const source = '{\n  function f() {}\n}';
    const diagnostics = scan('no-function-declaration-in-block', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report no-function-declaration-in-block for a top-level declaration', () => {
    const source = 'function f() {}';
    const diagnostics = scan('no-function-declaration-in-block', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-function-declaration-in-block for a nested function body declaration', () => {
    const source = 'function outer() {\n  function inner() {}\n}';
    const diagnostics = scan('no-function-declaration-in-block', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-function-declaration-in-block for a function expression in a block', () => {
    const source = 'if (cond) {\n  const f = function () {};\n}';
    const diagnostics = scan('no-function-declaration-in-block', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-inconsistent-returns for a function mixing value and bare returns', () => {
    const source = 'function f(x) {\n  if (!x) return;\n  return x.value;\n}';
    const diagnostics = scan('no-inconsistent-returns', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-inconsistent-returns');
    expect(diagnostics[0].messageId).toBe('inconsistentReturns');
  });

  it('reports no-inconsistent-returns for an arrow mixing value and bare returns', () => {
    const source = 'const f = (x) => {\n  if (!x) return;\n  return 1;\n};';
    const diagnostics = scan('no-inconsistent-returns', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report no-inconsistent-returns when all returns yield a value', () => {
    const source = 'function f(x) {\n  if (!x) return 0;\n  return x.value;\n}';
    const diagnostics = scan('no-inconsistent-returns', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-inconsistent-returns when all returns are bare', () => {
    const source = 'function f(x) {\n  if (!x) return;\n  doWork();\n  return;\n}';
    const diagnostics = scan('no-inconsistent-returns', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-inconsistent-returns only for the inconsistent nested function', () => {
    // outer returns only a value; inner mixes value and bare returns. Only inner
    // is flagged, proving each scope tracks its own returns on a separate frame.
    const source =
      'function outer() {\n  return 1;\n  function inner() {\n    if (a) return;\n    return 2;\n  }\n}';
    const diagnostics = scan('no-inconsistent-returns', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('reports no-same-line-conditional for an if on the closing brace line', () => {
    const source = 'if (a) {\n  doA();\n} if (b) {\n  doB();\n}';
    const diagnostics = scan('no-same-line-conditional', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-same-line-conditional');
    expect(diagnostics[0].messageId).toBe('sameLineConditional');
  });

  it('does not report no-same-line-conditional for an if on a new line', () => {
    const source = 'if (a) {\n  doA();\n}\nif (b) {\n  doB();\n}';
    const diagnostics = scan('no-same-line-conditional', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-same-line-conditional for an else-if chain', () => {
    const source = 'if (a) {\n  doA();\n} else if (b) {\n  doB();\n}';
    const diagnostics = scan('no-same-line-conditional', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-same-line-conditional when the preceding statement is not an if', () => {
    const source = 'doA(); if (b) {\n  doB();\n}';
    const diagnostics = scan('no-same-line-conditional', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-nested-assignment for an assignment in an if condition', () => {
    const source = 'if (x = compute()) {\n  use(x);\n}';
    const diagnostics = scan('no-nested-assignment', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-nested-assignment');
    expect(diagnostics[0].messageId).toBe('nestedAssignment');
  });

  it('reports no-nested-assignment for an assignment in a while condition', () => {
    const source = 'while (node = node.next) {\n  visit(node);\n}';
    const diagnostics = scan('no-nested-assignment', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('reports no-nested-assignment for the inner part of a chained assignment', () => {
    const source = 'a = b = c;';
    const diagnostics = scan('no-nested-assignment', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report no-nested-assignment for a plain assignment statement', () => {
    const source = 'x = compute();';
    const diagnostics = scan('no-nested-assignment', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-nested-assignment for the init and update of a for loop', () => {
    const source = 'for (i = 0; i < 10; i = i + 1) {\n  use(i);\n}';
    const diagnostics = scan('no-nested-assignment', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-nested-assignment for an equality comparison in a condition', () => {
    const source = 'if (x === compute()) {\n  use(x);\n}';
    const diagnostics = scan('no-nested-assignment', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-nested-incdec for an increment used as a call argument', () => {
    const source = 'foo(i++);';
    const diagnostics = scan('no-nested-incdec', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-nested-incdec');
    expect(diagnostics[0].messageId).toBe('nestedIncDec');
  });

  it('reports no-nested-incdec for a decrement used as a method call argument', () => {
    const source = 'arr.push(--count);';
    const diagnostics = scan('no-nested-incdec', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('reports no-nested-incdec for an increment used as a constructor argument', () => {
    const source = 'new Widget(n++);';
    const diagnostics = scan('no-nested-incdec', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report no-nested-incdec for a standalone increment statement', () => {
    const source = 'i++;';
    const diagnostics = scan('no-nested-incdec', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-nested-incdec for the update clause of a for loop', () => {
    const source = 'for (let i = 0; i < n; i++) {\n  use(i);\n}';
    const diagnostics = scan('no-nested-incdec', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-useless-increment for a postfix self-increment assignment', () => {
    const source = 'i = i++;';
    const diagnostics = scan('no-useless-increment', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-useless-increment');
    expect(diagnostics[0].messageId).toBe('uselessIncrement');
  });

  it('reports no-useless-increment for a postfix self-decrement assignment', () => {
    const source = 'j = j--;';
    const diagnostics = scan('no-useless-increment', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report no-useless-increment for a prefix increment assignment', () => {
    const source = 'i = ++i;';
    const diagnostics = scan('no-useless-increment', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-useless-increment for an increment of a different variable', () => {
    const source = 'i = j++;';
    const diagnostics = scan('no-useless-increment', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-useless-increment for a standalone increment statement', () => {
    const source = 'i++;';
    const diagnostics = scan('no-useless-increment', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports class-name for a class starting with a lowercase letter', () => {
    const source = 'class myClass {}';
    const diagnostics = scan('class-name', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('class-name');
    expect(diagnostics[0].messageId).toBe('className');
  });

  it('reports class-name for a class starting with an underscore', () => {
    const source = 'class _Helper {}';
    const diagnostics = scan('class-name', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report class-name for a PascalCase class', () => {
    const source = 'class MyClass {}';
    const diagnostics = scan('class-name', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report class-name for an anonymous default-exported class', () => {
    const source = 'export default class {}';
    const diagnostics = scan('class-name', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports class-name for a lowercase named class expression', () => {
    const source = 'const C = class widget {};';
    const diagnostics = scan('class-name', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('passes the max-switch-cases threshold through the native boundary', () => {
    const source = 'switch (x) { case 1: break; case 2: break; case 3: break; }';
    const flagged = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-switch-cases'],
      maxSwitchCasesThreshold: 2,
    });
    expect(flagged).toHaveLength(1);
    const allowed = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-switch-cases'],
      maxSwitchCasesThreshold: 3,
    });
    expect(allowed).toHaveLength(0);
  });

  it('passes the max-union-size threshold through the native boundary', () => {
    const source = 'type T = A | B | C;';
    const flagged = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-union-size'],
      maxUnionSizeThreshold: 2,
    });
    expect(flagged).toHaveLength(1);
  });

  it('reports max-lines when code lines exceed the threshold', () => {
    const source = 'const a = 1;\nconst b = 2;\nconst c = 3;';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-lines'],
      maxLinesThreshold: 2,
    });
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('max-lines');
    expect(diagnostics[0].messageId).toBe('maxLines');
    expect(diagnostics[0].loc.startLine).toBe(1);
    expect(diagnostics[0].loc.startColumn).toBe(0);
  });

  it('does not report max-lines when code lines equal the threshold', () => {
    const source = 'const a = 1;\nconst b = 2;\nconst c = 3;';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-lines'],
      maxLinesThreshold: 3,
    });
    expect(diagnostics).toHaveLength(0);
  });

  it('does not count blank or comment-only lines toward the max-lines total', () => {
    // 2 code lines + blank line + comment-only line = still only 2 code lines
    const source = 'const a = 1;\n\n// only a comment\nconst b = 2;';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-lines'],
      maxLinesThreshold: 2,
    });
    expect(diagnostics).toHaveLength(0);
  });

  it('reports nested-control-flow when threshold 2 is exceeded with a 3-level nest', () => {
    const source = 'if (a) { for (let i = 0; i < 10; i++) { while (b) {} } }';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['nested-control-flow'],
      nestedControlFlowThreshold: 2,
    });
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('nested-control-flow');
    expect(diagnostics[0].messageId).toBe('nestedControlFlow');
  });

  it('reports max-lines-per-function for a function over the threshold', () => {
    const source =
      'function f() {\n  const a = 1;\n  const b = 2;\n  const c = 3;\n  return a + b + c;\n}';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-lines-per-function'],
      maxLinesPerFunctionThreshold: 3,
    });
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('max-lines-per-function');
    expect(diagnostics[0].messageId).toBe('maxLinesPerFunction');
  });

  it('does not report max-lines-per-function for a function at exactly the threshold', () => {
    // 5 code lines (signature + 3 body + closing brace), exactly at the threshold
    const source = 'function f() {\n  const a = 1;\n  const b = 2;\n  return a + b;\n}';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['max-lines-per-function'],
      maxLinesPerFunctionThreshold: 5,
    });
    expect(diagnostics).toHaveLength(0);
  });

  it('passes the no-duplicate-string threshold through the native boundary', () => {
    // "hello wrld" = 10 chars, has a space → qualifies; appears twice
    const source = 'const a = "hello wrld"; const b = "hello wrld";';
    const flagged = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['no-duplicate-string'],
      noDuplicateStringThreshold: 2,
    });
    expect(flagged).toHaveLength(1);
    expect(flagged[0].ruleName).toBe('no-duplicate-string');
    expect(flagged[0].messageId).toBe('duplicateString');
    const allowed = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['no-duplicate-string'],
      noDuplicateStringThreshold: 3,
    });
    expect(allowed).toHaveLength(0);
  });

  it('reports no-empty-group for an empty non-capturing group', () => {
    const diagnostics = scan('no-empty-group', 'const r = /(?:)/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-empty-group');
    expect(diagnostics[0].messageId).toBe('emptyGroup');
  });

  it('does not report no-empty-group for a non-empty group', () => {
    const diagnostics = scan('no-empty-group', 'const r = /(?:abc)/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-empty-alternatives for a trailing empty alternative', () => {
    const diagnostics = scan('no-empty-alternatives', 'const r = /a|/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-empty-alternatives');
    expect(diagnostics[0].messageId).toBe('emptyAlternative');
  });

  it('does not report no-empty-alternatives when all alternatives have content', () => {
    const diagnostics = scan('no-empty-alternatives', 'const r = /a|b/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-regex-spaces for two consecutive spaces in a regex', () => {
    const diagnostics = scan('no-regex-spaces', 'const r = /a  b/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-regex-spaces');
    expect(diagnostics[0].messageId).toBe('multipleSpaces');
  });

  it('does not report no-regex-spaces for a single space', () => {
    const diagnostics = scan('no-regex-spaces', 'const r = /a b/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-control-regex for a hex escape control character', () => {
    const diagnostics = scan('no-control-regex', 'const r = /\\x1f/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-control-regex');
    expect(diagnostics[0].messageId).toBe('controlCharacter');
  });

  it('reports single-char-in-character-classes for a one-character class', () => {
    const diagnostics = scan('single-char-in-character-classes', 'const r = /[a]/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('single-char-in-character-classes');
    expect(diagnostics[0].messageId).toBe('singleCharInCharacterClass');
  });

  it('does not report single-char-in-character-classes for a multi-character class', () => {
    const diagnostics = scan('single-char-in-character-classes', 'const r = /[ab]/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports duplicates-in-character-class for a repeated character', () => {
    const diagnostics = scan('duplicates-in-character-class', 'const r = /[aa]/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('duplicates-in-character-class');
    expect(diagnostics[0].messageId).toBe('duplicateCharacter');
  });

  it('does not report duplicates-in-character-class for distinct characters', () => {
    const diagnostics = scan('duplicates-in-character-class', 'const r = /[abc]/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports anchor-precedence when ^ anchors only the first of three alternatives', () => {
    const diagnostics = scan('anchor-precedence', 'const r = /^a|b|c$/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('anchor-precedence');
    expect(diagnostics[0].messageId).toBe('anchorPrecedence');
  });

  it('reports anchor-precedence when ^ anchors only the first of two alternatives', () => {
    const diagnostics = scan('anchor-precedence', 'const r = /^a|b/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('anchorPrecedence');
  });

  it('does not report anchor-precedence for a grouped alternation', () => {
    const diagnostics = scan('anchor-precedence', 'const r = /^(a|b|c)$/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report anchor-precedence for the two-branch trim idiom', () => {
    const diagnostics = scan('anchor-precedence', 'const r = /^\\s+|\\s+$/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report anchor-precedence when every branch is fully anchored', () => {
    const diagnostics = scan('anchor-precedence', 'const r = /^a$|^b$|^c$/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports cyclomatic-complexity when a function exceeds the threshold', () => {
    // base 1 + 4 ifs = 5, custom threshold 3: 5 > 3 → 1 diagnostic
    const source = 'function f(a,b,c,d){if(a){}if(b){}if(c){}if(d){}}';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['cyclomatic-complexity'],
      cyclomaticComplexityThreshold: 3,
    });
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('cyclomatic-complexity');
    expect(diagnostics[0].messageId).toBe('cyclomaticComplexity');
  });

  it('does not report cyclomatic-complexity when a function is within the threshold', () => {
    // base 1 + 3 ifs = 4, threshold 4: 4 is not > 4 → 0 diagnostics
    const source = 'function f(a,b,c){if(a){}if(b){}if(c){}}';
    const diagnostics = scanSonarjs(source, 'sample.ts', {
      ruleNames: ['cyclomatic-complexity'],
      cyclomaticComplexityThreshold: 4,
    });
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-collection-size-mischeck for always-false length < 0', () => {
    const diagnostics = scan('no-collection-size-mischeck', 'const b = x.length < 0;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-collection-size-mischeck');
    expect(diagnostics[0].messageId).toBe('collectionSizeMischeck');
  });

  it('reports index-of-compare-to-positive-number for indexOf > 0', () => {
    const diagnostics = scan('index-of-compare-to-positive-number', 'const b = a.indexOf(x) > 0;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('index-of-compare-to-positive-number');
    expect(diagnostics[0].messageId).toBe('indexOfPositive');
  });

  it('does not report index-of-compare-to-positive-number for indexOf >= 0', () => {
    const diagnostics = scan('index-of-compare-to-positive-number', 'const b = a.indexOf(x) >= 0;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report index-of-compare-to-positive-number for indexOf > -1', () => {
    const diagnostics = scan('index-of-compare-to-positive-number', 'const b = a.indexOf(x) > -1;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report index-of-compare-to-positive-number for indexOf === -1', () => {
    const diagnostics = scan(
      'index-of-compare-to-positive-number',
      'const b = a.indexOf(x) === -1;',
    );
    expect(diagnostics).toHaveLength(0);
  });

  it('reports void-use for void applied to a function call', () => {
    const diagnostics = scan('void-use', 'void foo();');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('void-use');
    expect(diagnostics[0].messageId).toBe('voidUse');
  });

  it('reports void-use for void applied to a variable', () => {
    const diagnostics = scan('void-use', 'void x;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('voidUse');
  });

  it('reports void-use for void applied to a non-zero numeric literal', () => {
    const diagnostics = scan('void-use', 'void 1;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('voidUse');
  });

  it('does not report void-use for void 0', () => {
    const diagnostics = scan('void-use', 'void 0;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report void-use for void (0)', () => {
    const diagnostics = scan('void-use', 'void (0);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report void-use for a logical-not unary expression', () => {
    const diagnostics = scan('void-use', 'const b = !x;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report void-use for typeof', () => {
    const diagnostics = scan('void-use', 'typeof x;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports pseudo-random for Math.random()', () => {
    const diagnostics = scan('pseudo-random', 'const x = Math.random();');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('pseudo-random');
    expect(diagnostics[0].messageId).toBe('pseudoRandom');
  });

  it('does not report pseudo-random for Math.floor()', () => {
    const diagnostics = scan('pseudo-random', 'Math.floor(1.5);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report pseudo-random for foo.random()', () => {
    const diagnostics = scan('pseudo-random', 'foo.random();');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report pseudo-random for a bare Math.random reference', () => {
    const diagnostics = scan('pseudo-random', 'const f = Math.random;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-hardcoded-ip for a plain IPv4 address string literal', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "192.168.1.1";');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-hardcoded-ip');
    expect(diagnostics[0].messageId).toBe('hardcodedIp');
  });

  it('reports no-hardcoded-ip for a private IPv4 address', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "10.0.0.1";');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('hardcodedIp');
  });

  it('reports no-hardcoded-ip for an IPv4 address embedded in a URL string', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const url = "http://192.168.0.1/api";');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('hardcodedIp');
  });

  it('reports no-hardcoded-ip for a valid IPv6 address string', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "2001:db8:85a3::8a2e:370:7334";');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-hardcoded-ip for a non-documentation IPv6 address', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "fe80::1";');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('hardcodedIp');
  });

  it('does not report no-hardcoded-ip for loopback 127.0.0.1', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "127.0.0.1";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for 127.x.x.x loopback range', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "127.1.2.3";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for broadcast 255.255.255.255', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "255.255.255.255";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for unspecified 0.0.0.0', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "0.0.0.0";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for IPv6 loopback ::1', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "::1";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for IPv6 documentation range 2001:db8::', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "2001:db8::1";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for IPv4-mapped loopback ::ffff:127.0.0.1', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const ip = "::ffff:127.0.0.1";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for a string without any IP address', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const s = "hello world";');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-hardcoded-ip for a three-octet partial address', () => {
    const diagnostics = scan('no-hardcoded-ip', 'const s = "192.168.1";');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-global-this for a top-level this expression', () => {
    const diagnostics = scan('no-global-this', 'this.foo = 1;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-global-this');
    expect(diagnostics[0].messageId).toBe('noGlobalThis');
    expect(diagnostics[0].loc.startLine).toBe(1);
  });

  it('reports no-global-this for this inside a top-level arrow function', () => {
    const diagnostics = scan('no-global-this', 'const f = () => this.x;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-global-this');
    expect(diagnostics[0].messageId).toBe('noGlobalThis');
  });

  it('reports no-global-this for this inside nested top-level arrows', () => {
    const diagnostics = scan('no-global-this', 'const f = () => () => this;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noGlobalThis');
  });

  it('does not report no-global-this for this inside a regular function', () => {
    const diagnostics = scan('no-global-this', 'function f() { return this.x; }');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-global-this for this inside an object method shorthand', () => {
    const diagnostics = scan('no-global-this', 'const o = { m() { return this.x; } };');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-global-this for this inside a class method', () => {
    const diagnostics = scan('no-global-this', 'class C { m() { return this.x; } }');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-global-this for this inside a class field initializer', () => {
    const diagnostics = scan('no-global-this', 'class C { x = this.y; }');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-global-this for this inside a class static block', () => {
    const diagnostics = scan('no-global-this', 'class C { static { this.z(); } }');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-global-this for this inside an arrow inside a regular function', () => {
    const diagnostics = scan('no-global-this', 'function f() { const g = () => this.x; }');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports single-character-alternation for /a|b|c/', () => {
    const diagnostics = scan('single-character-alternation', 'const re = /a|b|c/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('single-character-alternation');
    expect(diagnostics[0].messageId).toBe('singleCharAlternation');
  });

  it('reports single-character-alternation for a disjunction inside a group /(a|b|c)/', () => {
    const diagnostics = scan('single-character-alternation', 'const re = /(a|b|c)/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('singleCharAlternation');
  });

  it('reports single-character-alternation for escaped chars /\\.|,/', () => {
    const diagnostics = scan('single-character-alternation', 'const re = /\\.|,/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('singleCharAlternation');
  });

  it('does not report single-character-alternation for /ab|c/ (multi-char alt)', () => {
    const diagnostics = scan('single-character-alternation', 'const re = /ab|c/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report single-character-alternation for /\\d|x/ (class escape)', () => {
    const diagnostics = scan('single-character-alternation', 'const re = /\\d|x/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report single-character-alternation for /abc/ (no disjunction)', () => {
    const diagnostics = scan('single-character-alternation', 'const re = /abc/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-same-argument-assert for assert.equal(x, x)', () => {
    const diagnostics = scan('no-same-argument-assert', 'assert.equal(x, x);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-same-argument-assert');
    expect(diagnostics[0].messageId).toBe('sameArgumentAssert');
  });

  it('reports no-same-argument-assert for assert.strictEqual(foo.bar, foo.bar)', () => {
    const diagnostics = scan('no-same-argument-assert', 'assert.strictEqual(foo.bar, foo.bar);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('sameArgumentAssert');
  });

  it('does not report no-same-argument-assert for assert.equal(x, y) (different args)', () => {
    const diagnostics = scan('no-same-argument-assert', 'assert.equal(x, y);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-same-argument-assert for foo(x, x) (not an assertion call)', () => {
    const diagnostics = scan('no-same-argument-assert', 'foo(x, x);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-same-argument-assert for assert.ok(x) (single argument)', () => {
    const diagnostics = scan('no-same-argument-assert', 'assert.ok(x);');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports inverted-assertion-arguments for assert.equal(42, x)', () => {
    const diagnostics = scan('inverted-assertion-arguments', 'assert.equal(42, x);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('inverted-assertion-arguments');
    expect(diagnostics[0].messageId).toBe('invertedArguments');
  });

  it("reports inverted-assertion-arguments for assert.strictEqual('foo', bar)", () => {
    const diagnostics = scan('inverted-assertion-arguments', "assert.strictEqual('foo', bar);");
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('invertedArguments');
  });

  it('does not report inverted-assertion-arguments for assert.equal(x, 42) (correct order)', () => {
    const diagnostics = scan('inverted-assertion-arguments', 'assert.equal(x, 42);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report inverted-assertion-arguments for assert.equal(1, 2) (both literals)', () => {
    const diagnostics = scan('inverted-assertion-arguments', 'assert.equal(1, 2);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report inverted-assertion-arguments for foo(42, x) (not an assertion call)', () => {
    const diagnostics = scan('inverted-assertion-arguments', 'foo(42, x);');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('for-loop-increment-sign rule', () => {
  it('reports an increasing condition with a decrementing update', () => {
    const diagnostics = scan('for-loop-increment-sign', 'for (let i = 0; i < 10; i--) {}');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('for-loop-increment-sign');
    expect(diagnostics[0].messageId).toBe('wrongDirection');
  });

  it('reports a decreasing condition with an incrementing update', () => {
    const diagnostics = scan('for-loop-increment-sign', 'for (let i = 10; i > 0; i++) {}');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('wrongDirection');
  });

  it('reports a counter that appears on the right of the comparison', () => {
    const diagnostics = scan('for-loop-increment-sign', 'for (let i = 0; 10 > i; i--) {}');
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report a correctly increasing loop', () => {
    const diagnostics = scan('for-loop-increment-sign', 'for (let i = 0; i < 10; i++) {}');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an equality condition with no direction', () => {
    const diagnostics = scan('for-loop-increment-sign', 'for (let i = 0; i != 10; i++) {}');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the update variable differs from the counter', () => {
    const diagnostics = scan('for-loop-increment-sign', 'for (let i = 0, j = 0; i < 10; j++) {}');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-equals-in-for-termination rule', () => {
  it('reports an inequality condition with a non-unit compound step', () => {
    const diagnostics = scan('no-equals-in-for-termination', 'for (let i = 0; i != 10; i += 2) {}');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-equals-in-for-termination');
    expect(diagnostics[0].messageId).toBe('noEqualsInForTermination');
  });

  it('reports a strict-inequality condition with a non-unit plain assignment', () => {
    const diagnostics = scan(
      'no-equals-in-for-termination',
      'for (let i = 0; i !== 10; i = i + 2) {}',
    );
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report a unit increment', () => {
    const diagnostics = scan('no-equals-in-for-termination', 'for (let i = 0; i != 10; i++) {}');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a relational condition', () => {
    const diagnostics = scan('no-equals-in-for-termination', 'for (let i = 0; i < 10; i += 2) {}');
    expect(diagnostics).toHaveLength(0);
  });
});
