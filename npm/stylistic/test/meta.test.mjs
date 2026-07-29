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
    if (['no-mixed-operators', 'jsx-child-element-spacing'].includes(ruleName)) {
      expect(rule.meta.fixable).toBeUndefined();
      expect(rule.meta.hasSuggestions).toBe(false);
    } else {
      expect(rule.meta.fixable).toBe(
        [
          'jsx-closing-bracket-location',
          'jsx-quotes',
          'no-confusing-arrow',
          'no-extra-parens',
          'wrap-iife',
        ].includes(ruleName)
          ? 'code'
          : 'whitespace',
      );
    }
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

  it('preserves the upstream jsx-quotes message template', () => {
    expect(plugin.rules['jsx-quotes'].meta.messages.unexpected).toBe(
      'Unexpected usage of {{description}}.',
    );
    expect(plugin.rules['jsx-quotes'].meta.fixable).toBe('code');
  });

  it('preserves the complete upstream jsx-child-element-spacing metadata contract', () => {
    expect(plugin.rules['jsx-child-element-spacing'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description:
          'Enforce or disallow spaces inside of curly braces in JSX attributes and expressions',
        recommended: false,
        requiresTypeChecking: false,
      },
      hasSuggestions: false,
      messages: {
        spacingAfterPrev: 'Ambiguous spacing after previous element {{element}}',
        spacingBeforeNext: 'Ambiguous spacing before next element {{element}}',
      },
      schema: { type: 'array' },
    });
    expect(plugin.rules['jsx-child-element-spacing'].meta.fixable).toBeUndefined();
  });

  it('preserves the stable lines-around-comment message catalog', () => {
    expect(plugin.rules['lines-around-comment'].meta.messages).toMatchObject({
      before: 'Expected line before comment.',
      after: 'Expected line after comment.',
    });
    expect(plugin.rules['lines-around-comment'].meta.fixable).toBe('whitespace');
  });

  it('preserves type-generic-spacing upstream metadata', () => {
    expect(plugin.rules['type-generic-spacing'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforces consistent spacing inside TypeScript type generics',
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        genericSpacingMismatch: 'Generic spaces mismatch',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves function-call-argument-newline upstream metadata', () => {
    expect(plugin.rules['function-call-argument-newline'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce line breaks between arguments of a function call.',
      },
      fixable: 'whitespace',
      messages: {
        unexpectedLineBreak: 'There should be no line break here.',
        missingLineBreak: 'There should be a line break after this argument.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the upstream array-element-newline metadata', () => {
    expect(plugin.rules['array-element-newline'].meta.messages).toEqual({
      missingLineBreak: 'There should be a linebreak after this element.',
      unexpectedLineBreak: 'There should be no linebreak here.',
    });
    expect(plugin.rules['array-element-newline'].meta.fixable).toBe('whitespace');
  });

  it('preserves the stable object-property-newline metadata', () => {
    expect(plugin.rules['object-property-newline'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce placing object properties on separate lines.',
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        propertiesOnNewline: 'Object properties must go on a new line.',
        propertiesOnNewlineAll:
          "Object properties must go on a new line if they aren't all on the same line.",
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the complete upstream object-curly-newline metadata contract', () => {
    expect(plugin.rules['object-curly-newline'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce consistent line breaks after opening and before closing braces.',
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        unexpectedLinebreakBeforeClosingBrace: 'Unexpected line break before this closing brace.',
        unexpectedLinebreakAfterOpeningBrace: 'Unexpected line break after this opening brace.',
        expectedLinebreakBeforeClosingBrace: 'Expected a line break before this closing brace.',
        expectedLinebreakAfterOpeningBrace: 'Expected a line break after this opening brace.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the stable function-paren-newline message catalog and fix metadata', () => {
    expect(plugin.rules['function-paren-newline'].meta.messages).toEqual({
      expectedBefore: "Expected newline before ')'.",
      expectedAfter: "Expected newline after '('.",
      expectedBetween: 'Expected newline between arguments/params.',
      unexpectedBefore: "Unexpected newline before ')'.",
      unexpectedAfter: "Unexpected newline after '('.",
    });
    expect(plugin.rules['function-paren-newline'].meta.fixable).toBe('whitespace');
    expect(plugin.rules['function-paren-newline'].meta.hasSuggestions).toBe(true);
  });

  it('preserves the stable curly-newline metadata', () => {
    expect(plugin.rules['curly-newline'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce consistent line breaks after opening and before closing braces.',
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        unexpectedLinebreakBeforeClosingBrace: 'Unexpected line break before this closing brace.',
        unexpectedLinebreakAfterOpeningBrace: 'Unexpected line break after this opening brace.',
        expectedLinebreakBeforeClosingBrace: 'Expected a line break before this closing brace.',
        expectedLinebreakAfterOpeningBrace: 'Expected a line break after this opening brace.',
      },
    });
  });

  it('preserves the stable jsx-closing-bracket-location metadata contract', () => {
    expect(plugin.rules['jsx-closing-bracket-location'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce closing bracket location in JSX',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'code',
      hasSuggestions: true,
      messages: {
        bracketLocation: 'The closing bracket must be {{location}}{{details}}',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves member-delimiter-style metadata from the stable upstream rule', () => {
    expect(plugin.rules['member-delimiter-style'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Require a specific member delimiter style for interfaces and type literals.',
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        unexpectedComma: 'Unexpected separator (,).',
        unexpectedSemi: 'Unexpected separator (;).',
        expectedComma: 'Expected a comma.',
        expectedSemi: 'Expected a semicolon.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the upstream multiline-comment-style metadata', () => {
    expect(plugin.rules['multiline-comment-style'].meta).toMatchObject({
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        expectedBlock: 'Expected a block comment instead of consecutive line comments.',
        expectedBareBlock: 'Expected a block comment without padding stars.',
        startNewline: "Expected a linebreak after '/*'.",
        endNewline: "Expected a linebreak before '*/'.",
        missingStar: "Expected a '*' at the start of this line.",
        alignment: 'Expected this line to be aligned with the start of the comment.',
        expectedLines: 'Expected multiple line comments instead of a block comment.',
        fixStyle: 'Apply the expected multiline comment style.',
      },
    });
  });

  it('preserves indent-binary-ops upstream metadata', () => {
    expect(plugin.rules['indent-binary-ops'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Indentation for binary operators',
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        wrongIndentation: 'Expected indentation of {{expected}}',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the complete upstream multiline-ternary metadata contract', () => {
    expect(plugin.rules['multiline-ternary'].meta).toMatchObject({
      type: 'layout',
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        expectedTestCons: 'Expected newline between test and consequent of ternary expression.',
        expectedConsAlt: 'Expected newline between consequent and alternate of ternary expression.',
        unexpectedTestCons: 'Unexpected newline between test and consequent of ternary expression.',
        unexpectedConsAlt:
          'Unexpected newline between consequent and alternate of ternary expression.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the stable brace-style metadata and complete message catalog', () => {
    expect(plugin.rules['brace-style'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce consistent brace style for blocks',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        nextLineOpen:
          'Opening curly brace does not appear on the same line as controlling statement.',
        sameLineOpen: 'Opening curly brace appears on the same line as controlling statement.',
        blockSameLine: 'Statement inside of curly braces should be on next line.',
        nextLineClose:
          'Closing curly brace does not appear on the same line as the subsequent block.',
        singleLineClose:
          'Closing curly brace should be on the same line as opening curly brace or on the line after the previous block.',
        sameLineClose: 'Closing curly brace appears on the same line as the subsequent block.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the complete nonblock-statement-body-position metadata contract', () => {
    expect(plugin.rules['nonblock-statement-body-position'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Enforce the location of single-line statements',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        expectNoLinebreak: 'Expected no linebreak before this statement.',
        expectLinebreak: 'Expected a linebreak before this statement.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the complete type-named-tuple-spacing metadata contract', () => {
    expect(plugin.rules['type-named-tuple-spacing'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Expect space before the type declaration in the named tuple',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        expectedSpaceAfter: "Expected a space after the ':'.",
        unexpectedSpaceBetween: "Unexpected space between '?' and the ':'.",
        unexpectedSpaceBefore: "Unexpected space before the ':'.",
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the stable semi metadata and complete message catalog', () => {
    expect(plugin.rules.semi.meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Require or disallow semicolons instead of ASI',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'whitespace',
      hasSuggestions: true,
      messages: {
        missingSemi: 'Missing semicolon.',
        extraSemi: 'Extra semicolon.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves stable no-extra-parens metadata', () => {
    expect(plugin.rules['no-extra-parens'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Disallow unnecessary parentheses',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'code',
      hasSuggestions: true,
      messages: {
        unexpected: 'Unnecessary parentheses around expression.',
      },
      schema: { type: 'array' },
    });
  });

  it('preserves the complete stable wrap-iife metadata contract', () => {
    expect(plugin.rules['wrap-iife'].meta).toMatchObject({
      type: 'layout',
      docs: {
        description: 'Require parentheses around immediate `function` invocations',
        recommended: false,
        requiresTypeChecking: false,
      },
      fixable: 'code',
      hasSuggestions: true,
      messages: {
        wrapInvocation: 'Wrap an immediate function invocation in parentheses.',
        wrapExpression: 'Wrap only the function expression in parens.',
        moveInvocation: 'Move the invocation into the parens that contain the function.',
      },
      schema: { type: 'array' },
    });
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
