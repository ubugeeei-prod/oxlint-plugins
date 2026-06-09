import { createRustNativeRule } from './native_bridge.js';

export const restrictTemplateExpressionsRule = createRustNativeRule(
  'restrict-template-expressions',
);
