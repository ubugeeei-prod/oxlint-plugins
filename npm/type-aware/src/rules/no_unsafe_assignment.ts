import { createRustNativeRule } from './native_bridge.js';

export const noUnsafeAssignmentRule = createRustNativeRule(
  'no-unsafe-assignment',
  {},
  { shouldRun: shouldRunNoUnsafeAssignment },
);

function shouldRunNoUnsafeAssignment(node: any): boolean {
  switch (node.type) {
    case 'AssignmentExpression':
      return node.operator === '=' && Boolean(node.right);
    case 'PropertyDefinition':
      return Boolean(node.value);
    case 'VariableDeclarator':
      return Boolean(node.init);
    default:
      return true;
  }
}
