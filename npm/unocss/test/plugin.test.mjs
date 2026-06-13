import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const expectedRuleNames = ['blocklist', 'enforce-class-compile', 'order', 'order-attributify'];

function runRule(ruleName, sourceText, options = [], settings = {}, filename = 'fixture.tsx') {
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
    options,
    settings,
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
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => report.data?.[key] ?? '');
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

  return candidates[0];
}

describe('unocss plugin adapter', () => {
  it('exposes rules and configs in @unocss shape', () => {
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.configs.recommended.rules).toEqual({
      '@unocss/order': 'warn',
      '@unocss/order-attributify': 'warn',
    });
    expect(plugin.configs.all.rules).toEqual(
      Object.fromEntries(expectedRuleNames.map((ruleName) => [`@unocss/${ruleName}`, 'warn'])),
    );
    expect(plugin.configs.off.rules).toEqual(
      Object.fromEntries(expectedRuleNames.map((ruleName) => [`@unocss/${ruleName}`, 'off'])),
    );
  });

  it('reports each focused rule through direct createOnce calls', () => {
    const cases = [
      [
        'blocklist',
        '<div className="border"></div>;',
        [],
        { unocss: { blocklist: [['border', { message: 'Use border-1 instead' }]] } },
        '"border" is in blocklist: Use border-1 instead',
      ],
      [
        'enforce-class-compile',
        '<div className="mr-1"></div>;',
        [],
        {},
        'prefix: `:uno:` is missing',
      ],
      [
        'order',
        'const classNames = { base: "mr-1 ml-1" };',
        [],
        {},
        'UnoCSS utilities are not ordered',
      ],
      [
        'order-attributify',
        'const node = <div p4 flex />;',
        [],
        {},
        'UnoCSS attributes are not ordered',
      ],
    ];

    for (const [ruleName, code, options, settings, message] of cases) {
      const reports = runRule(ruleName, code, options, settings);
      expect(reports, ruleName).toHaveLength(1);
      expect(renderMessage(ruleName, reports[0])).toBe(message);
    }
  });

  it('applies fixes with UTF-16 offsets through the JS adapter', () => {
    const code = '<div className="é mx1 m1"></div>;';
    const reports = runRule('enforce-class-compile', '<div className="é mx1 m1"></div>;');
    const report = reports[0];
    const fixes = [];

    report.fix({
      replaceTextRange(range, replacement) {
        fixes.push({ range, replacement });
      },
    });

    expect(fixes).toEqual([
      {
        range: [16, 24],
        replacement: ':uno: é mx1 m1',
      },
    ]);
    expect(code.slice(0, fixes[0].range[0])).toBe('<div className="');
  });

  it('passes rule options to the native scanner', () => {
    const reports = runRule('order', 'superclass("pr1 pl1");', [{ unoFunctions: ['superclass'] }]);

    expect(reports).toHaveLength(1);
  });

  it('loads through oxlint jsPlugins', () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'oxlint-unocss-'));
    try {
      writeFileSync(join(tempDir, 'fixture.tsx'), '<div className="mx1 m1"></div>;\n');
      writeFileSync(
        join(tempDir, 'oxlint.config.jsonc'),
        JSON.stringify({
          jsPlugins: [
            {
              name: '@unocss',
              specifier: join(packageRoot, 'index.js'),
            },
          ],
          rules: {
            '@unocss/order': 'error',
          },
        }),
      );

      const result = spawnSync(
        findOxlintCli(),
        ['--config', 'oxlint.config.jsonc', '--quiet', '--format', 'json', 'fixture.tsx'],
        {
          cwd: tempDir,
          encoding: 'utf8',
        },
      );
      const payload = JSON.parse(result.stdout);

      expect(result.status).toBe(1);
      expect(result.stderr).toBe('');
      expect(payload.diagnostics).toHaveLength(1);
      expect(payload.diagnostics[0].message).toBe('UnoCSS utilities are not ordered');
    } finally {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
