import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const validCases = [
  // no-invalid-regexp
  ['no-invalid-regexp', 'valid constructor', "new RegExp('a+', 'u');\n"],
  ['no-invalid-regexp', 'all valid flags', "new RegExp('a', 'gimsu');\n"],
  ['no-invalid-regexp', 'unicode set flag', "new RegExp('[a]', 'v');\n"],
  // no-empty-character-class
  ['no-empty-character-class', 'single-char class', 'const re = /[a]/u;\n'],
  ['no-empty-character-class', 'negated empty-looking class', 'const re = /[^]/u;\n'],
  ['no-empty-character-class', 'class containing escaped bracket', 'const re = /[\\]]/u;\n'],
  // no-empty-group
  ['no-empty-group', 'non-capturing group with content', 'const re = /(?:a)/u;\n'],
  ['no-empty-group', 'empty lookahead is allowed', 'const re = /(?=a)/u;\n'],
  ['no-empty-group', 'empty negative lookahead is allowed', 'const re = /(?!a)/u;\n'],
  // no-empty-capturing-group
  ['no-empty-capturing-group', 'capture with content', 'const re = /(a)/u;\n'],
  ['no-empty-capturing-group', 'named capture with content', 'const re = /(?<name>a)/u;\n'],
  // no-empty-alternative
  ['no-empty-alternative', 'simple alternation', 'const re = /a|b/u;\n'],
  ['no-empty-alternative', 'alternation inside group', 'const re = /(?:a|b|c)/u;\n'],
  // no-zero-quantifier
  ['no-zero-quantifier', 'positive quantifier', 'const re = /a{1}/u;\n'],
  ['no-zero-quantifier', 'open upper bound', 'const re = /a{0,}/u;\n'],
  ['no-zero-quantifier', 'positive range', 'const re = /a{2,5}/u;\n'],
  // no-octal
  ['no-octal', 'nul escape only', 'const re = /\\0/u;\n'],
  ['no-octal', 'nul followed by 8 (not octal)', 'const re = /\\08/u;\n'],
  // no-control-character
  ['no-control-character', 'named tab escape', 'const re = /\\t/u;\n'],
  ['no-control-character', 'named newline escape', 'const re = /\\n/u;\n'],
  ['no-control-character', 'printable hex escape', "const re = new RegExp('\\\\u0041', 'u');\n"],
  [
    'no-control-character',
    'constructor named newline escape',
    "const re = new RegExp('\\n', 'u');\n",
  ],
  // control-character-escape
  ['control-character-escape', 'named tab in constructor arg', "new RegExp('\\t');\n"],
  // sort-flags
  ['sort-flags', 'sorted flags', 'const re = /a/im;\n'],
  ['sort-flags', 'no flags', 'const re = /a/;\n'],
  ['sort-flags', 'single flag', 'const re = /a/u;\n'],
  // require-unicode-regexp
  ['require-unicode-regexp', 'unicode flag', 'const re = /a/u;\n'],
  ['require-unicode-regexp', 'unicode set flag', 'const re = /a/v;\n'],
  ['require-unicode-regexp', 'unicode with other flags', 'const re = /a/gu;\n'],
  // no-escape-backspace
  ['no-escape-backspace', 'plain word boundary', 'const re = /\\bword/u;\n'],
  ['no-escape-backspace', 'character class without \\b', 'const re = /[a-z]/u;\n'],
  // prefer-plus-quantifier
  ['prefer-plus-quantifier', 'plus quantifier', 'const re = /a+/u;\n'],
  ['prefer-plus-quantifier', 'two-or-more braced quantifier', 'const re = /a{2,}/u;\n'],
  // prefer-star-quantifier
  ['prefer-star-quantifier', 'star quantifier', 'const re = /a*/u;\n'],
  ['prefer-star-quantifier', 'open-upper-bound greater than zero', 'const re = /a{1,}/u;\n'],
  // prefer-question-quantifier
  ['prefer-question-quantifier', 'question quantifier', 'const re = /a?/u;\n'],
  ['prefer-question-quantifier', 'distinct range bounds', 'const re = /a{1,2}/u;\n'],
  // no-useless-two-nums-quantifier
  ['no-useless-two-nums-quantifier', 'single-bound quantifier', 'const re = /a{3}/u;\n'],
  ['no-useless-two-nums-quantifier', 'asymmetric range quantifier', 'const re = /a{2,5}/u;\n'],
  // prefer-named-capture-group
  ['prefer-named-capture-group', 'named capture', 'const re = /(?<name>a)/u;\n'],
  ['prefer-named-capture-group', 'non-capturing group', 'const re = /(?:a)/u;\n'],
  ['prefer-named-capture-group', 'lookahead', 'const re = /(?=a)/u;\n'],
  ['prefer-named-capture-group', 'no group', 'const re = /a/u;\n'],
  // match-any
  ['match-any', 'plain character class', 'const re = /[a-z]/u;\n'],
  ['match-any', 'half anti-pair', 'const re = /[\\s]/u;\n'],
  ['match-any', 'mixed family', 'const re = /[\\s\\D]/u;\n'],
  ['match-any', 'negated anti-pair', 'const re = /[^\\s\\S]/u;\n'],
  ['match-any', 'canonical \\s\\S form', 'const re = /[\\s\\S]/u;\n'],
  // no-legacy-features
  ['no-legacy-features', 'unrelated identifier', 'Foo.$1;\n'],
  ['no-legacy-features', 'lowercase regexp', 'regexp.lastMatch;\n'],
  ['no-legacy-features', 'modern prototype access', 'RegExp.prototype;\n'],
  ['no-legacy-features', '$10 out of legacy range', 'RegExp.$10;\n'],
  // prefer-d
  ['prefer-d', 'shorthand already used', 'const re = /\\d/u;\n'],
  ['prefer-d', 'subset range', 'const re = /[1-9]/u;\n'],
  ['prefer-d', 'extra element', 'const re = /[0-9a]/u;\n'],
  // prefer-w
  ['prefer-w', 'shorthand already used', 'const re = /\\w/u;\n'],
  ['prefer-w', 'missing element', 'const re = /[a-zA-Z0-9]/u;\n'],
  // letter-case
  ['letter-case', 'lowercase hex escape', 'const re = /\\xab/u;\n'],
  ['letter-case', 'lowercase unicode escape', 'const re = /\\uabcd/u;\n'],
  ['letter-case', 'decimal-only unicode escape', "const re = new RegExp('\\\\u0041', 'u');\n"],
  // no-non-standard-flag
  ['no-non-standard-flag', 'canonical flags', "const re = new RegExp('a', 'gimsuy');\n"],
  ['no-non-standard-flag', 'no flags', 'const re = /a/;\n'],
  // no-invisible-character
  ['no-invisible-character', 'plain pattern', 'const re = /ab/u;\n'],
  ['no-invisible-character', 'ascii space', 'const re = /a b/u;\n'],
  ['no-invisible-character', 'escaped hex NBSP', "const re = new RegExp('a\\\\xa0b', 'u');\n"],
  [
    'no-invisible-character',
    'constructor named tab escape',
    "const re = new RegExp('\\t', 'u');\n",
  ],
  // no-useless-string-literal
  ['no-useless-string-literal', 'empty literal', 'const re = /[\\q{}]/v;\n'],
  ['no-useless-string-literal', 'multi-char literal', 'const re = /[\\q{ab}]/v;\n'],
  // sort-character-class-elements
  ['sort-character-class-elements', 'sorted class', 'const re = /[ab]/u;\n'],
  ['sort-character-class-elements', 'class with escape', 'const re = /[a\\d]/u;\n'],
  ['sort-character-class-elements', 'class with range', 'const re = /[a-z]/u;\n'],
  // no-trivially-nested-assertion
  ['no-trivially-nested-assertion', 'non-cap with literal body', 'const re = /(?:a)/u;\n'],
  ['no-trivially-nested-assertion', 'lookaround at top level', 'const re = /(?=a)/u;\n'],
  // no-extra-lookaround-assertions
  ['no-extra-lookaround-assertions', 'lookaround with literal body', 'const re = /(?=a)/u;\n'],
  ['no-extra-lookaround-assertions', 'non-cap wrapping lookaround', 'const re = /(?:(?=a))/u;\n'],
  // no-trivially-nested-quantifier
  ['no-trivially-nested-quantifier', 'no outer quantifier', 'const re = /(?:a+)/u;\n'],
  ['no-trivially-nested-quantifier', 'no inner quantifier', 'const re = /(?:a)+/u;\n'],
  ['no-trivially-nested-quantifier', 'multi-byte body', 'const re = /(?:ab+)+/u;\n'],
  // prefer-character-class
  ['prefer-character-class', 'multi-byte alt', 'const re = /(?:a|bc)/u;\n'],
  ['prefer-character-class', 'escape alt', 'const re = /(?:a|\\d)/u;\n'],
  ['prefer-character-class', 'no alternation', 'const re = /(?:a)/u;\n'],
  ['prefer-character-class', 'two alternatives below threshold', 'const re = /(?:a|b)/u;\n'],
  // sort-alternatives
  ['sort-alternatives', 'already sorted', 'const re = /(?:a|b|c)/u;\n'],
  ['sort-alternatives', 'multi-byte alt', 'const re = /(?:bc|a)/u;\n'],
  ['sort-alternatives', 'no alternation', 'const re = /(?:a)/u;\n'],
  // prefer-predefined-assertion
  ['prefer-predefined-assertion', 'lookaround with literal body', 'const re = /(?=a)/u;\n'],
  ['prefer-predefined-assertion', 'bare anchor', 'const re = /^abc$/u;\n'],
  // prefer-unicode-codepoint-escapes
  [
    'prefer-unicode-codepoint-escapes',
    'surrogate pair without u flag',
    "const re = new RegExp('\\\\uD83D\\\\uDE00');\n",
  ],
  // unicode-escape
  [
    'unicode-escape',
    'surrogate half \\uHHHH',
    "const re = new RegExp('\\\\ud83d\\\\ude00', 'u');\n",
  ],
];

const invalidCases = [
  // no-invalid-regexp
  ['no-invalid-regexp', 'unclosed character class', "new RegExp('[', 'u');\n", ['error']],
  ['no-invalid-regexp', 'unclosed group', "new RegExp('(?:', 'u');\n", ['error']],
  ['no-invalid-regexp', 'duplicate flags', "new RegExp('a', 'gg');\n", ['duplicateFlag']],
  ['no-invalid-regexp', 'duplicate i flags', "new RegExp('a', 'ii');\n", ['duplicateFlag']],
  ['no-invalid-regexp', 'u and v flags together', "new RegExp('a', 'uv');\n", ['uvFlag']],
  ['no-invalid-regexp', 'v and u flags together', "new RegExp('a', 'vu');\n", ['uvFlag']],
  // no-empty-character-class
  ['no-empty-character-class', 'standalone empty class', 'const re = /[]/u;\n', ['empty']],
  ['no-empty-character-class', 'empty class between chars', 'const re = /abc[]def/u;\n', ['empty']],
  ['no-invisible-character', 'zwsp in pattern', 'const re = /a\u200Bb/u;\n', ['unexpected']],
  // hexadecimal-escape
  [
    'hexadecimal-escape',
    'lowercase \\xHH',
    "const re = new RegExp('\\\\xab', 'u');\n",
    ['unexpected'],
  ],
  [
    'hexadecimal-escape',
    'uppercase \\xHH still flagged',
    "const re = new RegExp('\\\\xAB', 'u');\n",
    ['unexpected'],
  ],
  // unicode-escape
  [
    'unicode-escape',
    'fixed-width \\uHHHH',
    "const re = new RegExp('\\\\uabcd', 'u');\n",
    ['unexpected'],
  ],
  // no-useless-range
  ['no-useless-range', 'literal a-a range', 'const re = /[a-a]/u;\n', ['unexpected']],
  ['no-useless-range', 'literal 0-0 range', 'const re = /[0-0]/u;\n', ['unexpected']],
  // no-empty-lookarounds-assertion
  [
    'no-empty-lookarounds-assertion',
    'empty positive lookahead',
    'const re = /(?=)/u;\n',
    ['unexpected'],
  ],
  [
    'no-empty-lookarounds-assertion',
    'empty negative lookbehind',
    'const re = /(?<!)/u;\n',
    ['unexpected'],
  ],
  // prefer-regexp-exec
  ['prefer-regexp-exec', 'match with non-global literal', 'str.match(/foo/u);\n', ['unexpected']],
  ['prefer-regexp-exec', 'member-chained receiver', 'obj.prop.match(/bar/);\n', ['unexpected']],
  // no-missing-g-flag
  ['no-missing-g-flag', 'matchAll without g', 'str.matchAll(/foo/u);\n', ['unexpected']],
  [
    'no-missing-g-flag',
    'replaceAll regex without g',
    "str.replaceAll(/foo/, 'bar');\n",
    ['unexpected'],
  ],
  // no-useless-character-class
  ['no-useless-character-class', 'single literal class', 'const re = /[a]/u;\n', ['unexpected']],
  ['no-useless-character-class', 'single digit class', 'const re = /[5]/u;\n', ['unexpected']],
  // no-empty-string-literal
  ['no-empty-string-literal', 'empty v literal', 'const re = /[\\q{}]/v;\n', ['unexpected']],
  // no-optional-assertion
  [
    'no-optional-assertion',
    'optional positive lookahead',
    'const re = /(?=a)?/u;\n',
    ['unexpected'],
  ],
  ['no-optional-assertion', 'optional lookbehind', 'const re = /(?<=a)?/u;\n', ['unexpected']],
  // require-unicode-sets-regexp
  ['require-unicode-sets-regexp', 'u flag only', 'const re = /a/u;\n', ['require']],
  ['require-unicode-sets-regexp', 'no flags', 'const re = /a/;\n', ['require']],
  // confusing-quantifier
  ['confusing-quantifier', 'lazy star', 'const re = /a*?/u;\n', ['unexpected']],
  ['confusing-quantifier', 'lazy optional', 'const re = /a??/u;\n', ['unexpected']],
  ['confusing-quantifier', 'lazy zero-or-more brace', 'const re = /a{0,}?/u;\n', ['unexpected']],
  // prefer-named-replacement
  [
    'prefer-named-replacement',
    'numbered backref with named regex',
    "str.replace(/(?<year>\\d{4})/u, '$1');\n",
    ['unexpected'],
  ],
  [
    'prefer-named-replacement',
    'replaceAll variant',
    "str.replaceAll(/(?<year>\\d{4})/gu, 'year: $1');\n",
    ['unexpected'],
  ],
  // no-obscure-range
  ['no-obscure-range', 'A-z range', 'const re = /[A-z]/u;\n', ['unexpected']],
  ['no-obscure-range', '0-A range', 'const re = /[0-A]/u;\n', ['unexpected']],
  // prefer-unicode-codepoint-escapes
  [
    'prefer-unicode-codepoint-escapes',
    'surrogate pair',
    "const re = new RegExp('\\\\uD83D\\\\uDE00', 'u');\n",
    ['unexpected'],
  ],
  // no-dupe-characters-character-class
  [
    'no-dupe-characters-character-class',
    'literal duplicate',
    'const re = /[aab]/u;\n',
    ['unexpected'],
  ],
  // prefer-range
  ['prefer-range', 'four consecutive letters', 'const re = /[abcd]/u;\n', ['unexpected']],
  ['prefer-range', 'five consecutive digits', 'const re = /[12345]/u;\n', ['unexpected']],
  // no-useless-escape
  ['no-useless-escape', 'escaped colon', 'const re = /\\:/u;\n', ['unexpected']],
  ['no-useless-escape', 'escaped at sign', 'const re = /a\\@b/u;\n', ['unexpected']],
  // no-useless-quantifier
  ['no-useless-quantifier', 'a{1}', 'const re = /a{1}/u;\n', ['unexpected']],
  ['no-useless-quantifier', 'a{1,1}', 'const re = /a{1,1}/u;\n', ['unexpected']],
  // prefer-named-backreference
  [
    'prefer-named-backreference',
    'mixed numbered backref',
    'const re = /(?<year>\\d{4})-\\1/u;\n',
    ['unexpected'],
  ],
  // no-useless-flag
  ['no-useless-flag', 's without dot', "const re = new RegExp('abc', 's');\n", ['unexpected']],
  ['no-useless-flag', 'm without anchor', "const re = new RegExp('abc', 'm');\n", ['unexpected']],
  // no-lazy-ends
  ['no-lazy-ends', 'star lazy at end', 'const re = /a*?/u;\n', ['unexpected']],
  ['no-lazy-ends', 'plus lazy at end', 'const re = /a+?/u;\n', ['unexpected']],
  ['no-lazy-ends', 'braced lazy at end', 'const re = /a{2,}?/u;\n', ['unexpected']],
  // no-useless-dollar-replacements
  [
    'no-useless-dollar-replacements',
    'dollar zero three in two-group pattern',
    "str.replace(/(\\w+)\\s(\\w+)/u, '$03');\n",
    ['unexpected'],
  ],
  [
    'no-useless-dollar-replacements',
    'replaceAll variant dollar zero nine in eight-group pattern',
    '"abc".replaceAll(/()()(()())()()(.)/gu, \'$09\');\n',
    ['unexpected'],
  ],
  // prefer-escape-replacement-dollar-char
  [
    'prefer-escape-replacement-dollar-char',
    'dollar followed by space',
    "'str'.replace(/a/u, 'pre $ post');\n",
    ['unexpected'],
  ],
  [
    'prefer-escape-replacement-dollar-char',
    'trailing dollar',
    "'str'.replace(/a/u, 'price$');\n",
    ['unexpected'],
  ],
  // use-ignore-case
  ['use-ignore-case', 'lower and upper of a', 'const re = /[aA]/u;\n', ['unexpected']],
  ['use-ignore-case', 'multi case pair', 'const re = /[aAbB]/u;\n', ['unexpected']],
  // control-character-escape
  [
    'control-character-escape',
    'literal SOH',
    "const re = new RegExp('\\x01', 'u');\n",
    ['unexpected'],
  ],
  // grapheme-string-literal
  [
    'grapheme-string-literal',
    'single-char string literal',
    'const re = /[\\q{a}]/v;\n',
    ['unexpected'],
  ],
  // no-useless-non-capturing-group
  ['no-useless-non-capturing-group', 'single-char body', 'const re = /(?:a)/u;\n', ['unexpected']],
  [
    'no-useless-non-capturing-group',
    'inline context',
    'const re = /pre(?:b)post/u;\n',
    ['unexpected'],
  ],
  // prefer-quantifier
  ['prefer-quantifier', 'braced quantifier', 'const re = /(?:a){3}/u;\n', ['unexpected']],
  ['prefer-quantifier', 'plus quantifier', 'const re = /(?:a)+/u;\n', ['unexpected']],
  // no-useless-string-literal
  ['no-useless-string-literal', 'single-char body', 'const re = /[\\q{a}]/v;\n', ['unexpected']],
  // sort-character-class-elements
  ['sort-character-class-elements', 'reversed letters', 'const re = /[ba]/u;\n', ['unexpected']],
  [
    'sort-character-class-elements',
    'mixed digits and letters',
    'const re = /[b1a]/u;\n',
    ['unexpected'],
  ],
  // no-trivially-nested-assertion
  [
    'no-trivially-nested-assertion',
    'non-cap wrapping lookahead',
    'const re = /(?:(?=a))/u;\n',
    ['unexpected'],
  ],
  [
    'no-trivially-nested-assertion',
    'non-cap wrapping lookbehind',
    'const re = /(?:(?<=a))/u;\n',
    ['unexpected'],
  ],
  // no-extra-lookaround-assertions
  [
    'no-extra-lookaround-assertions',
    'nested positive lookahead',
    'const re = /(?=(?=a))/u;\n',
    ['unexpected'],
  ],
  [
    'no-extra-lookaround-assertions',
    'nested negative lookbehind',
    'const re = /(?<!(?!b))/u;\n',
    ['unexpected'],
  ],
  // no-trivially-nested-quantifier
  [
    'no-trivially-nested-quantifier',
    'plus inside plus',
    'const re = /(?:a+)+/u;\n',
    ['unexpected'],
  ],
  [
    'no-trivially-nested-quantifier',
    'star inside star',
    'const re = /(?:b*)*/u;\n',
    ['unexpected'],
  ],
  // prefer-character-class
  [
    'prefer-character-class',
    'mixed letters and digits',
    'const re = /(?:a|1|b)/u;\n',
    ['unexpected'],
  ],
  // sort-alternatives
  ['sort-alternatives', 'two-letter unsorted', 'const re = /(?:b|a)/u;\n', ['unexpected']],
  ['sort-alternatives', 'three-letter unsorted', 'const re = /(?:c|a|b)/u;\n', ['unexpected']],
  // prefer-predefined-assertion
  ['prefer-predefined-assertion', 'lookahead end anchor', 'const re = /(?=$)/u;\n', ['unexpected']],
  [
    'prefer-predefined-assertion',
    'lookbehind start anchor',
    'const re = /(?<=^)/u;\n',
    ['unexpected'],
  ],
];

function runRule(ruleName, sourceText, filename = 'fixture.js') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const rule = plugin.rules[ruleName];
  const visitor = rule.createOnce({
    filename,
    options: [],
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function renderMessage(ruleName, report) {
  const template = plugin.rules[ruleName].meta.messages[report.messageId];
  return template.replace(/\{\{\s*(\w+)\s*\}\}/g, (_, key) => report.data?.[key] ?? '');
}

function findOxlintCli() {
  const store = join(workspaceRoot, 'node_modules/.pnpm');
  const candidates = readdirSync(store)
    .filter((entry) => entry.startsWith('oxlint@'))
    .map((entry) => join(store, entry, 'node_modules/oxlint/bin/oxlint'))
    .filter((candidate) => existsSync(candidate))
    .sort((a, b) => a.localeCompare(b));

  if (candidates.length === 0) {
    throw new Error('Could not find oxlint CLI in node_modules/.pnpm.');
  }

  return candidates[candidates.length - 1];
}

function runOxlint(ruleName, code) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'regexp-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.js');
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'regexp',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`regexp/${ruleName}`]: 'error',
        },
      }),
    );

    const result = spawnSync(
      oxlint,
      ['-c', configPath, '--quiet', '--format', 'json', sourcePath],
      {
        encoding: 'utf8',
      },
    );
    const payload = result.stdout.trim() === '' ? { diagnostics: [] } : JSON.parse(result.stdout);

    return {
      diagnostics: payload.diagnostics ?? [],
      status: result.status,
      stderr: result.stderr,
    };
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

describe('regexp plugin shape', () => {
  it('exports the regexp plugin surface', () => {
    expect(plugin.meta?.name).toBe('regexp');
    expect(plugin.implementedRegexpRuleNames).toEqual(Object.keys(plugin.rules));
    expect(plugin.rules['sort-flags'].meta.messages.sortFlags).toContain('{{sortedFlags}}');
  });

  it('ships upstream-compatible implemented configs', () => {
    expect(plugin.configs.recommended.rules['regexp/no-empty-alternative']).toBe('warn');
    expect(plugin.configs.recommended.rules['regexp/no-octal']).toBeUndefined();
    expect(plugin.configs.all.rules['regexp/no-octal']).toBe('error');
    expect(plugin.configs['flat/recommended']).toBe(plugin.configs.recommended);
  });
});

describe('regexp rules through direct Oxlint plugin adapter', () => {
  it.each(validCases)('accepts %s: %s', (ruleName, _name, code) => {
    expect(runRule(ruleName, code)).toEqual([]);
  });

  it.each(invalidCases)('reports %s: %s', (ruleName, _name, code, expectedMessageIds) => {
    const reports = runRule(ruleName, code);

    expect(reports.map((report) => report.messageId)).toEqual(expectedMessageIds);
  });

  it('renders data-bearing upstream messages', () => {
    expect(renderMessage('sort-flags', runRule('sort-flags', 'const re = /a/mi;\n')[0])).toBe(
      "The flags 'mi' should be in the order 'im'.",
    );
    expect(renderMessage('no-octal', runRule('no-octal', 'const re = /\\07/u;\n')[0])).toBe(
      "Unexpected octal escape sequence '\\07'.",
    );
    expect(
      renderMessage(
        'no-control-character',
        runRule('no-control-character', "const re = new RegExp('\\\\x01', 'u');\n")[0],
      ),
    ).toBe('Unexpected control character U+0001.');
    expect(
      renderMessage(
        'no-invalid-regexp',
        runRule('no-invalid-regexp', "new RegExp('a', 'gg');\n")[0],
      ),
    ).toBe('Duplicate g flag.');
    expect(
      renderMessage(
        'prefer-plus-quantifier',
        runRule('prefer-plus-quantifier', 'const re = /a{1,}/u;\n')[0],
      ),
    ).toBe("Unexpected quantifier '{1,}'. Use '+' instead.");
    expect(
      renderMessage(
        'prefer-star-quantifier',
        runRule('prefer-star-quantifier', 'const re = /a{0,}/u;\n')[0],
      ),
    ).toBe("Unexpected quantifier '{0,}'. Use '*' instead.");
    expect(
      renderMessage(
        'prefer-question-quantifier',
        runRule('prefer-question-quantifier', 'const re = /a{0,1}/u;\n')[0],
      ),
    ).toBe("Unexpected quantifier '{0,1}'. Use '?' instead.");
    expect(
      renderMessage(
        'no-useless-two-nums-quantifier',
        runRule('no-useless-two-nums-quantifier', 'const re = /a{3,3}/u;\n')[0],
      ),
    ).toBe("Unexpected quantifier '{3,3}'. Use '{3}' instead.");
    expect(
      renderMessage(
        'prefer-named-capture-group',
        runRule('prefer-named-capture-group', 'const re = /(a)/u;\n')[0],
      ),
    ).toBe('Capturing group should be converted to a named or non-capturing group.');
    expect(renderMessage('match-any', runRule('match-any', 'const re = /[\\S\\s]/u;\n')[0])).toBe(
      'Unexpected any character class. Use `.` with the `s` flag instead.',
    );
    expect(
      renderMessage('no-legacy-features', runRule('no-legacy-features', 'RegExp.$1;\n')[0]),
    ).toBe(
      "Unexpected use of the legacy 'RegExp.$1' static property; it is non-standard and not safe to rely on.",
    );
  });

  it('ignores non-RegExp callees with the same shape', () => {
    expect(runRule('no-empty-character-class', "new Foo('[]', 'u');\n")).toEqual([]);
    expect(runRule('no-invalid-regexp', "Bar('[', 'u');\n")).toEqual([]);
  });

  it('does not crash when constructor arguments are non-literal', () => {
    expect(runRule('no-empty-character-class', "new RegExp(pattern, 'u');\n")).toEqual([]);
    expect(runRule('no-invalid-regexp', 'new RegExp();\n')).toEqual([]);
  });

  it('reports each literal in a source independently', () => {
    const reports = runRule('no-empty-character-class', 'const a = /[]/u; const b = /[]/u;\n');
    expect(reports).toHaveLength(2);
    expect(reports.every((report) => report.messageId === 'empty')).toBe(true);
  });
});

describe('regexp rules through oxlint jsPlugins', () => {
  it('reports a native diagnostic through the CLI', () => {
    const result = runOxlint('sort-flags', 'const re = /a/mi;\n');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toMatchObject([
      {
        code: 'regexp(sort-flags)',
        message: "The flags 'mi' should be in the order 'im'.",
      },
    ]);
  });
});
