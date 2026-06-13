// Contract tests: every native stylistic rule must expose an ESLint-compatible
// meta object so that Oxlint's rule loader, doc generators, and downstream
// tooling can consume the plugin without surprises.

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const ruleNames = plugin.implementedStylisticRuleNames;

describe('stylistic plugin meta contract', () => {
  it('exposes at least one rule', () => {
    expect(ruleNames.length).toBeGreaterThan(0);
  });

  it('exposes the same rule names through the meta API and the rules object', () => {
    const byName = (a, b) => a.localeCompare(b);
    expect([...ruleNames].sort(byName)).toEqual(Object.keys(plugin.rules).sort(byName));
  });

  it('exposes a stable, frozen rule-name list', () => {
    expect(Object.isFrozen(ruleNames)).toBe(true);
  });

  it('does not list a rule that lacks native metadata', () => {
    const metas = plugin.nativeStylisticRuleMetas();
    const metaNames = new Set(metas.map((meta) => meta.name));
    for (const ruleName of ruleNames) {
      expect(metaNames.has(ruleName)).toBe(true);
    }
  });

  it.each(ruleNames)('rule %s exposes a well-formed meta object', (ruleName) => {
    const rule = plugin.rules[ruleName];
    expect(rule).toBeDefined();
    expect(rule.meta).toBeDefined();
    expect(rule.meta.type).toBe('layout');
    expect(rule.meta.docs.description).toEqual(expect.any(String));
    expect(rule.meta.docs.url).toMatch(/github\.com\/ubugeeei-prod\/oxlint-plugins/);
    expect(rule.meta.docs.recommended).toBe(false);
    expect(rule.meta.docs.requiresTypeChecking).toBe(false);
    expect(rule.meta.fixable).toBe('whitespace');
    expect(typeof rule.meta.hasSuggestions).toBe('boolean');
    expect(typeof rule.meta.messages).toBe('object');
    expect(rule.meta.messages).not.toBeNull();
    expect(rule.meta.schema).toEqual({ type: 'array' });
  });

  it.each(ruleNames)('rule %s declares at least one message', (ruleName) => {
    const messages = plugin.rules[ruleName].meta.messages;
    expect(Object.keys(messages).length).toBeGreaterThan(0);
    for (const value of Object.values(messages)) {
      expect(typeof value).toBe('string');
      expect(value.length).toBeGreaterThan(0);
    }
  });

  it.each(ruleNames)(
    'rule %s exposes a createOnce factory returning a Program listener',
    (ruleName) => {
      const rule = plugin.rules[ruleName];
      expect(typeof rule.createOnce).toBe('function');

      const context = {
        options: [],
        sourceCode: { text: '', getText: () => '' },
        report: () => {},
      };
      const visitor = rule.createOnce(context);
      expect(typeof visitor.Program).toBe('function');
    },
  );

  it('exposes the recommended config enabling every implemented rule', () => {
    const recommended = plugin.configs.recommended;
    expect(recommended.plugins).toContain('stylistic');
    for (const ruleName of ruleNames) {
      expect(recommended.rules[`stylistic/${ruleName}`]).toBe('error');
    }
  });

  it('re-exports the same plugin object across legacy aliases', () => {
    expect(plugin.corsaStylisticPlugin).toBe(plugin);
    expect(plugin.corsaStylisticRules).toBe(plugin.rules);
  });

  it('rejects unknown rule names referenced from settings', () => {
    const rule = plugin.rules.quotes;
    const sourceCode = {
      text: 'const a = 1;',
      getText() {
        return this.text;
      },
    };

    expect(() => {
      rule
        .createOnce({
          options: [],
          sourceCode,
          settings: {
            corsaStylistic: {
              rules: {
                'this-rule-does-not-exist': [],
              },
            },
          },
          report: () => {},
        })
        .Program({ type: 'Program', range: [0, sourceCode.text.length] });
    }).toThrow(/unknown stylistic rule/);
  });
});
