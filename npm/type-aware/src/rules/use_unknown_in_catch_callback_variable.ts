import { createRustNativeRule } from './native_bridge.js';

export const useUnknownInCatchCallbackVariableRule = createRustNativeRule(
  'use-unknown-in-catch-callback-variable',
);
