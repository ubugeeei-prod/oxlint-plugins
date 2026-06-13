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
  [
    'no-empty-character-class',
    'empty class via constructor',
    "const re = new RegExp('[]', 'u');\n",
    ['empty'],
  ],
  // no-empty-group
  ['no-empty-group', 'empty non-capturing group', 'const re = /(?:)/u;\n', ['unexpected']],
  ['no-empty-group', 'empty group between chars', 'const re = /a(?:)b/u;\n', ['unexpected']],
  // no-empty-capturing-group
  ['no-empty-capturing-group', 'empty capture', 'const re = /()/u;\n', ['unexpected']],
  ['no-empty-capturing-group', 'empty named capture', 'const re = /(?<name>)/u;\n', ['unexpected']],
  // no-empty-alternative
  ['no-empty-alternative', 'trailing empty alternative', 'const re = /a|/u;\n', ['empty']],
  ['no-empty-alternative', 'leading empty alternative', 'const re = /|a/u;\n', ['empty']],
  ['no-empty-alternative', 'middle empty alternative', 'const re = /a||b/u;\n', ['empty']],
  ['no-empty-alternative', 'empty alternative in group', 'const re = /(?:a|)/u;\n', ['empty']],
  // no-zero-quantifier
  ['no-zero-quantifier', 'zero quantifier', 'const re = /a{0}/u;\n', ['unexpected']],
  ['no-zero-quantifier', 'zero,zero quantifier', 'const re = /a{0,0}/u;\n', ['unexpected']],
  ['no-zero-quantifier', 'zero quantifier on group', 'const re = /(?:abc){0}/u;\n', ['unexpected']],
  // no-octal
  ['no-octal', 'two-digit octal escape', 'const re = /\\07/u;\n', ['unexpected']],
  ['no-octal', 'three-digit octal escape', 'const re = /\\012/u;\n', ['unexpected']],
  // no-control-character
  [
    'no-control-character',
    'hex escaped control character',
    "const re = new RegExp('\\\\x01', 'u');\n",
    ['unexpected'],
  ],
  [
    'no-control-character',
    'unicode escaped control character',
    "const re = new RegExp('\\\\u0002', 'u');\n",
    ['unexpected'],
  ],
  [
    'no-control-character',
    'curly unicode control character',
    "const re = new RegExp('\\\\u{3}', 'u');\n",
    ['unexpected'],
  ],
  // sort-flags
  ['sort-flags', 'unsorted flags', 'const re = /a/mi;\n', ['sortFlags']],
  ['sort-flags', 'unsorted unicode flag', 'const re = /a/ug;\n', ['sortFlags']],
  // require-unicode-regexp
  ['require-unicode-regexp', 'no flags', 'const re = /a/;\n', ['require']],
  ['require-unicode-regexp', 'only g flag', 'const re = /a/g;\n', ['require']],
  [
    'require-unicode-regexp',
    'constructor without flags',
    "const re = new RegExp('a');\n",
    ['require'],
  ],
  // no-escape-backspace
  ['no-escape-backspace', 'backspace alone in class', 'const re = /[\\b]/u;\n', ['unexpected']],
  [
    'no-escape-backspace',
    'backspace mixed with other class elements',
    'const re = /[a\\b]/u;\n',
    ['unexpected'],
  ],
  // prefer-plus-quantifier
  [
    'prefer-plus-quantifier',
    'one-or-more braced quantifier',
    'const re = /a{1,}/u;\n',
    ['unexpected'],
  ],
  // prefer-star-quantifier
  [
    'prefer-star-quantifier',
    'zero-or-more braced quantifier',
    'const re = /a{0,}/u;\n',
    ['unexpected'],
  ],
  // prefer-question-quantifier
  [
    'prefer-question-quantifier',
    'zero-or-one braced quantifier',
    'const re = /a{0,1}/u;\n',
    ['unexpected'],
  ],
  // no-useless-two-nums-quantifier
  [
    'no-useless-two-nums-quantifier',
    'equal-bounds quantifier',
    'const re = /a{3,3}/u;\n',
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
