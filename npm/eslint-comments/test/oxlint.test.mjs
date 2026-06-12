import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = resolve(packageRoot, '../..');

const invalidCases = [
  ['no-unlimited-disable', '// eslint-disable-next-line\nalert(x);\n', 'Unexpected unlimited'],
  ['no-use', '// eslint-disable-next-line no-alert\nalert(x);\n', 'Unexpected ESLint directive'],
  [
    'require-description',
    '// eslint-disable-next-line no-alert\nalert(x);\n',
    'Unexpected undescribed directive',
  ],
  [
    'disable-enable-pair',
    '/* eslint-disable no-alert */\nalert(x);\n',
    "Requires 'eslint-enable' directive",
  ],
  [
    'no-aggregating-enable',
    '/* eslint-disable no-alert */\n/* eslint-disable no-console */\n/* eslint-enable */\n',
    'affects 2 `eslint-disable` comments',
  ],
  [
    'no-duplicate-disable',
    '/* eslint-disable no-alert */\n/* eslint-disable no-alert */\n',
    'has been disabled already',
  ],
  [
    'no-unused-enable',
    '/* eslint-enable no-alert */\n',
    'is re-enabled but it has not been disabled',
  ],
  [
    'no-restricted-disable',
    '// eslint-disable-next-line no-alert\nalert(x);\n',
    "Disabling 'no-alert' is not allowed",
    ['error', 'no-alert'],
  ],
  [
    'no-unused-disable',
    '// eslint-disable-next-line no-alert\nconst x = 1;\n',
    'Unused eslint-disable directive',
  ],
];

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

function runOxlint(ruleName, code, ruleConfig = 'error') {
  const oxlint = findOxlintCli();
  const temp = mkdtempSync(join(tmpdir(), 'eslint-comments-plugin-'));

  try {
    const sourcePath = join(temp, 'fixture.js');
    const configPath = join(temp, 'oxlint.config.jsonc');

    writeFileSync(sourcePath, code);
    writeFileSync(
      configPath,
      JSON.stringify({
        jsPlugins: [
          {
            name: 'eslint-comments',
            specifier: join(packageRoot, 'index.js'),
          },
        ],
        rules: {
          [`eslint-comments/${ruleName}`]: ruleConfig,
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

describe('eslint-comments rules through oxlint jsPlugins', () => {
  it.each(invalidCases)('reports %s through the CLI', (ruleName, code, message, ruleConfig) => {
    const result = runOxlint(ruleName, code, ruleConfig);

    expect(result.status).toBe(1);
    expect(result.stderr).toBe('');
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0].code).toBe(`eslint-comments(${String(ruleName)})`);
    expect(result.diagnostics[0].message).toContain(message);
  });
});
