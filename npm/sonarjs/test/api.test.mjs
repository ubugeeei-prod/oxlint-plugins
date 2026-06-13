import { describe, expect, it } from 'vitest';

import { implementedSonarjsRuleNames, scanSonarjs } from '../api.js';

const expectedRuleNames = [
  'no-nested-template-literals',
  'no-nested-switch',
  'no-nested-conditional',
  'no-collapsible-if',
  'no-redundant-boolean',
  'comma-or-logical-or-case',
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
});
