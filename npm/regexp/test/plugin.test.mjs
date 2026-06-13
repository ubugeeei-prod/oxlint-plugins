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
  ['no-invalid-regexp', 'valid constructor', "new RegExp('a+', 'u');\n"],
  ['no-empty-character-class', 'non-empty class', 'const re = /[a]/u;\n'],
  ['no-empty-group', 'non-empty group', 'const re = /(?:a)/u;\n'],
  ['no-empty-capturing-group', 'non-empty capture', 'const re = /(a)/u;\n'],
  ['no-empty-alternative', 'no empty alternative', 'const re = /a|b/u;\n'],
  ['no-zero-quantifier', 'positive quantifier', 'const re = /a{1}/u;\n'],
  ['no-octal', 'nul escape only', 'const re = /\\0/u;\n'],
  ['no-control-character', 'named control escape', 'const re = /\\t/u;\n'],
  ['sort-flags', 'sorted flags', 'const re = /a/im;\n'],
  ['require-unicode-regexp', 'unicode flag', 'const re = /a/u;\n'],
];

const invalidCases = [
  ['no-invalid-regexp', 'invalid constructor pattern', "new RegExp('[', 'u');\n", ['error']],
  ['no-invalid-regexp', 'duplicate flags', "new RegExp('a', 'gg');\n", ['duplicateFlag']],
  ['no-invalid-regexp', 'u and v flags', "new RegExp('a', 'uv');\n", ['uvFlag']],
  ['no-empty-character-class', 'empty character class', 'const re = /[]/u;\n', ['empty']],
  ['no-empty-group', 'empty group', 'const re = /(?:)/u;\n', ['unexpected']],
  ['no-empty-capturing-group', 'empty capturing group', 'const re = /()/u;\n', ['unexpected']],
  ['no-empty-alternative', 'trailing empty alternative', 'const re = /a|/u;\n', ['empty']],
  ['no-zero-quantifier', 'zero quantifier', 'const re = /a{0}/u;\n', ['unexpected']],
  ['no-octal', 'octal escape', 'const re = /\\07/u;\n', ['unexpected']],
  [
    'no-control-character',
    'hex escaped control character',
    "const re = new RegExp('\\\\x01', 'u');\n",
    ['unexpected'],
  ],
  ['sort-flags', 'unsorted flags', 'const re = /a/mi;\n', ['sortFlags']],
  ['require-unicode-regexp', 'missing unicode flag', 'const re = /a/;\n', ['require']],
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
