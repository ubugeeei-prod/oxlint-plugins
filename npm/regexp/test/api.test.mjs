import { describe, expect, it } from 'vitest';

import { implementedRegexpRuleNames, scanRegexp } from '../api.js';

describe('regexp native API', () => {
  it('exposes the implemented eslint-plugin-regexp rule names', () => {
    expect(implementedRegexpRuleNames()).toEqual([
      'no-invalid-regexp',
      'no-empty-character-class',
      'no-empty-group',
      'no-empty-capturing-group',
      'no-empty-alternative',
      'no-zero-quantifier',
      'no-octal',
      'no-control-character',
      'sort-flags',
      'require-unicode-regexp',
    ]);
  });

  it('scans literals and constructors through one native call', () => {
    const diagnostics = scanRegexp(
      [
        'const emptyClass = /[]/mi;',
        "const invalid = new RegExp('[', 'u');",
        "const control = new RegExp('\\\\x01', 'u');",
      ].join('\n'),
      'fixture.js',
    );

    expect(diagnostics.map((diagnostic) => [diagnostic.ruleName, diagnostic.messageId])).toEqual([
      ['sort-flags', 'sortFlags'],
      ['require-unicode-regexp', 'require'],
      ['no-empty-character-class', 'empty'],
      ['no-invalid-regexp', 'error'],
      ['no-control-character', 'unexpected'],
    ]);
    expect(diagnostics[0].data).toMatchObject({
      flags: 'mi',
      sortedFlags: 'im',
    });
    expect(diagnostics[4].data.charText).toBe('U+0001');
  });

  it('returns LSP-shaped locations from Rust', () => {
    const diagnostics = scanRegexp('const a = /a/mi;\n', 'fixture.js');

    expect(diagnostics[0]).toMatchObject({
      ruleName: 'sort-flags',
      messageId: 'sortFlags',
      loc: {
        startLine: 1,
        startColumn: 10,
        endLine: 1,
        endColumn: 15,
      },
    });
  });
});
