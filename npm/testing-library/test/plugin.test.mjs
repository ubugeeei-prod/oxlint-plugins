import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const expectedRuleNames = plugin.implementedTestingLibraryRuleNames;

function runRule(ruleName, sourceText, { filename = 'fixture.test.tsx', options = [] } = {}) {
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
  const temp = mkdtempSync(join(tmpdir(), 'testing-library-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.test.tsx');
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'testing-library',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`testing-library/${ruleName}`]: 'error',
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

describe('testing-library plugin shape', () => {
  it('exposes all ported rules', () => {
    expect(plugin.meta?.name).toBe('testing-library');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(expectedRuleNames).toHaveLength(29);
    expect(typeof plugin.scanTestingLibrary).toBe('function');
  });

  it('ships configs', () => {
    expect(plugin.configs.recommended.rules['testing-library/no-container']).toBe('error');
    expect(plugin.configs.all.rules['testing-library/no-container']).toBe('error');
    expect(plugin.configs.off.rules['testing-library/no-container']).toBe('off');
  });
});

describe('testing-library rules through direct adapter harness', () => {
  it('reports a selected rule only', () => {
    const reports = runRule('prefer-user-event', 'fireEvent.click(button);');

    expect(reports).toHaveLength(1);
    expect(reports[0].data.message).toBe('Prefer userEvent over fireEvent.');
  });

  it('forwards consistent-data-testid options to the Rust scanner', () => {
    const reports = runRule('consistent-data-testid', 'render(<button data-testid="BadId" />);', {
      options: [{ testIdPattern: '^[a-z-]+$' }],
    });

    expect(reports).toHaveLength(1);
    expect(reports[0].data.message).toContain('data-testid');
    expect(reports[0].data.message).toContain('/^[a-z-]+$/');
  });

  it('treats consistent-data-testid as a no-op without a configured pattern', () => {
    const reports = runRule('consistent-data-testid', 'render(<button data-testid="BadId" />);');

    expect(reports).toHaveLength(0);
  });

  it('honors a custom consistent-data-testid message', () => {
    const reports = runRule('consistent-data-testid', 'render(<button data-testid="BadId" />);', {
      options: [{ testIdPattern: '^[a-z-]+$', customMessage: 'use kebab-case' }],
    });

    expect(reports).toHaveLength(1);
    expect(reports[0].data.message).toBe('use kebab-case');
  });
});

describe('testing-library rules through oxlint jsPlugins', () => {
  it('reports through the CLI', () => {
    const result = runOxlint('prefer-user-event', 'fireEvent.click(button);');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('testing-library(prefer-user-event)');
  });
});
