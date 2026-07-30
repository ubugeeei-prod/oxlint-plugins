// Captures a curated scalar-option contract for
// eslint-plugin-perfectionist/sort-named-imports v5.9.1 from the published
// package. The exact submodule commit and source hashes prevent the fixture
// from silently drifting away from the reviewed upstream implementation.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

type TestCase = {
  name: string;
  code: string;
  options?: Array<Record<string, unknown>>;
  filename?: string;
};

type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

const ROOT = process.cwd();
const RULE = 'sort-named-imports';
const PINNED_COMMIT = 'b35e8e4caf0c8d350cf386e504241f21827dd60b';
const ESLINT_VERSION = '9.39.2';
const TYPESCRIPT_ESLINT_VERSION = '8.60.0';
const SOURCE_HASHES = {
  'rules/sort-named-imports.ts': 'ef0f575cb4aca0248120e8eb831748b2fb7a170e2e2220b8cdee66f1c2740ae6',
  'rules/sort-named-imports/sort-named-import.ts':
    '3e35c4b385cab6e07acac20e2ee4f5049e3cfc1fb05403ac0158cd360e4b3415',
  'test/rules/sort-named-imports.test.ts':
    'a5fe43752e460a2d29432e01e01a21aaa92bf9bc6d5193840dc593c6767ddeee',
} as const;

const cases: TestCase[] = [
  {
    name: 'default alphabetical order',
    code: `import { AAA, BB, C } from 'module'`,
  },
  {
    name: 'default alphabetical inversion',
    code: `import { BB, AAA, C } from 'module'`,
  },
  {
    name: 'default complete reversal',
    code: `import { C, BB, AAA } from 'module'`,
  },
  {
    name: 'multiline alphabetical order',
    code: `import {\n  AAAA,\n  BBB,\n  CC,\n  D,\n} from 'module'`,
  },
  {
    name: 'multiline alphabetical inversion',
    code: `import {\n  AAAA,\n  CC,\n  BBB,\n  D,\n} from 'module'`,
  },
  {
    name: 'default ignores default import',
    code: `import Default, { A, B } from 'module'`,
  },
  {
    name: 'default sorts by local alias',
    code: `import { c as A, b as B, a as C } from 'module'`,
  },
  {
    name: 'default local alias inversion',
    code: `import { a as C, b as B, c as A } from 'module'`,
  },
  {
    name: 'ignoreAlias original names',
    code: `import { A as C, B as B, C as A } from 'module'`,
    options: [{ ignoreAlias: true }],
  },
  {
    name: 'ignoreAlias inversion',
    code: `import { C as A, B as B, A as C } from 'module'`,
    options: [{ ignoreAlias: true }],
  },
  {
    name: 'ignoreCase stable equivalent aliases',
    code: `import { b as a, a as A } from 'module'`,
  },
  {
    name: 'case-sensitive lowercase before uppercase',
    code: `import { b as a, a as A } from 'module'`,
    options: [{ ignoreCase: false }],
  },
  {
    name: 'case-sensitive inversion',
    code: `import { a as A, b as a } from 'module'`,
    options: [{ ignoreCase: false }],
  },
  {
    name: 'alphabetical descending',
    code: `import { C, BB, AAA } from 'module'`,
    options: [{ order: 'desc' }],
  },
  {
    name: 'alphabetical descending inversion',
    code: `import { AAA, BB, C } from 'module'`,
    options: [{ order: 'desc' }],
  },
  {
    name: 'natural numeric order',
    code: `import { item1, item2, item10 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural numeric inversion',
    code: `import { item10, item2, item1 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural leading zero order',
    code: `import { item01, item1, item2 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural leading zero inversion',
    code: `import { item2, item1, item01 } from 'module'`,
    options: [{ type: 'natural' }],
  },
  {
    name: 'natural descending inversion',
    code: `import { item1, item2, item10 } from 'module'`,
    options: [{ type: 'natural', order: 'desc' }],
  },
  {
    name: 'line length descending',
    code: `import { AAA, BB, C } from 'module'`,
    options: [{ type: 'line-length', order: 'desc' }],
  },
  {
    name: 'line length descending inversion',
    code: `import { C, AAA, BB } from 'module'`,
    options: [{ type: 'line-length', order: 'desc' }],
  },
  {
    name: 'line length ascending',
    code: `import { C, BB, AAA } from 'module'`,
    options: [{ type: 'line-length', order: 'asc' }],
  },
  {
    name: 'line length counts alias source',
    code: `import { long as A, B } from 'module'`,
    options: [{ type: 'line-length', order: 'asc' }],
  },
  {
    name: 'custom reverse alphabet',
    code: `import { c, b, a } from 'module'`,
    options: [{ type: 'custom', alphabet: 'cba' }],
  },
  {
    name: 'custom reverse alphabet inversion',
    code: `import { a, b, c } from 'module'`,
    options: [{ type: 'custom', alphabet: 'cba' }],
  },
  {
    name: 'custom unknown characters remain stable',
    code: `import { x, y, z } from 'module'`,
    options: [{ type: 'custom', alphabet: 'abc' }],
  },
  {
    name: 'custom unknown characters compare length',
    code: `import { xx, y, zzz } from 'module'`,
    options: [{ type: 'custom', alphabet: 'abc' }],
  },
  {
    name: 'custom descending inversion',
    code: `import { c, b, a } from 'module'`,
    options: [{ type: 'custom', alphabet: 'abc', order: 'desc' }],
  },
  {
    name: 'unsorted preserves arbitrary order',
    code: `import { c, a, b } from 'module'`,
    options: [{ type: 'unsorted' }],
  },
  {
    name: 'unsorted ignores configured fallback',
    code: `import { c, a, b } from 'module'`,
    options: [{ type: 'unsorted', fallbackSort: { type: 'alphabetical' } }],
  },
  {
    name: 'special characters keep identifier punctuation',
    code: `import { _a, $a, a } from 'module'`,
  },
  {
    name: 'special characters trim',
    code: `import { _a, b, _c } from 'module'`,
    options: [{ specialCharacters: 'trim' }],
  },
  {
    name: 'special characters trim inversion',
    code: `import { _b, a, _c } from 'module'`,
    options: [{ specialCharacters: 'trim' }],
  },
  {
    name: 'special characters remove stable equality',
    code: `import { ab, a_b } from 'module'`,
    options: [{ specialCharacters: 'remove' }],
  },
  {
    name: 'Chinese locale order',
    code: `import { 你好, 世界, a, A, b, B } from 'module'`,
    options: [{ locales: 'zh-CN' }],
  },
  {
    name: 'Chinese locale inversion',
    code: `import { b, B, a, A, 世界, 你好 } from 'module'`,
    options: [{ locales: 'zh-CN' }],
  },
  {
    name: 'locale preference array',
    code: `import { 你好, 世界, a, A, b, B } from 'module'`,
    options: [{ locales: ['zh-CN', 'en-US'] }],
  },
  {
    name: 'Swedish locale inversion',
    code: `import { ä, å, z, a } from 'module'`,
    options: [{ locales: 'sv' }],
  },
  {
    name: 'fallback resolves case-insensitive tie',
    code: `import { bb as a, a as A } from 'module'`,
    options: [
      {
        fallbackSort: { type: 'line-length', order: 'asc' },
      },
    ],
  },
  {
    name: 'fallback descending resolves line-length tie',
    code: `import { c, a, b } from 'module'`,
    options: [
      {
        type: 'line-length',
        order: 'asc',
        fallbackSort: { type: 'alphabetical', order: 'desc' },
      },
    ],
  },
  {
    name: 'type-only import',
    code: `import type { A, B } from 'module'`,
    filename: 'fixture.ts',
  },
  {
    name: 'type specifier inversion',
    code: `import { type B, type A } from 'module'`,
    filename: 'fixture.ts',
  },
  {
    name: 'arbitrary string import inversion',
    code: `import { "B" as b, "A" as a } from 'module'`,
    options: [{ ignoreAlias: true }],
    filename: 'fixture.ts',
  },
  {
    name: 'multiple declarations',
    code: `import { B, A } from 'one';\nimport { D, C } from 'two';`,
  },
  {
    name: 'CRLF multiline inversion',
    code: `import {\r\n  B,\r\n  A,\r\n} from 'module';`,
  },
  {
    name: 'UTF-16 prefix keeps fix offsets',
    code: `'😀';\nimport { B, A } from 'module';`,
  },
  {
    name: 'inline trailing comma inversion',
    code: `import { B, A, } from 'module';`,
  },
  {
    name: 'multiline without trailing comma',
    code: `import {\n  B,\n  A\n} from 'module';`,
  },
  {
    name: 'leading comments move with imports',
    code: `import {\n  // B docs\n  B,\n  // A docs\n  A,\n} from 'module';`,
  },
  {
    name: 'trailing comments stay attached to slots',
    code: `import {\n  B, // first\n  A, // second\n} from 'module';`,
  },
  {
    name: 'predefined type group before unknown',
    code: `import {\n  value,\n  type Type,\n} from 'module';`,
    options: [{ groups: ['type-import', 'unknown'] }],
    filename: 'fixture.ts',
  },
  {
    name: 'predefined unknown before type group',
    code: `import {\n  type Type,\n  value,\n} from 'module';`,
    options: [{ groups: ['unknown', 'type-import'] }],
    filename: 'fixture.ts',
  },
  {
    name: 'declaration type imports use type group',
    code: `import type {\n  B,\n  A,\n} from 'module';`,
    options: [{ groups: ['type-import', 'unknown'] }],
    filename: 'fixture.ts',
  },
  {
    name: 'custom element pattern orders matching group first',
    code: `import { zebra, alpha, apiClient } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api' }],
        groups: ['api', 'unknown'],
      },
    ],
  },
  {
    name: 'custom regex flags and alias name',
    code: `import { zed, source as APIClient } from 'module';`,
    options: [
      {
        customGroups: [
          {
            groupName: 'api',
            elementNamePattern: { pattern: '^api', flags: 'i' },
          },
        ],
        groups: ['api', 'unknown'],
      },
    ],
  },
  {
    name: 'custom regex pattern array',
    code: `import { zebra, beta, alpha } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'letters', elementNamePattern: ['^alpha$', '^beta$'] }],
        groups: ['letters', 'unknown'],
      },
    ],
  },
  {
    name: 'custom type modifier',
    code: `import { value, type Zebra, type Alpha } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'types', modifiers: ['type'] }],
        groups: ['types', 'unknown'],
      },
    ],
    filename: 'fixture.ts',
  },
  {
    name: 'custom value modifier',
    code: `import { type Type, zebra, alpha } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'values', modifiers: ['value'] }],
        groups: ['values', 'unknown'],
      },
    ],
    filename: 'fixture.ts',
  },
  {
    name: 'custom selector import',
    code: `import { zebra, alpha } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'imports', selector: 'import' }],
        groups: ['imports', 'unknown'],
        order: 'desc',
      },
    ],
  },
  {
    name: 'custom anyOf match',
    code: `import { type other, type FooType, fooValue } from 'module';`,
    options: [
      {
        customGroups: [
          {
            groupName: 'foo',
            anyOf: [
              { modifiers: ['type'], elementNamePattern: 'Foo' },
              { elementNamePattern: '^foo' },
            ],
          },
        ],
        groups: ['foo', 'unknown'],
      },
    ],
    filename: 'fixture.ts',
  },
  {
    name: 'custom groups ignore names absent from groups',
    code: `import { zebra, apiClient, alpha } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api' }],
        groups: ['unknown'],
      },
    ],
  },
  {
    name: 'custom group line length override',
    code: `import { type a, type bb, type cccc, value } from 'module';`,
    options: [
      {
        customGroups: [
          {
            groupName: 'types',
            modifiers: ['type'],
            type: 'line-length',
            order: 'desc',
          },
        ],
        groups: ['types', 'unknown'],
      },
    ],
    filename: 'fixture.ts',
  },
  {
    name: 'custom group unsorted preserves member order',
    code: `import { value, type Zebra, type Alpha } from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'types', modifiers: ['type'], type: 'unsorted' }],
        groups: ['types', 'unknown'],
      },
    ],
    filename: 'fixture.ts',
  },
  {
    name: 'group object alphabetical descending override',
    code: `import { alpha, beta } from 'module';`,
    options: [
      {
        type: 'unsorted',
        groups: [{ group: 'unknown', type: 'alphabetical', order: 'desc' }],
      },
    ],
  },
  {
    name: 'subgroup declaration order fallback',
    code: `import { beta, alpha } from 'module';`,
    options: [
      {
        customGroups: [
          { groupName: 'a', elementNamePattern: '^alpha$' },
          { groupName: 'b', elementNamePattern: '^beta$' },
        ],
        groups: [['a', 'b'], 'unknown'],
        fallbackSort: { type: 'subgroup-order' },
      },
    ],
  },
  {
    name: 'partition by newline sorts sections independently',
    code: `import {\n  D,\n  A,\n\n  C,\n\n  E,\n  B,\n} from 'module';`,
    options: [{ partitionByNewLine: true }],
  },
  {
    name: 'partition by any comment',
    code: `import {\n  B,\n  // section\n  A,\n} from 'module';`,
    options: [{ partitionByComment: true }],
  },
  {
    name: 'partition by string comment',
    code: `import {\n  C,\n  // Part: one\n  B,\n  // note\n  A,\n} from 'module';`,
    options: [{ partitionByComment: '^Part' }],
  },
  {
    name: 'partition by comment pattern array',
    code: `import {\n  D,\n  /* Section */\n  C,\n  // Part: one\n  B,\n  A,\n} from 'module';`,
    options: [{ partitionByComment: ['Section', '^Part'] }],
  },
  {
    name: 'partition by line comments only',
    code: `import {\n  C,\n  /* block */\n  B,\n  // line\n  A,\n} from 'module';`,
    options: [{ partitionByComment: { line: true } }],
  },
  {
    name: 'partition by block comments only',
    code: `import {\n  C,\n  // line\n  B,\n  /* block */\n  A,\n} from 'module';`,
    options: [{ partitionByComment: { block: true } }],
  },
  {
    name: 'partition line and block patterns',
    code: `import {\n  D,\n  // LINE A\n  C,\n  /* BLOCK B */\n  B,\n  A,\n} from 'module';`,
    options: [
      {
        partitionByComment: {
          line: { pattern: '^ LINE', flags: 'i' },
          block: ['BLOCK'],
        },
      },
    ],
  },
  {
    name: 'partition preserves comments with group ordering',
    code: `import {\n  // Part: A\n  value,\n  type Type,\n  // Part: B\n  Zebra,\n  Alpha,\n} from 'module';`,
    options: [
      {
        partitionByComment: '^Part',
        groups: ['type-import', 'unknown'],
      },
    ],
    filename: 'fixture.ts',
  },
  {
    name: 'zero newlines inside and between groups',
    code: `import {\n  api,\n\n  zebra,\n\n  alpha,\n} from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
        newlinesBetween: 0,
      },
    ],
  },
  {
    name: 'one newline between groups',
    code: `import {\n  api,\n  alpha,\n  beta,\n} from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
        newlinesBetween: 1,
      },
    ],
  },
  {
    name: 'two newlines between groups',
    code: `import {\n  api,\n  alpha,\n} from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
        newlinesBetween: 2,
      },
    ],
  },
  {
    name: 'zero newlines inside group',
    code: `import {\n  beta,\n\n  alpha,\n} from 'module';`,
    options: [{ newlinesInside: 0 }],
  },
  {
    name: 'one newline inside custom group',
    code: `import {\n  alpha,\n  beta,\n} from 'module';`,
    options: [
      {
        customGroups: [{ groupName: 'letters', elementNamePattern: '.*', newlinesInside: 1 }],
        groups: ['letters'],
      },
    ],
  },
  {
    name: 'group object newlinesInside override',
    code: `import {\n  alpha,\n  beta,\n} from 'module';`,
    options: [{ groups: [{ group: 'unknown', newlinesInside: 1 }] }],
  },
  {
    name: 'inline newlinesBetween override',
    code: `import {\n  alpha,\n\n  beta,\n  charlie,\n} from 'module';`,
    options: [
      {
        customGroups: [
          { groupName: 'a', elementNamePattern: '^alpha$' },
          { groupName: 'b', elementNamePattern: '^beta$' },
          { groupName: 'c', elementNamePattern: '^charlie$' },
        ],
        groups: ['a', { newlinesBetween: 0 }, 'b', { newlinesBetween: 1 }, 'c'],
        newlinesBetween: 2,
      },
    ],
  },
  {
    name: 'newlines ignore preserves spacing',
    code: `import {\n  alpha,\n\n\n  beta,\n} from 'module';`,
    options: [{ newlinesInside: 'ignore', newlinesBetween: 'ignore' }],
  },
  {
    name: 'partition newline suppresses spacing diagnostic',
    code: `import {\n  beta,\n\n  alpha,\n} from 'module';`,
    options: [{ partitionByNewLine: true }],
  },
  {
    name: 'spacing and ordering diagnostics combine',
    code: `import {\n  beta,\n\n  alpha,\n} from 'module';`,
    options: [{ newlinesInside: 0 }],
  },
  {
    name: 'conditional names picks first matching option',
    code: `import { b, g, r } from 'module';`,
    options: [
      {
        type: 'unsorted',
        useConfigurationIf: { allNamesMatchPattern: '^foo' },
      },
      {
        customGroups: [
          { groupName: 'r', elementNamePattern: '^r$' },
          { groupName: 'g', elementNamePattern: '^g$' },
          { groupName: 'b', elementNamePattern: '^b$' },
        ],
        groups: ['r', 'g', 'b'],
        useConfigurationIf: { allNamesMatchPattern: '^[rgb]$' },
      },
    ],
  },
  {
    name: 'conditional names regex object and alias',
    code: `import { first as B, second as A } from 'module';`,
    options: [
      {
        ignoreAlias: false,
        useConfigurationIf: {
          allNamesMatchPattern: { pattern: '^[ab]$', flags: 'i' },
        },
      },
      { type: 'unsorted' },
    ],
  },
  {
    name: 'conditional names ignore alias',
    code: `import { b as y, a as z } from 'module';`,
    options: [
      {
        ignoreAlias: true,
        useConfigurationIf: { allNamesMatchPattern: '^[ab]$' },
      },
      { type: 'unsorted' },
    ],
  },
  {
    name: 'conditional import selector match',
    code: `import { beta, alpha } from 'module';`,
    options: [
      {
        type: 'unsorted',
        useConfigurationIf: { matchesAstSelector: 'ImportDeclaration' },
      },
    ],
  },
  {
    name: 'conditional import selector child match',
    code: `import { beta, alpha } from 'module';`,
    options: [
      {
        type: 'unsorted',
        useConfigurationIf: { matchesAstSelector: '* > ImportDeclaration' },
      },
    ],
  },
  {
    name: 'conditional nonmatching selector falls through',
    code: `import { beta, alpha } from 'module';`,
    options: [
      {
        type: 'unsorted',
        useConfigurationIf: { matchesAstSelector: 'VariableDeclaration' },
      },
      { type: 'alphabetical' },
    ],
  },
  {
    name: 'conditional selector and names both required',
    code: `import { beta, alpha } from 'module';`,
    options: [
      {
        type: 'unsorted',
        useConfigurationIf: {
          matchesAstSelector: 'ImportDeclaration',
          allNamesMatchPattern: '^[ac]$',
        },
      },
      { type: 'alphabetical', order: 'desc' },
    ],
  },
  {
    name: 'no conditional match uses defaults',
    code: `import { beta, alpha } from 'module';`,
    options: [
      {
        type: 'unsorted',
        useConfigurationIf: { allNamesMatchPattern: '^never$' },
      },
    ],
  },
  {
    name: 'unicode custom group and utf16 prefix',
    code: `'😀';\nimport { 世界, 你好, api } from '模块';`,
    options: [
      {
        locales: 'zh-CN',
        customGroups: [{ groupName: 'api', elementNamePattern: '^api$' }],
        groups: ['api', 'unknown'],
      },
    ],
  },
  {
    name: 'comments move with group members',
    code: `import {\n  // value docs\n  value,\n  // type docs\n  type Type,\n} from 'module';`,
    options: [{ groups: ['type-import', 'unknown'] }],
    filename: 'fixture.ts',
  },
  {
    name: 'inline comment survives group movement and spacing',
    code: `import {\n  beta,\n  alpha, // alpha docs\n  type Type,\n} from 'module';`,
    options: [
      {
        groups: ['type-import', 'unknown'],
        newlinesBetween: 1,
        newlinesInside: 0,
      },
    ],
    filename: 'fixture.ts',
  },
];

const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-perfectionist');
if (!plugin) {
  throw new Error('eslint-plugin-perfectionist is not registered in tools/port-targets.json');
}
if (plugin.baselineVersion !== '5.9.1' || plugin.pinnedRef !== 'v5.9.1') {
  throw new Error(
    `Expected perfectionist v5.9.1 manifest pin, received ${plugin.baselineVersion} / ${plugin.pinnedRef}`,
  );
}

const submodule = join(ROOT, plugin.submodule);
if (!existsSync(join(submodule, '.git'))) {
  throw new Error(`Upstream checkout not found: ${plugin.submodule}`);
}
const actualCommit = execFileSync('git', ['-C', submodule, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== PINNED_COMMIT) {
  throw new Error(`Expected ${PINNED_COMMIT}, received ${actualCommit}`);
}
for (const [sourceFile, expectedHash] of Object.entries(SOURCE_HASHES)) {
  const actualHash = createHash('sha256')
    .update(readFileSync(join(submodule, sourceFile)))
    .digest('hex');
  if (actualHash !== expectedHash) {
    throw new Error(`Expected ${sourceFile} hash ${expectedHash}, received ${actualHash}`);
  }
}

const runnerDirectory = mkdtempSync(join(tmpdir(), 'perfectionist-named-imports-'));
try {
  writeFileSync(
    join(runnerDirectory, 'package.json'),
    `${JSON.stringify(
      {
        private: true,
        type: 'module',
        dependencies: {
          '@typescript-eslint/parser': TYPESCRIPT_ESLINT_VERSION,
          eslint: ESLINT_VERSION,
          'eslint-plugin-perfectionist': plugin.baselineVersion,
        },
      },
      null,
      2,
    )}\n`,
  );
  writeFileSync(join(runnerDirectory, 'cases.json'), `${JSON.stringify(cases)}\n`);
  writeFileSync(join(runnerDirectory, 'runner.mjs'), runnerSource());
  execFileSync(
    'pnpm',
    ['install', '--dir', runnerDirectory, '--ignore-workspace', '--lockfile=false', '--silent'],
    { stdio: 'inherit' },
  );
  execFileSync('node', [join(runnerDirectory, 'runner.mjs')], { stdio: 'inherit' });
  const captured = JSON.parse(
    readFileSync(join(runnerDirectory, 'captured.json'), 'utf8'),
  ) as Array<{ expectedDiagnostics: unknown[] }>;
  const valid = captured.filter((testCase) => testCase.expectedDiagnostics.length === 0).length;
  const invalid = captured.length - valid;
  const diagnostics = captured.reduce(
    (total, testCase) => total + testCase.expectedDiagnostics.length,
    0,
  );
  const fixture = {
    __generated: {
      source: plugin.npm,
      version: plugin.baselineVersion,
      sourceCommit: PINNED_COMMIT,
      sourceHashes: SOURCE_HASHES,
      license: plugin.license,
      eslintVersion: ESLINT_VERSION,
      typescriptEslintParserVersion: TYPESCRIPT_ESLINT_VERSION,
      capturePolicy:
        'Curated exact parity for scalar comparators, groups, custom groups, partitions, newline policies, and conditional configuration.',
      tool: 'tools/tasks/sync-perfectionist-sort-named-imports-options-tests.ts',
      inventory: {
        valid,
        invalid,
        diagnostics,
        total: captured.length,
      },
    },
    cases: captured,
  };
  const fixturesDirectory = join(ROOT, 'npm', 'perfectionist', 'test', 'fixtures');
  mkdirSync(fixturesDirectory, { recursive: true });
  const fixturePath = join(fixturesDirectory, `${RULE}-options-v${plugin.baselineVersion}.json`);
  writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  execFileSync('pnpm', ['exec', 'vp', 'fmt', fixturePath], {
    cwd: ROOT,
    stdio: 'inherit',
  });
  console.log(
    `Captured ${RULE} v${plugin.baselineVersion}: ${valid} valid, ${invalid} invalid, ${diagnostics} diagnostics.`,
  );
} finally {
  rmSync(runnerDirectory, { recursive: true, force: true });
}

function runnerSource(): string {
  return `
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Linter } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import perfectionistModule from 'eslint-plugin-perfectionist';

const here = fileURLToPath(new URL('.', import.meta.url));
const cases = JSON.parse(readFileSync(join(here, 'cases.json'), 'utf8'));
const perfectionist = perfectionistModule.default ?? perfectionistModule;
const linter = new Linter();

function configFor(testCase) {
  return [{
    files: ['**/*.{js,jsx,ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
      },
    },
    plugins: { perfectionist },
    rules: {
      'perfectionist/${RULE}': ['error', ...(testCase.options ?? [])],
    },
  }];
}

const captured = cases.map(testCase => {
  const filename = testCase.filename ?? 'fixture.tsx';
  const config = configFor(testCase);
  const diagnostics = linter.verify(testCase.code, config, { filename });
  const unexpected = diagnostics.filter(problem => problem.ruleId !== 'perfectionist/${RULE}');
  if (unexpected.length > 0) {
    throw new Error(
      testCase.name + ' produced non-rule diagnostics: ' + JSON.stringify(unexpected),
    );
  }
  const fixed = linter.verifyAndFix(testCase.code, config, { filename });
  return {
    ...testCase,
    options: testCase.options ?? [],
    filename,
    expectedDiagnostics: diagnostics.map(problem => ({
      messageId: problem.messageId,
      message: problem.message,
      data: dataFromMessage(problem.message),
      loc: {
        startLine: problem.line,
        startColumn: problem.column,
        endLine: problem.endLine,
        endColumn: problem.endColumn,
      },
      fix: problem.fix
        ? {
            range: problem.fix.range,
            text: problem.fix.text,
          }
        : null,
    })),
    output: fixed.fixed ? fixed.output : null,
  };
});

writeFileSync(join(here, 'captured.json'), JSON.stringify(captured));

function dataFromMessage(message) {
  const match = /^Expected "(.+)" to come before "(.+)"\\.$/u.exec(message);
  if (match) {
    return { right: match[1], left: match[2] };
  }
  const groupMatch = /^Expected "(.+)" \\((.+)\\) to come before "(.+)" \\((.+)\\)\\.$/u.exec(message);
  if (groupMatch) {
    return {
      right: groupMatch[1],
      rightGroup: groupMatch[2],
      left: groupMatch[3],
      leftGroup: groupMatch[4],
    };
  }
  const spacingMatch = /^(?:Extra|Missed) spacing between "(.+)" and "(.+)"\\.$/u.exec(message);
  if (spacingMatch) {
    return { left: spacingMatch[1], right: spacingMatch[2] };
  }
  return {};
}
`;
}
