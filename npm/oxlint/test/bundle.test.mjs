import { describe, expect, it } from 'vitest';

import bundle from '../index.js';

describe('@oxlint-plugins/oxlint bundle', () => {
  it('aggregates every plugin into one combined plugin', () => {
    expect(bundle.meta.name).toBe('oxlint');
    // One representative rule from each bundled plugin.
    expect(Object.keys(bundle.rules)).toEqual(
      expect.arrayContaining(['no-unlimited-disable', 'no-forbidden-identifiers', 'quotes']),
    );
  });

  it('re-keys each plugin recommended config under the oxlint namespace', () => {
    const recommended = bundle.configs.recommended.rules;
    expect(recommended).toHaveProperty('oxlint/no-unlimited-disable');
    for (const ruleId of Object.keys(recommended)) {
      expect(ruleId.startsWith('oxlint/')).toBe(true);
    }
  });

  it('re-exports the individual plugins for per-plugin namespaces', () => {
    expect(Object.keys(bundle.plugins)).toEqual(
      expect.arrayContaining(['eslint-comments', 'no-forbidden-identifiers', 'stylistic']),
    );
  });
});
