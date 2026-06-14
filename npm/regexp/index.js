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
    unexpected:
      "Unexpected unicode escape '{{expr}}'. Use the hexadecimal escape '{{replacement}}' instead.",
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
  'prefer-regexp-test': {
    disallow: 'Use the `RegExp#test()` method instead, if you need a boolean.',
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
  'prefer-escape-replacement-dollar-char': {
    unexpected:
      "Use '$$' to escape a literal '$' in the replacement string; a stray '$' is almost always a typo.",
  },
  'use-ignore-case': {
    unexpected:
      "Character class mixes lower- and upper-case forms of the same letter; add the 'i' flag instead.",
  },
  'control-character-escape': {
    unexpected:
      'Unexpected literal control character {{ char }} in the pattern; use a `\\xHH` or `\\uHHHH` escape sequence instead.',
  },
  'grapheme-string-literal': {
    unexpected:
      "Unexpected single-character string literal '{{expr}}'. Use '{{replacement}}' instead.",
  },
  'no-useless-non-capturing-group': {
    unexpected:
      'Unexpected non-capturing group with a single literal body; remove the `(?:` ... `)` wrapper.',
  },
  'prefer-quantifier': {
    unexpected:
      'Apply the quantifier directly to the single-character body instead of wrapping it in a non-capturing group.',
  },
  'no-useless-string-literal': {
    unexpected:
      "Unexpected single-character string literal '{{expr}}'. Drop the `\\q{...}` wrapper and use '{{replacement}}' instead.",
  },
  'sort-character-class-elements': {
    unexpected: 'Character class elements are not sorted; reorder them lexicographically.',
  },
  'no-trivially-nested-assertion': {
    unexpected:
      'Non-capturing group whose body is exactly one lookaround assertion; drop the `(?:` ... `)` wrapper.',
  },
  'no-extra-lookaround-assertions': {
    unexpected:
      'Lookaround assertion whose body is exactly one nested lookaround; drop the outer assertion.',
  },
  'no-trivially-nested-quantifier': {
    unexpected:
      'Non-capturing group with a quantified single-character body that is itself quantified; drop the wrapper and combine the quantifiers.',
  },
  'prefer-character-class': {
    unexpected: 'Alternation of single-character literals; use a character class `[abc]` instead.',
  },
  'sort-alternatives': {
    unexpected: 'Single-literal alternation is not in ascending order; sort the alternatives.',
  },
  'prefer-predefined-assertion': {
    unexpected:
      'Lookaround whose body is exactly the `^` or `$` anchor; use the bare anchor instead.',
  },
  'optimal-lookaround-quantifier': {
    unexpected:
      'Lookaround whose inner quantifier accepts the empty match; the assertion is always true.',
  },
  'no-dupe-disjunctions': {
    unexpected: 'Alternation contains the same single-literal alternative more than once.',
  },
  'no-useless-backreference': {
    unexpected:
      'Backreference to a capturing group that was not yet defined when the reference appears.',
  },
  negation: {
    unexpected:
      'Negated character class containing a single predefined shorthand can be replaced by the corresponding negated shorthand.',
  },
  'no-useless-lazy': {
    unexpected:
      'Lazy modifier is useless on a fixed-count quantifier; the engine always matches exactly the same number of repetitions.',
  },
  'no-misleading-unicode-character': {
    unexpected:
      'Character class contains a ZWJ (U+200D) and matches it as a separate atom; ZWJ-joined sequences cannot be matched as a single grapheme this way.',
  },
  'no-standalone-backslash': {
    unexpected:
      "Unexpected standalone backslash (`\\`). It looks like an escape sequence, but it's a single `\\` character pattern.",
  },
  'no-potentially-useless-backreference': {
    potentiallyUselessBackreference:
      'Some paths leading to the backreference do not go through the referenced capturing group or the captured text might be reset before reaching the backreference.',
  },
  strict: {
    invalidControlEscape:
      'Invalid or incomplete control escape sequence. Either use a valid control escape sequence or escape the standalone backslash.',
    incompleteEscapeSequence:
      'Incomplete escape sequence {{expr}}. Either use a valid escape sequence or remove the useless escaping.',
    invalidPropertyEscape:
      'Invalid property escape sequence {{expr}}. Either use a valid property escape sequence or remove the useless escaping.',
    incompleteBackreference:
      'Incomplete backreference {{expr}}. Either use a valid backreference or remove the useless escaping.',
    unescapedSourceCharacter: 'Unescaped source character {{expr}}.',
    octalEscape: 'Invalid legacy octal escape sequence {{expr}}. Use a hexadecimal escape instead.',
    uselessEscape: 'Useless identity escapes with non-syntax characters are forbidden.',
    invalidRange:
      'Invalid character class range. A character set cannot be the minimum or maximum of a character class range.',
    quantifiedAssertion: 'Assertion are not allowed to be quantified directly.',
    regexMessage: '{{message}}.',
  },
  'no-useless-assertions': {
    unexpected:
      'Useless word-boundary assertion: `\\b`/`\\B` between two characters of the same word class always rejects (`\\b`) or always accepts (`\\B`).',
  },
  'optimal-quantifier-concatenation': {
    unexpected:
      'Adjacent quantifiers on the same element can be merged into a single optimal quantifier (e.g. `aa*` to `a+`).',
  },
  'no-contradiction-with-assertion': {
    unexpected:
      'This quantifier contradicts the preceding `\\b` assertion and can never be entered.',
  },
  'no-useless-set-operand': {
    unexpected:
      'This set operation has a useless operand (the operands are disjoint or one is a subset of the other).',
  },
  'prefer-set-operation': {
    unexpected:
      'This character + lookaround can be expressed as a v-mode set operation (`[Y&&X]` or `[Y--X]`).',
  },
  'simplify-set-operations': {
    unexpected:
      'This set operation can be simplified (an intersection with a negated class is a subtraction, or De Morgan applies).',
  },
  'unicode-property': {
    unnecessaryGc:
      'Unnecessary explicit General_Category key in a Unicode property escape; drop the redundant `gc=` / `General_Category=` prefix.',
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
  'prefer-regexp-test':
    'enforce `RegExp#test` instead of `String#match` / `RegExp#exec` when the result is used only as a boolean',
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
  'prefer-escape-replacement-dollar-char':
    'enforce escaping a literal `$` as `$$` in replacement strings',
  'use-ignore-case':
    'enforce the `i` flag when a character class mixes lower- and upper-case letters',
  'control-character-escape':
    'enforce escaping literal control characters in regular-expression patterns',
  'grapheme-string-literal':
    'disallow single-character `\\q{X}` string literals in v-mode character classes',
  'no-useless-non-capturing-group':
    'disallow non-capturing groups that wrap a single literal character without a quantifier',
  'prefer-quantifier':
    'apply quantifiers directly to single-character atoms instead of wrapping them in a non-capturing group',
  'no-useless-string-literal':
    'disallow string literals in v-mode classes when a bare character would do',
  'sort-character-class-elements':
    'enforce sorted character class elements when the body is all literal alphanumerics',
  'no-trivially-nested-assertion':
    'disallow non-capturing groups whose entire body is a single lookaround assertion',
  'no-extra-lookaround-assertions':
    'disallow lookaround assertions whose entire body is a single nested lookaround',
  'no-trivially-nested-quantifier': 'disallow trivially nested quantifiers like `(?:X+)+`',
  'prefer-character-class':
    'enforce using a character class instead of alternation of single literals',
  'sort-alternatives': 'enforce sorted single-literal alternation alternatives',
  'prefer-predefined-assertion':
    'prefer the bare `^` or `$` anchor over a lookaround that wraps it',
  'optimal-lookaround-quantifier':
    'disallow lookarounds whose body always matches because of a zero-min quantifier',
  'no-dupe-disjunctions':
    'disallow duplicate single-literal alternatives within a non-capturing group',
  'no-useless-backreference':
    'disallow numbered backreferences that point to a not-yet-defined capture',
  negation:
    'enforce use of equivalent shorthand for negated character classes containing a single predefined shorthand',
  'no-useless-lazy': 'disallow lazy modifiers on fixed-count brace quantifiers (`{n}?`, `{n,n}?`)',
  'no-misleading-unicode-character':
    'disallow character classes that contain a ZWJ (U+200D) and match it as a separate atom',
  'no-standalone-backslash': 'disallow standalone backslashes (`\\`)',
  'no-potentially-useless-backreference':
    'disallow backreferences to capturing groups that might not have matched (narrow: optional/star-quantified groups)',
  strict: 'disallow not strictly valid regular expressions (narrow: escape and quantifier checks)',
  'no-useless-assertions':
    'disallow assertions that are known to always accept (or always reject) (narrow: word-boundary between same-class literals)',
  'optimal-quantifier-concatenation':
    'require optimal quantifiers for concatenated quantifiers (narrow: adjacent quantifiers on the same single element)',
  'no-contradiction-with-assertion':
    'disallow elements that contradict assertions (narrow: min-zero quantifier on a same-class literal right after `\\b`)',
  'no-useless-set-operand':
    'disallow unnecessary elements in expression character classes (narrow: shorthand `&&`/`--` operands that are disjoint or subsets)',
  'prefer-set-operation':
    'prefer character class set operations instead of lookarounds (narrow: v-mode char lookaround adjacent to a char element)',
  'simplify-set-operations':
    'simplify unnecessarily complex set operations (narrow: v-mode `&&` intersection with a negated nested-class operand)',
  'unicode-property':
    'enforce consistent naming of unicode properties (narrow: flag a redundant explicit `gc=` / `General_Category=` key)',
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
  'prefer-regexp-test': 'suggestion',
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
  'prefer-escape-replacement-dollar-char': 'suggestion',
  'use-ignore-case': 'suggestion',
  'control-character-escape': 'problem',
  'grapheme-string-literal': 'suggestion',
  'no-useless-non-capturing-group': 'suggestion',
  'prefer-quantifier': 'suggestion',
  'no-useless-string-literal': 'suggestion',
  'sort-character-class-elements': 'suggestion',
  'no-trivially-nested-assertion': 'suggestion',
  'no-extra-lookaround-assertions': 'suggestion',
  'no-trivially-nested-quantifier': 'suggestion',
  'prefer-character-class': 'suggestion',
  'sort-alternatives': 'suggestion',
  'prefer-predefined-assertion': 'suggestion',
  'optimal-lookaround-quantifier': 'problem',
  'no-dupe-disjunctions': 'problem',
  'no-useless-backreference': 'problem',
  negation: 'suggestion',
  'no-useless-lazy': 'suggestion',
  'no-misleading-unicode-character': 'problem',
  'no-standalone-backslash': 'suggestion',
  'no-potentially-useless-backreference': 'problem',
  strict: 'suggestion',
  'no-useless-assertions': 'problem',
  'optimal-quantifier-concatenation': 'suggestion',
  'no-contradiction-with-assertion': 'problem',
  'no-useless-set-operand': 'suggestion',
  'prefer-set-operation': 'suggestion',
  'simplify-set-operations': 'suggestion',
  'unicode-property': 'suggestion',
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
  'no-potentially-useless-backreference': 'warn',
  strict: 'error',
  'no-useless-assertions': 'error',
  'optimal-quantifier-concatenation': 'error',
  'no-contradiction-with-assertion': 'error',
  'no-useless-set-operand': 'error',
  'prefer-set-operation': 'error',
  'simplify-set-operations': 'error',
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
    ruleName === 'no-useless-dollar-replacements' ||
    ruleName === 'control-character-escape'
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
    ruleName === 'no-useless-flag' ||
    ruleName === 'prefer-escape-replacement-dollar-char' ||
    ruleName === 'use-ignore-case' ||
    ruleName === 'grapheme-string-literal' ||
    ruleName === 'no-useless-non-capturing-group' ||
    ruleName === 'prefer-quantifier' ||
    ruleName === 'no-useless-string-literal' ||
    ruleName === 'sort-character-class-elements' ||
    ruleName === 'no-trivially-nested-assertion' ||
    ruleName === 'no-extra-lookaround-assertions' ||
    ruleName === 'no-trivially-nested-quantifier' ||
    ruleName === 'prefer-character-class' ||
    ruleName === 'sort-alternatives' ||
    ruleName === 'prefer-predefined-assertion' ||
    ruleName === 'negation' ||
    ruleName === 'no-useless-lazy' ||
    ruleName === 'unicode-property'
  ) {
    return 'Stylistic Issues';
  }
  if (
    ruleName === 'optimal-lookaround-quantifier' ||
    ruleName === 'no-dupe-disjunctions' ||
    ruleName === 'no-useless-backreference' ||
    ruleName === 'no-misleading-unicode-character' ||
    ruleName === 'no-potentially-useless-backreference' ||
    ruleName === 'strict' ||
    ruleName === 'no-useless-assertions' ||
    ruleName === 'no-contradiction-with-assertion'
  ) {
    return 'Possible Errors';
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
