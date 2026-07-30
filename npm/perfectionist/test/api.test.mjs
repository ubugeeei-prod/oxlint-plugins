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

  it('returns exact array-includes groups, CRLF locations, and UTF-16 fixes', () => {
    const source = `'😀';\r\n['世界', 'api'].includes(value);`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-array-includes', [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
        locales: 'zh-CN',
      },
    ]);

    expect(diagnostic).toEqual({
      ruleName: 'sort-array-includes',
      messageId: 'unexpectedArrayIncludesGroupOrder',
      data: {
        left: '世界',
        right: 'api',
        leftGroup: 'unknown',
        rightGroup: 'api',
      },
      loc: {
        startLine: 2,
        startColumn: 7,
        endLine: 2,
        endColumn: 12,
      },
      fix: {
        start: 8,
        end: 19,
        replacement: `'api', '世界'`,
      },
    });
  });

  it('returns exact named-export data and UTF-16 fix offsets', () => {
    const source = `'😀';\r\nexport { item2, item10 } from "pkg";\r\n`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-named-exports', [
      { type: 'natural', order: 'desc' },
    ]);
    const start = source.indexOf('item2');
    const end = source.indexOf('item10') + 'item10'.length;

    expect(diagnostic).toEqual({
      ruleName: 'sort-named-exports',
      messageId: 'unexpectedNamedExportsOrder',
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

  it('returns exact export-declaration groups, CRLF locations, and UTF-16 fixes', () => {
    const source = `'😀';\r\nexport { 世界 } from '世界';\r\nexport { api } from 'api';\r\n`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-exports', [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
        locales: 'zh-CN',
      },
    ]);

    expect(diagnostic).toEqual({
      ruleName: 'sort-exports',
      messageId: 'unexpectedExportsGroupOrder',
      data: {
        left: '世界',
        right: 'api',
        leftGroup: 'unknown',
        rightGroup: 'api',
      },
      loc: {
        startLine: 3,
        startColumn: 0,
        endLine: 3,
        endColumn: 26,
      },
      fix: {
        start: 7,
        end: 59,
        replacement: `export { api } from 'api';\r\nexport { 世界 } from '世界';`,
      },
    });
  });

  it('returns exact import-declaration groups, CRLF locations, and UTF-16 fixes', () => {
    const source = `'😀';\r\nimport 世界 from '世界';\r\nimport api from 'api';\r\n`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-imports', [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
        locales: 'zh-CN',
      },
    ]);

    expect(diagnostic).toEqual({
      ruleName: 'sort-imports',
      messageId: 'unexpectedImportsGroupOrder',
      data: {
        left: '世界',
        right: 'api',
        leftGroup: 'unknown',
        rightGroup: 'api',
      },
      loc: {
        startLine: 3,
        startColumn: 0,
        endLine: 3,
        endColumn: 22,
      },
      fix: {
        start: 7,
        end: 51,
        replacement: `import api from 'api';\r\n\nimport 世界 from '世界';`,
      },
    });
  });

  it('returns exact dependency data without unrelated placeholders', () => {
    const [diagnostic] = scanPerfectionistRule(
      `import a = aImport.a1.a2;\nimport aImport from "b";`,
      'fixture.ts',
      'sort-imports',
      [{ groups: ['unknown'], useExperimentalDependencyDetection: true }],
    );

    expect(diagnostic).toEqual({
      ruleName: 'sort-imports',
      messageId: 'unexpectedImportsDependencyOrder',
      data: {
        right: 'b',
        nodeDependentOnRight: 'aImport.a1.a2',
      },
      loc: {
        startLine: 2,
        startColumn: 0,
        endLine: 2,
        endColumn: 24,
      },
      fix: {
        start: 0,
        end: 50,
        replacement: `import aImport from "b";\nimport a = aImport.a1.a2;`,
      },
    });
  });

  it('keeps side-effect imports stable unless sorting is explicitly enabled', () => {
    const source = `import 'z';\nimport 'a';`;

    expect(scanPerfectionistRule(source, 'fixture.ts', 'sort-imports', [])).toEqual([]);
    expect(
      scanPerfectionistRule(source, 'fixture.ts', 'sort-imports', [
        { groups: ['side-effect'], sortSideEffects: true },
      ]),
    ).toMatchObject([
      {
        messageId: 'unexpectedImportsOrder',
        data: { left: 'z', right: 'a' },
        fix: { replacement: `import 'a';\nimport 'z';` },
      },
    ]);
  });

  it('returns exact missing-comment data without unrelated placeholders', () => {
    const [diagnostic] = scanPerfectionistRule(
      `export type { value } from './types';`,
      'fixture.ts',
      'sort-exports',
      [{ groups: [{ group: 'type-export', commentAbove: 'Types' }] }],
    );

    expect(diagnostic).toEqual({
      ruleName: 'sort-exports',
      messageId: 'missedCommentAboveExport',
      data: {
        right: './types',
        missedCommentAbove: 'Types',
      },
      loc: {
        startLine: 1,
        startColumn: 0,
        endLine: 1,
        endColumn: 37,
      },
      fix: {
        start: 0,
        end: 0,
        replacement: '// Types\n',
      },
    });
  });

  it('returns exact group-order data and a comment-preserving fix', () => {
    const source = `import {
  // value docs
  value,
  // type docs
  type Type,
} from "pkg";
`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-named-imports', [
      { groups: ['type-import', 'unknown'] },
    ]);

    expect(diagnostic).toMatchObject({
      ruleName: 'sort-named-imports',
      messageId: 'unexpectedNamedImportsGroupOrder',
      data: {
        left: 'value',
        right: 'Type',
        leftGroup: 'unknown',
        rightGroup: 'type-import',
      },
      loc: {
        startLine: 5,
        startColumn: 2,
        endLine: 5,
        endColumn: 11,
      },
      fix: {
        replacement: '// type docs\n  type Type,\n  // value docs\n  value',
      },
    });
  });

  it('returns exact named-export group data and a comment-preserving fix', () => {
    const source = `export {
  // value docs
  value,
  // type docs
  type Type,
} from "pkg";
`;
    const [diagnostic] = scanPerfectionistRule(source, 'fixture.ts', 'sort-named-exports', [
      { groups: ['type-export', 'unknown'] },
    ]);

    expect(diagnostic).toMatchObject({
      ruleName: 'sort-named-exports',
      messageId: 'unexpectedNamedExportsGroupOrder',
      data: {
        left: 'value',
        right: 'Type',
        leftGroup: 'unknown',
        rightGroup: 'type-export',
      },
      loc: {
        startLine: 5,
        startColumn: 2,
        endLine: 5,
        endColumn: 11,
      },
      fix: {
        replacement: '// type docs\n  type Type,\n  // value docs\n  value',
      },
    });
  });

  it('isolates configured scanning to its implemented rule', () => {
    expect(
      scanPerfectionistRule('import { b, a } from "pkg";', 'fixture.ts', 'sort-objects', [
        { order: 'desc' },
      ]),
    ).toEqual([]);
    expect(
      scanPerfectionistRule('export { b, a };', 'fixture.ts', 'sort-named-imports', []),
    ).toEqual([]);
    expect(
      scanPerfectionistRule('import { b, a } from "pkg";', 'fixture.ts', 'sort-named-exports', []),
    ).toEqual([]);
    expect(
      scanPerfectionistRule('export { b, a } from "pkg";', 'fixture.ts', 'sort-exports', []),
    ).toEqual([]);
    expect(
      scanPerfectionistRule(
        'export { z } from "z";\nexport { a } from "a";',
        'fixture.ts',
        'sort-named-exports',
        [],
      ),
    ).toEqual([]);
  });

  it('fails closed for malformed named-export syntax', () => {
    expect(scanPerfectionistRule('export { b,', 'fixture.ts', 'sort-named-exports', [])).toEqual(
      [],
    );
  });

  it('fails closed for malformed export-declaration syntax and malformed options', () => {
    expect(scanPerfectionistRule('export { a } from', 'fixture.ts', 'sort-exports', [])).toEqual(
      [],
    );
    expect(
      scanPerfectionistRule(
        'export { b } from "b";\nexport { a } from "a";',
        'fixture.ts',
        'sort-exports',
        [{ type: 'not-a-sort', groups: [null, false, 1] }],
      ),
    ).toHaveLength(1);
  });

  it('isolates import declarations and fails closed for malformed import syntax', () => {
    expect(
      scanPerfectionistRule(
        `import z from "z";\nconst boundary = true;\nimport a from "a";`,
        'fixture.ts',
        'sort-imports',
        [],
      ),
    ).toEqual([]);
    expect(scanPerfectionistRule('import { a } from', 'fixture.ts', 'sort-imports', [])).toEqual(
      [],
    );
    expect(
      scanPerfectionistRule(
        `import z from "z";\nimport a from "a";`,
        'fixture.ts',
        'sort-imports',
        [{ type: 'not-a-sort', groups: [null, false, 1] }],
      ),
    ).toHaveLength(1);
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
