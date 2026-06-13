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
      'no-escape-backspace',
      'prefer-plus-quantifier',
      'prefer-star-quantifier',
      'prefer-question-quantifier',
      'no-useless-two-nums-quantifier',
      'prefer-named-capture-group',
      'match-any',
      'no-legacy-features',
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

  it('returns no diagnostics for clean sources', () => {
    expect(scanRegexp('const re = /a+/u;\n', 'fixture.js')).toEqual([]);
    expect(scanRegexp("const re = new RegExp('a', 'gimsu');\n", 'fixture.js')).toEqual([]);
  });

  it('returns no diagnostics when the source fails to parse', () => {
    // Parser failure must not surface phantom diagnostics.
    expect(scanRegexp('const = ;', 'fixture.js')).toEqual([]);
  });

  it('reports each literal separately', () => {
    const diagnostics = scanRegexp('const a = /[]/u; const b = /a|/u;\n', 'fixture.js');
    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'no-empty-character-class',
      'no-empty-alternative',
    ]);
    expect(diagnostics[0].loc.startLine).toBe(1);
    expect(diagnostics[1].loc.startLine).toBe(1);
    expect(diagnostics[1].loc.startColumn).toBeGreaterThan(diagnostics[0].loc.startColumn);
  });

  it('ignores callers that are not RegExp', () => {
    expect(scanRegexp("new Foo('[]', 'u');\n", 'fixture.js')).toEqual([]);
    expect(scanRegexp("Bar('[', 'u');\n", 'fixture.js')).toEqual([]);
  });
});
