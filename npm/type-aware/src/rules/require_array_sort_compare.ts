import { calleePropertyName } from './ast.js';
import { createRustNativeRule } from './native_bridge.js';

export const requireArraySortCompareRule = createRustNativeRule(
  'require-array-sort-compare',
  {
    schema: { type: 'array' },
  },
  { shouldRun: shouldRunRequireArraySortCompare },
);

function shouldRunRequireArraySortCompare(node: any): boolean {
  return (
    node.arguments?.length === 0 && ['sort', 'toSorted'].includes(calleePropertyName(node) ?? '')
  );
}
