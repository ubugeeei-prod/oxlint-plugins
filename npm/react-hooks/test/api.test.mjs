import { describe, expect, it } from 'vitest';

import {
  implementedReactHooksRuleNames,
  isHookName,
  isReactComponentName,
  scanReactHooks,
} from '../api.js';

describe('react-hooks native api', () => {
  it('exposes the implemented rule set', () => {
    expect(implementedReactHooksRuleNames()).toEqual(['rules-of-hooks']);
  });

  it('classifies component and hook names like upstream', () => {
    expect(isReactComponentName('Component')).toBe(true);
    expect(isReactComponentName('CMS')).toBe(true);
    expect(isReactComponentName('component')).toBe(false);
    expect(isReactComponentName('useState')).toBe(false);

    expect(isHookName('use')).toBe(true);
    expect(isHookName('useState')).toBe(true);
    expect(isHookName('use2')).toBe(true);
    expect(isHookName('use_state')).toBe(false);
    expect(isHookName('reuseState')).toBe(false);
  });

  it('runs the rules-of-hooks scan in native code', () => {
    const diagnostics = scanReactHooks(
      [
        'function Component() {',
        '  if (cond) { useState(); }',
        '}',
        'function normal() {',
        '  useEffect(() => {});',
        '}',
        'function ComponentTwo() {',
        '  try { use(resource); } catch (err) {}',
        '}',
      ].join('\n'),
      'Component.tsx',
    );

    expect(diagnostics.map((diagnostic) => diagnostic.messageId)).toEqual([
      'conditional',
      'invalidFunction',
      'tryCatch',
    ]);
    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'rules-of-hooks',
      'rules-of-hooks',
      'rules-of-hooks',
    ]);
    expect(diagnostics[0].data.hook).toBe('useState');
    expect(diagnostics[1].data.functionName).toBe('normal');
    expect(diagnostics[2].data.hook).toBe('use');
  });

  it('returns LSP-shaped source locations', () => {
    expect(scanReactHooks('useState();\n', 'Component.tsx')[0].loc).toEqual({
      startLine: 1,
      startColumn: 0,
      endLine: 1,
      endColumn: 8,
    });
  });
});
