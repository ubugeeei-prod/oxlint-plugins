import { describe, expect, it } from 'vitest';

import { implementedSimpleImportSortRuleNames, scanSimpleImportSort } from '../api.js';

const expectedRuleNames = ['exports', 'imports'];

describe('simple-import-sort native API', () => {
  it('exposes all eslint-plugin-simple-import-sort rule names', () => {
    expect(implementedSimpleImportSortRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans and fixes import chunks', () => {
    const source = [
      "import z from 'z';",
      "import { beta, alpha as renamed } from 'pkg';",
      "import fs from 'node:fs';",
      "import './setup';",
      "import local from './local';",
    ].join('\n');

    const diagnostics = scanSimpleImportSort(source, 'fixture.js');

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toMatchObject({
      ruleName: 'imports',
      messageId: 'sort',
      fix: {
        start: 0,
        replacement: [
          "import './setup';",
          '',
          "import fs from 'node:fs';",
          '',
          "import { alpha as renamed, beta } from 'pkg';",
          "import z from 'z';",
          '',
          "import local from './local';",
        ].join('\n'),
      },
    });
  });

  it('scans and fixes export chunks and local specifiers', () => {
    const source = [
      "export { zed } from 'z';",
      "export * from 'a';",
      'export { d, a as c, b };',
    ].join('\n');

    const diagnostics = scanSimpleImportSort(source, 'fixture.js');

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual(['exports', 'exports']);
    expect(diagnostics[0].fix.replacement).toBe(
      ["export * from 'a';", "export { zed } from 'z';"].join('\n'),
    );
    expect(diagnostics[1].fix.replacement).toBe('export { b, a as c, d };');
  });

  it('honors custom import groups', () => {
    const source = ["import rel from './rel';", "import pkg from 'pkg';"].join('\n');
    const diagnostics = scanSimpleImportSort(source, 'fixture.js', {
      importGroups: [['^\\.'], ['^@?\\w']],
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].fix.replacement).toBe(
      ["import rel from './rel';", '', "import pkg from 'pkg';"].join('\n'),
    );
  });
});
