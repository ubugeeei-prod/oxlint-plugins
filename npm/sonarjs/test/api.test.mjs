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
  'label-position',
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
  'function-name',
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
  'hashing',
  'no-clear-text-protocols',
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
  'declarations-in-global-scope',
  'no-wildcard-import',
  'updated-loop-counter',
  'misplaced-loop-counter',
  'no-array-delete',
  'no-literal-call',
  'shorthand-property-grouping',
  'process-argv',
  'standard-input',
  'no-code-after-done',
  'function-inside-loop',
  'no-useless-intersection',
  'use-type-alias',
  'public-static-readonly',
  'call-argument-line',
  'prefer-object-literal',
  'no-undefined-argument',
  'no-identical-functions',
  'no-in-misuse',
  'no-require-or-define',
  'no-invalid-regexp',
  'no-invariant-returns',
  'no-extra-arguments',
  'link-with-target-blank',
  'no-weak-cipher',
  'no-hardcoded-passwords',
  'no-ignored-exceptions',
  'no-unused-function-argument',
  'object-alt-content',
  'no-use-of-empty-return-value',
  'no-duplicated-branches',
  'block-scoped-var',
  'no-variable-usage-before-declaration',
  'arguments-order',
  'updated-const-var',
  'unicode-aware-regex',
  'no-undefined-assignment',
  'no-empty-after-reluctant',
  'no-ignored-return',
  'file-name-differ-from-class',
  'no-unenclosed-multiline-block',
  'inconsistent-function-call',
  'new-operator-misuse',
  'no-empty-test-file',
  'deprecation',
  'cognitive-complexity',
  'expression-complexity',
  'prefer-regexp-exec',
  'no-fallthrough',
  'no-commented-code',
  'no-incomplete-assertions',
  'destructuring-assignment-syntax',
  'no-element-overwrite',
  'no-redundant-assignments',
  'no-unused-collection',
  'no-empty-collection',
  'no-redundant-parentheses',
  'bool-param-default',
  'post-message',
  'in-operator-type-error',
  'different-types-comparison',
  'operation-returning-nan',
  'production-debug',
  'no-hardcoded-secrets',
  'concise-regex',
  'no-misleading-character-class',
  'slow-regex',
  'web-sql-database',
  'no-intrusive-permissions',
  'encryption-secure-mode',
  'no-unsafe-unzip',
  'disabled-timeout',
  'cookie-no-httponly',
  'content-security-policy',
  'certificate-transparency',
  'csrf',
  'file-permissions',
  'file-uploads',
  'cors',
  'dns-prefetching',
  'disabled-auto-escaping',
  'aws-s3-bucket-granted-access',
  'aws-rds-unencrypted-databases',
  'aws-iam-public-access',
  'hidden-files',
  'aws-sqs-unencrypted-queue',
  'aws-apigateway-public-api',
  'aws-iam-all-privileges',
  'aws-s3-bucket-versioning',
  'aws-ec2-rds-dms-public',
  'aws-s3-bucket-public-access',
  'confidential-information-logging',
  'aws-iam-all-resources-accessible',
  'aws-ec2-unencrypted-ebs-volume',
  'aws-efs-unencrypted',
  'aws-restricted-ip-admin-access',
  'redundant-type-aliases',
  'jsx-no-leaked-render',
  'no-uniq-key',
  'insecure-cookie',
  'no-hook-setter-in-body',
  'content-length',
  'unverified-certificate',
  'no-mime-sniff',
  'no-ip-forward',
  'no-angular-bypass-sanitization',
  'insecure-jwt-token',
  'xml-parser-xxe',
  'no-useless-react-setstate',
  'no-referrer-policy',
  'weak-ssl',
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

  it('reports label-position for labels that are not directly on a loop or switch', () => {
    const source = 'unused: doWork();\nblock: { doWork(); }\nconditional: if (ready) { doWork(); }';
    const diagnostics = scan('label-position', source);
    expect(diagnostics).toHaveLength(3);
    expect(diagnostics[0].ruleName).toBe('label-position');
    expect(diagnostics[0].messageId).toBe('removeLabel');
  });

  it('does not report label-position for directly labelled breakable statements', () => {
    const source = `
      labelled_for: for (;;) { break labelled_for; }
      labelled_for_in: for (const key in object) { break labelled_for_in; }
      labelled_for_of: for (const value of values) { break labelled_for_of; }
      labelled_while: while (condition) { break labelled_while; }
      labelled_do: do { break labelled_do; } while (condition);
      labelled_switch: switch (value) { case 1: break labelled_switch; }
    `;
    const diagnostics = scan('label-position', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('reports label-position for an outer nested label', () => {
    const source = 'outer: inner: for (;;) { break outer; }';
    const diagnostics = scan('label-position', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('removeLabel');
    expect(diagnostics[0].loc.startColumn).toBe(0);
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

  it('reports no-invariant-returns for a function always returning the same value', () => {
    const source = 'function f(x) {\n  if (x > 0) return 42;\n  return 42;\n}';
    const diagnostics = scan('no-invariant-returns', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-invariant-returns');
    expect(diagnostics[0].messageId).toBe('invariantReturn');
  });

  it('does not report no-invariant-returns when return values differ', () => {
    const source = 'function f(x) {\n  if (x > 0) return 1;\n  return 2;\n}';
    const diagnostics = scan('no-invariant-returns', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-invariant-returns with only one value return', () => {
    const source = 'function f(x) {\n  if (x) return 42;\n}';
    const diagnostics = scan('no-invariant-returns', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-invariant-returns when a bare return is present', () => {
    const source = 'function f(x) {\n  if (!x) return;\n  return 42;\n}';
    const diagnostics = scan('no-invariant-returns', source);
    expect(diagnostics).toHaveLength(0);
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

  it('reports function-name for declarations and method keys', () => {
    const source =
      'function Bad_name() {} const Bad_name2 = () => {}; class C { Bad_name3() {} #Bad_name4() {} }';
    const diagnostics = scan('function-name', source);
    expect(diagnostics).toHaveLength(3);
    expect(diagnostics.map((diagnostic) => diagnostic.data.value)).toEqual([
      'Bad_name',
      'Bad_name2',
      'Bad_name3',
    ]);
    expect(diagnostics[0].ruleName).toBe('function-name');
    expect(diagnostics[0].messageId).toBe('renameFunction');
    expect(diagnostics[0].data.format).toBe('^[_a-z][a-zA-Z0-9]*$');
  });

  it('passes the function-name format through the native boundary', () => {
    const diagnostics = scanSonarjs(
      'function goodName() {} function GoodName() {} const goodName2 = () => {};',
      'sample.ts',
      { ruleNames: ['function-name'], functionNameFormat: '^[A-Z][A-Za-z0-9]*$' },
    );
    expect(diagnostics).toHaveLength(2);
    expect(diagnostics.map((diagnostic) => diagnostic.data.value)).toEqual([
      'goodName',
      'goodName2',
    ]);
    expect(
      diagnostics.every((diagnostic) => diagnostic.data.format === '^[A-Z][A-Za-z0-9]*$'),
    ).toBe(true);
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

  it('reports hashing for crypto.createHash("md5")', () => {
    const diagnostics = scan('hashing', 'const h = crypto.createHash("md5");');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('hashing');
    expect(diagnostics[0].messageId).toBe('weakHash');
  });

  it('reports hashing for WebCrypto SHA-1 digest', () => {
    const diagnostics = scan('hashing', 'crypto.subtle.digest("SHA-1", data);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('weakHash');
  });

  it('does not report hashing for sha256', () => {
    const diagnostics = scan('hashing', 'const h = crypto.createHash("sha256");');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report hashing for a dynamic algorithm', () => {
    const diagnostics = scan('hashing', 'const h = crypto.createHash(algorithm);');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-clear-text-protocols for an http URL literal', () => {
    const diagnostics = scan('no-clear-text-protocols', 'const url = "http://example.com";');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-clear-text-protocols');
    expect(diagnostics[0].messageId).toBe('clearTextProtocol');
  });

  it('reports no-clear-text-protocols for a clear-text WebSocket URL', () => {
    const diagnostics = scan('no-clear-text-protocols', 'const url = "ws://example.com/socket";');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('clearTextProtocol');
  });

  it('does not report no-clear-text-protocols for encrypted protocols', () => {
    const diagnostics = scan(
      'no-clear-text-protocols',
      'const a = "https://example.com"; const b = "wss://example.com/socket";',
    );
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-clear-text-protocols for a non-URL http label', () => {
    const diagnostics = scan('no-clear-text-protocols', 'const label = "http: status";');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports process-argv for a direct process.argv access', () => {
    const diagnostics = scan('process-argv', 'const a = process.argv;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('process-argv');
    expect(diagnostics[0].messageId).toBe('processArgv');
  });

  it('reports process-argv once for process.argv[2]', () => {
    const diagnostics = scan('process-argv', 'process.argv[2];');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('processArgv');
  });

  it('does not report process-argv for process.env', () => {
    const diagnostics = scan('process-argv', 'process.env.PATH;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report process-argv for foo.argv', () => {
    const diagnostics = scan('process-argv', 'foo.argv;');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports standard-input for a direct process.stdin access', () => {
    const diagnostics = scan('standard-input', 'const x = process.stdin;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('standard-input');
    expect(diagnostics[0].messageId).toBe('standardInput');
  });

  it('reports standard-input once for process.stdin.on', () => {
    const diagnostics = scan('standard-input', "process.stdin.on('data', cb);");
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('standardInput');
  });

  it('does not report standard-input for process.stdout', () => {
    const diagnostics = scan('standard-input', 'process.stdout;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report standard-input for foo.stdin', () => {
    const diagnostics = scan('standard-input', 'foo.stdin;');
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

  it('reports no-weak-cipher for DES cipher creation', () => {
    const diagnostics = scan(
      'no-weak-cipher',
      'const c = crypto.createCipheriv("des-cbc", key, iv);',
    );
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-weak-cipher');
    expect(diagnostics[0].messageId).toBe('weakCipher');
  });

  it('reports no-weak-cipher for a bare RC4 cipher factory', () => {
    const diagnostics = scan('no-weak-cipher', 'const c = createCipher("rc4", password);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('weakCipher');
  });

  it('does not report no-weak-cipher for AES-GCM', () => {
    const diagnostics = scan(
      'no-weak-cipher',
      'const c = crypto.createCipheriv("aes-256-gcm", key, iv);',
    );
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-weak-cipher for a dynamic algorithm', () => {
    const diagnostics = scan(
      'no-weak-cipher',
      'const c = crypto.createCipheriv(algorithm, key, iv);',
    );
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

describe('misplaced-loop-counter rule', () => {
  it('reports when the update increments a variable absent from the condition', () => {
    const diagnostics = scan('misplaced-loop-counter', 'for (let i = 0; i < 10; j++) {}');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('misplaced-loop-counter');
    expect(diagnostics[0].messageId).toBe('misplacedCounter');
  });

  it('reports a compound assignment to a non-condition variable', () => {
    const diagnostics = scan('misplaced-loop-counter', 'for (let i = 0; i < 10; k += 1) {}');
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report when the update advances the condition counter', () => {
    const diagnostics = scan('misplaced-loop-counter', 'for (let i = 0; i < 10; i++) {}');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a comma update that overlaps the condition', () => {
    const source = 'for (let i = 0, j = 0; i < 10 && j < 5; i++, j++) {}';
    const diagnostics = scan('misplaced-loop-counter', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a loop with no test or update', () => {
    const diagnostics = scan('misplaced-loop-counter', 'for (;;) {}');
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

describe('no-array-delete rule', () => {
  it('reports delete on a resolved array variable element', () => {
    const diagnostics = scan('no-array-delete', 'const a = [1, 2, 3];\ndelete a[0];');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-array-delete');
    expect(diagnostics[0].messageId).toBe('noArrayDelete');
  });

  it('reports delete on a direct array-literal element', () => {
    const diagnostics = scan('no-array-delete', 'delete [1, 2][0];');
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report delete on an object property', () => {
    const diagnostics = scan('no-array-delete', 'const o = { x: 1 };\ndelete o.x;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report delete on a static array member', () => {
    const diagnostics = scan('no-array-delete', 'const a = [1, 2, 3];\ndelete a.foo;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report delete on an unprovable receiver', () => {
    const diagnostics = scan('no-array-delete', 'function f(p) {\n  delete p[0];\n}');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-literal-call rule', () => {
  it('reports a boolean literal called as a function', () => {
    const diagnostics = scan('no-literal-call', 'true();');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-literal-call');
    expect(diagnostics[0].messageId).toBe('noLiteralCall');
  });

  it('reports a string literal called as a function', () => {
    const diagnostics = scan('no-literal-call', '("foo")();');
    expect(diagnostics).toHaveLength(1);
  });

  it('reports a literal used as a tagged-template tag', () => {
    const diagnostics = scan('no-literal-call', 'true`text`;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noLiteralCall');
  });

  it('does not report an ordinary function call', () => {
    const diagnostics = scan('no-literal-call', 'foo();');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an object expression called', () => {
    const diagnostics = scan('no-literal-call', '({})();');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('shorthand-property-grouping rule', () => {
  it('reports shorthand properties split by a regular property', () => {
    const diagnostics = scan('shorthand-property-grouping', 'const o = { a, x: 1, b };');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('shorthand-property-grouping');
    expect(diagnostics[0].messageId).toBe('groupShorthand');
  });

  it('reports a lone shorthand property in the middle', () => {
    const diagnostics = scan('shorthand-property-grouping', 'const o = { x: 1, a, y: 2 };');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('groupShorthand');
  });

  it('does not report shorthand grouped at the beginning', () => {
    const diagnostics = scan('shorthand-property-grouping', 'const o = { a, b, x: 1 };');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report shorthand grouped at the end', () => {
    const diagnostics = scan('shorthand-property-grouping', 'const o = { x: 1, a, b };');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an object with no shorthand properties', () => {
    const diagnostics = scan('shorthand-property-grouping', 'const o = { x: 1, y: 2 };');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('block-scoped-var rule', () => {
  it('reports a var used outside the if-block where it is declared', () => {
    const diagnostics = scan(
      'block-scoped-var',
      'function f(c) { if (c) { var x = 1; } return x; }',
    );
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('block-scoped-var');
    expect(diagnostics[0].messageId).toBe('blockScopedVar');
  });

  it('does not report a var used only inside the block where it is declared', () => {
    const diagnostics = scan(
      'block-scoped-var',
      'function f(c) { if (c) { var x = 1; return x; } }',
    );
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a var declared at function top level', () => {
    const diagnostics = scan('block-scoped-var', 'function f() { var x = 1; return x; }');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report let or const even when used outside the block', () => {
    const diagnostics = scan('block-scoped-var', 'function f(c) { if (c) { let y = 1; } }');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('arguments-order rule', () => {
  it('reports swapped arguments matching parameter names', () => {
    const diagnostics = scan('arguments-order', 'function f(a, b) {} const a = 1, b = 2; f(b, a);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('arguments-order');
    expect(diagnostics[0].messageId).toBe('argumentsOrder');
  });

  it('does not report when arguments are in the correct order', () => {
    const diagnostics = scan('arguments-order', 'function f(a, b) {} const a = 1, b = 2; f(a, b);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when argument names do not match parameter names', () => {
    const diagnostics = scan('arguments-order', 'function f(a, b) {} const x = 1, y = 2; f(x, y);');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('updated-const-var rule', () => {
  it('reports a const binding reassignment', () => {
    const diagnostics = scan('updated-const-var', 'const x = 1; x = 2;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('updated-const-var');
    expect(diagnostics[0].messageId).toBe('updateConst');
    expect(diagnostics[0].data.value).toBe('x');
  });

  it('does not report let assignments or const property writes', () => {
    const diagnostics = scan('updated-const-var', 'let x = 1; x = 2; const obj = {}; obj.x = 1;');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('unicode-aware-regex rule', () => {
  it('reports a \\p{...} property escape without u flag', () => {
    const diagnostics = scan('unicode-aware-regex', 'const r = /\\p{Letter}/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('unicode-aware-regex');
    expect(diagnostics[0].messageId).toBe('unicodeAwareRegex');
  });

  it('reports a \\P{...} negative property escape without u flag', () => {
    const diagnostics = scan('unicode-aware-regex', 'const r = /\\P{ASCII}/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('unicode-aware-regex');
    expect(diagnostics[0].messageId).toBe('unicodeAwareRegex');
  });

  it('does not report when the u flag is present', () => {
    const diagnostics = scan('unicode-aware-regex', 'const r = /\\p{Letter}/u;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the v flag is present', () => {
    const diagnostics = scan('unicode-aware-regex', 'const r = /\\p{Letter}/v;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a regex without a property escape', () => {
    const diagnostics = scan('unicode-aware-regex', 'const r = /[a-z]/;');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-empty-after-reluctant', () => {
  it('reports a lazy star with nothing following', () => {
    const diagnostics = scan('no-empty-after-reluctant', 'const r = /a*?/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-empty-after-reluctant');
    expect(diagnostics[0].messageId).toBe('emptyAfterReluctant');
  });

  it('reports a lazy star followed by a boundary assertion', () => {
    const diagnostics = scan('no-empty-after-reluctant', 'const r = /a*?$/;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('emptyAfterReluctant');
  });

  it('does not report a lazy star followed by a literal character', () => {
    const diagnostics = scan('no-empty-after-reluctant', 'const r = /a*?b/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a lazy plus (min == 1)', () => {
    const diagnostics = scan('no-empty-after-reluctant', 'const r = /a+?/;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a greedy star', () => {
    const diagnostics = scan('no-empty-after-reluctant', 'const r = /a*/;');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('file-name-differ-from-class', () => {
  it('reports when the exported class name does not match the file stem', () => {
    const diagnostics = scan('file-name-differ-from-class', 'export class Foo {}', 'bar.ts');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('file-name-differ-from-class');
    expect(diagnostics[0].messageId).toBe('fileNameDifferFromClass');
  });

  it('does not report when the class name matches the stem exactly', () => {
    const diagnostics = scan('file-name-differ-from-class', 'export class Foo {}', 'foo.ts');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when PascalCase class matches a kebab-case stem', () => {
    const diagnostics = scan(
      'file-name-differ-from-class',
      'export class MyClass {}',
      'my-class.ts',
    );
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when there is no exported class', () => {
    const diagnostics = scan('file-name-differ-from-class', 'class Foo {} export {};', 'bar.ts');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when there are multiple exported classes', () => {
    const diagnostics = scan(
      'file-name-differ-from-class',
      'export class Foo {} export class Bar {}',
      'baz.ts',
    );
    expect(diagnostics).toHaveLength(0);
  });

  it('reports a function declaration called both as plain call and constructor', () => {
    const diagnostics = scan('inconsistent-function-call', 'function f() {} f(); new f();');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('inconsistent-function-call');
    expect(diagnostics[0].messageId).toBe('inconsistentFunctionCall');
  });

  it('does not report a function called only as a plain call', () => {
    const diagnostics = scan('inconsistent-function-call', 'function f() {} f(); f();');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a function called only as a constructor', () => {
    const diagnostics = scan('inconsistent-function-call', 'function f() {} new f(); new f();');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports no-empty-test-file for a test file with no it/test calls', () => {
    const diagnostics = scan('no-empty-test-file', "import {x} from './x';", 'foo.test.ts');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-empty-test-file');
    expect(diagnostics[0].messageId).toBe('emptyTestFile');
  });

  it('does not report no-empty-test-file when it() is present in a test file', () => {
    const diagnostics = scan('no-empty-test-file', "it('works', () => {});", 'foo.test.ts');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report no-empty-test-file for a non-test filename', () => {
    const diagnostics = scan('no-empty-test-file', "import {x} from './x';", 'foo.ts');
    expect(diagnostics).toHaveLength(0);
  });

  it('reports expression-complexity when expression has more than 3 operators', () => {
    // 4 logical && operators: a&&b&&c&&d&&e → 4 > default threshold 3 → 1 diagnostic
    const source = 'const x = a && b && c && d && e;';
    const diagnostics = scan('expression-complexity', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('expression-complexity');
    expect(diagnostics[0].messageId).toBe('expressionComplexity');
  });

  it('does not report expression-complexity when expression is at or below the threshold', () => {
    // 3 logical && operators: a&&b&&c&&d → 3 is not > 3 → 0 diagnostics
    const source = 'const x = a && b && c && d;';
    const diagnostics = scan('expression-complexity', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('prefer-regexp-exec rule', () => {
  it('reports String#match with a non-global RegExp literal', () => {
    const diagnostics = scan('prefer-regexp-exec', 'const result = str.match(/foo/u);');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('prefer-regexp-exec');
    expect(diagnostics[0].messageId).toBe('preferRegExpExec');
  });

  it('does not report global RegExp literals', () => {
    const diagnostics = scan('prefer-regexp-exec', 'const result = str.match(/foo/gu);');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report dynamic match arguments', () => {
    const diagnostics = scan('prefer-regexp-exec', 'const result = str.match(pattern);');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-redundant-assignments rule', () => {
  it('reports a self-assignment x = x', () => {
    const diagnostics = scan('no-redundant-assignments', 'x = x;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-redundant-assignments');
    expect(diagnostics[0].messageId).toBe('redundantAssignment');
  });

  it('reports the first of two adjacent assignments to the same identifier', () => {
    const diagnostics = scan('no-redundant-assignments', 'let y = 0;\ny = 1;\ny = 2;');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-redundant-assignments');
    expect(diagnostics[0].messageId).toBe('redundantAssignment');
  });

  it('does not report a regular assignment to a different identifier', () => {
    const diagnostics = scan('no-redundant-assignments', 'x = y;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a read-modify-write', () => {
    const diagnostics = scan('no-redundant-assignments', 'let x = 1;\nx = x + 1;');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when an intervening statement separates the assignments', () => {
    const diagnostics = scan('no-redundant-assignments', 'let x = 1;\nfoo();\nx = 2;');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-unused-collection rule', () => {
  it('reports an array that is only written to via push', () => {
    const source = 'const a = [];\na.push(1);\na.push(2);';
    const diagnostics = scan('no-unused-collection', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-unused-collection');
    expect(diagnostics[0].messageId).toBe('unusedCollection');
  });

  it('reports a Map that is only written to via set', () => {
    const source = "const m = new Map();\nm.set('k', 1);";
    const diagnostics = scan('no-unused-collection', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-unused-collection');
    expect(diagnostics[0].messageId).toBe('unusedCollection');
  });

  it('does not report when the array is returned', () => {
    const source = 'function f() { const a = [];\na.push(1);\nreturn a; }';
    const diagnostics = scan('no-unused-collection', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the array is passed to a function', () => {
    const source = 'const a = [];\na.push(1);\nconsole.log(a);';
    const diagnostics = scan('no-unused-collection', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the length property is read', () => {
    const source = 'const a = [];\na.push(1);\nconst b = a.length;';
    const diagnostics = scan('no-unused-collection', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-empty-collection rule', () => {
  it('reports an array that is read but never populated', () => {
    const source = 'const a = [];\nfunction f() { return a.length; }';
    const diagnostics = scan('no-empty-collection', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-empty-collection');
    expect(diagnostics[0].messageId).toBe('emptyCollection');
  });

  it('reports a Map that is queried but never populated', () => {
    const source = 'const m = new Map();\nfunction f(k) { return m.has(k); }';
    const diagnostics = scan('no-empty-collection', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('emptyCollection');
  });

  it('does not report when the array is populated via push', () => {
    const source = 'const a = [];\na.push(1);\nfunction f() { return a.length; }';
    const diagnostics = scan('no-empty-collection', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the array is passed to a function', () => {
    const source = 'const a = [];\nfill(a);\nfunction f() { return a.length; }';
    const diagnostics = scan('no-empty-collection', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report object literals', () => {
    const source = 'const o = {};\nfunction f() { return o.x; }';
    const diagnostics = scan('no-empty-collection', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-redundant-parentheses rule', () => {
  it('reports nested double parentheses', () => {
    const source = 'const x = ((1));';
    const diagnostics = scan('no-redundant-parentheses', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-redundant-parentheses');
    expect(diagnostics[0].messageId).toBe('redundantParentheses');
  });

  it('reports twice for triple nesting', () => {
    const source = 'const z = (((a)));';
    const diagnostics = scan('no-redundant-parentheses', source);
    expect(diagnostics).toHaveLength(2);
  });

  it('does not report a single pair', () => {
    const source = 'const x = (1);';
    const diagnostics = scan('no-redundant-parentheses', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report precedence grouping', () => {
    const source = 'const r = (a + b) * c;';
    const diagnostics = scan('no-redundant-parentheses', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('disabled-timeout rule', () => {
  it('reports a this.timeout value past the 32-bit maximum', () => {
    const source = 'this.timeout(2147483648);';
    const diagnostics = scan('disabled-timeout', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('disabled-timeout');
    expect(diagnostics[0].messageId).toBe('disabledTimeout');
  });

  it('does not report this.timeout(0)', () => {
    const source = 'this.timeout(0);';
    const diagnostics = scan('disabled-timeout', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a value within the valid range', () => {
    const source = 'this.timeout(5000);';
    const diagnostics = scan('disabled-timeout', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-this receiver', () => {
    const source = 'foo.timeout(2147483648);';
    const diagnostics = scan('disabled-timeout', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('cookie-no-httponly rule', () => {
  it('reports an httpOnly: false config property', () => {
    const source = 'const c = { httpOnly: false };';
    const diagnostics = scan('cookie-no-httponly', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('cookie-no-httponly');
    expect(diagnostics[0].messageId).toBe('cookieNoHttpOnly');
  });

  it('reports a nested cookie httpOnly: false property', () => {
    const source = 'session({ cookie: { httpOnly: false } });';
    const diagnostics = scan('cookie-no-httponly', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report httpOnly: true', () => {
    const source = 'const c = { httpOnly: true };';
    const diagnostics = scan('cookie-no-httponly', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a dynamic httpOnly value', () => {
    const source = 'const c = { httpOnly: x };';
    const diagnostics = scan('cookie-no-httponly', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key set to false', () => {
    const source = 'const c = { secure: false };';
    const diagnostics = scan('cookie-no-httponly', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('content-security-policy rule', () => {
  it('reports helmet contentSecurityPolicy: false', () => {
    const source = 'helmet({ contentSecurityPolicy: false });';
    const diagnostics = scan('content-security-policy', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('content-security-policy');
    expect(diagnostics[0].messageId).toBe('contentSecurityPolicy');
  });

  it('reports a direct contentSecurityPolicy: false property', () => {
    const source = 'const x = { contentSecurityPolicy: false };';
    const diagnostics = scan('content-security-policy', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report contentSecurityPolicy: true', () => {
    const source = 'helmet({ contentSecurityPolicy: true });';
    const diagnostics = scan('content-security-policy', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a dynamic contentSecurityPolicy value', () => {
    const source = 'const x = { contentSecurityPolicy: opts };';
    const diagnostics = scan('content-security-policy', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key set to false', () => {
    const source = 'const x = { csp: false };';
    const diagnostics = scan('content-security-policy', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('certificate-transparency rule', () => {
  it('reports helmet expectCt: false', () => {
    const source = 'helmet({ expectCt: false });';
    const diagnostics = scan('certificate-transparency', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('certificate-transparency');
    expect(diagnostics[0].messageId).toBe('certificateTransparency');
  });

  it('reports a direct expectCt: false config property', () => {
    const source = 'const x = { expectCt: false };';
    const diagnostics = scan('certificate-transparency', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report expectCt: true', () => {
    const source = 'const x = { expectCt: true };';
    const diagnostics = scan('certificate-transparency', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a dynamic expectCt value', () => {
    const source = 'const x = { expectCt: o };';
    const diagnostics = scan('certificate-transparency', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key set to false', () => {
    const source = 'const x = { other: false };';
    const diagnostics = scan('certificate-transparency', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('csrf rule', () => {
  it('reports an unsafe method listed alongside a safe one', () => {
    const source = 'csrf({ ignoreMethods: ["POST", "GET"] });';
    const diagnostics = scan('csrf', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('csrf');
    expect(diagnostics[0].messageId).toBe('csrf');
  });

  it('reports a single unsafe method', () => {
    const source = 'csrf({ ignoreMethods: ["PUT"] });';
    const diagnostics = scan('csrf', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report when only safe methods are ignored', () => {
    const source = 'csrf({ ignoreMethods: ["GET", "HEAD", "OPTIONS"] });';
    const diagnostics = scan('csrf', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a bare csrf() call', () => {
    const source = 'csrf();';
    const diagnostics = scan('csrf', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when there is no ignoreMethods key', () => {
    const source = 'csrf({ cookie: true });';
    const diagnostics = scan('csrf', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-csrf callee', () => {
    const source = 'foo({ ignoreMethods: ["POST"] });';
    const diagnostics = scan('csrf', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('file-permissions rule', () => {
  it('reports a chmodSync mode that grants rwx to others', () => {
    const source = 'fs.chmodSync("/x", 0o777);';
    const diagnostics = scan('file-permissions', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('file-permissions');
    expect(diagnostics[0].messageId).toBe('weakFilePermissions');
  });

  it('reports an async chmod with a trailing callback', () => {
    const source = 'fs.chmod("/x", 0o666, cb);';
    const diagnostics = scan('file-permissions', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('reports a permissive umask', () => {
    const source = 'process.umask(0o000);';
    const diagnostics = scan('file-permissions', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report a chmod mode without "others" bits', () => {
    const source = 'fs.chmodSync("/x", 0o750);';
    const diagnostics = scan('file-permissions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a restrictive umask', () => {
    const source = 'process.umask(0o077);';
    const diagnostics = scan('file-permissions', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a dynamic (non-literal) mode', () => {
    const source = 'fs.chmodSync("/x", mode);';
    const diagnostics = scan('file-permissions', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('file-uploads rule', () => {
  it('reports a diskStorage configuration without a destination', () => {
    const source = 'multer.diskStorage({ filename: fn });';
    const diagnostics = scan('file-uploads', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('file-uploads');
    expect(diagnostics[0].messageId).toBe('fileUploads');
  });

  it('does not report a diskStorage configuration with a destination', () => {
    const source = 'multer.diskStorage({ destination: "/up", filename: fn });';
    const diagnostics = scan('file-uploads', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('flags an aliased receiver missing a destination', () => {
    const source = 'foo.diskStorage({ filename: fn });';
    const diagnostics = scan('file-uploads', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report a diskStorage call without an object argument', () => {
    const source = 'multer.diskStorage();';
    const diagnostics = scan('file-uploads', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an unrelated call', () => {
    const source = 'bar();';
    const diagnostics = scan('file-uploads', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('cors rule', () => {
  it('reports a setHeader call with a wildcard Access-Control-Allow-Origin', () => {
    const source = 'res.setHeader("Access-Control-Allow-Origin", "*");';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('cors');
    expect(diagnostics[0].messageId).toBe('cors');
  });

  it('reports a cors() middleware call with origin "*"', () => {
    const source = 'cors({ origin: "*" });';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('reports a headers object literal with a wildcard origin', () => {
    const source = 'res.writeHead(200, { "Access-Control-Allow-Origin": "*" });';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report a specific origin in setHeader', () => {
    const source = 'res.setHeader("Access-Control-Allow-Origin", "https://ex.com");';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a specific origin in cors()', () => {
    const source = 'cors({ origin: "https://ex.com" });';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a bare cors() call', () => {
    const source = 'cors();';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an unrelated setHeader call', () => {
    const source = 'res.setHeader("Content-Type", "x");';
    const diagnostics = scan('cors', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('dns-prefetching rule', () => {
  it('reports dnsPrefetchControl with allow: true', () => {
    const source = 'helmet.dnsPrefetchControl({ allow: true });';
    const diagnostics = scan('dns-prefetching', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('dns-prefetching');
    expect(diagnostics[0].messageId).toBe('dnsPrefetching');
  });

  it('does not report dnsPrefetchControl with allow: false', () => {
    const source = 'helmet.dnsPrefetchControl({ allow: false });';
    const diagnostics = scan('dns-prefetching', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report dnsPrefetchControl with no arguments', () => {
    const source = 'helmet.dnsPrefetchControl();';
    const diagnostics = scan('dns-prefetching', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal allow value', () => {
    const source = 'helmet.dnsPrefetchControl({ allow: x });';
    const diagnostics = scan('dns-prefetching', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('disabled-auto-escaping rule', () => {
  it('reports a Handlebars noEscape: true compile option', () => {
    const source = 'Handlebars.compile(src, { noEscape: true });';
    const diagnostics = scan('disabled-auto-escaping', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('disabled-auto-escaping');
    expect(diagnostics[0].messageId).toBe('disabledAutoEscaping');
  });

  it('reports overriding Mustache.escape', () => {
    const source = 'Mustache.escape = function (t) { return t; };';
    const diagnostics = scan('disabled-auto-escaping', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('disabledAutoEscaping');
  });

  it('does not report noEscape: false', () => {
    const source = 'Handlebars.compile(src, { noEscape: false });';
    const diagnostics = scan('disabled-auto-escaping', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal noEscape value', () => {
    const source = 'Handlebars.compile(src, { noEscape: x });';
    const diagnostics = scan('disabled-auto-escaping', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a generic html: true option', () => {
    const source = 'md({ html: true });';
    const diagnostics = scan('disabled-auto-escaping', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-s3-bucket-granted-access rule', () => {
  it('reports a PUBLIC_READ_WRITE access control', () => {
    const source =
      "new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PUBLIC_READ_WRITE });";
    const diagnostics = scan('aws-s3-bucket-granted-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-s3-bucket-granted-access');
    expect(diagnostics[0].messageId).toBe('s3PublicAccess');
  });

  it('reports a PUBLIC_READ access control', () => {
    const source =
      "new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PUBLIC_READ });";
    const diagnostics = scan('aws-s3-bucket-granted-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('s3PublicAccess');
  });

  it('reports an AUTHENTICATED_READ access control', () => {
    const source =
      "new s3.Bucket(this, 'b', { accessControl: BucketAccessControl.AUTHENTICATED_READ });";
    const diagnostics = scan('aws-s3-bucket-granted-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('s3PublicAccess');
  });

  it('does not report a PRIVATE access control', () => {
    const source = "new s3.Bucket(this, 'b', { accessControl: s3.BucketAccessControl.PRIVATE });";
    const diagnostics = scan('aws-s3-bucket-granted-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-member access control value', () => {
    const source = "new s3.Bucket(this, 'b', { accessControl: x });";
    const diagnostics = scan('aws-s3-bucket-granted-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a granting member under a different key', () => {
    const source = "new s3.Bucket(this, 'b', { other: BucketAccessControl.PUBLIC_READ });";
    const diagnostics = scan('aws-s3-bucket-granted-access', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-rds-unencrypted-databases rule', () => {
  it('reports a CDK construct created with storageEncrypted: false', () => {
    const source = "new DatabaseInstance(this, 'db', { storageEncrypted: false });";
    const diagnostics = scan('aws-rds-unencrypted-databases', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-rds-unencrypted-databases');
    expect(diagnostics[0].messageId).toBe('rdsUnencrypted');
  });

  it('reports a direct storageEncrypted: false literal', () => {
    const source = 'const x = { storageEncrypted: false };';
    const diagnostics = scan('aws-rds-unencrypted-databases', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('rdsUnencrypted');
  });

  it('does not report storageEncrypted: true', () => {
    const source = 'const x = { storageEncrypted: true };';
    const diagnostics = scan('aws-rds-unencrypted-databases', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal storageEncrypted value', () => {
    const source = 'const x = { storageEncrypted: flag };';
    const diagnostics = scan('aws-rds-unencrypted-databases', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key', () => {
    const source = 'const x = { encrypted: false };';
    const diagnostics = scan('aws-rds-unencrypted-databases', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-iam-public-access rule', () => {
  it('reports new iam.AnyPrincipal()', () => {
    const source = 'new iam.AnyPrincipal();';
    const diagnostics = scan('aws-iam-public-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-iam-public-access');
    expect(diagnostics[0].messageId).toBe('iamPublicAccess');
  });

  it('reports a bare new AnyPrincipal()', () => {
    const source = 'new AnyPrincipal();';
    const diagnostics = scan('aws-iam-public-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('iamPublicAccess');
  });

  it('does not report new iam.AccountRootPrincipal()', () => {
    const source = 'new iam.AccountRootPrincipal();';
    const diagnostics = scan('aws-iam-public-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report new ArnPrincipal(arn)', () => {
    const source = 'new ArnPrincipal(arn);';
    const diagnostics = scan('aws-iam-public-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an AnyPrincipal reference without new', () => {
    const source = 'const p = iam.AnyPrincipal;';
    const diagnostics = scan('aws-iam-public-access', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('hidden-files rule', () => {
  it('reports a serve-static dotfiles: allow option', () => {
    const source = "serveStatic('public', { dotfiles: 'allow' });";
    const diagnostics = scan('hidden-files', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('hidden-files');
    expect(diagnostics[0].messageId).toBe('hiddenFiles');
  });

  it('reports a string-literal dotfiles key', () => {
    const source = "const x = { 'dotfiles': 'allow' };";
    const diagnostics = scan('hidden-files', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('hiddenFiles');
  });

  it('does not report dotfiles: ignore', () => {
    const source = "const x = { dotfiles: 'ignore' };";
    const diagnostics = scan('hidden-files', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal dotfiles value', () => {
    const source = 'const x = { dotfiles: y };';
    const diagnostics = scan('hidden-files', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key', () => {
    const source = "const x = { other: 'allow' };";
    const diagnostics = scan('hidden-files', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-apigateway-public-api rule', () => {
  it('reports authorizationType: AuthorizationType.NONE', () => {
    const source =
      "resource.addMethod('GET', i, { authorizationType: apigateway.AuthorizationType.NONE });";
    const diagnostics = scan('aws-apigateway-public-api', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-apigateway-public-api');
    expect(diagnostics[0].messageId).toBe('apigatewayPublicApi');
  });

  it('reports authorizationType: "NONE"', () => {
    const source = "new apigateway.CfnRoute(this, 'r', { authorizationType: 'NONE' });";
    const diagnostics = scan('aws-apigateway-public-api', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('apigatewayPublicApi');
  });

  it('does not report authorizationType: AuthorizationType.IAM', () => {
    const source = 'x = { authorizationType: AuthorizationType.IAM };';
    const diagnostics = scan('aws-apigateway-public-api', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report authorizationType: "AWS_IAM"', () => {
    const source = "x = { authorizationType: 'AWS_IAM' };";
    const diagnostics = scan('aws-apigateway-public-api', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal authorizationType value', () => {
    const source = 'x = { authorizationType: authType };';
    const diagnostics = scan('aws-apigateway-public-api', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report "NONE" on a different key', () => {
    const source = "x = { other: 'NONE' };";
    const diagnostics = scan('aws-apigateway-public-api', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-iam-all-privileges rule', () => {
  it('reports actions granting all privileges via "*"', () => {
    const source = 'new PolicyStatement({ actions: ["*"], resources: [bucket] });';
    const diagnostics = scan('aws-iam-all-privileges', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-iam-all-privileges');
    expect(diagnostics[0].messageId).toBe('iamAllPrivileges');
  });

  it('does not report specific actions', () => {
    const source = 'new PolicyStatement({ actions: ["s3:GetObject"] });';
    const diagnostics = scan('aws-iam-all-privileges', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an empty actions array', () => {
    const source = 'new PolicyStatement({ actions: [] });';
    const diagnostics = scan('aws-iam-all-privileges', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-array actions value', () => {
    const source = 'new PolicyStatement({ actions: x });';
    const diagnostics = scan('aws-iam-all-privileges', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a wildcard under a different key', () => {
    const source = 'new PolicyStatement({ other: ["*"] });';
    const diagnostics = scan('aws-iam-all-privileges', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-s3-bucket-versioning rule', () => {
  it('reports a CDK S3 bucket with versioned: false', () => {
    const source = "new s3.Bucket(this, 'b', { versioned: false });";
    const diagnostics = scan('aws-s3-bucket-versioning', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-s3-bucket-versioning');
    expect(diagnostics[0].messageId).toBe('s3BucketVersioning');
  });

  it('reports a plain object with versioned: false', () => {
    const source = 'const x = { versioned: false };';
    const diagnostics = scan('aws-s3-bucket-versioning', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('s3BucketVersioning');
  });

  it('does not report versioned: true', () => {
    const source = 'const x = { versioned: true };';
    const diagnostics = scan('aws-s3-bucket-versioning', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal versioned value', () => {
    const source = 'const x = { versioned: flag };';
    const diagnostics = scan('aws-s3-bucket-versioning', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key', () => {
    const source = 'const x = { other: false };';
    const diagnostics = scan('aws-s3-bucket-versioning', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-ec2-rds-dms-public rule', () => {
  it('reports publiclyAccessible set to true', () => {
    const source = "new ec2.Instance(this,'i',{ publiclyAccessible: true })";
    const diagnostics = scan('aws-ec2-rds-dms-public', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-ec2-rds-dms-public');
    expect(diagnostics[0].messageId).toBe('ec2RdsDmsPublic');
  });

  it('reports associatePublicIpAddress set to true', () => {
    const source =
      "new ec2.CfnInstance(this,'i',{ networkInterfaces: [{ associatePublicIpAddress: true }] })";
    const diagnostics = scan('aws-ec2-rds-dms-public', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('ec2RdsDmsPublic');
  });

  it('does not report publiclyAccessible set to false', () => {
    const source = "new ec2.Instance(this,'i',{ publiclyAccessible: false })";
    const diagnostics = scan('aws-ec2-rds-dms-public', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal value', () => {
    const source = "new ec2.Instance(this,'i',{ publiclyAccessible: x })";
    const diagnostics = scan('aws-ec2-rds-dms-public', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a true value under a different key', () => {
    const source = "new ec2.Instance(this,'i',{ other: true })";
    const diagnostics = scan('aws-ec2-rds-dms-public', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-s3-bucket-public-access rule', () => {
  it('reports blockPublicAcls set to false', () => {
    const source = 'new s3.BlockPublicAccess({ blockPublicAcls: false });';
    const diagnostics = scan('aws-s3-bucket-public-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-s3-bucket-public-access');
    expect(diagnostics[0].messageId).toBe('s3BucketPublicAccess');
  });

  it('reports restrictPublicBuckets set to false', () => {
    const source = 'new s3.BlockPublicAccess({ restrictPublicBuckets: false });';
    const diagnostics = scan('aws-s3-bucket-public-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('s3BucketPublicAccess');
  });

  it('does not report a sub-key set to true', () => {
    const source = 'new s3.BlockPublicAccess({ blockPublicAcls: true });';
    const diagnostics = scan('aws-s3-bucket-public-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal value', () => {
    const source = 'new s3.BlockPublicAccess({ blockPublicAcls: x });';
    const diagnostics = scan('aws-s3-bucket-public-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report false under a different key', () => {
    const source = 'new s3.BlockPublicAccess({ other: false });';
    const diagnostics = scan('aws-s3-bucket-public-access', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('confidential-information-logging rule', () => {
  it('reports a Signale logger with an empty secrets list', () => {
    const source = 'new Signale({ secrets: [] });';
    const diagnostics = scan('confidential-information-logging', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('confidential-information-logging');
    expect(diagnostics[0].messageId).toBe('confidentialLogging');
  });

  it('does not report a non-empty secrets list', () => {
    const source = 'new Signale({ secrets: ["password"] });';
    const diagnostics = scan('confidential-information-logging', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when there is no secrets key', () => {
    const source = 'new Signale({});';
    const diagnostics = scan('confidential-information-logging', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different callee', () => {
    const source = 'new Other({ secrets: [] });';
    const diagnostics = scan('confidential-information-logging', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-iam-all-resources-accessible rule', () => {
  it('reports resources granting access to all via "*"', () => {
    const source = 'new PolicyStatement({ resources: ["*"] });';
    const diagnostics = scan('aws-iam-all-resources-accessible', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-iam-all-resources-accessible');
    expect(diagnostics[0].messageId).toBe('iamAllResources');
  });

  it('does not report a specific resource', () => {
    const source = 'new PolicyStatement({ resources: ["arn:aws:s3:::x"] });';
    const diagnostics = scan('aws-iam-all-resources-accessible', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an empty resources array', () => {
    const source = 'new PolicyStatement({ resources: [] });';
    const diagnostics = scan('aws-iam-all-resources-accessible', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-array resources value', () => {
    const source = 'new PolicyStatement({ resources: x });';
    const diagnostics = scan('aws-iam-all-resources-accessible', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a wildcard under a different key', () => {
    const source = 'new PolicyStatement({ other: ["*"] });';
    const diagnostics = scan('aws-iam-all-resources-accessible', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-ec2-unencrypted-ebs-volume rule', () => {
  it('reports an EBS Volume created with encrypted: false', () => {
    const source = "new ec2.Volume(this, 'v', { encrypted: false });";
    const diagnostics = scan('aws-ec2-unencrypted-ebs-volume', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-ec2-unencrypted-ebs-volume');
    expect(diagnostics[0].messageId).toBe('ebsUnencrypted');
  });

  it('reports a bare Volume callee with an extra property', () => {
    const source = "new Volume(this, 'v', { encrypted: false, size: x });";
    const diagnostics = scan('aws-ec2-unencrypted-ebs-volume', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-ec2-unencrypted-ebs-volume');
  });

  it('does not report encrypted: true', () => {
    const source = "new ec2.Volume(this, 'v', { encrypted: true });";
    const diagnostics = scan('aws-ec2-unencrypted-ebs-volume', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an absent encrypted property', () => {
    const source = "new Volume(this, 'v', {});";
    const diagnostics = scan('aws-ec2-unencrypted-ebs-volume', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different construct (EFS FileSystem)', () => {
    const source = "new FileSystem(this, 'f', { encrypted: false });";
    const diagnostics = scan('aws-ec2-unencrypted-ebs-volume', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a Volume without an options object', () => {
    const source = "new ec2.Volume(this, 'v');";
    const diagnostics = scan('aws-ec2-unencrypted-ebs-volume', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-efs-unencrypted rule', () => {
  it('reports an efs.FileSystem created with encrypted: false', () => {
    const source = "new efs.FileSystem(this, 'f', { encrypted: false });";
    const diagnostics = scan('aws-efs-unencrypted', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-efs-unencrypted');
    expect(diagnostics[0].messageId).toBe('efsUnencrypted');
  });

  it('reports a bare FileSystem callee alongside other props', () => {
    const source = "new FileSystem(this, 'f', { encrypted: false, vpc: v });";
    const diagnostics = scan('aws-efs-unencrypted', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('efsUnencrypted');
  });

  it('does not report when encryption is enabled', () => {
    const source = "new efs.FileSystem(this, 'f', { encrypted: true });";
    const diagnostics = scan('aws-efs-unencrypted', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report when the encrypted property is absent', () => {
    const source = "new FileSystem(this, 'f', {});";
    const diagnostics = scan('aws-efs-unencrypted', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different construct', () => {
    const source = "new Volume(this, 'v', { encrypted: false });";
    const diagnostics = scan('aws-efs-unencrypted', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a FileSystem without an options object', () => {
    const source = "new FileSystem(this, 'f');";
    const diagnostics = scan('aws-efs-unencrypted', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('aws-restricted-ip-admin-access rule', () => {
  it('reports anyIpv4() with SSH port 22', () => {
    const source = 'sg.addIngressRule(ec2.Peer.anyIpv4(), ec2.Port.tcp(22));';
    const diagnostics = scan('aws-restricted-ip-admin-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('aws-restricted-ip-admin-access');
    expect(diagnostics[0].messageId).toBe('restrictedIpAdminAccess');
  });

  it('reports anyIpv6() with RDP port 3389', () => {
    const source = 'sg.addIngressRule(Peer.anyIpv6(), Port.tcp(3389));';
    const diagnostics = scan('aws-restricted-ip-admin-access', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('restrictedIpAdminAccess');
  });

  it('does not report a specific CIDR peer', () => {
    const source = 'sg.addIngressRule(Peer.ipv4("10.0.0.0/16"), Port.tcp(22));';
    const diagnostics = scan('aws-restricted-ip-admin-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-administration port', () => {
    const source = 'sg.addIngressRule(Peer.anyIpv4(), Port.tcp(443));';
    const diagnostics = scan('aws-restricted-ip-admin-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a single-argument call', () => {
    const source = 'sg.addIngressRule(Peer.anyIpv4());';
    const diagnostics = scan('aws-restricted-ip-admin-access', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an unrelated call', () => {
    const source = 'foo(a, b);';
    const diagnostics = scan('aws-restricted-ip-admin-access', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('redundant-type-aliases rule', () => {
  it('reports an alias to the string keyword', () => {
    const source = 'type MyString = string;';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('redundant-type-aliases');
    expect(diagnostics[0].messageId).toBe('redundantTypeAlias');
  });

  it('reports an alias to the boolean keyword', () => {
    const source = 'type B = boolean;';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('redundantTypeAlias');
  });

  it('reports a bare type reference alias', () => {
    const source = 'type X = Y;';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('redundant-type-aliases');
  });

  it('does not report a generic alias with a type parameter', () => {
    const source = 'type Box<T> = T;';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a union type', () => {
    const source = 'type U = string | number;';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a type reference with type arguments', () => {
    const source = 'type Arr = Array<string>;';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an object type', () => {
    const source = 'type O = { a: number };';
    const diagnostics = scan('redundant-type-aliases', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('jsx-no-leaked-render rule', () => {
  it('reports a .length numeric leak before JSX', () => {
    const source = 'const x = <div>{items.length && <List/>}</div>;';
    const diagnostics = scan('jsx-no-leaked-render', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('jsx-no-leaked-render');
    expect(diagnostics[0].messageId).toBe('jsxNoLeakedRender');
  });

  it('reports a numeric literal before JSX', () => {
    const source = 'const x = <div>{0 && <X/>}</div>;';
    const diagnostics = scan('jsx-no-leaked-render', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('jsxNoLeakedRender');
  });

  it('does not report an explicit boolean comparison', () => {
    const source = 'const x = <div>{items.length > 0 && <List/>}</div>;';
    const diagnostics = scan('jsx-no-leaked-render', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a plain identifier', () => {
    const source = 'const x = <div>{show && <X/>}</div>;';
    const diagnostics = scan('jsx-no-leaked-render', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report the || operator', () => {
    const source = 'const x = <div>{a.length || <X/>}</div>;';
    const diagnostics = scan('jsx-no-leaked-render', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-JSX right operand', () => {
    const source = 'cond && doThing();';
    const diagnostics = scan('jsx-no-leaked-render', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('insecure-cookie rule', () => {
  it('reports secure: false with an httpOnly cookie-marker sibling', () => {
    const source = 'const c = { secure: false, httpOnly: true };';
    const diagnostics = scan('insecure-cookie', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('insecure-cookie');
    expect(diagnostics[0].messageId).toBe('insecureCookie');
  });

  it('reports secure: false in a nested cookie config', () => {
    const source = "session({ cookie: { secure: false, sameSite: 'lax' } });";
    const diagnostics = scan('insecure-cookie', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('insecureCookie');
  });

  it('does not report secure: false without a cookie-marker sibling', () => {
    const source = 'const c = { secure: false };';
    const diagnostics = scan('insecure-cookie', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report secure: true', () => {
    const source = 'const c = { secure: true, httpOnly: true };';
    const diagnostics = scan('insecure-cookie', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a TLS config without a cookie marker', () => {
    const source = 'const tls = { secure: false, rejectUnauthorized: false };';
    const diagnostics = scan('insecure-cookie', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal secure value', () => {
    const source = 'const c = { secure: x, maxAge: 1 };';
    const diagnostics = scan('insecure-cookie', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-hook-setter-in-body rule', () => {
  it('reports a useState setter called directly in the component body', () => {
    const source = 'function C(){ const [v,setV]=useState(0); setV(1); return null; }';
    const diagnostics = scan('no-hook-setter-in-body', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-hook-setter-in-body');
    expect(diagnostics[0].messageId).toBe('noHookSetterInBody');
  });

  it('reports a React.useState setter called directly in the body', () => {
    const source = 'function C(){ const [v,setV]=React.useState(0); setV(1); return null; }';
    const diagnostics = scan('no-hook-setter-in-body', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noHookSetterInBody');
  });

  it('does not report a setter called inside an event handler', () => {
    const source =
      'function C(){ const [v,setV]=useState(0); const onClick=()=>setV(1); return null; }';
    const diagnostics = scan('no-hook-setter-in-body', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a setter called inside a conditional', () => {
    const source = 'function C(){ const [v,setV]=useState(0); if(x) setV(1); return null; }';
    const diagnostics = scan('no-hook-setter-in-body', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a setter called inside an effect callback', () => {
    const source =
      'function C(){ const [v,setV]=useState(0); useEffect(()=>setV(1)); return null; }';
    const diagnostics = scan('no-hook-setter-in-body', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a call that is not a useState setter', () => {
    const source = 'function C(){ foo(); return null; }';
    const diagnostics = scan('no-hook-setter-in-body', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('content-length rule', () => {
  it('reports a multer fileSize limit over 8MB', () => {
    const source = 'multer({ limits: { fileSize: 10000000 } });';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('content-length');
    expect(diagnostics[0].messageId).toBe('contentLength');
  });

  it('reports a maxFileSize object property over 8MB', () => {
    const source = 'const cfg = { maxFileSize: 9000000 };';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('contentLength');
  });

  it('reports a maxFileSize member assignment over 8MB', () => {
    const source = 'form.maxFileSize = 10000000;';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('contentLength');
  });

  it('does not report a fileSize value within the 8MB limit', () => {
    const source = 'const cfg = { fileSize: 1000000 };';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a string fileSize value', () => {
    const source = 'const cfg = { fileSize: "4mb" };';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a generic limit key', () => {
    const source = 'const cfg = { limit: 10000000 };';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal fileSize value', () => {
    const source = 'const cfg = { fileSize: x };';
    const diagnostics = scan('content-length', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('unverified-certificate rule', () => {
  it('reports rejectUnauthorized:false in https.request options', () => {
    const source = 'https.request({ rejectUnauthorized: false });';
    const diagnostics = scan('unverified-certificate', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('unverified-certificate');
    expect(diagnostics[0].messageId).toBe('unverifiedCertificate');
  });

  it('reports rejectUnauthorized:false in a direct options literal', () => {
    const source = 'const o = { rejectUnauthorized: false };';
    const diagnostics = scan('unverified-certificate', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('unverifiedCertificate');
  });

  it('does not report rejectUnauthorized:true', () => {
    const source = 'const o = { rejectUnauthorized: true };';
    const diagnostics = scan('unverified-certificate', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal rejectUnauthorized value', () => {
    const source = 'const o = { rejectUnauthorized: x };';
    const diagnostics = scan('unverified-certificate', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key', () => {
    const source = 'const o = { other: false };';
    const diagnostics = scan('unverified-certificate', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-mime-sniff rule', () => {
  it('reports helmet noSniff: false', () => {
    const source = 'helmet({ noSniff: false });';
    const diagnostics = scan('no-mime-sniff', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-mime-sniff');
    expect(diagnostics[0].messageId).toBe('noMimeSniff');
  });

  it('reports a direct noSniff: false property', () => {
    const source = 'const o = { noSniff: false };';
    const diagnostics = scan('no-mime-sniff', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report noSniff: true', () => {
    const source = 'const o = { noSniff: true };';
    const diagnostics = scan('no-mime-sniff', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal noSniff value', () => {
    const source = 'const o = { noSniff: x };';
    const diagnostics = scan('no-mime-sniff', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key set to false', () => {
    const source = 'const o = { other: false };';
    const diagnostics = scan('no-mime-sniff', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-ip-forward rule', () => {
  it('reports a createProxyServer config with xfwd: true', () => {
    const source = 'createProxyServer({ target: t, xfwd: true });';
    const diagnostics = scan('no-ip-forward', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-ip-forward');
    expect(diagnostics[0].messageId).toBe('noIpForward');
  });

  it('reports a direct xfwd: true property', () => {
    const source = 'const o = { xfwd: true };';
    const diagnostics = scan('no-ip-forward', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noIpForward');
  });

  it('does not report xfwd: false', () => {
    const source = 'const o = { xfwd: false };';
    const diagnostics = scan('no-ip-forward', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal xfwd value', () => {
    const source = 'const o = { xfwd: x };';
    const diagnostics = scan('no-ip-forward', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different key', () => {
    const source = 'const o = { other: true };';
    const diagnostics = scan('no-ip-forward', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-angular-bypass-sanitization rule', () => {
  it('reports a bypassSecurityTrustHtml call', () => {
    const source = 'this.sanitizer.bypassSecurityTrustHtml(x);';
    const diagnostics = scan('no-angular-bypass-sanitization', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-angular-bypass-sanitization');
    expect(diagnostics[0].messageId).toBe('angularBypassSanitization');
  });

  it('reports a bypassSecurityTrustResourceUrl call regardless of receiver', () => {
    const source = 'ds.bypassSecurityTrustResourceUrl(u);';
    const diagnostics = scan('no-angular-bypass-sanitization', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('angularBypassSanitization');
  });

  it('does not report an unrelated DomSanitizer method', () => {
    const source = 'this.sanitizer.sanitize(x);';
    const diagnostics = scan('no-angular-bypass-sanitization', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a method outside the bypass set', () => {
    const source = 'foo.bypassOther(x);';
    const diagnostics = scan('no-angular-bypass-sanitization', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a property access without a call', () => {
    const source = 'const f = this.sanitizer.bypassSecurityTrustHtml;';
    const diagnostics = scan('no-angular-bypass-sanitization', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('insecure-jwt-token rule', () => {
  it('reports algorithm: none in a sign options object', () => {
    const source = "jwt.sign(p, k, { algorithm: 'none' });";
    const diagnostics = scan('insecure-jwt-token', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('insecure-jwt-token');
    expect(diagnostics[0].messageId).toBe('insecureJwtToken');
  });

  it('reports algorithms: ["none"] in a verify options object', () => {
    const source = "jwt.verify(t, k, { algorithms: ['none'] });";
    const diagnostics = scan('insecure-jwt-token', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('insecureJwtToken');
  });

  it('reports the none algorithm case-insensitively', () => {
    const source = "const o = { algorithm: 'NONE' };";
    const diagnostics = scan('insecure-jwt-token', source);
    expect(diagnostics).toHaveLength(1);
  });

  it('does not report a strong algorithm', () => {
    const source = "const o = { algorithm: 'HS256' };";
    const diagnostics = scan('insecure-jwt-token', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a strong algorithms array', () => {
    const source = "const o = { algorithms: ['RS256'] };";
    const diagnostics = scan('insecure-jwt-token', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report an unrelated key', () => {
    const source = "const o = { other: 'none' };";
    const diagnostics = scan('insecure-jwt-token', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-useless-react-setstate rule', () => {
  it('reports a setter called with its own state in a handler', () => {
    const source = 'function C(){ const [v,setV]=useState(0); const h=()=>setV(v); return null; }';
    const diagnostics = scan('no-useless-react-setstate', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-useless-react-setstate');
    expect(diagnostics[0].messageId).toBe('noUselessReactSetstate');
  });

  it('reports a setter called with its own state inside JSX', () => {
    const source =
      'function C(){ const [v,setV]=React.useState(0); return <button onClick={()=>setV(v)}/>; }';
    const diagnostics = scan('no-useless-react-setstate', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noUselessReactSetstate');
  });

  it('does not report a derived value', () => {
    const source = 'function C(){ const [v,setV]=useState(0); setV(v+1); return null; }';
    const diagnostics = scan('no-useless-react-setstate', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a different variable', () => {
    const source = 'function C(){ const [v,setV]=useState(0); setV(other); return null; }';
    const diagnostics = scan('no-useless-react-setstate', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a setter call with no argument', () => {
    const source = 'function C(){ const [v,setV]=useState(0); setV(); return null; }';
    const diagnostics = scan('no-useless-react-setstate', source, 'sample.tsx');
    expect(diagnostics).toHaveLength(0);
  });
});

describe('no-referrer-policy rule', () => {
  it('reports a no-referrer-when-downgrade policy', () => {
    const source = "helmet.referrerPolicy({ policy: 'no-referrer-when-downgrade' });";
    const diagnostics = scan('no-referrer-policy', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('no-referrer-policy');
    expect(diagnostics[0].messageId).toBe('noReferrerPolicy');
  });

  it('reports an unsafe-url policy', () => {
    const source = "helmet.referrerPolicy({ policy: 'unsafe-url' });";
    const diagnostics = scan('no-referrer-policy', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('noReferrerPolicy');
  });

  it('does not report a no-referrer policy', () => {
    const source = "helmet.referrerPolicy({ policy: 'no-referrer' });";
    const diagnostics = scan('no-referrer-policy', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a same-origin policy', () => {
    const source = "helmet.referrerPolicy({ policy: 'same-origin' });";
    const diagnostics = scan('no-referrer-policy', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal value', () => {
    const source = 'const o = { policy: x };';
    const diagnostics = scan('no-referrer-policy', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a leaky value under a different key', () => {
    const source = "const o = { other: 'unsafe-url' };";
    const diagnostics = scan('no-referrer-policy', source);
    expect(diagnostics).toHaveLength(0);
  });
});

describe('weak-ssl rule', () => {
  it('reports a weak secureProtocol method', () => {
    const source = "const o = { secureProtocol: 'TLSv1_method' };";
    const diagnostics = scan('weak-ssl', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].ruleName).toBe('weak-ssl');
    expect(diagnostics[0].messageId).toBe('weakSsl');
  });

  it('reports a weak minVersion', () => {
    const source = "const o = { minVersion: 'TLSv1.1' };";
    const diagnostics = scan('weak-ssl', source);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].messageId).toBe('weakSsl');
  });

  it('does not report a strong secureProtocol method', () => {
    const source = "const o = { secureProtocol: 'TLSv1_2_method' };";
    const diagnostics = scan('weak-ssl', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a strong minVersion', () => {
    const source = "const o = { minVersion: 'TLSv1.2' };";
    const diagnostics = scan('weak-ssl', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a non-literal value', () => {
    const source = 'const o = { secureProtocol: x };';
    const diagnostics = scan('weak-ssl', source);
    expect(diagnostics).toHaveLength(0);
  });

  it('does not report a weak value under a different key', () => {
    const source = "const o = { other: 'TLSv1_method' };";
    const diagnostics = scan('weak-ssl', source);
    expect(diagnostics).toHaveLength(0);
  });
});
