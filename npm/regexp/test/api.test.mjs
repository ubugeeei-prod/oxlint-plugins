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
      'prefer-d',
      'prefer-w',
      'letter-case',
      'no-non-standard-flag',
      'no-invisible-character',
      'hexadecimal-escape',
      'unicode-escape',
      'no-useless-range',
      'no-empty-lookarounds-assertion',
      'prefer-regexp-exec',
      'no-missing-g-flag',
      'no-useless-character-class',
      'no-empty-string-literal',
      'no-optional-assertion',
      'require-unicode-sets-regexp',
      'confusing-quantifier',
      'prefer-named-replacement',
      'no-obscure-range',
      'prefer-unicode-codepoint-escapes',
      'no-dupe-characters-character-class',
      'prefer-range',
      'no-useless-escape',
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
      // /[]/mi — flag style + pattern checks all fire.
      ['sort-flags', 'sortFlags'],
      ['require-unicode-regexp', 'require'],
      ['require-unicode-sets-regexp', 'require'],
      ['no-empty-character-class', 'empty'],
      // new RegExp('[', 'u') — constructor parse error short-circuits the flag-style checks.
      ['no-invalid-regexp', 'error'],
      // new RegExp('\\x01', 'u') — u is present so require-unicode-sets-regexp fires
      // alongside the control-character diagnostic and `hexadecimal-escape` (the
      // `\xHH` escape is independently flagged regardless of the character value).
      ['require-unicode-sets-regexp', 'require'],
      ['no-control-character', 'unexpected'],
      ['hexadecimal-escape', 'unexpected'],
    ]);
    expect(diagnostics[0].data).toMatchObject({
      flags: 'mi',
      sortedFlags: 'im',
    });
    expect(diagnostics[6].data.charText).toBe('U+0001');
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
    // Sources that use the `v` flag stay quiet because `require-unicode-sets-regexp`
    // is the only flag-style rule that targets that flag specifically; everything
    // else needs an unrelated pattern issue.
    expect(scanRegexp('const re = /a+/v;\n', 'fixture.js')).toEqual([]);
    expect(scanRegexp("const re = new RegExp('a', 'gimsv');\n", 'fixture.js')).toEqual([]);
  });

  it('returns no diagnostics when the source fails to parse', () => {
    // Parser failure must not surface phantom diagnostics.
    expect(scanRegexp('const = ;', 'fixture.js')).toEqual([]);
  });

  it('reports each literal separately', () => {
    const diagnostics = scanRegexp('const a = /[]/u; const b = /a|/u;\n', 'fixture.js');
    // Each `u`-only literal fires require-unicode-sets-regexp once on top of
    // the pattern-specific diagnostic.
    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'require-unicode-sets-regexp',
      'no-empty-character-class',
      'require-unicode-sets-regexp',
      'no-empty-alternative',
    ]);
    expect(diagnostics[0].loc.startLine).toBe(1);
    expect(diagnostics[3].loc.startLine).toBe(1);
    expect(diagnostics[3].loc.startColumn).toBeGreaterThan(diagnostics[1].loc.startColumn);
  });

  it('ignores callers that are not RegExp', () => {
    expect(scanRegexp("new Foo('[]', 'u');\n", 'fixture.js')).toEqual([]);
    expect(scanRegexp("Bar('[', 'u');\n", 'fixture.js')).toEqual([]);
  });
});
