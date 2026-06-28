// Replay capability self-test: prove the parity runner's autofix path through oxlint's
// RuleTester — `output` (applied-fix string) comparison, multi-pass `recursive` convergence,
// and that a wrong fix turns the suite red (no false-green on fixes). This is the replay-side
// counterpart to the captured `simple-import-sort` fix corpus; a full byte-parity port of the
// sort rule (shared.js is ~900 lines of whitespace/comment-preserving logic) is tracked
// separately. Uses a small convergent fixer so the capability is proven in isolation.

import { RuleTester } from 'oxlint/plugins-dev';
import { describe, expect, it } from 'vitest';

import { runRuleParity } from '../../../tools/parity/replay/runner.mjs';

// Collapse a double negation `!!e` -> `e`, but only report the OUTERMOST one so each fix pass
// removes one pair and re-linting the fixed source reveals the next. `!!!!x` therefore needs
// two passes to converge to `x` — a genuine multi-pass fixer.
const collapseDoubleNegation = {
  meta: {
    type: 'suggestion',
    fixable: 'code',
    schema: [],
    messages: { double: 'Collapse redundant double negation.' },
  },
  create(context) {
    return {
      UnaryExpression(node) {
        if (node.operator !== '!') return;
        const arg = node.argument;
        if (!arg || arg.type !== 'UnaryExpression' || arg.operator !== '!') return;
        const parent = node.parent;
        if (parent && parent.type === 'UnaryExpression' && parent.operator === '!') return;
        context.report({
          node,
          messageId: 'double',
          // Remove the two leading `!` (from this node's start to the inner-inner argument).
          fix: (fixer) => fixer.removeRange([node.range[0], arg.argument.range[0]]),
        });
      },
    };
  },
};

function corpusOf(cases) {
  return { corpusVersion: 1, provenance: { plugin: 'self-test' }, cases };
}

describe('parity replay: autofix output', () => {
  it('compares single-pass applied fix against the corpus fixOutput', () => {
    const corpus = corpusOf([
      { kind: 'valid', code: '!x', options: [], expectedErrors: [], fixOutput: null },
      {
        kind: 'invalid',
        code: '!!x',
        options: [],
        expectedErrors: [{ messageId: 'double', message: 'Collapse redundant double negation.' }],
        fixOutput: 'x',
      },
    ]);
    const counts = runRuleParity({
      RuleTester,
      rule: collapseDoubleNegation,
      ruleName: 'collapse-double-negation',
      corpus,
    });
    expect(counts.invalid).toBe(1);
  });

  it('converges a multi-pass fixer when the case sets recursive', () => {
    const corpus = corpusOf([
      {
        kind: 'invalid',
        code: '!!!!x',
        options: [],
        expectedErrors: [{ messageId: 'double', message: 'Collapse redundant double negation.' }],
        fixOutput: 'x', // only reachable after two passes
        recursive: true,
      },
    ]);
    expect(() =>
      runRuleParity({
        RuleTester,
        rule: collapseDoubleNegation,
        ruleName: 'collapse-double-negation',
        corpus,
      }),
    ).not.toThrow();
  });

  it('goes red when the corpus fixOutput does not match the port (no false-green on fixes)', () => {
    const corpus = corpusOf([
      {
        kind: 'invalid',
        code: '!!x',
        options: [],
        expectedErrors: [{ messageId: 'double', message: 'Collapse redundant double negation.' }],
        fixOutput: 'WRONG',
      },
    ]);
    expect(() =>
      runRuleParity({
        RuleTester,
        rule: collapseDoubleNegation,
        ruleName: 'collapse-double-negation',
        corpus,
      }),
    ).toThrow();
  });
});
