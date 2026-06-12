import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const expectedRuleNames = ['exports', 'imports'];

const invalidCases = [
  [
    'imports',
    'sorts import chunks and specifiers',
    [
      "import z from 'z';",
      "import { beta, alpha as renamed } from 'pkg';",
      "import fs from 'node:fs';",
      "import './setup';",
      "import local from './local';",
    ].join('\n'),
    [
      "import './setup';",
      '',
      "import fs from 'node:fs';",
      '',
      "import { alpha as renamed, beta } from 'pkg';",
      "import z from 'z';",
      '',
      "import local from './local';",
    ].join('\n'),
  ],
  [
    'exports',
    'sorts export chunks',
    ["export { zed } from 'z';", "export * from 'a';"].join('\n'),
    ["export * from 'a';", "export { zed } from 'z';"].join('\n'),
  ],
  [
    'exports',
    'sorts local export specifiers',
    'const d = 1, a = 1, b = 1;\nexport { d, a as c, b };',
    'const d = 1, a = 1, b = 1;\nexport { b, a as c, d };',
  ],
];

function runRule(ruleName, sourceText, options = [], filename = 'fixture.js') {
  const reports = [];
  const sourceCode = {
    text: sourceText,
    getText() {
      return this.text;
    },
  };
  const visitor = plugin.rules[ruleName].createOnce({
    filename,
    options,
    sourceCode,
    report(descriptor) {
      reports.push(descriptor);
    },
  });

  visitor.Program({ type: 'Program', range: [0, sourceText.length] });
  return reports;
}

function applyFix(sourceText, report) {
  const fix = report.fix({
    replaceTextRange(range, replacement) {
      return { range, text: replacement };
    },
  });
  return sourceText.slice(0, fix.range[0]) + fix.text + sourceText.slice(fix.range[1]);
}

function renderMessage(ruleName, report) {
  return plugin.rules[ruleName].meta.messages[report.messageId];
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

function runOxlint(ruleName, code, options) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'simple-import-sort-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.js');
    const configPath = join(temp, 'oxlint.config.jsonc');
    const ruleConfig = options == null ? 'error' : ['error', options];

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'simple-import-sort',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`simple-import-sort/${ruleName}`]: ruleConfig,
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

describe('simple-import-sort plugin shape', () => {
  it('exposes all ported rules', () => {
    expect(plugin.meta?.name).toBe('simple-import-sort');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedSimpleImportSortRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanSimpleImportSort).toBe('function');
  });

  it('ships recommended config', () => {
    expect(plugin.configs.recommended.rules).toEqual({
      'simple-import-sort/imports': 'error',
      'simple-import-sort/exports': 'error',
    });
  });
});

describe('simple-import-sort rules through direct adapter harness', () => {
  it.each(invalidCases)('reports %s: %s', (ruleName, _name, code, fixed) => {
    const reports = runRule(ruleName, code);

    expect(reports.map((report) => renderMessage(ruleName, report))).toEqual([
      plugin.rules[ruleName].meta.messages.sort,
    ]);
    expect(applyFix(code, reports[0])).toBe(fixed);
  });

  it('honors selected import groups', () => {
    const code = ["import rel from './rel';", "import pkg from 'pkg';"].join('\n');
    const reports = runRule('imports', code, [{ groups: [['^\\.'], ['^@?\\w']] }]);

    expect(reports).toHaveLength(1);
    expect(applyFix(code, reports[0])).toBe(
      ["import rel from './rel';", '', "import pkg from 'pkg';"].join('\n'),
    );
  });
});

describe('simple-import-sort rules through oxlint jsPlugins', () => {
  it.each(invalidCases)('reports %s through the CLI: %s', (ruleName, _name, code) => {
    const actualRuleName = /** @type {string} */ (ruleName);
    const result = runOxlint(actualRuleName, code);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe(`simple-import-sort(${actualRuleName})`);
  });
});
