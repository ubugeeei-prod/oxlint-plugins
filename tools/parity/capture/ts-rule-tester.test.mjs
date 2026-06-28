// Phase-3 capture capability self-test.
//
// Proves the parity capture machinery handles the @typescript-eslint/rule-tester family used
// by the TS plugins (storybook, functional, …): intercept its `run()`, run the real rule as a
// TS/JSX oracle through @typescript-eslint/parser, and materialize *suggestions* (messageId +
// applied output) — exactly the shape storybook rules assert. It uses a self-contained rule so
// it does not drag in a plugin's heavy ecosystem deps; capturing the real TS plugins (which
// need their upstream runtime installed) is deferred — see tools/parity/README.md.

import { RuleTester } from '@typescript-eslint/rule-tester';
import { Linter } from 'eslint';
import tsParser from '@typescript-eslint/parser';
import { describe, expect, it } from 'vitest';

import core from './core.cjs';

// A faithful miniature of storybook's no-redundant-story-name shape: a Property whose key is
// `name` with a string literal value is reported, with a suggestion that removes the property.
const rule = {
  meta: {
    type: 'suggestion',
    fixable: 'code',
    hasSuggestions: true,
    schema: [],
    messages: {
      redundant: 'Redundant name {{name}}.',
      remove: 'Remove the redundant name.',
    },
  },
  create(context) {
    return {
      Property(node) {
        if (node.key?.name === 'name' && node.value?.type === 'Literal') {
          context.report({
            node,
            messageId: 'redundant',
            data: { name: String(node.value.value) },
            suggest: [
              { messageId: 'remove', fix: (f) => f.removeRange([node.range[0], node.range[1]]) },
            ],
          });
        }
      },
    };
  },
};

const TS_LANGUAGE_OPTIONS = {
  parser: tsParser,
  parserOptions: { ecmaFeatures: { jsx: true }, sourceType: 'module' },
};

function capture() {
  core.installRuleTesterHooks(RuleTester);
  const runs = [];
  RuleTester.prototype.run = function (name, r, cases) {
    runs.push({ name, rule: r, valid: cases.valid, invalid: cases.invalid });
  };
  const tester = new RuleTester();
  tester.run('demo', rule, {
    // TS-only syntax (type annotation) the espree path could not parse.
    valid: ['const a: Id = { id: 1 }'],
    invalid: [
      {
        code: 'const x: Foo = { name: "Primary" }',
        errors: [
          {
            messageId: 'redundant',
            suggestions: [{ messageId: 'remove', output: 'const x: Foo = {  }' }],
          },
        ],
      },
    ],
  });
  return runs[0];
}

describe('parity capture: @typescript-eslint/rule-tester path', () => {
  const run = capture();

  it('intercepts ts-eslint RuleTester.run and harvests both branches', () => {
    expect(run).toBeTruthy();
    expect(run.valid).toHaveLength(1);
    expect(run.invalid).toHaveLength(1);
  });

  it('oracles a valid TS case to zero diagnostics', () => {
    const c = core.normalizeCase(run.valid[0], 'valid');
    const oracle = core.runOracle(Linter, run.rule, c, TS_LANGUAGE_OPTIONS);
    expect(oracle.expectedErrors).toEqual([]);
    expect(core.selfValidate('demo', 0, c, oracle)).toBeNull();
  });

  it('materializes messageId + suggestion output from the real rule on a TS/JSX case', () => {
    const c = core.normalizeCase(run.invalid[0], 'invalid');
    const oracle = core.runOracle(Linter, run.rule, c, TS_LANGUAGE_OPTIONS);
    expect(oracle.expectedErrors).toHaveLength(1);
    const [err] = oracle.expectedErrors;
    expect(err.messageId).toBe('redundant');
    expect(err.message).toBe('Redundant name Primary.');
    expect(err.suggestions).toEqual([
      { messageId: 'remove', desc: 'Remove the redundant name.', output: 'const x: Foo = {  }' },
    ]);
    // Self-validation against the upstream-style inline assertion must agree.
    expect(core.selfValidate('demo', 0, c, oracle)).toBeNull();
  });
});
