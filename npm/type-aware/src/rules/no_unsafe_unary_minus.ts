import { createRustNativeRule } from './native_bridge.js';

export const noUnsafeUnaryMinusRule = createRustNativeRule(
  'no-unsafe-unary-minus',
  {},
  { shouldRun: shouldRunNoUnsafeUnaryMinus },
);

function shouldRunNoUnsafeUnaryMinus(node: any): boolean {
  return node.type !== 'UnaryExpression' || node.operator === '-';
}
