import { describe, expect, it } from 'vitest';

import { implementedUnusedImportsRuleNames, scanUnusedImports } from '../api.js';

const expectedRuleNames = ['no-unused-imports', 'no-unused-vars'];

function applyFix(sourceText, fix) {
  return sourceText.slice(0, fix.start) + fix.replacement + sourceText.slice(fix.end);
}

describe('unused-imports native API', () => {
  it('exposes all eslint-plugin-unused-imports rule names', () => {
    expect(implementedUnusedImportsRuleNames()).toEqual(expectedRuleNames);
  });

  it('scans and fixes unused named imports', () => {
    const source = [
      'import x from "package";',
      'import { a, b } from "./utils";',
      '',
      'const c = b(x);',
    ].join('\n');

    const diagnostics = scanUnusedImports(source, 'fixture.js', {
      ruleNames: ['no-unused-imports'],
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toMatchObject({
      ruleName: 'no-unused-imports',
      message: "'a' is defined but never used.",
      fix: {
        replacement: '',
      },
    });
    expect(applyFix(source, diagnostics[0].fix)).toBe(
      ['import x from "package";', 'import { b } from "./utils";', '', 'const c = b(x);'].join(
        '\n',
      ),
    );
  });

  it('removes a whole unused import while preserving following comments', () => {
    const source = [
      'import y from "package";',
      'import { a } from "./utils";',
      '',
      '/** c is the number 4 */',
      'const c = y;',
    ].join('\n');

    const diagnostics = scanUnusedImports(source, 'fixture.js', {
      ruleNames: ['no-unused-imports'],
    });

    expect(applyFix(source, diagnostics[0].fix)).toBe(
      ['import y from "package";', '', '/** c is the number 4 */', 'const c = y;'].join('\n'),
    );
  });

  it('keeps imports referenced only from JSDoc tags', () => {
    const source = [
      'import { UsedInJSDoc } from "./used";',
      '',
      '/** Reference to {@link UsedInJSDoc} */',
      'const example = "test";',
    ].join('\n');

    expect(scanUnusedImports(source, 'fixture.js', { ruleNames: ['no-unused-imports'] })).toEqual(
      [],
    );
  });

  it('does not treat shadowed inner identifiers as import usage', () => {
    const source = [
      'import { a } from "./utils";',
      'function fn(a) {',
      '  return a;',
      '}',
      'fn(1);',
    ].join('\n');

    expect(
      scanUnusedImports(source, 'fixture.js', { ruleNames: ['no-unused-imports'] }).map(
        (diagnostic) => diagnostic.message,
      ),
    ).toEqual(["'a' is defined but never used."]);
  });

  it('treats TypeScript type references as usage', () => {
    const source = [
      'import type { SomeType } from "./types";',
      'const value: SomeType = {} as SomeType;',
    ].join('\n');

    expect(scanUnusedImports(source, 'fixture.ts', { ruleNames: ['no-unused-imports'] })).toEqual(
      [],
    );
  });

  it('can report non-import unused variables separately', () => {
    const source = ['const used = 1;', 'const unused = 2;', 'console.log(used);'].join('\n');

    expect(
      scanUnusedImports(source, 'fixture.js', { ruleNames: ['no-unused-vars'] }).map(
        (diagnostic) => diagnostic.message,
      ),
    ).toEqual(["'unused' is defined but never used."]);
  });
});
