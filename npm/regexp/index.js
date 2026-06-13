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
  'hexadecimal-escape': {
    unexpected: "Unexpected hexadecimal escape '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'unicode-escape': {
    unexpected: "Unexpected fixed-width unicode escape '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'no-useless-range': {
    unexpected: "Unexpected useless range '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'no-empty-lookarounds-assertion': {
    unexpected:
      'Unexpected empty lookaround assertion; this assertion can never fail or succeed meaningfully.',
  },
  'prefer-regexp-exec': {
    unexpected:
      'Use `RegExp.prototype.exec` instead of `String.prototype.match` for non-global regular expressions.',
  },
  'no-missing-g-flag': {
    unexpected: "`String.prototype.{{expr}}` requires a regular expression with the 'g' flag.",
  },
  'no-useless-character-class': {
    unexpected: "Unexpected single-character class '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'no-empty-string-literal': {
    unexpected: 'Unexpected empty string literal inside a character class.',
  },
  'no-optional-assertion': {
    unexpected:
      'Unexpected optional lookaround assertion; an assertion does not consume input, so the `?` is meaningless.',
  },
  'require-unicode-sets-regexp': {
    require: "Use the 'v' flag.",
  },
  'confusing-quantifier': {
    unexpected:
      'Unexpected lazy quantifier with a minimum of zero. It always prefers the empty match first.',
  },
  'prefer-named-replacement': {
    unexpected:
      'Use a named-group replacement (`$<name>`) instead of a numbered backreference when the regular expression has named capture groups.',
  },
  'no-obscure-range': {
    unexpected:
      "Unexpected character class range '{{expr}}' crosses character category boundaries.",
  },
  'prefer-unicode-codepoint-escapes': {
    unexpected:
      "Unexpected surrogate-pair escape '{{expr}}'. Use the unicode code-point escape '{{replacement}}' instead.",
  },
  'no-dupe-characters-character-class': {
    unexpected: "Duplicate character '{{expr}}' in a character class.",
  },
  'prefer-range': {
    unexpected: "Unexpected consecutive characters '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'no-useless-escape': {
    unexpected:
      "Unnecessary escape '{{expr}}'. Use '{{replacement}}' instead — the character has no special meaning here.",
  },
  'no-useless-quantifier': {
    unexpected: "Unexpected quantifier '{{expr}}' that matches exactly once and can be removed.",
  },
  'prefer-named-backreference': {
    unexpected:
      "Numbered backreference '{{expr}}' is used alongside a named capture group; prefer a named backreference `\\k<name>`.",
  },
  'no-useless-flag': {
    unexpected:
      "Unexpected useless flag '{{flag}}'; the pattern does not use the syntax this flag affects.",
  },
  'no-lazy-ends': {
    unexpected:
      'Unexpected lazy quantifier at the end of the pattern; it will always prefer to match nothing.',
  },
  'no-useless-dollar-replacements': {
    unexpected:
      "Unexpected '\\$0' in replacement string; `$0` is not a valid backreference (capture groups start at 1).",
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
  'hexadecimal-escape': 'disallow `\\xHH` escape sequences (default `never`)',
  'unicode-escape': 'enforce using `\\u{HHHH}` over `\\uHHHH` (default `unicodeCodePointEscape`)',
  'no-useless-range': 'disallow character class ranges whose start equals their end',
  'no-empty-lookarounds-assertion': 'disallow lookaround assertions with an empty body',
  'prefer-regexp-exec':
    'enforce `RegExp.prototype.exec` over `String.prototype.match` for non-global regexes',
  'no-missing-g-flag':
    'enforce that `String.prototype.matchAll` and `replaceAll` arguments use the `g` flag',
  'no-useless-character-class':
    'disallow character classes that contain only a single literal character',
  'no-empty-string-literal': 'disallow empty string literals (`\\q{}`) inside character classes',
  'no-optional-assertion':
    'disallow optional quantifiers (`?`) immediately after a lookaround assertion',
  'require-unicode-sets-regexp': 'enforce the use of the `v` flag (unicode sets mode)',
  'confusing-quantifier':
    'disallow lazy quantifiers (`*?`, `??`, `{0,}?`, `{0,1}?`) whose minimum is zero',
  'prefer-named-replacement':
    'enforce using named-group replacements (`$<name>`) when the regular expression has named captures',
  'no-obscure-range':
    'disallow character class ranges that cross ASCII category boundaries (digits/uppercase/lowercase)',
  'prefer-unicode-codepoint-escapes':
    'enforce using `\\u{H+}` over surrogate-pair `\\uHHHH\\uHHHH` escapes',
  'no-dupe-characters-character-class':
    'disallow duplicate literal characters in a character class',
  'prefer-range': 'enforce using a range (`a-c`) instead of three or more consecutive characters',
  'no-useless-escape': 'disallow escape sequences that have no effect on the matched character',
  'no-useless-quantifier':
    'disallow `{1}` and `{1,1}` quantifiers (they match exactly once and can be removed)',
  'prefer-named-backreference':
    'enforce using named backreferences (`\\k<name>`) when the pattern has named capture groups',
  'no-useless-flag':
    'disallow regular-expression flags that have no effect on the pattern (narrow: `s` without `.`, `m` without `^`/`$`)',
  'no-lazy-ends':
    'disallow lazy quantifiers at the very end of a pattern (they prefer to match nothing)',
  'no-useless-dollar-replacements':
    'disallow `$0` in replacement strings (capture groups start at 1; `$0` is always literal)',
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
  'hexadecimal-escape': 'suggestion',
  'unicode-escape': 'suggestion',
  'no-useless-range': 'suggestion',
  'no-empty-lookarounds-assertion': 'problem',
  'prefer-regexp-exec': 'suggestion',
  'no-missing-g-flag': 'problem',
  'no-useless-character-class': 'suggestion',
  'no-empty-string-literal': 'problem',
  'no-optional-assertion': 'problem',
  'require-unicode-sets-regexp': 'suggestion',
  'confusing-quantifier': 'suggestion',
  'prefer-named-replacement': 'suggestion',
  'no-obscure-range': 'suggestion',
  'prefer-unicode-codepoint-escapes': 'suggestion',
  'no-dupe-characters-character-class': 'problem',
  'prefer-range': 'suggestion',
  'no-useless-escape': 'suggestion',
  'no-useless-quantifier': 'suggestion',
  'prefer-named-backreference': 'suggestion',
  'no-useless-flag': 'suggestion',
  'no-lazy-ends': 'problem',
  'no-useless-dollar-replacements': 'problem',
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
    ruleName === 'no-invisible-character' ||
    ruleName === 'no-empty-lookarounds-assertion' ||
    ruleName === 'no-missing-g-flag' ||
    ruleName === 'no-empty-string-literal' ||
    ruleName === 'no-optional-assertion' ||
    ruleName === 'no-dupe-characters-character-class' ||
    ruleName === 'no-lazy-ends' ||
    ruleName === 'no-useless-dollar-replacements'
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
    ruleName === 'letter-case' ||
    ruleName === 'hexadecimal-escape' ||
    ruleName === 'unicode-escape' ||
    ruleName === 'no-useless-range' ||
    ruleName === 'no-useless-character-class' ||
    ruleName === 'confusing-quantifier' ||
    ruleName === 'prefer-named-replacement' ||
    ruleName === 'no-obscure-range' ||
    ruleName === 'prefer-unicode-codepoint-escapes' ||
    ruleName === 'prefer-range' ||
    ruleName === 'no-useless-escape' ||
    ruleName === 'no-useless-quantifier' ||
    ruleName === 'prefer-named-backreference' ||
    ruleName === 'no-useless-flag'
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
