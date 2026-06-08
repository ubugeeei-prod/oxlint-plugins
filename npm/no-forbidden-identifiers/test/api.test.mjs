import { describe, expect, it } from 'vitest';

import { isForbiddenIdentifierName, scanForbiddenIdentifiers } from '../api.js';

describe('JS API', () => {
  it('exposes NAPI-backed scanning', () => {
    expect(scanForbiddenIdentifiers('const event = data.error;')).toEqual([
      'event',
      'error',
      'data',
    ]);
  });

  it('exposes NAPI-backed name checks', () => {
    expect(isForbiddenIdentifierName('ctx', { names: ['ctx'] })).toBe(true);
    expect(isForbiddenIdentifierName('value', { names: ['ctx'] })).toBe(false);
  });
});
