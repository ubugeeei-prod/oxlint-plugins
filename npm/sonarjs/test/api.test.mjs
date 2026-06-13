import { describe, expect, it } from 'vitest';

import { implementedSonarjsRuleNames, scanSonarjs } from '../api.js';

const expectedRuleNames = ['no-nested-template-literals'];

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

  it('ignores rules that are not enabled', () => {
    const diagnostics = scanSonarjs('const x = `outer ${`inner`}`;', 'sample.ts', {
      ruleNames: [],
    });
    expect(diagnostics).toHaveLength(0);
  });
});
