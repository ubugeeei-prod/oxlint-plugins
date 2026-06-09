import { describe, expect, it } from 'vitest';

import { scanNoUnlimitedDisable } from '../api.js';

describe('JS API', () => {
  it('reports unlimited disables and ignores scoped ones', () => {
    const comments = [
      {
        kind: 'Block',
        value: 'eslint-disable ',
        startLine: 1,
        startColumn: 0,
        endLine: 1,
        endColumn: 19,
      },
      {
        kind: 'Block',
        value: 'eslint-disable eqeqeq',
        startLine: 2,
        startColumn: 0,
        endLine: 2,
        endColumn: 25,
      },
    ];

    expect(scanNoUnlimitedDisable(comments)).toEqual([
      {
        messageId: 'unexpected',
        data: { kind: 'eslint-disable' },
        loc: { startLine: 1, startColumn: -1, endLine: 1, endColumn: 19 },
      },
    ]);
  });

  it('rejects non-array input', () => {
    expect(() => scanNoUnlimitedDisable(null)).toThrow(TypeError);
  });
});
