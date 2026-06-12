import { describe, expect, it } from 'vitest';

import { implementedSecurityRuleNames, scanSecurity } from '../api.js';

describe('security native API', () => {
  it('exposes all eslint-plugin-security rule names', () => {
    expect(implementedSecurityRuleNames()).toEqual([
      'detect-bidi-characters',
      'detect-buffer-noassert',
      'detect-child-process',
      'detect-disable-mustache-escape',
      'detect-eval-with-expression',
      'detect-new-buffer',
      'detect-no-csrf-before-method-override',
      'detect-non-literal-fs-filename',
      'detect-non-literal-regexp',
      'detect-non-literal-require',
      'detect-object-injection',
      'detect-possible-timing-attacks',
      'detect-pseudoRandomBytes',
      'detect-unsafe-regex',
    ]);
  });

  it('scans multiple security rules through one native call', () => {
    const diagnostics = scanSecurity(
      [
        "var fs = require('fs');",
        'fs.readFile(filename);',
        'eval(input);',
        'if (password === secret) {}',
      ].join('\n'),
      'fixture.js',
    );

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'detect-non-literal-fs-filename',
      'detect-eval-with-expression',
      'detect-possible-timing-attacks',
    ]);
    expect(diagnostics[0].data).toMatchObject({
      fnName: 'readFile',
      packageName: 'fs',
      indices: '0',
    });
  });

  it('returns LSP-shaped locations from Rust', () => {
    const diagnostics = scanSecurity('const a = "‮";\n', 'fixture.js');

    expect(diagnostics).toMatchObject([
      {
        ruleName: 'detect-bidi-characters',
        messageId: 'code',
        loc: {
          startLine: 1,
          startColumn: 11,
          endLine: 1,
          endColumn: 12,
        },
      },
    ]);
  });
});
