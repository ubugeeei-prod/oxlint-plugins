import { describe, expect, it } from 'vitest';

import { implementedE18eRuleNames, scanE18e } from '../api.js';

const expectedRuleNames = [
  'prefer-array-at',
  'prefer-array-fill',
  'prefer-array-from-map',
  'prefer-includes',
  'prefer-array-to-reversed',
  'prefer-array-to-sorted',
  'prefer-array-to-spliced',
  'prefer-exponentiation-operator',
  'prefer-nullish-coalescing',
  'prefer-object-has-own',
  'prefer-spread-syntax',
  'prefer-url-canparse',
  'no-indexof-equality',
  'prefer-timer-args',
  'prefer-date-now',
  'prefer-regex-test',
  'prefer-array-some',
  'prefer-static-regex',
  'prefer-inline-equality',
  'prefer-string-fromcharcode',
  'prefer-includes-over-regex-test',
  'no-delete-property',
  'no-spread-in-reduce',
  'prefer-static-collator',
  'ban-dependencies',
];

function scan(ruleName, sourceText, options = {}, filename = 'sample.ts') {
  return scanE18e(sourceText, filename, {
    ...options,
    ruleNames: [ruleName],
  });
}

function applyFix(sourceText, diagnostic) {
  const fix = diagnostic.fix;
  return sourceText.slice(0, fix.start) + fix.replacement + sourceText.slice(fix.end);
}

describe('e18e native API', () => {
  it('exposes all @e18e/eslint-plugin rule names', () => {
    expect(implementedE18eRuleNames()).toEqual(expectedRuleNames);
  });

  it.each([
    ['prefer-array-at', 'const last = items[items.length - 1];', 'preferAt', 'items.at(-1)'],
    [
      'prefer-array-fill',
      'const xs = Array.from({length: 3}, () => 0);',
      'preferFillArrayFrom',
      'Array.from({length: 3}).fill(0)',
    ],
    [
      'prefer-array-from-map',
      'const out = [...items].map(x => x.id);',
      'preferArrayFrom',
      'Array.from(items, x => x.id)',
    ],
    [
      'prefer-includes',
      'if (items.indexOf(id) !== -1) ok();',
      'preferIncludes',
      'items.includes(id)',
    ],
    [
      'prefer-array-to-reversed',
      'const r = [...items].reverse();',
      'preferToReversed',
      'items.toReversed()',
    ],
    [
      'prefer-array-to-sorted',
      'const s = items.slice().sort(compare);',
      'preferToSorted',
      'items.toSorted(compare)',
    ],
    [
      'prefer-array-to-spliced',
      'const s = Array.from(items).splice(0, 1);',
      'preferToSpliced',
      'items.toSpliced(0, 1)',
    ],
    [
      'prefer-exponentiation-operator',
      'const x = Math.pow(a, 2);',
      'preferExponentiation',
      '(a) ** (2)',
    ],
    [
      'prefer-nullish-coalescing',
      'const x = value == null ? fallback : value;',
      'preferNullishCoalescing',
      'value ?? fallback',
    ],
    [
      'prefer-object-has-own',
      'Object.prototype.hasOwnProperty.call(obj, key);',
      'preferObjectHasOwn',
      'Object.hasOwn(obj, key)',
    ],
    ['prefer-spread-syntax', 'const x = Array.from(items);', 'preferSpreadArrayFrom', '[...items]'],
    [
      'prefer-url-canparse',
      'function can(input){ try { new URL(input); return true; } catch { return false; } }',
      'preferCanParse',
      'return URL.canParse(input)',
    ],
    [
      'no-indexof-equality',
      'if (items.indexOf(id) === 2) ok();',
      'preferDirectAccess',
      'items[2] === id',
    ],
    ['prefer-timer-args', 'setTimeout(() => run(a), 10);', 'preferArgs', 'setTimeout(run, 10, a)'],
    ['prefer-date-now', 'const t = new Date().getTime();', 'preferDateNow', 'Date.now()'],
    ['prefer-regex-test', 'if (/foo/.exec(text)) ok();', 'preferTest', '/foo/.test(text)'],
    [
      'prefer-array-some',
      'if (items.filter(fn).length > 0) ok();',
      'preferArraySome',
      'items.some(fn)',
    ],
    [
      'prefer-inline-equality',
      'if (["a", "b"].includes(x)) ok();',
      'preferEquality',
      '"a" === x || "b" === x',
    ],
    [
      'prefer-string-fromcharcode',
      'String.fromCodePoint(65);',
      'preferFromCharCode',
      'fromCharCode',
    ],
    [
      'prefer-includes-over-regex-test',
      'if (/foo/.test(text)) ok();',
      'preferIncludes',
      'text.includes("foo")',
    ],
    ['no-delete-property', 'delete obj.foo;', 'noDeleteProperty', 'obj.foo = undefined'],
  ])('reports and fixes %s', (ruleName, source, messageId, replacement) => {
    const diagnostic = scan(ruleName, source)[0];

    expect(diagnostic).toMatchObject({ ruleName, messageId });
    expect(diagnostic.fix.replacement).toBe(replacement);
    expect(applyFix(source, diagnostic)).toContain(replacement);
  });

  it.each([
    ['prefer-static-regex', 'function f(){ return /foo/.test(x); }', 'preferStatic'],
    [
      'no-spread-in-reduce',
      'items.reduce((acc, x) => ({...acc, [x]: true}), {});',
      'noSpreadInReduce',
    ],
    [
      'prefer-static-collator',
      'function f(){ return Intl.Collator().compare(a, b); }',
      'preferStaticCollator',
    ],
  ])('reports non-fixable %s', (ruleName, source, messageId) => {
    expect(scan(ruleName, source)[0]).toMatchObject({ ruleName, messageId });
  });

  it('reports ban-dependencies using adapter-supplied mappings', () => {
    const diagnostics = scan(
      'ban-dependencies',
      'import merge from "lodash.merge";',
      {
        bannedDependencies: [
          {
            moduleName: 'lodash.merge',
            messageId: 'documentedReplacement',
            replacement: 'deepmerge-ts',
            url: 'https://example.com/lodash.merge',
          },
        ],
      },
      'sample.js',
    );

    expect(diagnostics[0]).toMatchObject({
      ruleName: 'ban-dependencies',
      messageId: 'documentedReplacement',
      data: {
        name: 'lodash.merge',
        replacement: 'deepmerge-ts',
      },
    });
  });
});
