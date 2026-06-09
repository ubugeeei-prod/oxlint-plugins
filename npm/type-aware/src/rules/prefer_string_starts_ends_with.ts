import { createRustNativeRule } from './native_bridge.js';

export const preferStringStartsEndsWithRule = createRustNativeRule(
  'prefer-string-starts-ends-with',
);
