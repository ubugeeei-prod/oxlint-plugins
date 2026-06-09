import { createRustNativeRule } from './native_bridge.js';

export const awaitThenableRule = createRustNativeRule('await-thenable');
