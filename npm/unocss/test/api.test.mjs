import { describe, expect, it } from 'vitest';

import { implementedUnocssRuleNames, scanUnocss } from '../api.js';

describe('unocss native API', () => {
  it('exposes all @unocss/eslint-plugin rule names', () => {
    expect(implementedUnocssRuleNames()).toEqual([
      'blocklist',
      'enforce-class-compile',
      'order',
      'order-attributify',
    ]);
  });

  it('scans representative class, blocklist, and attributify rules', () => {
    const diagnostics = scanUnocss(
      [
        '<div className="mx1 m1 border"></div>;',
        'const value = clsx("mr-1 ml-1");',
        'const node = <section p4 flex />;',
      ].join('\n'),
      'fixture.tsx',
      {
        blocklist: [['border', { message: 'Use border-1 instead' }]],
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'blocklist',
      'enforce-class-compile',
      'order',
      'order',
      'order-attributify',
    ]);
    expect(diagnostics[0]).toMatchObject({
      ruleName: 'blocklist',
      messageId: 'in-blocklist',
      name: 'border',
      reason: 'Use border-1 instead',
    });
  });

  it('returns byte offsets for fixes', () => {
    const code = '<div className="é mx1 m1"></div>;';
    const diagnostics = scanUnocss(code, 'fixture.tsx');
    const diagnostic = diagnostics.find((item) => item.ruleName === 'enforce-class-compile');

    expect(diagnostic).toMatchObject({
      ruleName: 'enforce-class-compile',
      messageId: 'missing',
      prefix: ':uno:',
      loc: {
        startLine: 1,
        startColumn: 16,
        endLine: 1,
        endColumn: 24,
      },
    });
    expect(diagnostic.fix.start).toBe(Buffer.byteLength(code.slice(0, 16)));
    expect(diagnostic.fix.replacement).toBe(':uno: é mx1 m1');
  });

  it('passes custom order options to Rust', () => {
    const diagnostics = scanUnocss(
      [
        'superclass("pr1 pl1");',
        'const CLS_BUTTON = "top-1 bottom-1";',
        'const untouched = "top-1 bottom-1";',
      ].join('\n'),
      'fixture.tsx',
      {
        unoFunctions: ['superclass'],
        unoVariables: ['^CLS_'],
      },
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual(['order', 'order']);
  });

  it('supports class compile options', () => {
    const diagnostics = scanUnocss('<div className="mr-1"></div>;', 'fixture.tsx', {
      classCompilePrefix: ':some:',
      classCompileEnableFix: false,
    });

    const diagnostic = diagnostics.find((item) => item.ruleName === 'enforce-class-compile');
    expect(diagnostic).toMatchObject({
      prefix: ':some:',
    });
    expect(diagnostic).not.toHaveProperty('fix');
  });
});
