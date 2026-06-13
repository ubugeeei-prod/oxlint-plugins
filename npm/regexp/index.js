'use strict';

// Oxlint plugin port of eslint-plugin-regexp (MIT).
// The JavaScript layer is only an Oxlint/NAPI adapter; parsing and the
// implemented regular-expression checks run in Rust through Oxc.

const { eslintCompatPlugin } = require('@oxlint/plugins');
const { implementedRegexpRuleNames, scanRegexp } = require('./api.js');

const PLUGIN_NAME = 'regexp';
const DOCS_BASE = 'https://github.com/ota-meshi/eslint-plugin-regexp/tree/master/docs/rules';
const diagnosticsCache = new WeakMap();

const messages = Object.freeze({
  'no-invalid-regexp': {
    error: '{{message}}',
    duplicateFlag: 'Duplicate {{flag}} flag.',
    uvFlag: "Regex 'u' and 'v' flags cannot be used together.",
  },
  'no-empty-character-class': {
    empty: 'This character class matches no characters because it is empty.',
    cannotMatchAny: 'This character class cannot match any characters.',
  },
  'no-empty-group': {
    unexpected: 'Unexpected empty group.',
  },
  'no-empty-capturing-group': {
    unexpected: 'Unexpected capture empty.',
  },
  'no-empty-alternative': {
    empty: 'This empty alternative might be a mistake. If not, use a quantifier instead.',
    suggest: 'Use a quantifier instead.',
  },
  'no-zero-quantifier': {
    unexpected:
      'Unexpected zero quantifier. The quantifier and its quantified element can be removed without affecting the pattern.',
    withCapturingGroup:
      'Unexpected zero quantifier. The quantifier and its quantified element do not affecting the pattern. Try to remove the elements but be careful because it contains at least one capturing group.',
    remove: 'Remove this zero quantifier.',
  },
  'no-octal': {
    unexpected: "Unexpected octal escape sequence '{{expr}}'.",
    replaceHex: 'Replace the octal escape sequence with a hexadecimal escape sequence.',
  },
  'no-control-character': {
    unexpected: 'Unexpected control character {{ char }}.',
    escape: 'Use {{ escape }} instead.',
  },
  'sort-flags': {
    sortFlags: "The flags '{{flags}}' should be in the order '{{sortedFlags}}'.",
  },
  'require-unicode-regexp': {
    require: "Use the 'u' flag.",
  },
  'no-escape-backspace': {
    unexpected:
      "Unexpected '\\b' inside a character class. Use '\\x08' to match the backspace character.",
  },
  'prefer-plus-quantifier': {
    unexpected: "Unexpected quantifier '{{expr}}'. Use '+' instead.",
  },
  'prefer-star-quantifier': {
    unexpected: "Unexpected quantifier '{{expr}}'. Use '*' instead.",
  },
  'prefer-question-quantifier': {
    unexpected: "Unexpected quantifier '{{expr}}'. Use '?' instead.",
  },
  'no-useless-two-nums-quantifier': {
    unexpected: "Unexpected quantifier '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'prefer-named-capture-group': {
    required: 'Capturing group should be converted to a named or non-capturing group.',
  },
  'match-any': {
    unexpected: 'Unexpected any character class. Use `.` with the `s` flag instead.',
  },
  'no-legacy-features': {
    staticProperty:
      "Unexpected use of the legacy 'RegExp.{{expr}}' static property; it is non-standard and not safe to rely on.",
  },
  'prefer-d': {
    unexpected: "Unexpected character class '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'prefer-w': {
    unexpected: "Unexpected character class. Use '{{replacement}}' instead.",
  },
  'letter-case': {
    unexpected: "Unexpected uppercase escape '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'no-non-standard-flag': {
    unexpected: "Unexpected non-standard flag '{{flag}}'.",
  },
  'no-invisible-character': {
    unexpected: 'Unexpected invisible character {{ char }}.',
  },
});

const ruleDescriptions = Object.freeze({
  'no-invalid-regexp': 'disallow invalid regular expression strings in `RegExp` constructors',
  'no-empty-character-class': 'disallow character classes that match no characters',
  'no-empty-group': 'disallow empty group',
  'no-empty-capturing-group': 'disallow capturing group that captures empty.',
  'no-empty-alternative': 'disallow alternatives without elements',
  'no-zero-quantifier': 'disallow quantifiers with a maximum of zero',
  'no-octal': 'disallow octal escape sequence',
  'no-control-character': 'disallow control characters',
  'sort-flags': 'require regex flags to be sorted',
  'require-unicode-regexp': 'enforce the use of the `u` flag',
  'no-escape-backspace': 'disallow escape backspace (`[\\b]`)',
  'prefer-plus-quantifier': 'enforce using `+` quantifier',
  'prefer-star-quantifier': 'enforce using `*` quantifier',
  'prefer-question-quantifier': 'enforce using `?` quantifier',
  'no-useless-two-nums-quantifier': 'disallow unnecessary `{n,m}` quantifier',
  'prefer-named-capture-group': 'enforce using named capture group',
  'match-any':
    'enforce using `.` (with the `s` flag) instead of character classes that match any character',
  'no-legacy-features': 'disallow legacy `RegExp` features',
  'prefer-d': 'enforce using `\\d` instead of `[0-9]`',
  'prefer-w': 'enforce using `\\w` instead of `[a-zA-Z0-9_]`',
  'letter-case': 'enforce consistent case for escape sequences (default lowercase)',
  'no-non-standard-flag': 'disallow non-standard flags on regular expressions',
  'no-invisible-character': 'disallow invisible characters in regular expressions',
});

const ruleTypes = Object.freeze({
  'no-invalid-regexp': 'problem',
  'no-empty-character-class': 'suggestion',
  'no-empty-group': 'suggestion',
  'no-empty-capturing-group': 'suggestion',
  'no-empty-alternative': 'problem',
  'no-zero-quantifier': 'suggestion',
  'no-octal': 'suggestion',
  'no-control-character': 'suggestion',
  'sort-flags': 'suggestion',
  'require-unicode-regexp': 'suggestion',
  'no-escape-backspace': 'suggestion',
  'prefer-plus-quantifier': 'suggestion',
  'prefer-star-quantifier': 'suggestion',
  'prefer-question-quantifier': 'suggestion',
  'no-useless-two-nums-quantifier': 'suggestion',
  'prefer-named-capture-group': 'suggestion',
  'match-any': 'suggestion',
  'no-legacy-features': 'problem',
  'prefer-d': 'suggestion',
  'prefer-w': 'suggestion',
  'letter-case': 'suggestion',
  'no-non-standard-flag': 'problem',
  'no-invisible-character': 'problem',
});

const recommendedRuleConfig = Object.freeze({
  'no-invalid-regexp': 'error',
  'no-empty-character-class': 'error',
  'no-empty-group': 'error',
  'no-empty-capturing-group': 'error',
  'no-empty-alternative': 'warn',
  'no-zero-quantifier': 'error',
  'sort-flags': 'error',
  'no-escape-backspace': 'error',
  'prefer-plus-quantifier': 'error',
  'prefer-star-quantifier': 'error',
  'prefer-question-quantifier': 'error',
  'no-useless-two-nums-quantifier': 'error',
  'no-legacy-features': 'error',
  'no-non-standard-flag': 'error',
  'no-invisible-character': 'error',
});

const implementedRuleNames = Object.freeze(implementedRegexpRuleNames());
const rules = Object.freeze(
  Object.fromEntries(
    implementedRuleNames.map((ruleName) => [ruleName, createRegexpRule(ruleName)]),
  ),
);

const plugin = eslintCompatPlugin({
  meta: {
    name: PLUGIN_NAME,
    version: '0.0.0',
  },
  rules,
  rulesConfig: Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 0])),
  configs: {
    recommended: configFromRuleConfig('recommended', recommendedRuleConfig),
    all: configFromRuleConfig(
      'all',
      Object.fromEntries(implementedRuleNames.map((ruleName) => [ruleName, 'error'])),
    ),
  },
});

plugin.configs['flat/recommended'] = plugin.configs.recommended;
plugin.configs['flat/all'] = plugin.configs.all;
plugin.implementedRegexpRuleNames = implementedRuleNames;
plugin.scanRegexp = scanRegexp;

function configFromRuleConfig(name, ruleConfig) {
  return {
    name: `${PLUGIN_NAME}/${name}`,
    plugins: [PLUGIN_NAME],
    rules: Object.fromEntries(
      Object.entries(ruleConfig).map(([ruleName, config]) => [
        `${PLUGIN_NAME}/${ruleName}`,
        config,
      ]),
    ),
  };
}

function createRegexpRule(ruleName) {
  return {
    meta: {
      type: ruleTypes[ruleName],
      docs: {
        description: ruleDescriptions[ruleName],
        category: ruleCategory(ruleName),
        recommended: Object.hasOwn(recommendedRuleConfig, ruleName),
        url: `${DOCS_BASE}/${ruleName}.md`,
      },
      messages: messages[ruleName],
      schema: [],
    },
    createOnce(context) {
      return {
        Program() {
          for (const diagnostic of diagnosticsForContext(context)) {
            if (diagnostic.ruleName !== ruleName) {
              continue;
            }
            context.report({
              messageId: diagnostic.messageId,
              data: compactData(diagnostic.data),
              loc: {
                start: {
                  line: diagnostic.loc.startLine,
                  column: diagnostic.loc.startColumn,
                },
                end: {
                  line: diagnostic.loc.endLine,
                  column: diagnostic.loc.endColumn,
                },
              },
            });
          }
        },
      };
    },
  };
}

function ruleCategory(ruleName) {
  if (
    ruleName === 'no-invalid-regexp' ||
    ruleName === 'no-empty-character-class' ||
    ruleName === 'no-empty-group' ||
    ruleName === 'no-empty-alternative' ||
    ruleName === 'no-control-character' ||
    ruleName === 'no-escape-backspace' ||
    ruleName === 'no-legacy-features' ||
    ruleName === 'no-non-standard-flag' ||
    ruleName === 'no-invisible-character'
  ) {
    return 'Possible Errors';
  }
  if (ruleName === 'sort-flags') {
    return 'Stylistic Issues';
  }
  if (
    ruleName === 'prefer-plus-quantifier' ||
    ruleName === 'prefer-star-quantifier' ||
    ruleName === 'prefer-question-quantifier' ||
    ruleName === 'no-useless-two-nums-quantifier' ||
    ruleName === 'prefer-named-capture-group' ||
    ruleName === 'match-any' ||
    ruleName === 'prefer-d' ||
    ruleName === 'prefer-w' ||
    ruleName === 'letter-case'
  ) {
    return 'Stylistic Issues';
  }
  return 'Best Practices';
}

function diagnosticsForContext(context) {
  const sourceCode = context.sourceCode || {};
  const sourceText = sourceTextForContext(context);
  const filename = typeof context.filename === 'string' ? context.filename : 'file.js';
  const cached = diagnosticsCache.get(sourceCode);

  if (cached && cached.sourceText === sourceText && cached.filename === filename) {
    return cached.diagnostics;
  }

  const diagnostics = scanRegexp(sourceText, filename);
  diagnosticsCache.set(sourceCode, { sourceText, filename, diagnostics });
  return diagnostics;
}

function sourceTextForContext(context) {
  const sourceCode = context.sourceCode || {};
  if (typeof sourceCode.getText === 'function') {
    return sourceCode.getText();
  }
  if (typeof sourceCode.text === 'string') {
    return sourceCode.text;
  }
  return '';
}

function compactData(data) {
  const out = {};
  for (const [key, value] of Object.entries(data || {})) {
    if (value != null) {
      out[key] = value;
    }
  }
  if (out.charText != null && out.char == null) {
    out.char = out.charText;
  }
  return out;
}

module.exports = plugin;
module.exports.default = plugin;
module.exports.implementedRegexpRuleNames = implementedRuleNames;
module.exports.scanRegexp = scanRegexp;
