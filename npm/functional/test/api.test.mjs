import { describe, expect, it } from 'vitest';

import { implementedFunctionalRuleNames, scanFunctional } from '../api.js';

const expectedRuleNames = [
  'functional-parameters',
  'immutable-data',
  'no-class-inheritance',
  'no-classes',
  'no-conditional-statements',
  'no-expression-statements',
  'no-let',
  'no-loop-statements',
  'no-mixed-types',
  'no-promise-reject',
  'no-return-void',
  'no-this-expressions',
  'no-throw-statements',
  'no-try-statements',
  'prefer-immutable-types',
  'prefer-property-signatures',
  'prefer-readonly-type',
  'prefer-tacit',
  'readonly-type',
  'type-declaration-immutability',
];

const focusedRuleCases = [
  {
    ruleName: 'functional-parameters',
    source: 'function f() { return 1; }',
  },
  {
    ruleName: 'immutable-data',
    source: 'items.push(1);',
  },
  {
    ruleName: 'no-class-inheritance',
    source: 'class Derived extends Base {}',
  },
  {
    ruleName: 'no-classes',
    source: 'class Example {}',
  },
  {
    ruleName: 'no-conditional-statements',
    source: 'if (value) { value += 1; }',
  },
  {
    ruleName: 'no-expression-statements',
    source: 'effect();',
  },
  {
    ruleName: 'no-let',
    source: 'let value = 1;',
  },
  {
    ruleName: 'no-loop-statements',
    source: 'while (ready) { break; }',
  },
  {
    ruleName: 'no-mixed-types',
    source: 'interface Mixed { value: string; run(): string; }',
  },
  {
    ruleName: 'no-promise-reject',
    source: 'Promise.reject(error);',
  },
  {
    ruleName: 'no-return-void',
    source: 'function f(value: string): void { return; }',
  },
  {
    ruleName: 'no-this-expressions',
    source: 'class Example { method() { return this.value; } }',
  },
  {
    ruleName: 'no-throw-statements',
    source: 'throw error;',
  },
  {
    ruleName: 'no-try-statements',
    source: 'try { work(); } catch (error) {}',
  },
  {
    ruleName: 'prefer-immutable-types',
    source: 'const values: Array<string> = [];',
  },
  {
    ruleName: 'prefer-property-signatures',
    source: 'interface Actions { run(): void; }',
  },
  {
    ruleName: 'prefer-readonly-type',
    source: 'interface Values { items: string[]; }',
  },
  {
    ruleName: 'prefer-tacit',
    source: 'const mapped = (value) => compute(value);',
  },
  {
    ruleName: 'readonly-type',
    source: 'interface Values { readonly value: string; }',
  },
  {
    ruleName: 'type-declaration-immutability',
    source: 'type Values = { items: string[] };',
  },
];

describe('functional native API', () => {
  it('exposes all eslint-plugin-functional rule names', () => {
    expect(implementedFunctionalRuleNames()).toEqual(expectedRuleNames);
  });

  it('reports each ported rule with a focused fixture', () => {
    for (const { ruleName, source } of focusedRuleCases) {
      const diagnostics = scanFunctional(source, 'fixture.ts', { ruleNames: [ruleName] });

      expect(
        diagnostics.map((diagnostic) => diagnostic.ruleName),
        ruleName,
      ).toContain(ruleName);
    }
  });

  it('scans representative syntax and type rules', () => {
    const diagnostics = scanFunctional(
      [
        'let value = 1;',
        'class Derived extends Base { method() { this.x = 1; } }',
        'if (value) { value += 1; }',
        'for (let i = 0; i < 1; i++) {}',
        'try { throw new Error("x"); } catch (err) {}',
        'Promise.reject(err);',
        'const tacit = (value) => compute(value);',
        'interface Mixed { readonly items: string[]; run(): void; }',
        'type Bag = { value: Array<string> };',
        'const takes = (items: string[]): void => {};',
      ].join('\n'),
      'fixture.ts',
    );

    expect(new Set(diagnostics.map((diagnostic) => diagnostic.ruleName))).toEqual(
      new Set([
        'functional-parameters',
        'immutable-data',
        'no-class-inheritance',
        'no-classes',
        'no-conditional-statements',
        'no-expression-statements',
        'no-let',
        'no-loop-statements',
        'no-mixed-types',
        'no-promise-reject',
        'no-return-void',
        'no-this-expressions',
        'no-throw-statements',
        'no-try-statements',
        'prefer-immutable-types',
        'prefer-property-signatures',
        'prefer-readonly-type',
        'prefer-tacit',
        'readonly-type',
        'type-declaration-immutability',
      ]),
    );
  });

  it('filters rule names and honors simple options', () => {
    const diagnostics = scanFunctional('for (let i = 0; i < 1; i++) {}', 'fixture.ts', {
      ruleNames: ['no-let'],
      allowLetInForLoopInit: true,
    });

    expect(diagnostics).toEqual([]);
  });

  it('honors try, throw, and readonly style options', () => {
    expect(
      scanFunctional('try { work(); } catch (error) {} finally { cleanup(); }', 'x.ts', {
        ruleNames: ['no-try-statements'],
        allowTryCatch: true,
      }).map((diagnostic) => diagnostic.ruleName),
    ).toEqual(['no-try-statements']);
    expect(
      scanFunctional('try { work(); } catch (error) {} finally { cleanup(); }', 'x.ts', {
        ruleNames: ['no-try-statements'],
        allowTryCatch: true,
        allowTryFinally: true,
      }),
    ).toEqual([]);
    expect(
      scanFunctional('async function f(error) { throw error; }', 'x.ts', {
        ruleNames: ['no-throw-statements'],
        allowThrowToRejectPromises: true,
      }),
    ).toEqual([]);
    expect(
      scanFunctional('interface Values { readonly value: string; }', 'x.ts', {
        ruleNames: ['readonly-type'],
        readonlyTypeMode: 'keyword',
      }),
    ).toEqual([]);
  });

  it('can allow rest parameters and arguments explicitly', () => {
    const diagnostics = scanFunctional(
      'function f(...items) { return arguments.length; }',
      'x.ts',
      {
        ruleNames: ['functional-parameters'],
        allowRestParameter: true,
        allowArgumentsKeyword: true,
      },
    );

    expect(diagnostics).toEqual([]);
  });
});
