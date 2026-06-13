import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import plugin from '../index.js';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');
const expectedRuleNames = plugin.implementedFunctionalRuleNames;

function runRule(ruleName, sourceText, options = [], filename = 'fixture.ts') {
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

function runOxlint(ruleName, code, options) {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'functional-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.ts');
    const configPath = join(temp, 'oxlint.config.jsonc');
    const ruleConfig = options == null ? 'error' : ['error', options];

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'functional',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`functional/${ruleName}`]: ruleConfig,
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

describe('functional plugin shape', () => {
  it('exposes all ported rules', () => {
    expect(plugin.meta?.name).toBe('functional');
    expect(Object.keys(plugin.rules)).toEqual(expectedRuleNames);
    expect(expectedRuleNames).toHaveLength(20);
    expect(typeof plugin.scanFunctional).toBe('function');
  });

  it('ships configs', () => {
    expect(plugin.configs.recommended.rules['functional/no-let']).toEqual([
      'error',
      { allowInForLoopInit: true },
    ]);
    expect(plugin.configs.all.rules['functional/no-classes']).toBe('error');
    expect(plugin.configs.off.rules['functional/no-classes']).toBe('off');
  });
});

describe('functional rules through direct adapter harness', () => {
  it('reports a selected rule only', () => {
    const reports = runRule('no-let', 'let value = 1;');

    expect(reports).toHaveLength(1);
    expect(reports[0].data.message).toBe('Unexpected let, use const instead.');
  });

  it('passes rule options to Rust', () => {
    expect(
      runRule('no-let', 'for (let i = 0; i < 1; i++) {}', [{ allowInForLoopInit: true }]),
    ).toEqual([]);
    expect(
      runRule('functional-parameters', 'function f(...items) { return arguments.length; }', [
        { allowRestParameter: true, allowArgumentsKeyword: true },
      ]),
    ).toEqual([]);
  });

  it('reports type rules', () => {
    const reports = runRule(
      'prefer-readonly-type',
      'interface Values { items: string[]; readonly cached: Array<string>; }',
    );

    expect(reports.length).toBeGreaterThanOrEqual(2);
  });
});

describe('functional rules through oxlint jsPlugins', () => {
  it('reports through the CLI', () => {
    const result = runOxlint('no-let', 'let value = 1;');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe('functional(no-let)');
  });

  it('reports TypeScript type rules through the CLI', () => {
    const result = runOxlint('prefer-readonly-type', 'interface Values { items: string[]; }');

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics.map((diagnostic) => diagnostic.code)).toContain(
      'functional(prefer-readonly-type)',
    );
  });
});
