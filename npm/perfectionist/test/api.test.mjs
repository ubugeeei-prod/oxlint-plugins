import { describe, expect, it } from 'vitest';

import {
  implementedPerfectionistRuleNames,
  scanPerfectionist,
  scanPerfectionistRule,
} from '../api.js';

const expectedRuleNames = [
  'sort-array-includes',
  'sort-arrays',
  'sort-classes',
  'sort-decorators',
  'sort-enums',
  'sort-export-attributes',
  'sort-exports',
  'sort-heritage-clauses',
  'sort-import-attributes',
  'sort-imports',
  'sort-interfaces',
  'sort-intersection-types',
  'sort-jsx-props',
  'sort-maps',
  'sort-modules',
  'sort-named-exports',
  'sort-named-imports',
  'sort-object-types',
  'sort-objects',
  'sort-sets',
  'sort-switch-case',
  'sort-union-types',
  'sort-variable-declarations',
];

const representativeSource = `
import { b, a } from "pkg";
export { b, a };
import z from "z";
import a from "a";
export { z } from "z";
export { a } from "a";
import data from "./data.json" with { type: "json", foo: "bar" };
export { data } from "./data.json" with { type: "json", foo: "bar" };
@Z @A class Decorated {}
class Derived implements Z, A {}
const array = ["b", "a"];
["b", "a"].includes(value);
const set = new Set(["b", "a"]);
const map = new Map([["b", 1], ["a", 2]]);
const object = { b: 1, a: 2 };
type ObjectType = { b: string; a: string };
interface Interface { b: string; a: string }
enum Enum { B, A }
class Class { b() {} a() {} }
const jsx = <Component b={1} a={2} />;
const b = 1, a = 2;
type Union = B | A;
type Intersection = B & A;
switch (value) { case "b": break; case "a": break; }
const z = 1;
function a() {}
`;

describe('perfectionist native API', () => {
  it('exposes all eslint-plugin-perfectionist rule names', () => {
    expect(implementedPerfectionistRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans representative unsorted constructs for every rule', () => {
    const diagnostics = scanPerfectionist(representativeSource, 'fixture.tsx');

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName).sort()).toEqual(
      [...expectedRuleNames].sort(),
    );
  });

  it('returns LSP-shaped locations', () => {
    const [diagnostic] = scanPerfectionist('import { b, a } from "pkg";\n', 'fixture.ts');

    expect(diagnostic).toMatchObject({
      ruleName: 'sort-named-imports',
      messageId: 'unexpected',
      loc: {
        startLine: 1,
        startColumn: 0,
        endLine: 1,
      },
    });
  });

  it('returns exact configured data and UTF-16 fix offsets', () => {
    const source = `'😀';\r\nimport { item2, item10 } from "pkg";\r\n`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-named-imports', [
      { type: 'natural', order: 'desc' },
    ]);
    const start = source.indexOf('item2');
    const end = source.indexOf('item10') + 'item10'.length;

    expect(diagnostic).toEqual({
      ruleName: 'sort-named-imports',
      messageId: 'unexpectedNamedImportsOrder',
      data: {
        left: 'item2',
        right: 'item10',
      },
      loc: {
        startLine: 2,
        startColumn: 16,
        endLine: 2,
        endColumn: 22,
      },
      fix: {
        start,
        end,
        replacement: 'item10, item2',
      },
    });
  });

  it('isolates configured scanning to its implemented rule', () => {
    expect(
      scanPerfectionistRule('import { b, a } from "pkg";', 'fixture.ts', 'sort-objects', [
        { order: 'desc' },
      ]),
    ).toEqual([]);
  });

  it('validates public API scalar arguments', () => {
    expect(() => scanPerfectionistRule(null)).toThrowError('sourceText must be a string.');
    expect(() => scanPerfectionistRule('import {} from "pkg";', null)).toThrowError(
      'filename must be a string.',
    );
    expect(() => scanPerfectionistRule('import {} from "pkg";', 'fixture.ts', null)).toThrowError(
      'ruleName must be a string.',
    );
  });
});
