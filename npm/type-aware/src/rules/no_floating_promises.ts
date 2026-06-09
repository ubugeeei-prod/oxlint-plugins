import { stripChainExpression } from './ast.js';
import { createRustNativeRule } from './native_bridge.js';

export const noFloatingPromisesRule = createRustNativeRule(
  'no-floating-promises',
  {},
  { shouldRun: shouldRunNoFloatingPromises },
);

function shouldRunNoFloatingPromises(node: any): boolean {
  const expression = stripChainExpression(node.expression);
  return !(expression?.type === 'UnaryExpression' && expression.operator === 'void');
}
