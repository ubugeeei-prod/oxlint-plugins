import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

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

const invalidCases = [
  ['sort-array-includes', '["b", "a"].includes(value);'],
  ['sort-arrays', 'const array = ["b", "a"];'],
  ['sort-classes', 'class Class { b() {} a() {} }'],
  ['sort-decorators', '@Z @A class Decorated {}'],
  ['sort-enums', 'enum Enum { B, A }'],
  [
    'sort-export-attributes',
    'export { data } from "./data.json" with { type: "json", foo: "bar" };',
  ],
  ['sort-exports', 'export { z } from "z";\nexport { a } from "a";'],
  ['sort-heritage-clauses', 'class Derived implements Z, A {}'],
  ['sort-import-attributes', 'import data from "./data.json" with { type: "json", foo: "bar" };'],
  ['sort-imports', 'import z from "z";\nimport a from "a";'],
  ['sort-interfaces', 'interface Interface { b: string; a: string }'],
  ['sort-intersection-types', 'type Intersection = B & A;'],
  ['sort-jsx-props', 'const jsx = <Component b={1} a={2} />;'],
  ['sort-maps', 'const map = new Map([["b", 1], ["a", 2]]);'],
  ['sort-modules', 'const z = 1;\nfunction a() {}'],
  ['sort-named-exports', 'export { b, a };'],
  ['sort-named-imports', 'import { b, a } from "pkg";'],
  ['sort-object-types', 'type ObjectType = { b: string; a: string };'],
  ['sort-objects', 'const object = { b: 1, a: 2 };'],
  ['sort-sets', 'const set = new Set(["b", "a"]);'],
  ['sort-switch-case', 'switch (value) { case "b": break; case "a": break; }'],
  ['sort-union-types', 'type Union = B | A;'],
  ['sort-variable-declarations', 'const b = 1, a = 2;'],
];

function runRule(ruleName, sourceText, filename = 'fixture.tsx', options = [], settings) {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const rule = plugin.rules[ruleName];
  const visitor = rule.createOnce({
    filename,
    options,
    settings,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((a, b) => a.localeCompare(b));

  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }

  return candidates[0];
}

describe('perfectionist plugin adapter', () => {
  it('exposes rules and recommended configs', () => {
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.configs['recommended-alphabetical'].rules).toHaveProperty(
      'perfectionist/sort-imports',
    );
    expect(plugin.configs['recommended-alphabetical'].rules).not.toHaveProperty(
      'perfectionist/sort-arrays',
    );
    expect(plugin.configs['recommended-natural-legacy'].plugins).toEqual(['perfectionist']);
  });

  it.each(invalidCases)('reports %s through direct createOnce', (ruleName, code) => {
    const reports = runRule(ruleName, code);

    expect(reports).toHaveLength(1);
    expect(plugin.rules[ruleName].meta.messages[reports[0].messageId]).toBe(
      ['sort-exports', 'sort-imports', 'sort-named-exports', 'sort-named-imports'].includes(
        ruleName,
      )
        ? 'Expected "{{right}}" to come before "{{left}}".'
        : 'Expected sorted order.',
    );
  });

  it('declares the complete sort-exports schema without named-specifier-only options', () => {
    const schema = plugin.rules['sort-exports'].meta.schema;

    expect(schema).toHaveLength(1);
    expect(Object.keys(schema[0].properties).sort()).toEqual([
      'alphabet',
      'customGroups',
      'fallbackSort',
      'groups',
      'ignoreCase',
      'locales',
      'newlinesBetween',
      'newlinesInside',
      'order',
      'partitionByComment',
      'partitionByNewLine',
      'specialCharacters',
      'type',
    ]);
    expect(
      schema[0].properties.customGroups.items.oneOf[0].properties.modifiers.items.enum,
    ).toEqual(['value', 'type', 'named', 'wildcard', 'singleline', 'multiline']);
    expect(schema[0].properties.customGroups.items.oneOf[0].properties.selector.enum).toEqual([
      'export',
    ]);
    expect(schema[0].additionalProperties).toBe(false);
  });

  it('declares every sort-imports option, selector, modifier, and message', () => {
    const rule = plugin.rules['sort-imports'];
    const [schema] = rule.meta.schema;

    expect(Object.keys(schema.properties).sort()).toEqual([
      'alphabet',
      'customGroups',
      'environment',
      'fallbackSort',
      'groups',
      'ignoreCase',
      'internalPattern',
      'locales',
      'maxLineLength',
      'newlinesBetween',
      'newlinesInside',
      'order',
      'partitionByComment',
      'partitionByNewLine',
      'sortBy',
      'sortSideEffects',
      'specialCharacters',
      'tsconfig',
      'type',
      'useExperimentalDependencyDetection',
    ]);
    expect(schema.additionalProperties).toBe(false);
    expect(schema.properties.type.enum).toContain('type-import-first');
    expect(schema.properties.sortBy.enum).toEqual(['specifier', 'path']);
    expect(schema.properties.customGroups.items.oneOf[0].properties.modifiers.items.enum).toEqual([
      'default',
      'multiline',
      'named',
      'require',
      'side-effect',
      'singleline',
      'ts-equals',
      'type',
      'value',
      'wildcard',
    ]);
    expect(schema.properties.customGroups.items.oneOf[0].properties.selector.enum).toEqual([
      'side-effect-style',
      'tsconfig-path',
      'side-effect',
      'external',
      'internal',
      'builtin',
      'sibling',
      'subpath',
      'import',
      'parent',
      'index',
      'style',
      'type',
    ]);
    expect(Object.keys(rule.meta.messages).sort()).toEqual([
      'extraSpacingBetweenImports',
      'missedCommentAboveImport',
      'missedSpacingBetweenImports',
      'unexpectedImportsDependencyOrder',
      'unexpectedImportsGroupOrder',
      'unexpectedImportsOrder',
    ]);
  });

  it.each([
    ['sort-named-imports', 'import'],
    ['sort-named-exports', 'export'],
  ])('declares the complete implemented option schema for %s', (ruleName, selector) => {
    const schema = plugin.rules[ruleName].meta.schema;

    expect(schema).toHaveLength(1);
    expect(Object.keys(schema[0].properties).sort()).toEqual([
      'alphabet',
      'customGroups',
      'fallbackSort',
      'groups',
      'ignoreAlias',
      'ignoreCase',
      'locales',
      'newlinesBetween',
      'newlinesInside',
      'order',
      'partitionByComment',
      'partitionByNewLine',
      'specialCharacters',
      'type',
      'useConfigurationIf',
    ]);
    expect(schema[0].additionalProperties).toBe(false);
    expect(schema[0].properties.locales).toEqual({
      oneOf: [
        { type: 'string' },
        {
          type: 'array',
          items: { type: 'string' },
        },
      ],
    });
    expect(schema[0].properties.fallbackSort).toMatchObject({
      required: ['type'],
      additionalProperties: false,
    });
    expect(schema[0].properties.groups).toMatchObject({
      type: 'array',
      items: { oneOf: expect.any(Array) },
    });
    expect(schema[0].properties.customGroups).toMatchObject({
      type: 'array',
      items: { oneOf: expect.any(Array) },
    });
    expect(schema[0].properties.customGroups.items.oneOf[0].properties.selector.enum).toEqual([
      selector,
    ]);
    expect(schema[0].properties.partitionByComment.oneOf).toHaveLength(5);
    expect(schema[0].properties.useConfigurationIf).toMatchObject({
      minProperties: 1,
      additionalProperties: false,
    });
  });

  it('threads recommended comparator options into every configured sorting rule', () => {
    const rules = plugin.configs['recommended-natural'].rules;

    expect(rules['perfectionist/sort-named-imports']).toEqual([
      'error',
      { type: 'natural', order: 'asc' },
    ]);
    expect(rules['perfectionist/sort-named-exports']).toEqual([
      'error',
      { type: 'natural', order: 'asc' },
    ]);
    expect(rules['perfectionist/sort-exports']).toEqual([
      'error',
      { type: 'natural', order: 'asc' },
    ]);
    expect(rules['perfectionist/sort-imports']).toEqual([
      'error',
      { type: 'natural', order: 'asc' },
    ]);
  });

  it('merges global perfectionist settings with explicit sort-imports options', () => {
    const source = `import item2 from "item2";\nimport item10 from "item10";`;
    const settings = { perfectionist: { type: 'natural', order: 'desc' } };

    expect(runRule('sort-imports', source, 'fixture.ts', [], settings)).toHaveLength(1);
    expect(runRule('sort-imports', source, 'fixture.ts', [{ order: 'asc' }], settings)).toEqual([]);
  });

  it('keeps sort-imports createOnce caches isolated by options and source object', () => {
    const source = `import item2 from "item2";\nimport item10 from "item10";`;

    expect(
      runRule('sort-imports', source, 'fixture.ts', [{ type: 'natural', order: 'desc' }]),
    ).toHaveLength(1);
    expect(
      runRule('sort-imports', source, 'fixture.ts', [{ type: 'natural', order: 'asc' }]),
    ).toEqual([]);
  });

  it('loads configured options and fixes through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-perfectionist-'));
    try {
      const fixturePath = join(tempDir, 'fixture.ts');
      writeFileSync(fixturePath, 'import { item2, item10 } from "pkg";\n');
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: 'perfectionist',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'perfectionist/sort-named-imports': ['error', { type: 'natural', order: 'desc' }],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics).toHaveLength(1);
      expect(payload.diagnostics[0]).toMatchObject({
        code: 'perfectionist(sort-named-imports)',
        message: 'Expected "item10" to come before "item2".',
      });

      const fixed = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--fix', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(fixturePath, 'utf8')).toBe('import { item10, item2 } from "pkg";\n');
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('loads grouping and newline options through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-perfectionist-groups-'));
    try {
      const fixturePath = join(tempDir, 'fixture.ts');
      writeFileSync(fixturePath, 'import {\n  value,\n  type Type,\n} from "pkg";\n');
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: 'perfectionist',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'perfectionist/sort-named-imports': [
              'error',
              {
                groups: ['type-import', 'unknown'],
                newlinesBetween: 1,
              },
            ],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Expected "Type" (type-import) to come before "value" (unknown).',
      ]);

      const fixed = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--fix', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(fixturePath, 'utf8')).toBe(
        'import {\n  type Type,\n\n  value,\n} from "pkg";\n',
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('loads named-export options and fixes through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-perfectionist-named-exports-'));
    try {
      const fixturePath = join(tempDir, 'fixture.ts');
      writeFileSync(fixturePath, 'export { value, type Type } from "pkg";\n');
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: 'perfectionist',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'perfectionist/sort-named-exports': [
              'error',
              {
                groups: ['type-export', 'unknown'],
                newlinesBetween: 1,
              },
            ],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics).toHaveLength(1);
      expect(payload.diagnostics[0]).toMatchObject({
        code: 'perfectionist(sort-named-exports)',
        message: 'Expected "Type" (type-export) to come before "value" (unknown).',
      });

      const fixed = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--fix', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(fixturePath, 'utf8')).toBe(
        'export { type Type, \n\nvalue } from "pkg";\n',
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('loads export-declaration groups, comments, and fixes through real oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-perfectionist-exports-'));
    try {
      const fixturePath = join(tempDir, 'fixture.ts');
      writeFileSync(
        fixturePath,
        `export type { Type } from "./types";\nexport { value } from "./value";\n`,
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: 'perfectionist',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'perfectionist/sort-exports': [
              'error',
              {
                groups: [
                  { group: 'value-export', commentAbove: 'Values' },
                  { group: 'type-export', commentAbove: 'Types' },
                ],
              },
            ],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics.map((diagnostic) => diagnostic.message)).toEqual([
        'Missed comment "Types" above "./types".',
        'Expected "./value" (value-export) to come before "./types" (type-export).',
      ]);

      let fixed;
      for (let pass = 0; pass < 4; pass += 1) {
        fixed = spawnSync(
          findOxlintCli(),
          ['--config', 'oxlint.config.jsonc', '--fix', 'fixture.ts'],
          {
            cwd: tempDir,
            encoding: 'utf8',
          },
        );
        if (fixed.status === 0) {
          break;
        }
      }
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(fixturePath, 'utf8')).toBe(
        `// Values\nexport { value } from "./value";\n// Types\nexport type { Type } from "./types";\n`,
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('loads import groups, comments, dependencies, and iterative fixes through real oxlint', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-perfectionist-imports-'));
    try {
      const fixturePath = join(tempDir, 'fixture.ts');
      writeFileSync(
        fixturePath,
        `import a = aImport.a1.a2;\nimport type { Type } from "./types";\nimport aImport from "b";\n`,
      );
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: 'perfectionist',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            'perfectionist/sort-imports': [
              'error',
              {
                groups: [
                  { group: 'unknown', commentAbove: 'Runtime' },
                  { group: 'type-import', commentAbove: 'Types' },
                ],
                useExperimentalDependencyDetection: true,
              },
            ],
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.ts'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics.map((diagnostic) => diagnostic.message)).toContain(
        'Expected dependency "b" to come before "aImport.a1.a2".',
      );
      expect(payload.diagnostics.map((diagnostic) => diagnostic.message)).toContain(
        'Missed comment "Types" above "./types".',
      );

      let fixed;
      for (let pass = 0; pass < 8; pass += 1) {
        fixed = spawnSync(
          findOxlintCli(),
          ['--config', 'oxlint.config.jsonc', '--fix', 'fixture.ts'],
          {
            cwd: tempDir,
            encoding: 'utf8',
          },
        );
        if (fixed.status === 0) {
          break;
        }
      }
      expect(fixed.status).toBe(0);
      expect(fixed.stderr).toBe('');
      expect(readFileSync(fixturePath, 'utf8')).toBe(
        `// Runtime\nimport aImport from "b";\nimport a = aImport.a1.a2;\n\n// Types\nimport type { Type } from "./types";\n`,
      );
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
