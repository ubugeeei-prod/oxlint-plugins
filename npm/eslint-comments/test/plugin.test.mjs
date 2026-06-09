import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

describe('eslint-comments plugin shape', () => {
  it('exposes a named, versioned plugin', () => {
    expect(plugin.meta?.name).toBe('eslint-comments');
    expect(typeof plugin.meta?.version).toBe('string');
  });

  it('registers no-unlimited-disable with createOnce', () => {
    const rule = plugin.rules['no-unlimited-disable'];
    expect(typeof rule.createOnce).toBe('function');
    expect(rule.meta.messages.unexpected).toContain('Specify some rule names');
    expect(rule.meta.schema).toEqual([]);
  });

  it('ships a recommended config for ported rules', () => {
    expect(plugin.configs.recommended.rules).toEqual({
      'eslint-comments/disable-enable-pair': 'error',
      'eslint-comments/no-unlimited-disable': 'error',
    });
  });
});
