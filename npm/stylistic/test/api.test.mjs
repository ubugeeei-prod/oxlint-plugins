import { describe, expect, it } from 'vitest';

import { nativeStylisticRuleMetas, runNativeStylisticLint } from '../api.js';

describe('stylistic native API', () => {
  it('exposes native stylistic rule metadata', () => {
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('quotes');
    expect(nativeStylisticRuleMetas().map((meta) => meta.name)).toContain('no-trailing-spaces');
  });

  it('runs multiple stylistic rules through one native call', () => {
    const diagnostics = runNativeStylisticLint('\u{feff}const\tlabel = "value";  \r\n\n\n', {
      rules: [
        { name: 'unicode-bom', options: ['never'] },
        { name: 'quotes', options: ['single'] },
        { name: 'no-trailing-spaces', options: [] },
        { name: 'no-tabs', options: [] },
        { name: 'linebreak-style', options: ['unix'] },
        { name: 'no-multiple-empty-lines', options: [{ max: 1 }] },
      ],
    });

    expect(diagnostics.map((diagnostic) => diagnostic.ruleName)).toEqual([
      'unicode-bom',
      'quotes',
      'no-trailing-spaces',
      'no-tabs',
      'linebreak-style',
      'no-multiple-empty-lines',
    ]);
    expect(diagnostics[1].suggestions?.[0]?.fixes[0]?.replacementText).toBe("'value'");
  });
});
