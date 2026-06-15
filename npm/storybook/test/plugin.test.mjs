import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const expectedRuleNames = [
  'await-interactions',
  'context-in-play-function',
  'csf-component',
  'default-exports',
  'hierarchy-separator',
  'meta-inline-properties',
  'meta-satisfies-type',
  'no-redundant-story-name',
  'no-renderer-packages',
  'no-stories-of',
  'no-title-property-in-meta',
  'no-uninstalled-addons',
  'prefer-pascal-case',
  'story-exports',
  'use-storybook-expect',
  'use-storybook-testing-library',
];

function runRule(ruleName, sourceText, { filename = 'Button.stories.tsx', options = [] } = {}) {
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

function applyReportFix(sourceText, report) {
  const rawFixes = report.fix({
    replaceTextRange(range, replacement) {
      return { range, text: replacement };
    },
  });
  const fixes = Array.isArray(rawFixes) ? rawFixes : [rawFixes];
  return fixes
    .sort((a, b) => b.range[0] - a.range[0])
    .reduce(
      (text, fix) => text.slice(0, fix.range[0]) + fix.text + text.slice(fix.range[1]),
      sourceText,
    );
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

function runOxlint(ruleName, code, filename = 'Button.stories.tsx') {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'storybook-plugin-'));

  try {
    const sourcePath = join(temp, filename);
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'storybook',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`storybook/${ruleName}`]: 'error',
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

describe('storybook plugin shape', () => {
  it('exposes all ported rules and configs', () => {
    expect(plugin.meta?.name).toBe('storybook');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(plugin.implementedStorybookRuleNames).toEqual(expectedRuleNames);
    expect(typeof plugin.scanStorybook).toBe('function');
    expect(Object.keys(plugin.configs)).toEqual([
      'csf',
      'csf-strict',
      'addon-interactions',
      'recommended',
      'flat/csf',
      'flat/csf-strict',
      'flat/addon-interactions',
      'flat/recommended',
    ]);
  });

  it('ships recommended story and main rules', () => {
    expect(plugin.configs.recommended.overrides[0].rules['storybook/await-interactions']).toBe(
      'error',
    );
    expect(plugin.configs.recommended.overrides[1].rules).toEqual({
      'storybook/no-uninstalled-addons': 'error',
    });
  });
});

describe('storybook rules through direct adapter harness', () => {
  it('reports and fixes await-interactions', () => {
    const code = 'Basic.play = async () => { userEvent.click(button) }';
    const reports = runRule('await-interactions', code);

    expect(reports).toHaveLength(1);
    expect(reports[0].messageId).toBe('interactionShouldBeAwaited');
    expect(reports[0].data.method).toBe('userEvent');
    expect(applyReportFix(code, reports[0])).toBe(
      'Basic.play = async () => { await userEvent.click(button) }',
    );
  });

  it('reports use-storybook-testing-library with multi-range fixes', () => {
    const code = "import userEvent, { within } from '@testing-library/user-event'";
    const reports = runRule('use-storybook-testing-library', code);

    expect(reports).toHaveLength(1);
    expect(reports[0].data.library).toBe('@testing-library/user-event');
    expect(applyReportFix(code, reports[0])).toBe(
      "import { userEvent, within } from 'storybook/test'",
    );
  });

  it('reads package.json for no-uninstalled-addons options', () => {
    const temp = mkdtempSync(join(tmpdir(), 'storybook-package-json-'));
    const packageJsonPath = join(temp, 'package.json');
    writeFileSync(
      packageJsonPath,
      JSON.stringify({ devDependencies: { '@storybook/addon-essentials': 'latest' } }),
    );

    try {
      const reports = runRule(
        'no-uninstalled-addons',
        "export default { addons: ['@storybook/addon-essentials', '@storybook/not-installed'] }",
        { options: [{ packageJsonLocation: packageJsonPath }] },
      );

      expect(reports).toHaveLength(1);
      expect(reports[0].data.addonName).toBe('@storybook/not-installed');
      expect(reports[0].data.packageJsonPath).toBe(packageJsonPath);
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
});

describe('storybook rules through oxlint jsPlugins', () => {
  it('reports no-renderer-packages through the CLI', () => {
    const result = runOxlint('no-renderer-packages', "import { Meta } from '@storybook/react'");

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('storybook(no-renderer-packages)');
  });
});
