import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
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

function runRule(ruleName, sourceText, filename = 'fixture.tsx') {
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
    options: [],
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
      'Expected sorted order.',
    );
  });

  it('loads through oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-perfectionist-'));
    try {
      writeFileSync(join(tempDir, 'fixture.ts'), 'import { b, a } from "pkg";\n');
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
            'perfectionist/sort-named-imports': 'error',
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
      expect(payload.diagnostics[0].message).toBe('Expected sorted order.');
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
