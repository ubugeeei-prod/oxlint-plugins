import { describe, expect, it } from 'vitest';

import { pathToFileUri } from './index.js';

describe('type-aware helpers', () => {
  it('converts paths to file uris without loading Corsa', () => {
    expect(pathToFileUri('/repo/src/file.ts')).toBe('file:///repo/src/file.ts');
    expect(pathToFileUri('C:\\repo\\src\\file.ts')).toBe('file:///C:/repo/src/file.ts');
  });
});
